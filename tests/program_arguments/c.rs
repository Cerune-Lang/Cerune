use super::{directory, execute, hir, process, snapshot};
use cerune_lang::{codegen, ir, runtime::arguments::Arguments};
use std::{
    ffi::OsStr,
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

fn build(text: &str, opt: &str, flags: &[&str]) -> Option<(PathBuf, PathBuf)> {
    let configured = std::env::var_os("CERUNE_TEST_CC");
    let cc = configured
        .clone()
        .unwrap_or_else(|| if cfg!(windows) { "clang" } else { "cc" }.into());
    if !Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(
            configured.is_none(),
            "required C compiler unavailable: {cc:?}"
        );
        eprintln!("C arguments test skipped: {cc:?} unavailable; set CERUNE_TEST_CC to require it");
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = directory(&format!("c-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let source = dir.join("program.c");
    let exe = dir.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&source, text).unwrap();
    let mut command = Command::new(&cc);
    command
        .args(["-std=c11", "-pedantic-errors", opt])
        .args(flags)
        .arg(&source)
        .arg("-o")
        .arg(&exe);
    if cfg!(windows) && text.contains("int wmain(") {
        let target = Command::new(&cc).arg("-dumpmachine").output().unwrap();
        let target = String::from_utf8_lossy(&target.stdout);
        if target.contains("mingw") || target.contains("windows-gnu") {
            command.arg("-municode");
        }
    }
    if !cfg!(windows) {
        command.arg("-lm");
    }
    if std::env::var_os("CERUNE_TEST_SANITIZE").is_some() {
        command.args([
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-fno-omit-frame-pointer",
        ]);
    }
    let built = process::bounded_output(
        &mut command,
        &dir,
        "arguments-c-build",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        built.status.success(),
        "{}\n{text}",
        String::from_utf8_lossy(&built.stderr)
    );
    Some((dir, exe))
}
fn run(dir: &std::path::Path, exe: &std::path::Path, args: &[&OsStr]) -> Output {
    process::bounded_output(
        Command::new(exe).args(args),
        dir,
        "arguments-c-run",
        Duration::from_secs(30),
    )
    .unwrap()
}
fn compare(program: &ir::Program, values: &[&str], actual: &Output) {
    let args = snapshot(values);
    let expected = execute(0, program, &args);
    for route in 1..4 {
        assert_eq!(execute(route, program, &args), expected);
    }
    match expected {
        Ok(output) => {
            assert!(actual.status.success(), "{actual:?}");
            assert_eq!(actual.stdout, output.as_bytes());
            assert!(actual.stderr.is_empty(), "{actual:?}");
        }
        Err((failure, output)) => {
            assert_eq!(actual.status.code(), Some(1), "{actual:?}");
            assert_eq!(actual.stdout, output.as_bytes());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: {}\n", failure.record())
            );
        }
    }
}
fn harness(program: &ir::Program, body: &str) -> String {
    let text = codegen::emit_c(program).unwrap();
    let text = text
        .split("/* Selected by the C compilation target")
        .next()
        .unwrap();
    format!("{text}\nint main(void) {{ {body} }}\n")
}
#[test]
fn generated_c_example_preserves_os_arguments_at_both_optimization_levels() {
    let program = hir(include_str!("../../examples/arguments/list.ceru"));
    let text = codegen::emit_c(&program).unwrap();
    assert!(text.contains("int wmain("));
    assert!(text.contains("WC_ERR_INVALID_CHARS"));
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&text, opt, &[]) else {
            return;
        };
        for values in [
            vec![],
            vec![""],
            vec![
                "日本語",
                "a\r\nb",
                "--ssa",
                "e\u{301}",
                "🙂",
                "a\" b\\",
                "--",
            ],
        ] {
            let os: Vec<_> = values.iter().map(OsStr::new).collect();
            compare(&program, &values, &run(&dir, &exe, &os));
        }
    }
}
#[test]
fn generated_c_ownership_evaluation_order_and_heap_failures_match_executors() {
    let source = r#"
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
        print(a); print(b);
        for (mut i: i64 = 0; i < 8; i = i + 1) { print(text(0)); }
    "#;
    for (source, limit) in [
        (source, 64),
        (
            "import \"std:env\" as env; print(7); print(env::arg(-1)); print(99);",
            64,
        ),
        (
            "import \"std:env\" as env; print(7); print(env::arg(0)); print(99);",
            4,
        ),
    ] {
        let mut program = hir(source);
        program.string_heap_limit = limit;
        for opt in ["-O0", "-O2"] {
            let text = codegen::emit_c(&program).unwrap().replace(
                "    return 0;\n}",
                "    if (cerune_string_live_bytes != 0) return 77;\n    return 0;\n}",
            );
            let Some((dir, exe)) = build(&text, opt, &[]) else {
                return;
            };
            compare(
                &program,
                &["first", "second"],
                &run(&dir, &exe, &[OsStr::new("first"), OsStr::new("second")]),
            );
        }
    }
    let mut empty = hir("import \"std:env\" as env; print(env::arg(0));");
    empty.string_heap_limit = 0;
    if let Some((dir, exe)) = build(&codegen::emit_c(&empty).unwrap(), "-O2", &[]) {
        compare(&empty, &[""], &run(&dir, &exe, &[OsStr::new("")]));
    }
}
#[test]
fn generated_c_capture_limits_validate_before_execution_and_redact_contents() {
    let p = hir("import \"std:env\" as env; print(99); print(env::arg_count());");
    let text = codegen::emit_c(&p).unwrap();
    let flags = [
        "-DCERUNE_ARGUMENT_COUNT_LIMIT=2",
        "-DCERUNE_ARGUMENT_BYTE_LIMIT=3",
    ];
    let Some((dir, exe)) = build(&text, "-O2", &flags) else {
        return;
    };
    compare(
        &p,
        &["あ", ""],
        &run(&dir, &exe, &[OsStr::new("あ"), OsStr::new("")]),
    );
    for (args, suffix) in [
        (vec!["", "", ""], "count-limit-exceeded"),
        (vec!["SECRET"], "byte-limit-exceeded index=0"),
        (vec!["ab", "cd"], "byte-limit-exceeded index=1"),
    ] {
        let actual = run(&dir, &exe, &args.iter().map(OsStr::new).collect::<Vec<_>>());
        assert_eq!(actual.status.code(), Some(1));
        assert!(actual.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: arguments-v1 phase=capture code={suffix}\n")
        );
    }
}
#[test]
fn generated_c_snapshot_is_independent_and_unconfigured_is_not_empty() {
    let p = hir("import \"std:env\" as env; print(env::arg_count()); print(env::arg(0));");
    let text = harness(
        &p,
        r#"
        unsigned char data[] = "before";
        cerune_string values[] = {{data, 6}};
        cerune_arguments_capture(1, values, 1, 6);
        memset(data, 'x', 6);
        int result = cerune_program();
        if (cerune_string_live_bytes != 0) return 77;
        cerune_arguments_dispose(); return result;
    "#,
    );
    let Some((dir, exe)) = build(&text, "-O2", &[]) else {
        return;
    };
    compare(&p, &["before"], &run(&dir, &exe, &[]));
    let text = harness(&p, "return cerune_program();");
    if let Some((dir, exe)) = build(&text, "-O2", &[]) {
        let actual = run(&dir, &exe, &[]);
        let (failure, output) = execute(0, &p, &Arguments::unconfigured()).unwrap_err();
        assert_eq!(actual.status.code(), Some(1));
        assert_eq!(actual.stdout, output.as_bytes());
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: {}\n", failure.record())
        );
    }
}
#[test]
fn generated_c_raw_validation_precedence_and_allocation_failures() {
    let p = hir("import \"std:env\" as env; print(env::arg_count());");
    for (body, expected) in [
        (
            "cerune_arguments_capture(0, NULL, UINT64_MAX, 0);",
            "invalid-limit",
        ),
        (
            "unsigned char b[] = {255, 0}; cerune_string v[] = {{b, 2}}; cerune_arguments_capture(1, v, 1, 1);",
            "byte-limit-exceeded index=0",
        ),
        (
            "unsigned char b[] = {255, 0}; cerune_string v[] = {{b, 2}}; cerune_arguments_capture(1, v, 1, 2);",
            "invalid-utf8 index=0",
        ),
        (
            "unsigned char b[] = {65, 0}; cerune_string v[] = {{b, 2}}; cerune_arguments_capture(1, v, 1, 2);",
            "embedded-nul index=0",
        ),
    ] {
        let Some((dir, exe)) = build(&harness(&p, body), "-O2", &[]) else {
            return;
        };
        let actual = run(&dir, &exe, &[]);
        assert_eq!(actual.status.code(), Some(1));
        assert!(actual.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: arguments-v1 phase=capture code={expected}\n")
        );
    }
    for (fail_at, expected) in [(0, "allocation-failed"), (1, "allocation-failed index=0")] {
        let text = format!(
            "#include <stdlib.h>\nstatic int remaining = {fail_at};\nstatic void *test_malloc(size_t n) {{ return remaining-- == 0 ? NULL : malloc(n); }}\n#define malloc test_malloc\n{}",
            harness(
                &p,
                "cerune_string v[] = {{(const unsigned char *)\"SECRET\", 6}}; cerune_arguments_capture(1, v, 1, 6); return 0;"
            )
        );
        let Some((dir, exe)) = build(&text, "-O2", &[]) else {
            return;
        };
        let actual = run(&dir, &exe, &[]);
        assert_eq!(actual.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: arguments-v1 phase=capture code={expected}\n")
        );
    }
}
#[test]
fn generated_c_invalid_os_unicode_is_redacted_before_program_execution() {
    let p = hir("import \"std:env\" as env; print(99); print(env::arg_count());");
    let Some((dir, exe)) = build(&codegen::emit_c(&p).unwrap(), "-O2", &[]) else {
        return;
    };
    #[cfg(windows)]
    let bad = {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[83, 0xd800])
    };
    #[cfg(unix)]
    let bad = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(vec![83, 255])
    };
    let actual = run(&dir, &exe, &[&bad]);
    assert_eq!(actual.status.code(), Some(1));
    assert!(actual.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        "cerune: arguments-v1 phase=capture code=invalid-unicode index=0\n"
    );
}

