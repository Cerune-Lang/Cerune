use super::{
    c::{compare, run},
    directory, execute, hir, process, snapshot,
};
use cerune_lang::{
    codegen::qbe::{self, Target},
    ir,
    runtime::arguments::{Arguments, Code, Limits},
};
use std::{
    ffi::OsStr,
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
fn target() -> Target {
    if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    }
}
fn artifact(p: &ir::Program, limits: Limits) -> String {
    qbe::emit_qbe_with_argument_limits(p, Some(target()), limits).unwrap()
}
fn build(text: &str, object: Option<Target>) -> Option<(PathBuf, PathBuf)> {
    if object.is_none() && !cfg!(any(windows, target_os = "linux")) {
        eprintln!("QBE arguments execution requires Linux/Windows");
        return None;
    }
    let configured = std::env::var_os("CERUNE_TEST_QBE");
    let qbe = configured.clone().unwrap_or_else(|| "qbe".into());
    if !Command::new(&qbe)
        .arg("-h")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(configured.is_none(), "required QBE unavailable: {qbe:?}");
        eprintln!("QBE arguments test skipped; set CERUNE_TEST_QBE to require it");
        return None;
    }
    let cc = std::env::var_os("CERUNE_TEST_QBE_CLANG").unwrap_or_else(|| "clang".into());
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = directory(&format!("qbe-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let source = dir.join("program.ssa");
    let assembly = dir.join("program.s");
    let exe = dir.join(if object.is_some() {
        "program.o"
    } else if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&source, text).unwrap();
    let selected = object.unwrap_or_else(target);
    let out = process::bounded_output(
        Command::new(qbe)
            .args(["-t", selected.qbe_name(), "-o"])
            .arg(&assembly)
            .arg(&source),
        &dir,
        "arguments-qbe-generate",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        out.status.success(),
        "{}\nartifact: {}",
        String::from_utf8_lossy(&out.stderr),
        source.display()
    );
    // Older non-Windows Clang assemblers reject QBE's quoted COFF sections.
    // Always validate both IL targets through QBE; Windows CI assembles COFF.
    if object == Some(Target::X86_64PcWindowsMsvc) && !cfg!(windows) {
        assert!(!fs::read(&assembly).unwrap().is_empty());
        return Some((dir, assembly));
    }
    let mut command = Command::new(cc);
    command
        .args(["-target", selected.triple()])
        .arg(&assembly)
        .arg("-o")
        .arg(&exe);
    if object.is_some() {
        command.arg("-c");
    }
    let out = process::bounded_output(
        &mut command,
        &dir,
        "arguments-qbe-link",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        out.status.success(),
        "{}\nartifact: {}",
        String::from_utf8_lossy(&out.stderr),
        assembly.display()
    );
    Some((dir, exe))
}
fn harness(p: &ir::Program, globals: &str, body: &str) -> String {
    let text = artifact(p, Limits::default());
    let text = text.split("# Explicit target startup:").next().unwrap();
    format!("{text}\n{globals}\nexport function w $main() {{\n@start\n{body}\n}}\n")
}
const VALUES: &str = "data $test_data = { b 83, b 69, b 67, b 82, b 69, b 84 }\ndata $test_values = { l $test_data, l 6 }";
#[test]
fn qbe_arguments_require_target_and_validate_both_targets_without_mutating_ir() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    let original = p.clone();
    assert!(
        qbe::emit_qbe(&p)
            .unwrap_err()
            .message()
            .contains("arguments require an explicit --target")
    );
    assert!(
        qbe::emit_qbe_with_argument_limits(
            &p,
            Some(target()),
            Limits {
                max_count: u64::MAX,
                max_bytes: 0
            }
        )
        .unwrap_err()
        .message()
        .contains("i64::MAX")
    );
    for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
        let text = qbe::emit_qbe_with_target(&p, Some(target)).unwrap();
        assert!(text.contains("$cerune_arguments_capture("));
        assert!(text.contains("$cerune_arguments_get("));
        assert_eq!(
            text.contains("$WideCharToMultiByte"),
            target == Target::X86_64PcWindowsMsvc
        );
        assert_eq!(
            text.contains("export function w $wmain("),
            target == Target::X86_64PcWindowsMsvc
        );
        if build(&text, Some(target)).is_none() {
            return;
        }
    }
    assert_eq!(p, original);
}
#[test]
fn qbe_example_keeps_empty_unicode_quotes_and_option_like_values() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    let Some((dir, exe)) = build(&artifact(&p, Limits::default()), None) else {
        return;
    };
    let c_executable = super::c::build(&cerune_lang::codegen::emit_c(&p).unwrap(), "-O2", &[]);
    for values in [
        vec![],
        vec![""],
        vec![
            "日本語",
            "a\r\nb",
            "e\u{301}",
            "🙂",
            "a\" b\\",
            "--ssa",
            "--",
        ],
    ] {
        let os_values = values.iter().map(OsStr::new).collect::<Vec<_>>();
        let actual = run(&dir, &exe, &os_values);
        compare(&p, &values, &actual);
        if let Some((c_dir, c_exe)) = &c_executable {
            let c_output = run(c_dir, c_exe, &os_values);
            assert_eq!(c_output.status.success(), actual.status.success());
            assert_eq!(c_output.stdout, actual.stdout);
            assert_eq!(c_output.stderr, actual.stderr);
        }
    }
}
#[test]
fn qbe_ownership_order_short_circuit_and_failures_match_four_executors() {
    let source = r#"import "std:env" as env;
        fn text(i: i64) -> string { match env::arg(i) {
            env::Argument::Value { text: t } => { return t; },
            env::Argument::Missing {} => { return "missing"; },
        } }
        fn index() -> i64 { print("index"); return 0; }
        print(false && byte_len(text(-1)) == 0);
        mut a: [string; 2] = [text(index()), text(1)];
        b: [string; 2] = a; a[0] = "changed"; print(a); print(b);
        for (mut i: i64 = 0; i < 8; i = i + 1) { print(text(0)); }
    "#;
    for (source, limit, values) in [
        (source, 64, vec!["first", "second"]),
        (
            "import \"std:env\" as env; fn main() -> void { print(env::arg_count()); print(env::arg(0)); print(env::arg(9223372036854775807)); }",
            64,
            vec!["entry"],
        ),
        (
            "import \"std:env\" as env; print(7); print(env::arg(-1)); print(99);",
            64,
            vec!["SECRET"],
        ),
        (
            "import \"std:env\" as env; print(7); print(env::arg(0)); print(99);",
            4,
            vec!["SECRET"],
        ),
        (
            "import \"std:env\" as env; print(env::arg(0));",
            0,
            vec![""],
        ),
        (
            "import \"std:env\" as env; for (mut i: i64 = 0; i < 8; i = i + 1) { print(env::arg(0)); }",
            6,
            vec!["SECRET"],
        ),
    ] {
        let mut p = hir(source);
        p.string_heap_limit = limit;
        let text = artifact(&p, Limits::default()).replace("  call $cerune_arguments_dispose()\n  ret %result", "  %live =l loadl $cerune_string_live\n  %clean =w ceql %live, 0\n  jnz %clean, @clean, @leaked\n@leaked\n  ret 77\n@clean\n  call $cerune_arguments_dispose()\n  ret %result");
        let Some((dir, exe)) = build(&text, None) else {
            return;
        };
        compare(
            &p,
            &values,
            &run(
                &dir,
                &exe,
                &values.iter().map(OsStr::new).collect::<Vec<_>>(),
            ),
        );
    }
}
#[test]
fn qbe_cli_limits_are_explicit_and_emission_never_consumes_program_arguments() {
    let dir = directory("qbe-cli");
    let file = dir.join("source.ceru");
    fs::write(&file, "import \"std:env\" as env; print(env::arg_count());").unwrap();
    let out = process::bounded_output(
        Command::new(env!("CARGO_BIN_EXE_cerune"))
            .args(["emit-qbe"])
            .arg(&file)
            .args([
                "--target",
                target().triple(),
                "--argument-count-limit",
                "2",
                "--argument-byte-limit",
                "3",
            ]),
        &dir,
        "qbe-cli-limits",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("$cerune_arguments_max_count = { l 2 }"));
    assert!(text.contains("$cerune_arguments_max_bytes = { l 3 }"));
    if let Some((dir, exe)) = build(&text, None) {
        compare(
            &hir(&fs::read_to_string(&file).unwrap()),
            &["あ", ""],
            &run(&dir, &exe, &[OsStr::new("あ"), OsStr::new("")]),
        );
        for (values, expected) in [
            (vec!["", "", ""], "count-limit-exceeded"),
            (vec!["SECRET"], "byte-limit-exceeded index=0"),
            (vec!["ab", "cd"], "byte-limit-exceeded index=1"),
        ] {
            let actual = run(
                &dir,
                &exe,
                &values.iter().map(OsStr::new).collect::<Vec<_>>(),
            );
            assert_eq!(actual.status.code(), Some(1));
            assert!(actual.stdout.is_empty());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: arguments-v1 phase=capture code={expected}\n")
            );
        }
    }
    for flags in [
        vec!["--", "SECRET"],
        vec!["--argument-count-limit", "2", "--argument-count-limit", "3"],
        vec!["--argument-count-limit", "18446744073709551615"],
        vec!["--argument-byte-limit", "-1"],
    ] {
        let out = process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("emit-qbe")
                .arg(&file)
                .args(["--target", target().triple()])
                .args(flags),
            &dir,
            "qbe-invalid-cli",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("SECRET"));
    }
}
#[test]
fn qbe_invalid_os_unicode_fails_before_program_output() {
    let p = hir("import \"std:env\" as env; print(99); print(env::arg_count());");
    let Some((dir, exe)) = build(&artifact(&p, Limits::default()), None) else {
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
fn qbe_arguments_and_stdin_share_utf8_validation_without_changing_order() {
    let p = hir(r#"import "std:env" as env; import "std:io" as io;
        print(env::arg_count()); print(io::read_line(8)); print(env::arg(0));"#);
    let Some((dir, exe)) = build(&artifact(&p, Limits::default()), None) else {
        return;
    };
    let file = dir.join("input.txt");
    fs::write(&file, b"line\n").unwrap();
    let actual = process::bounded_output_with_stdin(
        Command::new(exe).arg("arg"),
        &dir,
        "qbe-arguments-and-stdin",
        Duration::from_secs(30),
        std::process::Stdio::from(fs::File::open(file).unwrap()),
    )
    .unwrap();
    assert!(actual.status.success(), "{actual:?}");
    assert!(actual.stderr.is_empty());
    assert_eq!(
        actual.stdout,
        b"1\nLine{text: \"line\\n\"}\nValue{text: \"arg\"}\n"
    );
}
#[test]
fn qbe_snapshot_owns_contents_and_distinguishes_unconfigured() {
    let p = hir("import \"std:env\" as env; print(env::arg_count()); print(env::arg(0));");
    let text = harness(
        &p,
        VALUES,
        "  call $cerune_arguments_capture(l 1, l $test_values, l 1, l 6)\n  storeb 120, $test_data\n  %result =w call $cerune_program()\n  call $cerune_arguments_dispose()\n  ret %result",
    );
    let Some((dir, exe)) = build(&text, None) else {
        return;
    };
    compare(&p, &["SECRET"], &run(&dir, &exe, &[]));
    let text = harness(&p, "", "  %result =w call $cerune_program()\n  ret %result");
    let Some((dir, exe)) = build(&text, None) else {
        return;
    };
    let (failure, output) = execute(0, &p, &Arguments::unconfigured()).unwrap_err();
    let actual = run(&dir, &exe, &[]);
    assert_eq!(actual.status.code(), Some(1));
    assert_eq!(actual.stdout, output.as_bytes());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: {}\n", failure.record())
    );
}
#[test]
fn qbe_capture_validation_and_fallible_allocations_are_redacted() {
    let p = hir("import \"std:env\" as env; print(7); print(env::arg(0)); print(99);");
    for (globals, body, expected) in [
        (
            "",
            "  call $cerune_arguments_capture(l 0, l 0, l -1, l 0)",
            "invalid-limit",
        ),
        (
            "data $test_data = { b 255, b 0 }\ndata $test_values = { l $test_data, l 2 }",
            "  call $cerune_arguments_capture(l 1, l $test_values, l 1, l 1)",
            "byte-limit-exceeded index=0",
        ),
        (
            "data $test_data = { b 255, b 0 }\ndata $test_values = { l $test_data, l 2 }",
            "  call $cerune_arguments_capture(l 1, l $test_values, l 1, l 2)",
            "invalid-utf8 index=0",
        ),
        (
            "data $test_data = { b 65, b 0 }\ndata $test_values = { l $test_data, l 2 }",
            "  call $cerune_arguments_capture(l 1, l $test_values, l 1, l 2)",
            "embedded-nul index=0",
        ),
    ] {
        let text = harness(&p, globals, &format!("{body}\n  ret 0"));
        let Some((dir, exe)) = build(&text, None) else {
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
    for n in 0..3 {
        let mut text = harness(&p, VALUES, "  call $cerune_arguments_capture(l 1, l $test_values, l 1, l 6)\n  %result =w call $cerune_program()\n  call $cerune_arguments_dispose()\n  ret %result").replace("call $malloc(", "call $test_malloc(");
        text.push_str(&format!("\ndata $test_remaining = {{ l {n} }}\nfunction l $test_malloc(l %size) {{\n@start\n  %left =l loadl $test_remaining\n  %next =l sub %left, 1\n  storel %next, $test_remaining\n  %fail =w ceql %left, 0\n  jnz %fail, @failed, @allocate\n@failed\n  ret 0\n@allocate\n  %p =l call $malloc(l %size)\n  ret %p\n}}\n"));
        let Some((dir, exe)) = build(&text, None) else {
            return;
        };
        let actual = run(&dir, &exe, &[]);
        assert_eq!(actual.status.code(), Some(1));
        let expected = if n < 2 {
            assert!(actual.stdout.is_empty());
            format!(
                "cerune: arguments-v1 phase=capture code=allocation-failed{}\n",
                if n == 0 { "" } else { " index=0" }
            )
        } else {
            let mut limited = p.clone();
            limited.string_heap_limit = 5;
            let (mut failure, output) = execute(0, &limited, &snapshot(&["SECRET"])).unwrap_err();
            failure.code = Code::AllocationFailed;
            assert_eq!(actual.stdout, output.as_bytes());
            format!("cerune: {}\n", failure.record())
        };
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            expected
        );
    }
}

#[test]
fn integer_parsing_qbe_matches_independent_expectations() {
    for group in super::integer_parsing::compiled_cases() {
        let p = &group.program;
        let Some((dir, exe)) = build(&artifact(p, Limits::default()), None) else {
            return;
        };
        for (args, expected) in &group.runs {
            let os: Vec<_> = args.iter().map(OsStr::new).collect();
            super::integer_parsing::check(&run(&dir, &exe, &os), expected);
        }
    }
}
