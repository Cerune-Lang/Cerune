use super::super::{Target, emit, lower};
use crate::codegen::test_file_reference::{Step, execute, hir};
use crate::codegen::test_process as process;
use crate::runtime::files::{Files, OpenError};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
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
fn artifact(p: &crate::ir::Program, target: Target, annotate: bool) -> String {
    let mut module = lower(p);
    module.target = Some(target);
    let text = emit::emit_with_origins(&module, annotate);
    let start = text.find("; File OS connection begin").unwrap();
    let end = text.find("; File OS connection end").unwrap();
    let text = text[..start].to_owned() + HOST_DECLARATIONS + &text[end..];
    let start = text.find("; Explicit target startup:").unwrap();
    let end = text.find("; End explicit target startup").unwrap();
    let text = text[..start].to_owned() + &text[end..];
    text.replace(
        "define internal i32 @cerune.program()",
        "define internal i32 @test.program()",
    ) + OWNERSHIP_CHECK
}
fn build(text: &str, host: Option<&str>, opt: &str, target: Target) -> Option<(PathBuf, PathBuf)> {
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
        eprintln!("LLVM files skipped: {cc:?}; set CERUNE_TEST_LLVM_CLANG to require it");
        return None;
    }
    if host.is_some()
        && !cfg!(all(
            target_arch = "x86_64",
            any(windows, target_os = "linux")
        ))
    {
        eprintln!(
            "LLVM file connection execution requires x86-64 Windows/Linux; object checks still run"
        );
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cerune-llvm-files-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let ll = dir.join("program.ll");
    let exe = dir.join(if host.is_none() {
        "program.o"
    } else if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&ll, text).unwrap();
    let mut cmd = Command::new(cc);
    cmd.args([
        "-Werror",
        "-Wno-override-module",
        "-target",
        target.triple(),
        opt,
    ])
    .arg(&ll);
    if let Some(host) = host {
        let c = dir.join("connection.c");
        fs::write(&c, host).unwrap();
        cmd.arg(c);
        if let Ok(flags) = std::env::var("CERUNE_TEST_LLVM_FILE_FLAGS") {
            cmd.args(flags.split_whitespace());
        }
    } else {
        cmd.arg("-c");
    }
    cmd.arg("-o").arg(&exe);
    let result =
        process::bounded_output(&mut cmd, &dir, "llvm-files-build", Duration::from_secs(30))
            .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stderr),
        ll.display()
    );
    Some((dir, exe))
}
use crate::codegen::test_file_connection::Case;

