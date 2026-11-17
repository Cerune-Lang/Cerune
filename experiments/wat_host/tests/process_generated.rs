#![cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
#[path = "../src/process.rs"]
mod host;
use cerune_lang::{self as language, codegen::wat, ir_executor, mir, mir_executor, ssa_executor};
use std::sync::{Arc, Mutex};
use wasmi::{Engine, Linker, Module, Store};
const EXAMPLE: &str = include_str!("../../../examples/process_execution/registered.ceru");
fn target() -> u64 {
    if cfg!(windows) { 1 } else { 2 }
}
fn hir(source: &str) -> language::ir::Program {
    language::compile_to_ir(source).unwrap()
}
fn output_imports(
    linker: &mut Linker<host::Process>,
    stdout: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
) {
    linker
        .func_wrap("cerune", "write_stdout_v1", move |bits: i64, n: i32| {
            assert!((1..=8).contains(&n));
            stdout
                .lock()
                .unwrap()
                .extend_from_slice(&bits.to_le_bytes()[..n as usize]);
            n
        })
        .unwrap();
    linker
        .func_wrap("cerune", "flush_stdout_v1", || 0i32)
        .unwrap();
    linker
        .func_wrap("cerune", "write_error_byte", move |b: i32| {
            stderr.lock().unwrap().push(b as u8);
        })
        .unwrap();
}
fn run(p: &language::ir::Program, config: &[u8]) -> String {
    execute(wat::emit_wat(p).unwrap(), config)
}
fn execute(text: String, config: &[u8]) -> String {
    assert!(text.contains("  )\n  (export \"main\""));
    let text = text.replace("  )\n  (export \"main\"", "    (if (i64.ne (global.get $cerune_array_live) (i64.const 0)) (then unreachable))\n  )\n  (export \"main\"");
    assert!(!text.contains("(export \"memory\""));
    let engine = Engine::default();
    let module = Module::new(&engine, &text).unwrap();
    assert_eq!(
        module
            .exports()
            .map(|x| x.name().to_owned())
            .collect::<Vec<_>>(),
        ["main"]
    );
    let stdout = Arc::new(Mutex::new(vec![]));
    let stderr = Arc::new(Mutex::new(vec![]));
    let mut linker = Linker::new(&engine);
    host::imports(&mut linker).unwrap();
    output_imports(&mut linker, stdout.clone(), stderr.clone());
    let mut store = Store::new(&engine, host::Process::new(target(), config).unwrap());
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    let main = instance.get_typed_func::<(), ()>(&store, "main").unwrap();
    let result = main.call(&mut store, ());
    assert!(
        result.is_ok(),
        "{result:?}: {}",
        String::from_utf8_lossy(&stderr.lock().unwrap())
    );
    assert!(stderr.lock().unwrap().is_empty());
    let before = stdout.lock().unwrap().clone();
    assert!(
        main.call(&mut store, ()).is_err(),
        "second main must not run a child"
    );
    assert_eq!(*stdout.lock().unwrap(), before);
    String::from_utf8(before).unwrap()
}
#[test]
fn unconfigured_examples_match_four_executors() {
    for source in [
        EXAMPLE,
        include_str!("../../../examples/process_execution/unconfigured.ceru"),
    ] {
        let p = hir(source);
        let expected = ir_executor::run(&p).unwrap();
        let m = mir::lower(&p).unwrap();
        assert_eq!(mir_executor::run(&m).unwrap(), expected);
        assert_eq!(
            ssa_executor::run(&mir::ssa::construct(&m).unwrap()).unwrap(),
            expected
        );
        assert_eq!(
            language::run_bytecode(&language::bytecode::lower(&p).unwrap()).unwrap(),
            expected
        );
        assert_eq!(run(&p, &[]), expected);
    }
}
fn configuration() -> serde_json::Value {
    serde_json::json!({"schema":"cerune-process-v1","platform":std::env::consts::OS,
        "linux_supervisor":if cfg!(windows){serde_json::Value::Null}else{serde_json::json!(std::env::current_exe().unwrap())},
        "limits":{"arguments":32,"argument_bytes":16384,"stdin_bytes":8192,"output_bytes":32768,"timeout_ms":10000,"cleanup_timeout_ms":5000},
        "transfer_bytes":65536,"targets":[]})
}
fn compare(p: &language::ir::Program, config: &[u8]) -> String {
    let mut expected = None;
    for route in 0..4 {
        let (mut processes, n) = language::runtime::process::configuration::decode(config).unwrap();
        let out = match route {
            0 => ir_executor::run_with_processes(p, &mut processes, n).unwrap(),
            1 => mir_executor::run_with_processes(&mir::lower(p).unwrap(), &mut processes, n)
                .unwrap(),
            2 => ssa_executor::run_with_processes(
                &mir::ssa::construct(&mir::lower(p).unwrap()).unwrap(),
                &mut processes,
                n,
            )
            .unwrap(),
            _ => language::vm::run_with_processes(
                &language::bytecode::lower(p).unwrap(),
                &mut processes,
                n,
            )
            .unwrap(),
        };
        if let Some(expected) = &expected {
            assert_eq!(&out, expected)
        } else {
            expected = Some(out)
        }
    }
    let actual = run(p, config);
    assert_eq!(actual, expected.unwrap());
    actual
}
#[test]
fn configured_validation_and_budget_failures_match_four_executors() {
    let config = serde_json::to_vec(&configuration()).unwrap();
    for source in [
        EXAMPLE.to_owned(),
        EXAMPLE.replace("target: \"tool\"", "target: \"\""),
        EXAMPLE.replace("timeout_ms: 10000", "timeout_ms: 0"),
        EXAMPLE.replace("stdout_limit: 32", "stdout_limit: 18446744073709551615"),
    ] {
        compare(&hir(&source), &config);
    }
    let mut p = hir(EXAMPLE);
    p.array_heap_limit = 400;
    assert!(compare(&p, &config).contains("OutputBudget{}"));
}
#[test]
fn partial_reservation_failure_does_not_leak_or_launch() {
    let config = serde_json::to_vec(&configuration()).unwrap();
    // Fail the second result allocation, after the first owner has been reserved.
    let text = wat::emit_wat(&hir(EXAMPLE)).unwrap();
    let needle = "(call $cerune_array_allocate (local.get $err) (i64.const 8) (i64.const 8))";
    assert!(text.contains(needle));
    let text = text.replace(needle, "(i32.const 0) (i32.const 3)");
    let result = execute(text, &config);
    assert!(result.contains("AllocationFailed{}"), "{result}");
    assert!(result.contains("Absent{}"));
    assert!(result.ends_with("0\ncontinued\n"));
}
#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "requires CERUNE_TEST_PROCESS_SUPERVISOR; Linux WAT CI runs explicitly"
)]
fn registered_child_preserves_binary_data_exit_and_repeated_ownership() {
    use std::{fs, process::Command, time::Duration};
    #[path = "../../../tests/support/process.rs"]
    mod bounded;
    let root = std::env::temp_dir().join(format!("cerune-wat-process-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let child = root.join(if cfg!(windows) { "child.exe" } else { "child" });
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/support/process_child.rs");
    let compiled = bounded::bounded_output(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(fixture)
            .arg("-o")
            .arg(&child),
        &root,
        "compile child",
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    fs::write(root.join("child-directory-marker"), b"registered cwd").unwrap();
    let mut config = configuration();
    if cfg!(target_os = "linux") {
        config["linux_supervisor"] = serde_json::json!(
            std::env::var("CERUNE_TEST_PROCESS_SUPERVISOR").expect("explicit supervisor path")
        );
    }
    config["targets"] = serde_json::json!([{"name":"tool","executable":child,"directory":root,"environment":[["CERUNE_CHILD_VALUE","日本語\r\n"]]}]);
    let config = serde_json::to_vec(&config).unwrap();
    let expected = include_str!("../../../examples/process_execution/registered.stdout");
    assert_eq!(compare(&hir(EXAMPLE), &config), expected);
    // Exercise multiple response chunks and retained capacity greater than length.
    let bulk = EXAMPLE
        .replace(
            "array_copy([\"echo\", \"\", \"日本語\", \"a\\\"b\", \"tail\\\\\", \"--\"])",
            "array_copy([\"bulk\"])",
        )
        .replace(
            "array_copy([u8(255), u8(0), u8(13), u8(10)])",
            "array_repeat::<u8>(u8(255), 4097)",
        )
        .replace("stdout_limit: 32", "stdout_limit: 8192");
    assert_ne!(bulk, EXAMPLE);
    compare(&hir(&bulk), &config);
    let prefix = EXAMPLE.split("print(\"before\");").next().unwrap();
    let repeated = format!(
        "{prefix}\nfor (mut i: i64 = 0; i < 4; i = i + 1) {{ result: types::Outcome=process::run(request); copy: types::Outcome=result; print(copy.stdout); print(result.report.exit); }} print(\"continued\");"
    );
    compare(&hir(&repeated), &config);
    fs::remove_dir_all(&root).unwrap();
}
#[path = "process_generated/faults.rs"]
mod faults;
#[test]
fn request_evaluation_short_circuit_and_report_origin_match_four_executors() {
    for example in [
        EXAMPLE.replace("\r\n", "\n"),
        EXAMPLE.replace("\r\n", "\n").replace("\n", "\r\n"),
    ] {
        let declaration = example
            .split("request: types::Request = ")
            .nth(1)
            .unwrap()
            .split("print(\"before\");")
            .next()
            .unwrap()
            .trim()
            .trim_end_matches(';');
        let source = format!(
            r#"import "std:process" as types; import "std:process::run" as process;
        fn request()->types::Request {{print("request");return {declaration};}}
        fn skip()->bool {{print("unexpected"); result: types::Outcome=process::run(request());return true;}}
        print(false && skip()); print("before"); result:types::Outcome=process::run(request());
        print(result.report); print("continued");"#
        );
        let p = hir(&source);
        let config = serde_json::to_vec(&configuration()).unwrap();
        let out = compare(&p, &config);
        assert_eq!(out.matches("request\n").count(), 1);
        assert!(!out.contains("unexpected"));
        assert!(out.starts_with("false\nbefore\nrequest\n"));
    }
}
