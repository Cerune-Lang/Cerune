//! 同じ完成済みIRを独立した3経路で実行し、既知の期待値も照合します。
use string_cases::aggregate_cases;
#[allow(dead_code)]
#[path = "support/runtime_cases.rs"]
mod runtime_cases;
#[allow(dead_code)]
#[path = "support/string_cases.rs"]
mod string_cases;
#[allow(dead_code)]
#[path = "support/u64_cases.rs"]
mod u64_cases;
use cerune_lang::{bytecode, compile_to_ir, ir, ir_executor, modules, run_bytecode};
use std::{fs, path::Path};

fn compare(program: &ir::Program) -> Result<String, ir_executor::ExecutionError> {
    let before = ir::text::emit(program);
    let direct = ir_executor::run(program);
    let mir_program = cerune_lang::mir::lower(program).unwrap();
    let mir = cerune_lang::mir_executor::run(&mir_program);
    match (&direct, &mir) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => {
            assert!(a.runtime_failure().is_some(), "{a:?}");
            assert_eq!(a.runtime_failure(), b.runtime_failure(), "{b:?}");
            assert_eq!(a.output(), b.output());
        }
        _ => panic!("HIR: {direct:?}\nMIR: {mir:?}"),
    }
    let vm = run_bytecode(&bytecode::lower(program).unwrap());
    match (&direct, &vm) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => {
            assert!(a.runtime_failure().is_some(), "{a:?}");
            assert_eq!(a.runtime_failure(), b.runtime_failure());
            assert_eq!(a.output(), b.vm_error().output());
        }
        _ => panic!("IR: {direct:?}\nVM: {vm:?}"),
    }
    assert_eq!(ir::text::emit(program), before);
    direct
}

#[test]
fn all_root_examples_execute_without_changing_ir() {
    for entry in fs::read_dir("examples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|x| x == "ceru") {
            let source = fs::read_to_string(&path).unwrap();
            let program =
                compile_to_ir(&source).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
            compare(&program).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        }
    }
}

#[test]
fn shared_success_fixtures_have_known_outputs() {
    for &(source, expected) in string_cases::CASES
        .iter()
        .chain(string_cases::PRODUCT_UPDATES)
        .chain(aggregate_cases::CASES)
        .chain(&[
            string_cases::BYTE_LENGTH,
            u64_cases::EXAMPLE,
            u64_cases::BOUNDARIES,
        ])
    {
        let program = compile_to_ir(source).unwrap();
        assert_eq!(compare(&program).unwrap(), expected, "{source}");
        assert_eq!(
            ir_executor::run(&program).unwrap(),
            expected,
            "fresh execution"
        );
    }
}

#[test]
fn failures_preserve_reason_origin_and_prior_output() {
    for &(body, code) in runtime_cases::FAILURES {
        let source = format!(
            "// 日本語\r\nprint(\"開始\\0\\r\\n\");\r\nprint(false && (1 / 0 == 0));\r\n{body}"
        );
        let error = compare(&compile_to_ir(&source).unwrap()).unwrap_err();
        assert_eq!(
            error.runtime_failure().unwrap().code.name(),
            code,
            "{source}"
        );
    }
    for source in u64_cases::FAILURES
        .iter()
        .chain(string_cases::OUT_OF_BOUNDS)
    {
        compare(&compile_to_ir(source).unwrap()).unwrap_err();
    }
}

#[test]
fn module_examples_share_resolved_ir_and_file_origins() {
    fn walk(path: &Path) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path);
            } else if path.extension().is_some_and(|n| n == "ceru") {
                let compilation = modules::load(&path).unwrap();
                let program = compilation.to_ir().unwrap();
                let result = compare(&program);
                if path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("failure")
                    || path.file_name().unwrap() == "array_update.ceru"
                {
                    let error = result.unwrap_err();
                    assert!(compilation.render(&error.diagnostic()).contains(".ceru"));
                } else {
                    result.unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
                }
            }
        }
    }
    walk(Path::new("examples/modules"));
}