#[test]
fn generated_c_arguments_and_stdin_preserve_order_and_bytes() {
    let p = hir(r#"import "std:env" as env; import "std:io" as io;
        print(env::arg_count()); print(io::read_line(8)); print(env::arg(0));"#);
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&codegen::emit_c(&p).unwrap(), opt, &[]) else {
            return;
        };
        let file = dir.join("input.txt");
        fs::write(&file, b"line\n").unwrap();
        let actual = process::bounded_output_with_stdin(
            Command::new(exe).arg("arg"),
            &dir,
            "arguments-and-stdin",
            Duration::from_secs(30),
            std::process::Stdio::from(fs::File::open(file).unwrap()),
        )
        .unwrap();
        if !cfg!(any(windows, target_os = "linux")) {
            let mut input = cerune_lang::runtime::input::Input::<std::io::Empty>::unconfigured();
            let mut output = cerune_lang::runtime::output::Output::new(Vec::new());
            cerune_lang::ir_executor::run_with_io_and_arguments(
                &p,
                &mut input,
                &mut output,
                &snapshot(&["arg"]),
            )
            .unwrap_err();
            let mut failure = input.failure().unwrap();
            failure.code = cerune_lang::runtime::input::Code::UnsupportedConnection;
            assert_eq!(actual.status.code(), Some(1));
            assert_eq!(actual.stdout, output.into_inner());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: {}\n", failure.record())
            );
            continue;
        }
        assert!(actual.status.success(), "{actual:?}");
        assert!(actual.stderr.is_empty());
        assert_eq!(
            actual.stdout,
            b"1\nLine{text: \"line\\n\"}\nValue{text: \"arg\"}\n"
        );
    }
}
#[test]
fn generated_c_returned_allocation_failure_has_the_source_operation_origin() {
    let p = hir("import \"std:env\" as env; print(7); print(env::arg(0)); print(99);");
    let text = format!(
        "#include <stdlib.h>\nstatic int fail_alloc;\nstatic void *test_malloc(size_t n) {{ return fail_alloc ? NULL : malloc(n); }}\n#define malloc test_malloc\n{}",
        harness(
            &p,
            "cerune_string v[] = {{(const unsigned char *)\"SECRET\", 6}}; cerune_arguments_capture(1, v, 1, 6); fail_alloc = 1; return cerune_program();"
        )
    );
    let Some((dir, exe)) = build(&text, "-O2", &[]) else {
        return;
    };
    let mut limited = p.clone();
    limited.string_heap_limit = 5;
    let (mut failure, expected) = execute(0, &limited, &snapshot(&["SECRET"])).unwrap_err();
    failure.code = cerune_lang::runtime::arguments::Code::AllocationFailed;
    let actual = run(&dir, &exe, &[]);
    assert_eq!(actual.status.code(), Some(1));
    assert_eq!(actual.stdout, expected.as_bytes());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: {}\n", failure.record())
    );
}
