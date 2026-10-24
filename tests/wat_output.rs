use cerune_lang as reference_api;
use cerune_lang::ir;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
#[path = "support/output_reference.rs"]
mod output_reference;
#[path = "support/process.rs"]
mod process;
use output_reference::reference;

fn tools() -> Option<(std::ffi::OsString, PathBuf)> {
    let configured = std::env::var_os("CERUNE_TEST_NODE");
    let node = configured.clone().unwrap_or_else(|| "node".into());
    let wabt_config = std::env::var_os("CERUNE_TEST_WAT2WASM_JS");
    let wabt = wabt_config
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/wasm-tools/node_modules/wabt/bin/wat2wasm"));
    if !Command::new(&node)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
        || !wabt.is_file()
    {
        assert!(
            configured.is_none() && wabt_config.is_none(),
            "configured WAT tools unavailable"
        );
        eprintln!(
            "WAT output execution skipped; configure CERUNE_TEST_NODE and CERUNE_TEST_WAT2WASM_JS"
        );
        return None;
    }
    Some((node, wabt))
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn run(command: &mut Command, dir: &Path, name: &str) -> std::process::Output {
    process::bounded_output(command, dir, name, Duration::from_secs(30)).unwrap()
}
fn build(hir: &ir::Program, dir: &Path, node: &std::ffi::OsStr, wabt: &Path) -> PathBuf {
    let wat = cerune_lang::codegen::wat::emit_wat(hir).unwrap();
    assert!(!wat.contains("(export \"memory\""));
    assert!(!wat.contains("(import \"cerune\" \"print_"));
    assert!(!wat.contains("(import \"cerune\" \"write_byte\""));
    fs::write(dir.join("program.wat"), wat).unwrap();
    let wasm = dir.join("program.wasm");
    let o = run(
        Command::new(node)
            .arg(wabt)
            .arg(dir.join("program.wat"))
            .arg("-o")
            .arg(&wasm),
        dir,
        "assemble",
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    wasm
}
fn directory(name: &str) -> PathBuf {
    let dir = root()
        .join("target")
        .join(format!("wat-output-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}
#[test]
fn writes_match_four_executors_with_partial_failure_and_observation() {
    let Some((node, wabt)) = tools() else { return };
    let dir = directory("comparison");
    let long = format!(
        "print(\"{}\"); print(999); flush();",
        "日本\\0\\r\\ne\\u{301}".repeat(6000)
    );
    let quoted = format!(
        "print([\"{}\"]); print(999); flush();",
        "\\u{7f}\\\"\\\\".repeat(100)
    );
    let sources = [
        "print(\"日本\\0\\r\\n\"); print(999); flush();",
        "print(18446744073709551615u64); print(-9223372036854775808); print(true); print(false); flush();",
        "print([1.5, -0.0, 0.0000001]); print([true, false]); print([\"日本\", \"\\\"\\\\\\t\\u{7f}\"]); flush();",
        include_str!("../examples/io_protocol/output_values.ceru"),
        "fn next() -> i64 { print(7); return 42; } print(next()); print(false && (next() == 42)); flush();",
        &long,
        &quoted,
        "print(\"\"); flush(); print(\"after\");",
    ];
    for (index, source) in sources.iter().enumerate() {
        let hir = cerune_lang::compile_to_ir(source).unwrap();
        let wasm = build(&hir, &dir, &node, &wabt);
        for (limit, chunk, mode) in [
            (0, 2, "error"),
            (1, 2, "error"),
            (2, 1, "error"),
            (3, 1, "error"),
            (7, 3, "zero"),
            (12, 2, "invalid"),
            (3, 2, "negative"),
            (300, 17, "error"),
            (65537, 65536, "error"),
            (usize::MAX, 3, "error"),
        ] {
            let expected = reference(&hir, 0, limit, chunk, mode, 0);
            for route in 0..4 {
                for trace in [0, 1, 256] {
                    assert_eq!(reference(&hir, route, limit, chunk, mode, trace), expected);
                }
            }
            let config = format!(
                r#"{{"limit":{},"chunk":{chunk},"mode":"{mode}"}}"#,
                limit.min(9_007_199_254_740_991)
            );
            let actual = run(
                Command::new(&node)
                    .arg(root().join("tests/support/run_wasm.cjs"))
                    .arg(&wasm)
                    .env("CERUNE_TEST_WAT_OUTPUT", config),
                &dir,
                "execute",
            );
            assert_eq!(
                actual.status.success(),
                expected.1.is_none(),
                "case {index}: {}",
                String::from_utf8_lossy(&actual.stderr)
            );
            assert_eq!(actual.stdout, expected.0, "case {index}, {limit}, {mode}");
            assert_eq!(
                String::from_utf8(actual.stderr).unwrap(),
                expected
                    .1
                    .map(|f| format!("cerune: {}\n", f.record()))
                    .unwrap_or_default(),
                "case {index}, {limit}, {mode}"
            );
            if index == 3 && expected.1.is_none() {
                assert_eq!(
                    actual.stdout,
                    include_bytes!("../examples/io_protocol/output_values.stdout")
                );
            }
        }
    }
}
#[test]
fn module_failure_keeps_file_origin() {
    let Some((node, wabt)) = tools() else { return };
    let dir = directory("modules");
    fs::write(
        dir.join("lib.ceru"),
        "pub fn send() -> void { print(\"日本\"); flush(); print(999); }",
    )
    .unwrap();
    fs::write(
        dir.join("entry.ceru"),
        "import \"lib.ceru\" as lib; lib::send();",
    )
    .unwrap();
    let hir = cerune_lang::modules::load(&dir.join("entry.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    let wasm = build(&hir, &dir, &node, &wabt);
    let expected = reference(&hir, 0, 2, 1, "error", 32);
    let record = format!("cerune: {}\n", expected.1.unwrap().record());
    assert!(record.contains(" file="));
    let actual = run(
        Command::new(&node)
            .arg(root().join("tests/support/run_wasm.cjs"))
            .arg(wasm)
            .env(
                "CERUNE_TEST_WAT_OUTPUT",
                r#"{"limit":2,"chunk":1,"mode":"error"}"#,
            ),
        &dir,
        "module",
    );
    assert_eq!(actual.status.code(), Some(1));
    assert_eq!(actual.stdout, expected.0);
    assert_eq!(String::from_utf8(actual.stderr).unwrap(), record);
}
#[test]
fn versioned_imports_and_formatter_failures_are_explicit() {
    let Some((node, wabt)) = tools() else { return };
    let dir = directory("contracts");
    let host = fs::read_to_string(root().join("tests/support/run_wasm.cjs")).unwrap();
    for (source, float) in [("print(1);", false), ("print(1.5);", true)] {
        let hir = cerune_lang::compile_to_ir(source).unwrap();
        let wasm = build(&hir, &dir, &node, &wabt);
        let wat = fs::read_to_string(dir.join("program.wat")).unwrap();
        assert_eq!(wat.contains("(import \"cerune\" \"format_f64_v1\""), float);
        assert!(!wat.contains("(import \"cerune\" \"format_f32_v1\""));
        assert!(!wat.contains("(import \"cerune\" \"flush_stdout_v1\""));
        for missing in if float {
            vec!["write_stdout_v1", "format_f64_v1"]
        } else {
            vec!["write_stdout_v1"]
        } {
            let path = dir.join("missing.cjs");
            fs::write(
                &path,
                host.replace(&format!("{missing}:"), &format!("old_{missing}:")),
            )
            .unwrap();
            let result = run(
                Command::new(&node).arg(path).arg(&wasm),
                &dir,
                "missing-import",
            );
            assert!(!result.status.success());
            assert!(result.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("function import requires a callable")
            );
            assert!(!String::from_utf8_lossy(&result.stderr).contains("io-v1"));
        }
    }
    for source in ["", "flush();", "print(true);"] {
        let hir = cerune_lang::compile_to_ir(source).unwrap();
        let wasm = build(&hir, &dir, &node, &wabt);
        let path = dir.join("minimal.cjs");
        let source_host = host
            .replace("format_f32_v1:", "old_format_f32_v1:")
            .replace("format_f64_v1:", "old_format_f64_v1:");
        fs::write(
            &path,
            if source.contains("print") {
                source_host
            } else {
                source_host.replace("write_stdout_v1:", "old_write_stdout_v1:")
            },
        )
        .unwrap();
        let result = run(
            Command::new(&node).arg(path).arg(&wasm),
            &dir,
            "minimal-imports",
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let hir = cerune_lang::compile_to_ir("print(1.234567891234567); flush(); print(999);").unwrap();
    let wasm = build(&hir, &dir, &node, &wabt);
    for response in [
        "[0n, -1]",
        "[0n, 0]",
        "[0n, 129]",
        "[0n, 3]",
        "offset ? [0n, 9] : [0x3131313131313131n, 10]",
    ] {
        let path = dir.join("format-failed.cjs");
        fs::write(
            &path,
            host.replace("floatFormat.chunk(value, 17, offset)", response),
        )
        .unwrap();
        let result = run(
            Command::new(&node).arg(path).arg(&wasm),
            &dir,
            "format-failed",
        );
        assert!(!result.status.success());
        assert!(
            result.stdout.is_empty(),
            "formatting must finish before any write"
        );
        let diagnostic = String::from_utf8(result.stderr).unwrap();
        assert!(
            diagnostic.starts_with("cerune: output formatting failed node="),
            "{diagnostic}"
        );
        assert!(!diagnostic.contains("io-v1"));
    }
}
#[test]
fn float_host_matches_existing_rounding_and_extreme_values() {
    let Some((node, wabt)) = tools() else { return };
    let dir = directory("floats");
    let source = "print(12345678.5f32); print(0.1f32); print(-0.0f32); print(1.401298464324817e-45f32); print(3.4028234663852886e38f32); print(0.1); print(-0.0); print(0.0001); print(0.00001); print(-1e-20); print(1.7976931348623157e308); print(4.9406564584124654e-324);";
    let hir = cerune_lang::compile_to_ir(source).unwrap();
    let expected = reference(&hir, 0, usize::MAX, 8, "error", 0);
    let wasm = build(&hir, &dir, &node, &wabt);
    let result = run(
        Command::new(&node)
            .arg(root().join("tests/support/run_wasm.cjs"))
            .arg(&wasm),
        &dir,
        "float-example",
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, expected.0);
}

#[test]
fn library_exports_execute_typed_functions_and_preserve_dependency_failure_origins() {
    use cerune_lang::{
        bytecode,
        embedding::{self, HostValue},
        modules,
    };
    use std::sync::Arc;
    let Some((node, wabt)) = tools() else { return };
    let dir = directory("library");
    let c = modules::load(&root().join("examples/host_calls/library.ceru")).unwrap();
    let library =
        cerune_lang::codegen::wat::emit_library(&c, &["step", "sum_to", "checked"]).unwrap();
    let canonical = c.to_ir().unwrap();
    assert_eq!(library.program(), &canonical);
    let program = Arc::new(bytecode::lower(&canonical).unwrap());
    let resolve = |name: &str| {
        let d = c
            .function_declarations()
            .iter()
            .find(|d| d.is_entry() && d.name() == name)
            .unwrap();
        embedding::resolve_function(Arc::clone(&program), d.resolved_name()).unwrap()
    };
    let step = resolve("step");
    for i in 0..100 {
        let result = embedding::invoke_function(
            &step,
            &[
                HostValue::F64(f64::from(i)),
                HostValue::F64(2.0),
                HostValue::F64(0.5),
            ],
        )
        .unwrap();
        assert_eq!(
            result.return_value(),
            Some(&HostValue::F64(f64::from(i) + 1.0))
        );
    }
    let embedding::FunctionInvocationError::Execution(error) =
        embedding::invoke_function(&resolve("checked"), &[HostValue::F64(1.5)]).unwrap_err()
    else {
        panic!()
    };
    fs::write(
        dir.join("failure.txt"),
        error.runtime_failure().unwrap().record(),
    )
    .unwrap();
    fs::write(dir.join("library.wat"), library.wat()).unwrap();
    fs::write(dir.join("library.mjs"), library.javascript()).unwrap();
    fs::write(dir.join("library.d.mts"), library.typescript()).unwrap();
    fs::write(dir.join("exports.json"), library.export_manifest()).unwrap();
    fs::write(dir.join("sources.json"), c.source_manifest()).unwrap();
    fs::write(
        dir.join("library.ceir"),
        cerune_lang::ir::text::emit(library.program()),
    )
    .unwrap();
    let wasm = dir.join("library.wasm");
    let output = run(
        Command::new(&node)
            .arg(&wabt)
            .arg(dir.join("library.wat"))
            .arg("-o")
            .arg(&wasm),
        &dir,
        "library-assemble",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Deliberately foreign modules exercise host-contract failures independently.
    for (name, imports, checked, tail) in [
        ("trap", "", "(param f64) (result f64) unreachable", ""),
        (
            "instantiate",
            "",
            "(param f64) (result f64) local.get 0",
            "(func $bad unreachable) (start $bad)",
        ),
        (
            "host",
            "(import \"cerune\" \"write_error_byte\" (func $error (param i32)))",
            "(param f64) (result f64) i32.const 255 call $error local.get 0",
            "",
        ),
        ("result", "", "(param f64) (result i64) i64.const 1", ""),
        (
            "extra",
            "",
            "(param f64) (result f64) local.get 0",
            "(memory (export \"memory\") 1)",
        ),
    ] {
        let wat = format!(
            "(module {imports} (func (export \"step\") (param f64 f64 f64) (result f64) local.get 0) (func (export \"sum_to\") (param i32) (result i32) local.get 0) (func (export \"checked\") {checked}) {tail})"
        );
        fs::write(dir.join(format!("{name}.wat")), wat).unwrap();
        let output = run(
            Command::new(&node)
                .arg(&wabt)
                .arg(dir.join(format!("{name}.wat")))
                .arg("-o")
                .arg(dir.join(format!("{name}.wasm"))),
            &dir,
            name,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let names=modules::from_source("pub fn __proto__()->i32{return 7;} pub fn constructor(x:f64)->f64{return x;} pub fn then()->f64{return 8.0;} pub fn identity(x:i32)->i32{return x;}").unwrap();
    let names = cerune_lang::codegen::wat::emit_library(
        &names,
        &["__proto__", "constructor", "then", "identity"],
    )
    .unwrap();
    fs::write(dir.join("names.mjs"), names.javascript()).unwrap();
    fs::write(dir.join("names.wat"), names.wat()).unwrap();
    let output = run(
        Command::new(&node)
            .arg(&wabt)
            .arg(dir.join("names.wat"))
            .arg("-o")
            .arg(dir.join("names.wasm")),
        &dir,
        "names",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run(
        Command::new(&node)
            .arg(root().join("tests/support/wat_library_host.cjs"))
            .arg(wasm)
            .arg(dir.join("exports.json"))
            .arg(dir.join("failure.txt")),
        &dir,
        "library-host",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let demo = modules::load(&root().join("examples/host_calls/library_demo.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    let expected = "11\n10\n42\n";
    assert_eq!(cerune_lang::ir_executor::run(&demo).unwrap(), expected);
    assert_eq!(
        cerune_lang::vm::run(&bytecode::lower(&demo).unwrap()).unwrap(),
        expected
    );
}

#[path = "wat_frames/mod.rs"]
mod frames;