#[test]
fn string_budget_tracks_lifetimes_across_loops_calls_and_copies() {
    for (source, limit, expected) in [
        (
            aggregate_cases::CASES[7].0,
            24,
            Some(aggregate_cases::CASES[7].1),
        ),
        (aggregate_cases::CASES[7].0, 23, None),
        (
            r#"fn value()->string{v:string=concat("a","b");return v;} for(mut i:i64=0;i<1000;i=i+1){s:string=value();if i<999{continue;}print(s);} print(concat("",""));"#,
            2,
            Some("ab\n\n"),
        ),
        (
            r#"print("before"); mut s:string=concat("a","b"); saved:string=s; s=concat(s,"c"); print(saved);print(s);"#,
            4,
            None,
        ),
        (
            r#"print("before"); mut s:string=concat("a","b"); saved:string=s; s=concat(s,"c"); print(saved);print(s);"#,
            5,
            Some("before\nab\nabc\n"),
        ),
        (
            r#"fn right()->string{print("bad");return "";} print("before");print(concat(concat("a","b"),right()));"#,
            1,
            None,
        ),
        (
            r#"fn later()->i64{print("bad");return 0;} mut a:[[string;1];1]=[[concat("a","b")]];print("before");a[1][later()]=concat("x","y");"#,
            2,
            None,
        ),
        (
            r#"const TEXT:string=concat("日","本"); print(TEXT);print(concat("","")); "#,
            0,
            Some("日本\n\n"),
        ),
    ] {
        let mut program = compile_to_ir(source).unwrap();
        program.string_heap_limit = limit;
        for _ in 0..2 {
            let result = compare(&program);
            if let Some(expected) = expected {
                assert_eq!(result.unwrap(), expected);
            } else {
                result.unwrap_err();
            }
        }
    }
}

#[test]
fn malformed_ir_is_distinct_from_language_failure() {
    let mut program = compile_to_ir("print(1);").unwrap();
    let ir::StatementKind::Print { value } = &mut program.statements[0].kind else {
        panic!()
    };
    value.ty = ir::Type::Bool;
    let error = ir_executor::run(&program).unwrap_err();
    assert!(matches!(error.kind(), ir_executor::ErrorKind::InvalidIr(_)));
    assert_eq!(error.runtime_failure(), None);
}

#[test]
fn source_api_distinguishes_compilation_and_execution_errors() {
    use cerune_lang::{IrRunError, run_ir};
    assert_eq!(run_ir("print(42);").unwrap(), "42\n");
    assert!(matches!(
        run_ir("print(missing);"),
        Err(IrRunError::Compilation(_))
    ));
    let Err(IrRunError::Execution(error)) = run_ir("print(1); print(1 / 0);") else {
        panic!()
    };
    assert_eq!(error.output(), "1\n");
    assert_eq!(
        error.runtime_failure().unwrap().code.name(),
        "division-by-zero"
    );
}

