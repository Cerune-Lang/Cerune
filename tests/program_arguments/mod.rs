mod c;
mod llvm;
mod native;
mod qbe;
mod wat;
use cerune_lang::{
    bytecode, ir, ir_executor, mir, mir_executor, modules,
    runtime::arguments::{Arguments, Code, Failure, Limits},
    ssa_executor, vm,
};
use std::{fs, path::PathBuf, process::Command, time::Duration};
#[path = "../support/process.rs"]
mod process;

fn snapshot(values: &[&str]) -> Arguments {
    let bytes: Vec<_> = values.iter().map(|s| s.as_bytes()).collect();
    Arguments::capture(
        &bytes,
        Limits {
            max_count: 4096,
            max_bytes: 1048576,
        },
    )
    .unwrap()
}
fn hir(source: &str) -> ir::Program {
    modules::from_source(source).unwrap().to_ir().unwrap()
}
fn execute(route: usize, hir: &ir::Program, args: &Arguments) -> Result<String, (Failure, String)> {
    match route {
        0 => ir_executor::run_with_arguments(hir, args).map_err(|e| {
            let ir_executor::ErrorKind::Arguments(f) = e.kind() else {
                panic!("{e:?}")
            };
            (**f, e.output().to_owned())
        }),
        1 => mir_executor::run_with_arguments(&mir::lower(hir).unwrap(), args).map_err(|e| {
            let mir_executor::ErrorKind::Arguments(f) = e.kind() else {
                panic!("{e:?}")
            };
            (**f, e.output().to_owned())
        }),
        2 => ssa_executor::run_with_arguments(
            &mir::ssa::construct(&mir::lower(hir).unwrap()).unwrap(),
            args,
        )
        .map_err(|e| {
            let ssa_executor::ErrorKind::Arguments(f) = e.kind() else {
                panic!("{e:?}")
            };
            (**f, e.output().to_owned())
        }),
        _ => vm::run_with_arguments(&bytecode::lower(hir).unwrap(), args).map_err(|e| {
            let vm::VmErrorKind::Arguments(f) = e.kind() else {
                panic!("{e:?}")
            };
            (*f, e.output().to_owned())
        }),
    }
}

#[test]
fn example_preserves_arguments_and_observation_does_not_change_execution() {
    let hir = hir(include_str!("../../examples/arguments/list.ceru"));
    let original = hir.clone();
    for (values, expected) in [
        (vec![], "0\nend\n"),
        (vec![""], "1\n[\"\"]\n0\nend\n"),
        (
            vec!["日本語", "a\r\nb", "--ssa", "e\u{301}"],
            "4\n[\"日本語\"]\n9\n[\"a\\r\\nb\"]\n4\n[\"--ssa\"]\n5\n[\"e\u{301}\"]\n3\nend\n",
        ),
    ] {
        let args = snapshot(&values);
        for route in 0..4 {
            assert_eq!(
                execute(route, &hir, &args).unwrap(),
                expected,
                "route {route}"
            );
            assert!(ir::text::emit(&hir).contains("arguments.get("));
            let mir = mir::lower(&hir).unwrap();
            assert!(mir::text::emit(&mir).contains("arguments.count"));
            assert!(
                bytecode::format_program(&bytecode::lower(&hir).unwrap()).contains("arguments.get")
            );
            assert_eq!(execute(route, &hir, &args).unwrap(), expected);
        }
    }
    assert_eq!(hir, original);
}

