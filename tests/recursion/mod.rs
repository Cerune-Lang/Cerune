use cerune_lang::{bytecode, compile_to_ir, ir_executor, mir, mir_executor, ssa_executor, vm};
use std::{fs, num::NonZeroU32, process::Command, time::Duration};
#[path = "../support/recursion_cases.rs"]
mod cases;
#[path = "../support/process.rs"]
mod process;
fn compare(source: &str, n: u32) {
    let p = cerune_lang::modules::from_source(source)
        .unwrap()
        .to_ir()
        .unwrap();
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(&p).unwrap();
    let limit = NonZeroU32::new(n).unwrap();
    let expected = ir_executor::run_with_call_limit(&p, limit);
    let mm = mir_executor::run_with_call_limit(&m, limit);
    let ss = ssa_executor::run_with_call_limit(&s, limit);
    let vv = vm::run_with_call_limit(&b, limit);
    match expected {
        Ok(text) => {
            assert_eq!(mm.unwrap(), text);
            assert_eq!(ss.unwrap(), text);
            assert_eq!(vv.unwrap(), text);
        }
        Err(e) => {
            let mm = mm.unwrap_err();
            let ss = ss.unwrap_err();
            let vv = vv.unwrap_err();
            assert_eq!(mm.output(), e.output());
            assert_eq!(ss.output(), e.output());
            assert_eq!(vv.output(), e.output());
            if let Some(f) = e.call_failure() {
                assert_eq!(mm.call_failure(), Some(f));
                assert_eq!(ss.call_failure(), Some(f));
                assert_eq!(vv.call_failure(), Some(f));
            } else {
                assert_eq!(mm.runtime_failure(), e.runtime_failure());
                assert_eq!(ss.runtime_failure(), e.runtime_failure());
                assert!(matches!(
                    vv.kind(),
                    cerune_lang::vm::VmErrorKind::DivisionByZero
                ));
                let instructions = vv
                    .function_id()
                    .map(|id| &b.functions[id].instructions)
                    .unwrap_or(&b.instructions);
                let failure = e.runtime_failure().unwrap();
                assert_eq!(
                    instructions[vv.instruction_index()].origin,
                    bytecode::InstructionOrigin::Source {
                        node_id: failure.node_id,
                        span: failure.span
                    }
                );
            }
        }
    }
}
#[test]
fn source_recursion_agrees_in_four_executors() {
    for &(source, expected) in cases::EXAMPLES {
        assert_eq!(
            ir_executor::run_with_call_limit(
                &compile_to_ir(source).unwrap(),
                NonZeroU32::new(128).unwrap()
            )
            .unwrap(),
            expected.replace("\r\n", "\n")
        );
        for n in [1, 4, 5, 7, 8, 128] {
            compare(source, n);
        }
    }
    for &source in cases::STOPS {
        for n in [1, 4, 128] {
            compare(source, n);
        }
    }
}
#[test]
fn generic_recursive_instances_are_reused_and_keep_recursive_origins() {
    for source in [
        "fn same<T,const N:i64>(x:[T;N],n:i64)->[T;N]{if n==0{return x;}return same::<T,N>(x,n-1);} print(same::<i64,2>([1,2],3));",
        "fn left<T>(x:T,n:i64)->T{if n==0{return x;}return right::<T>(x,n-1);}fn right<T>(x:T,n:i64)->T{return left::<T>(x,n);}print(left::<i64>(42,3));",
    ] {
        let p = compile_to_ir(source).unwrap();
        let instances: Vec<_> = p
            .function_definitions
            .iter()
            .filter_map(|f| f.generic_origin.as_ref())
            .collect();
        assert!(instances.len() <= 2);
        assert!(instances.iter().any(|o| o.calls.len() >= 2));
        for origin in instances {
            for usage in &origin.calls {
                assert!(source[usage.span.start()..usage.span.end()].contains("::<"));
            }
        }
        compare(source, 8);
    }
}
#[test]
fn growing_generic_specializations_stop_during_compilation() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("recursive-generics-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("growing.ceru");
    fs::write(
        &file,
        "fn grow<T>(x:T)->i64{return grow::<[T;1]>([x]);}print(grow::<i64>(1));",
    )
    .unwrap();
    for command in ["check", "emit-ir"] {
        let r = process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(command)
                .arg(&file),
            &dir,
            command,
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(r.status.code(), Some(1), "{r:?}");
        assert!(
            String::from_utf8_lossy(&r.stderr)
                .contains("generic expansion exceeds 256 instances or depth 64"),
            "{r:?}"
        );
        assert!(r.stdout.is_empty());
    }
}
#[test]
fn recursive_calls_in_implicit_and_nested_expressions_keep_limits() {
    for source in [
        "fn cycle()->[i64;1]{for(v:infer in cycle()){}return [1];}print(cycle());",
        "fn cycle()->i64{for(v:infer in [1]){print(cycle());}return 0;}print(cycle());",
        "fn cycle()->[i64;1]{print(array_len(cycle()));return [1];}print(cycle());",
        "fn again()->string{print(byte_len(again()));return \"\";}print(again());",
        "type P{x:i64}fn cycle()->P{return P{..cycle(),x:1};}print(cycle());",
        "fn cycle()->i64{return array_repeat::<i64>(cycle(),1)[0];}print(cycle());",
        "fn cycle()->i64{return array_repeat::<i64>(1,cycle())[0];}print(cycle());",
    ] {
        compare(source, 4);
    }
}