#[test]
fn cli_runs_examples_and_preserves_failure_records() {
    use std::process::Command;
    let exe = env!("CARGO_BIN_EXE_cerune");
    for file in [
        "examples/ir_execution.ceru",
        "examples/modules/main.ceru",
        "examples/modules/failure.ceru",
        "examples/modules/product_update_failure.ceru",
        "examples/runtime_failures/overflow.ceru",
        "examples/string_concat.ceru",
    ] {
        for limit in ["0", "67108864"] {
            let args = [
                file,
                "--diagnostic-format",
                "runtime-v1",
                "--string-heap-limit",
                limit,
            ];
            let direct = Command::new(exe).arg("run-ir").args(args).output().unwrap();
            let vm = Command::new(exe).arg("run-vm").args(args).output().unwrap();
            let default = Command::new(exe).arg("run").args(args).output().unwrap();
            assert_eq!(default.status, direct.status, "{file}");
            assert_eq!(default.stdout, direct.stdout, "{file}");
            assert_eq!(default.stderr, direct.stderr, "{file}");
            assert_eq!(direct.status, vm.status, "{file}");
            assert_eq!(direct.stdout, vm.stdout, "{file}");
            assert_eq!(direct.stderr, vm.stderr, "{file}");
        }
    }
    let output = Command::new(exe)
        .args(["run-ir", "examples/ir_execution.ceru"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        "Hello, IR\nHello, 世界\n[\"Cerune\", \"世界\"]\n".as_bytes()
    );
    let error = Command::new(exe)
        .args(["run-ir", "examples/modules/failure.ceru"])
        .output()
        .unwrap();
    assert!(!error.status.success());
    assert!(
        String::from_utf8(error.stderr.clone())
            .unwrap()
            .contains("values.ceru")
    );
    let default = Command::new(exe)
        .args(["run", "examples/modules/failure.ceru"])
        .output()
        .unwrap();
    assert_eq!(default.status, error.status);
    assert_eq!(default.stdout, error.stdout);
    assert_eq!(default.stderr, error.stderr);
    let vm = Command::new(exe)
        .args(["run-vm", "examples/modules/failure.ceru"])
        .output()
        .unwrap();
    assert_eq!(vm.status.code(), Some(1));
    assert_eq!(vm.stdout, error.stdout);
    assert!(String::from_utf8_lossy(&vm.stderr).contains("bytecode instruction"));
    assert!(!String::from_utf8_lossy(&default.stderr).contains("bytecode instruction"));

    for args in [
        vec!["run-ir"],
        vec![
            "run-ir",
            "examples/ir_execution.ceru",
            "--target",
            "x86_64-unknown-linux-gnu",
        ],
        vec![
            "run-ir",
            "examples/ir_execution.ceru",
            "--diagnostic-format",
            "unknown",
        ],
        vec![
            "run-ir",
            "examples/ir_execution.ceru",
            "--string-heap-limit",
            "-1",
        ],
        vec![
            "run-ir",
            "examples/ir_execution.ceru",
            "--string-heap-limit",
            "1",
            "--string-heap-limit",
            "2",
        ],
    ] {
        for command in ["run", "run-ir", "run-vm"] {
            let output = Command::new(exe)
                .arg(command)
                .args(&args[1..])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{command}");
            assert!(output.stdout.is_empty(), "{command}");
        }
    }
}

#[test]
fn structured_flow_evaluation_order_and_scalar_edges() {
    for (source, expected) in [
        (
            "fn unused()->void{print(999);} fn main()->void{print(42);}",
            "42\n",
        ),
        ("fn unused()->void{print(999);}", ""),
        (
            "print(-128i8 % -1);print(-32768i16 % -1);print(-2147483648i32 % -1);print(-9223372036854775808i64 % -1);",
            "0\n0\n0\n0\n",
        ),
        (
            "print(0.0 / 0.0);print(1.0 / 0.0);print(-1.0 / 0.0);print(-0.0);print(-0.0f32);print((0.0/0.0)==(0.0/0.0));",
            "NaN\ninf\n-inf\n-0\n-0\nfalse\n",
        ),
        (
            "fn mark(n:i64)->i64{print(n);return n;} fn sum(a:i64,b:i64)->i64{return a+b;} print(sum(mark(1),mark(2)));",
            "1\n2\n3\n",
        ),
        (
            "mut total:i64=0;for(mut i:i64=0;i<5;i=i+1){if i==1{continue;} if i==4{break;}total=total+i;}print(total);",
            "5\n",
        ),
    ] {
        assert_eq!(compare(&compile_to_ir(source).unwrap()).unwrap(), expected);
    }
}

mod call_depth;

#[path = "recursion/mod.rs"]
mod recursion;
