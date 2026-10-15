use super::*;
use cerune_lang::runtime::input::Code;
fn failure(hir: &cerune_lang::ir::Program, code: Code, consumed: usize, known: bool) -> Failure {
    let error = ir_executor::run(hir).unwrap_err();
    let ir_executor::ErrorKind::Input(f) = error.kind() else {
        panic!("{error:?}")
    };
    let mut f = **f;
    f.code = code;
    f.consumed = consumed;
    f.consumption_known = known;
    f
}
#[test]
fn fixture_matches_known_bytes() {
    let dir = directory("wat-fixture");
    let hir = modules::from_source(include_str!("../fixtures/line_input.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    if let Some(mut cmd) = wat_command(&dir, "input", &generated_wat(&hir)) {
        let out = file_run(
            &mut cmd,
            &dir,
            include_bytes!("../fixtures/line_input.stdin"),
        );
        assert_eq!(out.stdout, include_bytes!("../fixtures/line_input.stdout"));
        compare(
            out,
            &expected(&hir, include_bytes!("../fixtures/line_input.stdin")),
        );
    }
}
#[test]
fn versioned_imports_are_required_only_for_input_and_memory_stays_private() {
    let dir = directory("wat-imports");
    let source = r#"import "std:io" as io;v:io::ReadLine=io::read_line(4);"#;
    let hir = modules::from_source(source).unwrap().to_ir().unwrap();
    let wat = generated_wat(&hir);
    assert!(!wat.contains("(export \"memory\""));
    assert!(!wat.contains("(import \"cerune\" \"memory\""));
    for name in ["stdin_status_v1", "read_stdin_byte_v1"] {
        let Some(mut cmd) = wat_command(&dir, name, &wat) else {
            continue;
        };
        cmd.env("CERUNE_TEST_WAT_INPUT", format!(r#"{{"omit":["{name}"]}}"#));
        let out = file_run(&mut cmd, &dir, b"UNREAD");
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains(name));
    }
    if let Some(cmd) = wat_command(&dir, "old-host", &wat) {
        let mut old = Command::new(cmd.get_program());
        old.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/run_wasm.cjs"))
            .arg(cmd.get_args().nth(1).unwrap());
        let out = file_run(&mut old, &dir, b"UNREAD");
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("stdin_status_v1"));
    }
    let hir = cerune_lang::compile_to_ir("print(123);").unwrap();
    let wat = generated_wat(&hir);
    assert!(!wat.contains("stdin_status_v1"));
    assert!(!wat.contains("read_stdin_byte_v1"));
    if let Some(mut cmd) = wat_command(&dir, "no-input", &wat) {
        cmd.env(
            "CERUNE_TEST_WAT_INPUT",
            r#"{"omit":["stdin_status_v1","read_stdin_byte_v1"]}"#,
        );
        let out = file_run(&mut cmd, &dir, b"UNREAD");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"123\n");
    }
}
#[test]
fn controlled_responses_keep_consumption_and_original_diagnostics() {
    let dir = directory("wat-injection");
    let hir=modules::from_source(r#"import "std:io" as io;print("before");flush();v:io::ReadLine=io::read_line(4);print("after");"#).unwrap().to_ir().unwrap();
    let cases = [
        (r#"{"interrupt":true}"#, None, 0, true),
        (
            r#"{"fault":"error","interrupt":true}"#,
            Some(Code::ReadFailed),
            2,
            true,
        ),
        (r#"{"fault":"byte"}"#, Some(Code::InvalidResponse), 2, false),
        (
            r#"{"fault":"negative-byte"}"#,
            Some(Code::InvalidResponse),
            2,
            false,
        ),
        (
            r#"{"fault":"status"}"#,
            Some(Code::InvalidResponse),
            2,
            false,
        ),
        (
            r#"{"fault":"nonzero-eof"}"#,
            Some(Code::InvalidResponse),
            2,
            false,
        ),
        (
            r#"{"fault":"nonzero-interrupt"}"#,
            Some(Code::InvalidResponse),
            2,
            false,
        ),
        (r#"{"connection":1}"#, Some(Code::Unconfigured), 0, true),
        (
            r#"{"connection":2}"#,
            Some(Code::UnsupportedConnection),
            0,
            true,
        ),
        (r#"{"connection":3}"#, Some(Code::ReadFailed), 0, true),
        (r#"{"connection":4}"#, Some(Code::InvalidResponse), 0, false),
        (
            r#"{"connection":-1}"#,
            Some(Code::InvalidResponse),
            0,
            false,
        ),
    ];
    let Some(mut cmd) = wat_command(&dir, "controlled", &generated_wat(&hir)) else {
        return;
    };
    for (config, code, consumed, known) in cases {
        cmd.env("CERUNE_TEST_WAT_INPUT", config);
        let out = file_run(&mut cmd, &dir, b"abc\n");
        if let Some(code) = code {
            compare(
                out,
                &(
                    b"before\n".to_vec(),
                    Some(failure(&hir, code, consumed, known)),
                ),
            );
        } else {
            compare(out, &expected(&hir, b"abc\n"));
        }
    }
}
#[test]
fn allocation_and_preflight_fail_before_any_read() {
    let dir = directory("wat-preflight");
    for (name, limit, budget, config, code) in [
        ("invalid", 0, 64, r#"{"connection":2}"#, Code::InvalidLimit),
        ("minimum", i64::MIN, 64, r#"{}"#, Code::InvalidLimit),
        (
            "unsupported",
            4,
            0,
            r#"{"connection":2}"#,
            Code::UnsupportedConnection,
        ),
        ("budget", 4, 3, r#"{}"#, Code::BudgetExceeded),
        ("physical", 70000, 70000, r#"{}"#, Code::AllocationFailed),
    ] {
        let source = format!("import \"std:io\" as io;v:io::ReadLine=io::read_line({limit});");
        let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
        hir.string_heap_limit = budget;
        let mut wat = generated_wat(&hir);
        if name == "physical" {
            assert!(wat.contains("(memory 1)"));
            wat = wat.replace("(memory 1)", "(memory 1 1)");
        }
        let Some(mut cmd) = wat_command(&dir, name, &wat) else {
            continue;
        };
        let trace = dir.join("trace.json");
        cmd.env("CERUNE_TEST_WAT_INPUT", config)
            .env("CERUNE_TEST_WAT_INPUT_TRACE", &trace);
        let out = file_run(&mut cmd, &dir, b"UNREAD");
        compare(out, &(Vec::new(), Some(failure(&hir, code, 0, true))));
        let trace = fs::read_to_string(trace).unwrap();
        assert!(trace.contains("\"reads\":0"));
        assert!(trace.contains("\"consumed\":0"));
        if limit <= 0 {
            assert!(!trace.contains("\"connection\""));
        }
    }
}
#[test]
fn failed_flush_and_failed_instance_never_read_again() {
    let dir = directory("wat-stops");
    let hir = modules::from_source(
        r#"import "std:io" as io;print("before");flush();v:io::ReadLine=io::read_line(4);"#,
    )
    .unwrap()
    .to_ir()
    .unwrap();
    if let Some(mut cmd) = wat_command(&dir, "flush", &generated_wat(&hir)) {
        let trace = dir.join("trace.json");
        cmd.env("CERUNE_TEST_WAT_INPUT", r#"{"flush_failure":true}"#)
            .env("CERUNE_TEST_WAT_INPUT_TRACE", &trace);
        let out = file_run(&mut cmd, &dir, b"UNREAD");
        assert!(!out.status.success());
        assert_eq!(out.stdout, b"before\n");
        assert!(String::from_utf8_lossy(&out.stderr).contains("io-v1 operation=flush"));
        assert!(fs::read_to_string(trace).unwrap().contains("\"reads\":0"));
    }
    let hir = modules::from_source(r#"import "std:io" as io;v:io::ReadLine=io::read_line(4);"#)
        .unwrap()
        .to_ir()
        .unwrap();
    if let Some(mut cmd) = wat_command(&dir, "failed-instance", &generated_wat(&hir)) {
        cmd.env("CERUNE_TEST_WAT_INPUT", r#"{"retry_after_failure":true}"#);
        compare(
            file_run(&mut cmd, &dir, b"SECRET\n"),
            &expected(&hir, b"SECRET\n"),
        );
    }
}