fn compare(source: &str, name: &str, case: &Case, opt: &str, heap_limit: Option<u64>) -> bool {
    let mut p = hir(source);
    if let Some(limit) = heap_limit {
        p.string_heap_limit = limit;
    }
    let before = p.clone();
    let plain = artifact(&p, target(), false);
    let annotated = artifact(&p, target(), true);
    let strip = |s: &str| {
        s.lines()
            .filter(|s| !s.starts_with(";"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip(&plain), strip(&annotated));
    assert_eq!(p, before);
    let Some((dir, exe)) = build(&annotated, Some(&case.host()), opt, target()) else {
        return false;
    };
    let actual = process::bounded_output(
        Command::new(exe).current_dir(&dir),
        &dir,
        "llvm-files-run",
        Duration::from_secs(10),
    )
    .unwrap();
    for route in 0..4 {
        let memory = case.memory(name);
        let mut files = if case.configured {
            Files::with_trace_limit(memory.clone(), 32)
        } else {
            Files::unconfigured()
        };
        let expected = execute(route, &p, &mut files);
        check_result(&actual, expected);
        let state = memory.0.borrow();
        let trace = fs::read_to_string(dir.join("host-trace.txt")).unwrap_or_default();
        assert_eq!(
            trace.lines().collect::<Vec<_>>(),
            state.calls.iter().map(String::as_str).collect::<Vec<_>>(),
            "route {route}"
        );
        if case.write && state.files.contains_key(name) && case.open_error.is_none() && !case.exists
        {
            assert_eq!(
                fs::read(dir.join("host-bytes.bin")).unwrap(),
                state.files[name]
            );
        } else {
            assert!(!dir.join("host-bytes.bin").exists());
        }
    }
    fs::remove_dir_all(dir).unwrap();
    true
}
fn check_result(
    actual: &Output,
    expected: Result<String, (crate::runtime::files::Failure, String)>,
) {
    match expected {
        Ok(text) => {
            assert!(actual.status.success(), "{actual:?}");
            assert_eq!(actual.stdout, text.as_bytes());
            assert!(actual.stderr.is_empty(), "{actual:?}");
        }
        Err((failure, text)) => {
            assert_eq!(actual.status.code(), Some(1), "{actual:?}");
            assert_eq!(actual.stdout, text.as_bytes());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: {}\n", failure.record())
            );
        }
    }
}
#[test]
fn llvm_files_lowering_preserves_origins_and_compiles_both_targets() {
    let p = hir(include_str!("../../../../examples/files/roundtrip.ceru"));
    let before = p.clone();
    let error = super::super::emit_llvm(&p).unwrap_err();
    assert!(
        error
            .message()
            .contains("file I/O requires an explicit --target")
    );
    assert_eq!(error.primary_span(), p.first_file_span());
    for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
        let text = super::super::emit_llvm_with_target(&p, Some(target)).unwrap();
        assert!(text.contains("call %cerune.string @cerune.files.read_text"));
        assert!(text.contains("call void @cerune.files.write_new"));
        for opt in ["-O0", "-O2"] {
            if let Some((dir, _)) = build(&text, None, opt, target) {
                fs::remove_dir_all(dir).unwrap();
            }
        }
    }
    assert_eq!(p, before);
}
#[test]
fn llvm_files_example_and_transfer_failures_match_four_executors() {
    for opt in ["-O0", "-O2"] {
        let mut roundtrip = Case::write();
        roundtrip.steps = vec![Step::Interrupted, Step::Count(1), Step::Count(2)];
        if !compare(
            include_str!("../../../../examples/files/roundtrip.ceru"),
            "sample.txt",
            &roundtrip,
            opt,
            None,
        ) {
            return;
        }
        for write in [false, true] {
            let source = if write {
                r#"import "std:fs" as fs; print(7); fs::write_new("x","abc",3); print(8);"#
            } else {
                r#"import "std:fs" as fs; print(7); print(fs::read_text("x",3)); print(8);"#
            };
            for steps in [
                vec![],
                vec![Step::Count(1), Step::Interrupted, Step::Count(1)],
                vec![Step::Count(1), Step::Fail],
                vec![Step::Count(1), Step::Invalid],
                vec![Step::Zero],
            ] {
                for close_error in [false, true] {
                    let mut case = if write {
                        Case::write()
                    } else {
                        Case::read(b"abc")
                    };
                    case.steps = steps.clone();
                    case.close_error = close_error;
                    compare(source, "x", &case, opt, None);
                }
            }
            for error in [
                OpenError::NotFound,
                OpenError::AlreadyExists,
                OpenError::UnsupportedTarget,
                OpenError::Failed,
                OpenError::InvalidResponse,
            ] {
                let mut case = if write {
                    Case::write()
                } else {
                    Case::read(b"abc")
                };
                case.open_error = Some(error);
                compare(source, "x", &case, opt, None);
            }
        }
    }
}
#[test]
fn llvm_files_validation_limits_and_utf8_match_four_executors() {
    for opt in ["-O0", "-O2"] {
        for (bytes, limit) in [
            (b"".as_slice(), 0),
            (b"a".as_slice(), 0),
            (b"abc".as_slice(), 3),
            (b"abcd".as_slice(), 3),
            (b"\xff".as_slice(), 1),
            (b"ab\xc2".as_slice(), 3),
        ] {
            let source = format!(
                r#"import "std:fs" as fs; print(7); print(fs::read_text("x",{limit})); print(8);"#
            );
            if !compare(&source, "x", &Case::read(bytes), opt, None) {
                return;
            }
        }
        let source = r#"import "std:fs" as fs; print(7); print(fs::read_text("x",3)); print(8);"#;
        compare(source, "x", &Case::read(b"abc"), opt, Some(2));
        for limit in [-1_i64, 2147483648] {
            let source = format!(
                r#"import "std:fs" as fs; print(7); print(fs::read_text("../private",{limit})); print(8);"#
            );
            compare(
                &source,
                "../private",
                &Case {
                    configured: false,
                    ..Case::read(b"")
                },
                opt,
                None,
            );
        }
        compare(
            source,
            "x",
            &Case {
                configured: false,
                ..Case::read(b"")
            },
            opt,
            None,
        );
        compare(
            r#"import "std:fs" as fs; fs::write_new("x","",0); print(8);"#,
            "x",
            &Case::write(),
            opt,
            None,
        );
        compare(
            r#"import "std:fs" as fs; fs::write_new("x","abc",2); print(8);"#,
            "x",
            &Case::write(),
            opt,
            None,
        );
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "x:",
            "a\u{7f}",
            "a ",
            "a.",
            "con",
            "con .txt",
            "CONIN$",
            "ConOut$.a",
            "com1",
            "LPT9.a",
            "COM¹",
            "lpt².txt",
            "COM³",
            "COM0",
            "COM10",
            "COM¹x",
            "LPT0",
            ".hidden",
            "日本語",
            "e\u{301}",
            "é",
        ] {
            let literal = serde_json::to_string(name).unwrap();
            let source =
                format!("import \"std:fs\" as fs; fs::write_new({literal},\"abc\",3); print(8);");
            compare(&source, name, &Case::write(), opt, None);
        }
        for size in [255, 256] {
            let name = "a".repeat(size);
            let source = format!("import \"std:fs\" as fs; fs::write_new(\"{name}\",\"abc\",3);");
            compare(&source, &name, &Case::write(), opt, None);
        }
    }
}
#[test]
fn llvm_files_evaluation_short_circuit_recursion_and_owned_copies() {
    let source = r#"import "std:fs" as fs;
        fn name()->string { print(1); return "x"; }
        fn limit()->i64 { print(2); return 3; }
        fn read(n:i64)->string { if n==0 { return fs::read_text(name(),limit()); } return read(n-1); }
        print(false && byte_len(fs::read_text("absent",3))==0);
        mut a:[string;2]=[read(3),"unchanged"]; b:[string;2]=a;
        a[0]="changed"; print(b[0]); print(a[0]); print(b[1]);
        "#;
    for opt in ["-O0", "-O2"] {
        if !compare(source, "x", &Case::read(b"abc"), opt, None) {
            return;
        }
        compare(
            r#"import "std:fs" as fs;
            fn name()->string { print(1); return "x"; }
            fn text()->string { print(2); return concat("a","bc"); }
            fn limit()->i64 { print(3); return 3; }
            fs::write_new(name(),text(),limit()); print(4);"#,
            "x",
            &Case::write(),
            opt,
            None,
        );
        let loop_source = r#"import "std:fs" as fs; mut i:i64=0; while i<20 { text:string=fs::read_text("x",3); print(text); i=i+1; }"#;
        compare(loop_source, "x", &Case::read(b"abc"), opt, Some(3));
    }
}