#[test]
fn owned_strings_cross_functions_arrays_and_short_circuit_without_leaks() {
    let program = hir(r#"
        import "std:env" as env;
        fn text(index: i64) -> string {
            match env::arg(index) {
                env::Argument::Value { text: text } => { return text; },
                env::Argument::Missing {} => { return "missing"; },
            }
        }
        fn index() -> i64 { print("index"); return 0; }
        print(false && byte_len(text(-1)) == 0);
        mut a: [string; 2] = [text(index()), text(1)];
        b: [string; 2] = a;
        a[0] = "changed";
        print(a);
        print(b);
        for (mut i: i64 = 0; i < 3; i = i + 1) { print(text(0)); }
    "#);
    for route in 0..4 {
        assert_eq!(
            execute(route, &program, &snapshot(&["first", "second"])).unwrap(),
            "false\nindex\n[\"changed\", \"second\"]\n[\"first\", \"second\"]\nfirst\nfirst\nfirst\n"
        );
    }
}

#[test]
fn failures_match_reason_origin_and_prior_output_and_do_not_include_contents() {
    for (operation, args, code, heap_limit) in [
        (
            "env::arg_count()",
            Arguments::unconfigured(),
            Code::Unconfigured,
            64,
        ),
        (
            "env::arg(-1)",
            snapshot(&["SECRET"]),
            Code::InvalidIndex,
            64,
        ),
        (
            "env::arg(0)",
            snapshot(&["SECRET"]),
            Code::BudgetExceeded,
            5,
        ),
    ] {
        let source = format!(
            "import \"std:env\" as env; print(\"before\"); print({operation}); print(\"after\");"
        );
        let compilation = modules::from_source(&source).unwrap();
        let mut program = compilation.to_ir().unwrap();
        program.string_heap_limit = heap_limit;
        let baseline = execute(0, &program, &args).unwrap_err();
        assert_eq!(baseline.0.code, code);
        assert_eq!(baseline.1, "before\n");
        assert!(!baseline.0.record().contains("SECRET"));
        let file = compilation
            .sources
            .get(baseline.0.origin.span.source_id())
            .unwrap();
        assert!(matches!(
            file.kind(),
            cerune_lang::source::SourceKind::Standard {
                identifier: "std:env",
                ..
            }
        ));
        for route in 1..4 {
            assert_eq!(execute(route, &program, &args).unwrap_err(), baseline);
        }
    }
}

#[test]
fn default_embedding_connections_remain_unconfigured() {
    let program = hir(r#"import "std:env" as env; print(env::arg_count());"#);
    assert!(matches!(
        ir_executor::run(&program).unwrap_err().kind(),
        ir_executor::ErrorKind::Arguments(_)
    ));
    assert!(matches!(
        mir_executor::run(&mir::lower(&program).unwrap())
            .unwrap_err()
            .kind(),
        mir_executor::ErrorKind::Arguments(_)
    ));
    assert!(matches!(
        ssa_executor::run(&mir::ssa::construct(&mir::lower(&program).unwrap()).unwrap())
            .unwrap_err()
            .kind(),
        ssa_executor::ErrorKind::Arguments(_)
    ));
    assert!(matches!(
        vm::run(&bytecode::lower(&program).unwrap())
            .unwrap_err()
            .kind(),
        vm::VmErrorKind::Arguments(_)
    ));
}

#[test]
fn virtual_module_shares_types_and_rejects_private_intrinsic_spelling() {
    let source = r#"import "std:env" as a; import "std:env" as b; import "std:io" as io;
        x:a::Argument=b::Argument::Value{text:"same"}; print(x);"#;
    let compilation = modules::from_source(source).unwrap();
    assert_eq!(
        compilation
            .sources
            .files()
            .filter(|s| matches!(s.kind(), cerune_lang::source::SourceKind::Standard { .. }))
            .count(),
        2
    );
    let program = compilation.to_ir().unwrap();
    assert_eq!(
        execute(0, &program, &snapshot(&[])).unwrap(),
        "Value{text: \"same\"}\n"
    );
    for source in [
        "print(__cerune_arg_count());",
        "print($arg_count());",
        "import \"std:env\" as env; print(env::arg(true));",
    ] {
        assert!(cerune_lang::compile_to_ir(source).is_err(), "{source}");
    }
}

fn directory(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("arguments-{name}-{}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}
const ROUTES: &[(&str, bool)] = &[
    ("run", false),
    ("run-ir", false),
    ("run-mir", false),
    ("run-mir", true),
    ("run-vm", false),
];

#[test]
fn cli_separator_preserves_empty_unicode_and_option_like_values() {
    let dir = directory("cli");
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/arguments/list.ceru");
    for (n, &(route, ssa)) in ROUTES.iter().enumerate() {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
        command.arg(route).arg(&example);
        if ssa {
            command.arg("--ssa");
        }
        command.args([
            "--argument-count-limit",
            "5",
            "--argument-byte-limit",
            "34",
            "--",
            "",
            "日本語",
            "--ssa",
            "--string-heap-limit",
            "0",
        ]);
        let out = process::bounded_output(
            &mut command,
            &dir,
            &format!("route-{n}"),
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            "5\n[\"\"]\n0\n[\"日本語\"]\n9\n[\"--ssa\"]\n5\n[\"--string-heap-limit\"]\n19\n[\"0\"]\n1\nend\n"
        );
    }
}

#[test]
fn cli_capture_failures_precede_program_output_and_redact_contents() {
    let dir = directory("limits");
    let file = dir.join("case.ceru");
    fs::write(&file, "print(\"must not run\");").unwrap();
    for (n, args, code) in [
        (
            0,
            vec!["--argument-count-limit", "0", "--", "SECRET"],
            "count-limit-exceeded",
        ),
        (
            1,
            vec!["--argument-byte-limit", "5", "--", "SECRET"],
            "byte-limit-exceeded",
        ),
        (
            2,
            vec!["--argument-count-limit", "18446744073709551615"],
            "invalid-limit",
        ),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
        command.arg("run").arg(&file).args(args);
        let out = process::bounded_output(
            &mut command,
            &dir,
            &format!("case-{n}"),
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let error = String::from_utf8(out.stderr).unwrap();
        assert!(error.contains(code), "{error}");
        assert!(!error.contains("SECRET"));
        assert!(!error.contains("node="));
    }
}

#[cfg(unix)]
#[test]
fn cli_invalid_unicode_is_a_redacted_error_not_a_panic() {
    use std::os::unix::ffi::OsStringExt;
    let dir = directory("invalid-unicode");
    let file = dir.join("case.ceru");
    fs::write(&file, "print(1);").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
    command
        .arg("run")
        .arg(&file)
        .arg("--")
        .arg(std::ffi::OsString::from_vec(b"SECRET\xff".to_vec()));
    let out =
        process::bounded_output(&mut command, &dir, "invalid", Duration::from_secs(10)).unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains("code=invalid-unicode"));
    assert!(!error.contains("SECRET"));
    assert!(!error.contains("panicked"));
}

#[test]
fn cli_execution_failures_have_the_same_record_and_stop_after_prior_output() {
    let dir = directory("execute-failures");
    for (case, operation, code) in [
        ("negative", "env::arg(-1)", "invalid-index"),
        ("budget", "env::arg(0)", "budget-exceeded"),
    ] {
        let file = dir.join(format!("{case}.ceru"));
        fs::write(&file, format!("import \"std:env\" as env; print(\"before\"); print({operation}); print(\"after\");")).unwrap();
        let mut expected = None;
        for (n, &(route, ssa)) in ROUTES.iter().enumerate() {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command.arg(route).arg(&file);
            if ssa {
                command.arg("--ssa");
            }
            command.args([
                "--diagnostic-format",
                "runtime-v1",
                "--string-heap-limit",
                "5",
                "--",
                "SECRET",
            ]);
            let out = process::bounded_output(
                &mut command,
                &dir,
                &format!("{case}-{n}"),
                Duration::from_secs(10),
            )
            .unwrap();
            assert!(!out.status.success());
            assert_eq!(out.stdout, b"before\n");
            let error = String::from_utf8(out.stderr).unwrap();
            assert!(error.contains("arguments-v1 phase=execute"), "{error}");
            assert!(error.contains(code), "{error}");
            assert!(error.contains("node=") && error.contains("file="));
            assert!(!error.contains("SECRET"));
            if let Some(expected) = &expected {
                assert_eq!(&error, expected);
            } else {
                expected = Some(error);
            }
        }
    }
}

#[test]
fn arguments_and_line_input_share_execution_without_changing_order() {
    let program = hir(r#"import "std:env" as env; import "std:io" as io;
        print(env::arg_count());
        print(io::read_line(8));
        print(env::arg(0));"#);
    let arguments = snapshot(&["arg"]);
    for route in 0..4 {
        let mut input = cerune_lang::runtime::input::Input::new(std::io::Cursor::new(b"line\n"));
        let mut output = cerune_lang::runtime::output::Output::new(Vec::new());
        match route {
            0 => ir_executor::run_with_io_and_arguments(
                &program,
                &mut input,
                &mut output,
                &arguments,
            )
            .unwrap(),
            1 => mir_executor::run_with_io_and_arguments(
                &mir::lower(&program).unwrap(),
                &mut input,
                &mut output,
                &arguments,
            )
            .unwrap(),
            2 => ssa_executor::run_with_io_and_arguments(
                &mir::ssa::construct(&mir::lower(&program).unwrap()).unwrap(),
                &mut input,
                &mut output,
                &arguments,
            )
            .unwrap(),
            _ => vm::run_with_io_and_arguments(
                &bytecode::lower(&program).unwrap(),
                &mut input,
                &mut output,
                &arguments,
            )
            .unwrap(),
        }
        assert_eq!(
            output.into_inner(),
            b"1\nLine{text: \"line\\n\"}\nValue{text: \"arg\"}\n"
        );
        assert_eq!(input.into_inner().unwrap().position(), 5);
    }
}

#[test]
fn returned_strings_obey_the_heap_limit_and_release_between_calls() {
    let mut program = hir(r#"import "std:env" as env;
        for (mut i: i64 = 0; i < 8; i = i + 1) {
            match env::arg(0) {
                env::Argument::Value { text: text } => { print(text); },
                env::Argument::Missing {} => {},
            }
        }"#);
    program.string_heap_limit = 6;
    for route in 0..4 {
        assert_eq!(
            execute(route, &program, &snapshot(&["abcdef"])).unwrap(),
            "abcdef\n".repeat(8)
        );
    }
}

mod integer_parsing;

// Share the required external-tool harnesses with source-formatting comparisons.
#[path = "../formatting/routes.rs"]
mod formatting;
