use super::super::{NativeProgram, Target, build_program_from_lir, lower};
use crate::codegen::test_file_connection::Case;
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
fn artifact(p: &crate::ir::Program, t: Target) -> NativeProgram {
    let mir = crate::mir::lower(p).unwrap();
    let module = lower::lower_file_tests(&mir, t).unwrap();
    let mut native = build_program_from_lir(&module, true).unwrap();
    // 試験用mainから、Ceruneの処理終了時に所有領域が回収済みか確認します。
    for s in &mut native.program.symbols {
        match s.name.as_str() {
            "main" => s.name = "cerune_test_program".into(),
            ".Lcerune_heap_live" => {
                s.name = "cerune_test_live".into();
                s.linkage = super::Linkage::Global;
            }
            ".Lcerune_heap_head" => {
                s.name = "cerune_test_head".into();
                s.linkage = super::Linkage::Global;
            }
            _ => {}
        }
    }
    native
}
const CHECK: &str = "\nextern int cerune_test_program(void); extern uint64_t cerune_test_live; extern void *cerune_test_head;\nint main(void) { int r=cerune_test_program(); return (cerune_test_live || cerune_test_head) ? 94 : r; }\n";
fn build(p: &NativeProgram, host: &str, route: &str) -> Option<(PathBuf, PathBuf)> {
    let configured = std::env::var_os("CERUNE_TEST_ASM_CLANG");
    let cc = configured.clone().unwrap_or_else(|| "clang".into());
    if !cfg!(all(
        target_arch = "x86_64",
        any(windows, target_os = "linux")
    )) {
        return None;
    }
    if !Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(configured.is_none(), "required clang unavailable");
        eprintln!("Native files skipped; set CERUNE_TEST_ASM_CLANG to require clang");
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cerune-native-files-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join(if route == "asm" {
        "program.s"
    } else {
        "program.o"
    });
    if route == "asm" {
        fs::write(&input, p.render_asm().unwrap()).unwrap();
    } else {
        fs::write(&input, p.assemble().unwrap().object).unwrap();
    }
    let c = dir.join("connection.c");
    fs::write(&c, host.to_owned() + CHECK).unwrap();
    let exe = dir.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let result = process::bounded_output(
        Command::new(cc)
            .args(["-Werror", "-target", target().triple()])
            .arg(&input)
            .arg(c)
            .arg("-o")
            .arg(&exe),
        &dir,
        "native-files-link",
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stderr),
        input.display()
    );
    Some((dir, exe))
}
fn compare(source: &str, name: &str, case: &Case, route: &str, heap_limit: Option<u64>) -> bool {
    let mut p = hir(source);
    if let Some(n) = heap_limit {
        p.string_heap_limit = n;
    }
    let before = p.clone();
    let native = artifact(&p, target());
    assert_eq!(p, before);
    let Some((dir, exe)) = build(&native, &case.host(), route) else {
        return false;
    };
    let actual = process::bounded_output(
        Command::new(exe).current_dir(&dir),
        &dir,
        "native-files-run",
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
        check_result(&actual, execute(route, &p, &mut files));
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
        Err((f, text)) => {
            assert_eq!(actual.status.code(), Some(1), "{actual:?}");
            assert_eq!(actual.stdout, text.as_bytes());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: {}\n", f.record())
            );
        }
    }
}
#[test]
fn native_files_lowering_preserves_origins_and_both_object_targets() {
    let p = hir(include_str!("../../../../examples/files/roundtrip.ceru"));
    let mir = crate::mir::lower(&p).unwrap();
    for t in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
        assert!(
            super::super::lower_mir(&mir, t).is_err(),
            "OS connection is not public yet"
        );
        let m = lower::lower_file_tests(&mir, t).unwrap();
        let native = build_program_from_lir(&m, true).unwrap();
        let asm = native.render_asm().unwrap();
        assert!(asm.contains("cerune_file_read_text"));
        assert!(asm.contains("cerune_file_write_new"));
        assert!(!native.assemble().unwrap().object.is_empty());
    }
}
#[test]
fn native_files_example_and_transfer_failures_match_four_executors() {
    for opt in ["asm", "object"] {
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
fn native_files_validation_limits_and_utf8_match_four_executors() {
    for opt in ["asm", "object"] {
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
fn native_files_evaluation_short_circuit_recursion_and_owned_copies() {
    let source = r#"import "std:fs" as fs;
        fn name()->string { print(1); return "x"; }
        fn limit()->i64 { print(2); return 3; }
        fn read(n:i64)->string { if n==0 { return fs::read_text(name(),limit()); } return read(n-1); }
        print(false && byte_len(fs::read_text("absent",3))==0);
        mut a:[string;2]=[read(3),"unchanged"]; b:[string;2]=a;
        a[0]="changed"; print(b[0]); print(a[0]); print(b[1]);
        "#;
    for opt in ["asm", "object"] {
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

#[test]
fn native_files_large_transfers_preserve_chunk_requests_and_eof_probe() {
    for opt in ["asm", "object"] {
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
fn native_files_allocation_failure_precedes_open_and_redacts_input() {
    use crate::runtime::files::{
        Cleanup, Code, Created, Details, Failure, Operation, Origin, Phase,
    };
    let p = hir(
        r#"import "std:fs" as fs; print(7); print(fs::read_text("private-name",3)); print(8);"#,
    );
    let mir = crate::mir::lower(&p).unwrap();
    let m = lower::lower_file_tests(&mir, target()).unwrap();
    let source = m
        .instructions
        .iter()
        .zip(&m.origins)
        .chain(
            m.functions
                .iter()
                .flat_map(|f| f.instructions.iter().zip(&f.origins)),
        )
        .find_map(|(i, o)| matches!(i, super::Instruction::FileReadText { .. }).then_some(*o))
        .unwrap();
    let super::super::ir::Origin::Source { node_id, span } = source else {
        panic!("missing origin")
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
    let mut native = artifact(&p, target());
    for s in &mut native.program.symbols {
        if s.name == "malloc" {
            s.name = "cerune_test_malloc".into();
        }
    }
    let host = Case::read(b"abc").host()
        + "\nvoid *cerune_test_malloc(uint64_t n) { (void)n; return NULL; }\n";
    for route in ["asm", "object"] {
        let Some((dir, exe)) = build(&native, &host, route) else {
            return;
        };
        let actual = process::bounded_output(
            Command::new(exe).current_dir(&dir),
            &dir,
            "native-files-allocation",
            Duration::from_secs(10),
        )
        .unwrap();
        check_result(&actual, Err((failure, "7\n".into())));
        assert!(!dir.join("host-trace.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
#[test]
fn native_files_lir_requires_origin_frame_and_heap() {
    let p = hir(include_str!("../../../../examples/files/roundtrip.ceru"));
    let mir = crate::mir::lower(&p).unwrap();
    let m = lower::lower_file_tests(&mir, target()).unwrap();
    let original = m.clone();
    let native = build_program_from_lir(&m, true).unwrap();
    assert_eq!(m, original);
    for (f, instructions, origins) in std::iter::once((None, &m.instructions, &m.origins)).chain(
        m.functions
            .iter()
            .enumerate()
            .map(|(i, f)| (Some(i), &f.instructions, &f.origins)),
    ) {
        for (index, i) in instructions.iter().enumerate() {
            if !matches!(
                i,
                super::Instruction::FileReadText { .. } | super::Instruction::FileWriteNew { .. }
            ) {
                continue;
            }
            assert!(native.origins().iter().any(|o| o.function == f
                && o.instruction == index
                && o.source == origins[index]
                && o.mir.is_some()));
            let mut bad = m.clone();
            let os = if let Some(f) = f {
                &mut bad.functions[f].origins
            } else {
                &mut bad.origins
            };
            os[index] = super::super::ir::Origin::Synthetic;
            assert!(build_program_from_lir(&bad, false).is_err());
            let mut bad = m.clone();
            let is = if let Some(f) = f {
                &mut bad.functions[f].instructions
            } else {
                &mut bad.instructions
            };
            match &mut is[index] {
                super::Instruction::FileReadText { name_offset }
                | super::Instruction::FileWriteNew { name_offset, .. } => *name_offset = 0,
                _ => unreachable!(),
            }
            assert!(build_program_from_lir(&bad, false).is_err());
        }
    }
    let mut bad = m;
    bad.string_heap_limit = None;
    assert!(build_program_from_lir(&bad, false).is_err());
}

#[test]
fn native_files_read_failures_release_reserved_storage() {
    let source = r#"import "std:fs" as fs; print(7); print(fs::read_text("x",3)); print(8);"#;
    let p = hir(source);
    let mut native = artifact(&p, target());
    for s in &mut native.program.symbols {
        if ["malloc", "free", "_Exit"].contains(&s.name.as_str()) {
            s.name = format!("cerune_test_{}", s.name);
        }
    }
    let wrappers = r#"
extern uint64_t cerune_test_live; extern void *cerune_test_head;
static int allocations;
void *cerune_test_malloc(uint64_t n) { void *p=malloc((size_t)n); if(p) ++allocations; return p; }
void cerune_test_free(void *p) { if(p) --allocations; free(p); }
void cerune_test__Exit(int n) { _Exit(allocations || cerune_test_live || cerune_test_head ? 94 : n); }
"#;
    for case in [
        Case {
            steps: vec![Step::Count(1), Step::Fail],
            ..Case::read(b"abc")
        },
        Case {
            steps: vec![Step::Count(1), Step::Invalid],
            ..Case::read(b"abc")
        },
        Case {
            close_error: true,
            ..Case::read(b"abc")
        },
        Case::read(b"ab\xff"),
        Case::read(b"abcd"),
        Case {
            open_error: Some(OpenError::Failed),
            ..Case::read(b"abc")
        },
    ] {
        for route in ["asm", "object"] {
            let Some((dir, exe)) = build(&native, &(case.host() + wrappers), route) else {
                return;
            };
            let actual = process::bounded_output(
                Command::new(exe).current_dir(&dir),
                &dir,
                "native-files-cleanup",
                Duration::from_secs(10),
            )
            .unwrap();
            check_result(&actual, execute(0, &p, &mut Files::new(case.memory("x"))));
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