const OWNERSHIP_CHECK: &str = r#"
define i32 @main() {
entry:
  %result = call i32 @test.program()
  %live = load i64, ptr @cerune.string.live
  %owners = load ptr, ptr @cerune.string.owners
  %empty = icmp eq i64 %live, 0
  %released = icmp eq ptr %owners, null
  %ok = and i1 %empty, %released
  %exit = select i1 %ok, i32 %result, i32 94
  ret i32 %exit
}
"#;

#[test]
fn llvm_files_large_transfers_preserve_chunk_requests_and_eof_probe() {
    for opt in ["-O0", "-O2"] {
        for (size, limit) in [(4096, 4096), (4097, 4096), (8193, 8193)] {
            let source = format!(
                r#"import "std:fs" as fs; print(7); print(byte_len(fs::read_text("x",{limit}))); print(8);"#
            );
            if !compare(&source, "x", &Case::read(&vec![b'a'; size]), opt, None) {
                return;
            }
        }
        let source = r#"import "std:fs" as fs; print(7); print(byte_len(fs::read_text("x",8193))); print(8);"#;
        let case = Case {
            steps: vec![Step::Count(4096), Step::Interrupted, Step::Invalid],
            ..Case::read(&vec![b'a'; 8193])
        };
        compare(source, "x", &case, opt, None);
    }
}
#[test]
fn llvm_files_allocation_failure_precedes_open_and_redacts_input() {
    use crate::runtime::files::{
        Cleanup, Code, Created, Details, Failure, Operation, Origin, Phase,
    };
    let p = hir(
        r#"import "std:fs" as fs; print(7); print(fs::read_text("private-name",3)); print(8);"#,
    );
    let module = lower(&p);
    let origin = module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .find(|i| {
            matches!(
                i.instruction,
                super::super::ir::Instruction::FileReadText { .. }
            )
        })
        .unwrap()
        .origin;
    let super::super::ir::Origin::Source { node_id, span } = origin else {
        panic!()
    };
    let failure = Failure {
        code: Code::AllocationFailed,
        details: Details {
            operation: Operation::ReadText,
            phase: Phase::Reserve,
            limit: 3,
            requested: None,
            transferred: 0,
            progress_known: true,
            created: Created::No,
            cleanup: Cleanup::NotAttempted,
            origin: Origin { node_id, span },
        },
    };
    // LLVM内の確保だけを失敗させます。試験接続やプロセス起動の確保には影響しません。
    let text = artifact(&p, target(), true).replace("@malloc(", "@cerune_test_malloc(");
    let host = Case::read(b"abc").host()
        + "\nvoid *cerune_test_malloc(uint64_t n) { (void)n; return NULL; }\n";
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&text, Some(&host), opt, target()) else {
            return;
        };
        let out = process::bounded_output(
            Command::new(exe).current_dir(&dir),
            &dir,
            "llvm-files-allocation",
            Duration::from_secs(10),
        )
        .unwrap();
        check_result(&out, Err((failure, "7\n".into())));
        assert!(!dir.join("host-trace.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}

const HOST_DECLARATIONS: &str = r#"
declare i32 @cerune_file_host_configured()
declare i32 @cerune_file_host_open(ptr, i64, i32, ptr)
declare i64 @cerune_file_host_read(i64, ptr, i64)
declare i64 @cerune_file_host_write(i64, ptr, i64)
declare i32 @cerune_file_host_close(i64)
"#;
