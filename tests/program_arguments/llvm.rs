use super::{
    c::{compare, run},
    directory, execute, hir, process, snapshot,
};
use cerune_lang::{
    codegen::llvm::{self, Options, Target},
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
    llvm::emit_llvm_with_argument_limits(
        p,
        Options {
            target: Some(target()),
            annotate_origins: true,
        },
        limits,
    )
    .unwrap()
}
fn build(text: &str, opt: &str, object: Option<Target>) -> Option<(PathBuf, PathBuf)> {
    if object.is_none() && !cfg!(any(windows, target_os = "linux")) {
        eprintln!(
            "LLVM arguments execution needs a supported Linux/Windows target; object validation still runs"
        );
        return None;
    }
    let configured = std::env::var_os("CERUNE_TEST_LLVM_CLANG");
    let cc = configured.clone().unwrap_or_else(|| "clang".into());
    if !Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(
            configured.is_none(),
            "required LLVM compiler unavailable: {cc:?}"
        );
        eprintln!("LLVM arguments test skipped: {cc:?}; set CERUNE_TEST_LLVM_CLANG to require it");
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = directory(&format!("llvm-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let source = dir.join("program.ll");
    let exe = dir.join(if object.is_some() {
        "program.o"
    } else if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&source, text).unwrap();
    let mut command = Command::new(cc);
    command
        .args([
            "-Werror",
            "-Wno-override-module",
            opt,
            "-target",
            object.unwrap_or_else(target).triple(),
        ])
        .arg(&source)
        .arg("-o")
        .arg(&exe);
    if object.is_some() {
        command.arg("-c");
    }
    let out = process::bounded_output(
        &mut command,
        &dir,
        "arguments-llvm-build",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        out.status.success(),
        "{}\nartifact: {}",
        String::from_utf8_lossy(&out.stderr),
        source.display()
    );
    Some((dir, exe))
}
fn harness(p: &ir::Program, globals: &str, body: &str) -> String {
    let text = artifact(p, Limits::default());
    let text = text.split("; Explicit target startup:").next().unwrap();
    format!("{text}\n{globals}\ndefine i32 @main() {{\nentry:\n{body}\n}}\n")
}
const VALUES: &str = r#"
@test.data = internal global [6 x i8] c"SECRET"
@test.values = internal global [1 x %cerune.string] [%cerune.string { ptr @test.data, i64 6 }]
"#;
#[test]
fn llvm_arguments_require_target_and_validate_both_targets_without_mutating_ir() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    let original = p.clone();
    assert!(
        llvm::emit_llvm(&p)
            .unwrap_err()
            .message()
            .contains("arguments require an explicit --target")
    );
    assert!(
        llvm::emit_llvm_with_argument_limits(
            &p,
            Options {
                target: Some(target()),
                annotate_origins: false
            },
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
        let plain = llvm::emit_llvm_with_target(&p, Some(target)).unwrap();
        let annotated = llvm::emit_llvm_with_options(
            &p,
            Options {
                target: Some(target),
                annotate_origins: true,
            },
        )
        .unwrap();
        let strip = |s: &str| {
            s.lines()
                .filter(|s| !s.trim_start().starts_with(';'))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(strip(&plain), strip(&annotated));
        assert!(annotated.contains("@cerune.arguments.capture("));
        assert!(annotated.contains("@cerune.arguments.get("));
        assert_eq!(
            annotated.contains("@WideCharToMultiByte"),
            target == Target::X86_64PcWindowsMsvc
        );
        assert_eq!(
            annotated.contains("define i32 @wmain("),
            target == Target::X86_64PcWindowsMsvc
        );
        for opt in ["-O0", "-O2"] {
            if build(&annotated, opt, Some(target)).is_none() {
                return;
            }
        }
    }
    assert_eq!(p, original);
}
#[test]
fn llvm_example_keeps_empty_unicode_quotes_and_option_like_values() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&artifact(&p, Limits::default()), opt, None) else {
            return;
        };
        let c_executable = super::c::build(&cerune_lang::codegen::emit_c(&p).unwrap(), opt, &[]);
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
}
#[test]
fn llvm_ownership_order_short_circuit_and_failures_match_four_executors() {
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
        let text = artifact(&p, Limits::default()).replace("  call void @cerune.arguments.dispose()\n  ret i32 %result", "  %live = load i64, ptr @cerune.string.live\n  %clean = icmp eq i64 %live, 0\n  %status = select i1 %clean, i32 %result, i32 77\n  call void @cerune.arguments.dispose()\n  ret i32 %status");
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&text, opt, None) else {
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
}
#[test]
fn llvm_cli_limits_are_explicit_and_emission_never_consumes_program_arguments() {
    let dir = directory("llvm-cli");
    let file = dir.join("source.ceru");
    fs::write(&file, "import \"std:env\" as env; print(env::arg_count());").unwrap();
    let out = process::bounded_output(
        Command::new(env!("CARGO_BIN_EXE_cerune"))
            .args(["emit-llvm"])
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
        "llvm-cli-limits",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("@cerune.arguments.max_count = internal constant i64 2"));
    assert!(text.contains("@cerune.arguments.max_bytes = internal constant i64 3"));
    if let Some((dir, exe)) = build(&text, "-O2", None) {
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
                .arg("emit-llvm")
                .arg(&file)
                .args(["--target", target().triple()])
                .args(flags),
            &dir,
            "llvm-invalid-cli",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("SECRET"));
    }
}
#[test]
fn llvm_snapshot_owns_contents_and_distinguishes_unconfigured() {
    let p = hir("import \"std:env\" as env; print(env::arg_count()); print(env::arg(0));");
    let text = harness(
        &p,
        VALUES,
        "  call void @cerune.arguments.capture(i64 1, ptr @test.values, i64 1, i64 6)\n  store i8 120, ptr @test.data\n  %result = call i32 @cerune.program()\n  call void @cerune.arguments.dispose()\n  ret i32 %result",
    );
    let Some((dir, exe)) = build(&text, "-O2", None) else {
        return;
    };
    compare(&p, &["SECRET"], &run(&dir, &exe, &[]));
    let text = harness(
        &p,
        "",
        "  %result = call i32 @cerune.program()\n  ret i32 %result",
    );
    let Some((dir, exe)) = build(&text, "-O2", None) else {
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
fn llvm_capture_validation_and_fallible_allocations_are_redacted() {
    let p = hir("import \"std:env\" as env; print(7); print(env::arg(0)); print(99);");
    for (globals, body, expected) in [
        (
            "",
            "  call void @cerune.arguments.capture(i64 0, ptr null, i64 -1, i64 0)",
            "invalid-limit",
        ),
        (
            r#"@test.data = global [2 x i8] c"\FF\00"
@test.values = global [1 x %cerune.string] [%cerune.string { ptr @test.data, i64 2 }]"#,
            "  call void @cerune.arguments.capture(i64 1, ptr @test.values, i64 1, i64 1)",
            "byte-limit-exceeded index=0",
        ),
        (
            r#"@test.data = global [2 x i8] c"\FF\00"
@test.values = global [1 x %cerune.string] [%cerune.string { ptr @test.data, i64 2 }]"#,
            "  call void @cerune.arguments.capture(i64 1, ptr @test.values, i64 1, i64 2)",
            "invalid-utf8 index=0",
        ),
        (
            r#"@test.data = global [2 x i8] c"A\00"
@test.values = global [1 x %cerune.string] [%cerune.string { ptr @test.data, i64 2 }]"#,
            "  call void @cerune.arguments.capture(i64 1, ptr @test.values, i64 1, i64 2)",
            "embedded-nul index=0",
        ),
    ] {
        let text = harness(&p, globals, &format!("{body}\n  ret i32 0"));
        let Some((dir, exe)) = build(&text, "-O2", None) else {
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
        let mut text = harness(&p, VALUES, "  call void @cerune.arguments.capture(i64 1, ptr @test.values, i64 1, i64 6)\n  %result = call i32 @cerune.program()\n  call void @cerune.arguments.dispose()\n  ret i32 %result").replace("call ptr @malloc(","call ptr @test.malloc(");
        text.push_str(&format!("\n@test.remaining = global i64 {n}\ndefine ptr @test.malloc(i64 %size) {{\nentry:\n  %left = load i64, ptr @test.remaining\n  %next = sub i64 %left, 1\n  store i64 %next, ptr @test.remaining\n  %fail = icmp eq i64 %left, 0\n  br i1 %fail, label %failed, label %allocate\nfailed:\n  ret ptr null\nallocate:\n  %p = call ptr @malloc(i64 %size)\n  ret ptr %p\n}}\n"));
        let Some((dir, exe)) = build(&text, "-O2", None) else {
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
fn llvm_invalid_os_unicode_fails_before_program_output() {
    let p = hir("import \"std:env\" as env; print(99); print(env::arg_count());");
    let Some((dir, exe)) = build(&artifact(&p, Limits::default()), "-O2", None) else {
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
fn llvm_arguments_and_stdin_share_utf8_validation_without_changing_order() {
    let p = hir(r#"import "std:env" as env; import "std:io" as io;
        print(env::arg_count()); print(io::read_line(8)); print(env::arg(0));"#);
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&artifact(&p, Limits::default()), opt, None) else {
            return;
        };
        let file = dir.join("input.txt");
        fs::write(&file, b"line\n").unwrap();
        let actual = process::bounded_output_with_stdin(
            Command::new(exe).arg("arg"),
            &dir,
            "llvm-arguments-and-stdin",
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
}

#[test]
fn integer_parsing_llvm_matches_independent_expectations() {
    for group in super::integer_parsing::compiled_cases() {
        let p = &group.program;
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&artifact(p, Limits::default()), opt, None) else {
                return;
            };
            for (args, expected) in &group.runs {
                let os: Vec<_> = args.iter().map(OsStr::new).collect();
                super::integer_parsing::check(&run(&dir, &exe, &os), expected);
            }
        }
    }
}

#[test]
fn optional_let_formatting_llvm_preserves_execution() {
    for (p, expected) in super::formatting::cases() {
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&artifact(&p, Limits::default()), opt, None) else {
                return;
            };
            super::formatting::check(&run(&dir, &exe, &[]), expected);
        }
    }
}