#[test]
fn recursive_module_cli_preserves_call_sites_and_default_limit() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("recursive-modules-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("lib.ceru"),
        "pub fn walk(n:i64)->i64{if n==0{return 1;}return walk(n-1);}",
    )
    .unwrap();
    let file = dir.join("main.ceru");
    fs::write(
        &file,
        "import \"lib.ceru\" as lib; print(7); print(lib::walk(4)); print(99);",
    )
    .unwrap();
    for limit in [Some(4), Some(5), None] {
        let mut expected = None;
        for (route, ssa) in [
            ("run", false),
            ("run-ir", false),
            ("run-mir", false),
            ("run-mir", true),
            ("run-vm", false),
        ] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
            cmd.arg(route)
                .arg(&file)
                .args(["--diagnostic-format", "runtime-v1"]);
            if ssa {
                cmd.arg("--ssa");
            }
            if let Some(n) = limit {
                cmd.args(["--call-depth-limit", &n.to_string()]);
            }
            let r =
                process::bounded_output(&mut cmd, &dir, route, Duration::from_secs(30)).unwrap();
            if limit == Some(4) {
                assert_eq!(r.status.code(), Some(1));
                assert_eq!(r.stdout, b"7\n");
                let text = String::from_utf8_lossy(&r.stderr);
                assert!(
                    text.contains("call-v1 reason=depth-limit limit=4")
                        && text.contains("file=")
                        && text.contains("bytes="),
                    "{text}"
                );
            } else {
                assert!(r.status.success(), "{r:?}");
                assert_eq!(r.stdout, b"7\n1\n99\n");
                assert!(r.stderr.is_empty());
            }
            let got = (r.stdout, r.stderr);
            if let Some(expected) = &expected {
                assert_eq!(&got, expected);
            } else {
                expected = Some(got);
            }
        }
    }
}
#[test]
fn finite_type_changing_recursion_reuses_the_reached_instance() {
    let source = "fn change<T>(x:T,n:i64)->i64{if n==0{return 7;}return change::<i64>(1,n-1);} print(change::<bool>(true,5));";
    let p = compile_to_ir(source).unwrap();
    assert_eq!(
        p.function_definitions
            .iter()
            .filter(|f| f.generic_origin.is_some())
            .count(),
        2
    );
    compare(source, 8);
    compare(source, 4);
}
