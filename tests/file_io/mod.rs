use cerune_lang as language;
use cerune_lang::runtime::files::*;
use cerune_lang::{bytecode, ir, ir_executor, mir, mir_executor, modules, ssa_executor, vm};
use std::io::{self, Write};
mod support;
use support::*;

#[test]
fn roundtrip_example_preserves_bytes_and_ir_with_independent_connections() {
    let p = hir(include_str!("../../examples/files/roundtrip.ceru"));
    let before = p.clone();
    let mut baseline = None;
    for route in 0..4 {
        for trace in [0, 1, 32] {
            let memory = Memory::default();
            memory.0.borrow_mut().writes.extend([
                Step::Interrupted,
                Step::Count(1),
                Step::Count(2),
            ]);
            memory
                .0
                .borrow_mut()
                .reads
                .extend([Step::Interrupted, Step::Count(1), Step::Count(2)]);
            let mut files = Files::with_trace_limit(memory.clone(), trace);
            assert_eq!(execute(route, &p, &mut files).unwrap(), "true\n16\n");
            assert_eq!(
                memory.0.borrow().files["sample.txt"],
                "日本語\0\r\né\n".as_bytes()
            );
            if trace == 32 {
                let events = files.events().to_vec();
                if let Some(expected) = &baseline {
                    assert_eq!(&events, expected);
                } else {
                    baseline = Some(events);
                }
            }
            let duplicate = execute(route, &p, &mut files).unwrap_err();
            assert_eq!(duplicate.0.code, Code::AlreadyExists);
            assert_eq!(duplicate.1, "");
            assert_eq!(p, before);
        }
    }
    let m = mir::lower(&p).unwrap();
    let ssa = mir::ssa::construct(&m).unwrap();
    let lowered = mir::ssa::lower(&ssa).unwrap();
    assert!(mir::text::emit(&lowered.program).contains("file.write_new.utf8"));
    for text in [
        mir::ssa::text::emit(&ssa).unwrap(),
        ir::text::emit(&p),
        mir::text::emit(&m),
        bytecode::format_program(&bytecode::lower(&p).unwrap()),
    ] {
        assert!(text.contains("file.read_text.utf8"));
        assert!(text.contains("file.write_new.utf8"));
    }
}

#[test]
fn read_failures_and_limits_have_identical_progress_and_origins() {
    for (bytes, limit, step, close_error, code, phase, transferred) in [
        (
            b"abc".as_slice(),
            2,
            None,
            false,
            Code::LimitExceeded,
            Phase::Read,
            3,
        ),
        (
            b"\xff".as_slice(),
            1,
            None,
            false,
            Code::InvalidUtf8,
            Phase::Decode,
            1,
        ),
        (
            b"abc".as_slice(),
            3,
            Some(Step::Fail),
            false,
            Code::ReadFailed,
            Phase::Read,
            0,
        ),
        (
            b"abc".as_slice(),
            3,
            Some(Step::Invalid),
            false,
            Code::InvalidResponse,
            Phase::Read,
            0,
        ),
        (
            b"abc".as_slice(),
            3,
            None,
            true,
            Code::CloseFailed,
            Phase::Close,
            3,
        ),
    ] {
        let p = hir(&format!(
            r#"import "std:fs" as fs; print("before"); print(fs::read_text("secret.txt", {limit})); print("after");"#
        ));
        let mut baseline = None;
        for route in 0..4 {
            let memory = Memory::default();
            {
                let mut s = memory.0.borrow_mut();
                s.files.insert("secret.txt".into(), bytes.into());
                s.reads.extend(step);
                s.close_error = close_error;
            }
            let mut files = Files::with_trace_limit(memory.clone(), 32);
            let (f, output) = execute(route, &p, &mut files).unwrap_err();
            assert_eq!(
                (f.code, f.details.phase, f.details.transferred),
                (code, phase, transferred)
            );
            assert_eq!(output, "before\n");
            assert!(!f.record().contains("secret"));
            if let Some(expected) = baseline {
                assert_eq!(f, expected);
            } else {
                baseline = Some(f);
            }
            let calls = memory.0.borrow().calls.clone();
            assert_eq!(files.write_new("later", "x", 1, f.details.origin), Err(f));
            assert_eq!(memory.0.borrow().calls, calls);
            assert_eq!(files.events().len(), 1);
        }
    }
}

#[test]
fn failed_writes_keep_partial_file_and_do_not_execute_later_operations() {
    for (step, code, written, known) in [
        (Step::Fail, Code::WriteFailed, 1, true),
        (Step::Zero, Code::ZeroProgress, 1, true),
        (Step::Invalid, Code::InvalidResponse, 1, false),
    ] {
        let p = hir(
            r#"import "std:fs" as fs; print("before"); fs::write_new("secret.txt", "abc", 3); fs::write_new("later", "", 0); print("after");"#,
        );
        let mut baseline = None;
        for route in 0..4 {
            let memory = Memory::default();
            memory.0.borrow_mut().writes.extend([Step::Count(1), step]);
            memory.0.borrow_mut().close_error = true;
            let mut files = Files::with_trace_limit(memory.clone(), 32);
            let (f, output) = execute(route, &p, &mut files).unwrap_err();
            assert_eq!(
                (f.code, f.details.transferred, f.details.progress_known),
                (code, written, known)
            );
            assert_eq!(f.details.created, Created::Yes);
            assert_eq!(f.details.cleanup, Cleanup::Failed);
            assert_eq!(output, "before\n");
            assert_eq!(memory.0.borrow().files["secret.txt"], b"a");
            assert!(!memory.0.borrow().files.contains_key("later"));
            assert_eq!(memory.0.borrow().calls.last().unwrap(), "close");
            if let Some(expected) = baseline {
                assert_eq!(f, expected);
            } else {
                baseline = Some(f);
            }
        }
    }
}

#[test]
fn empty_exact_limit_and_repeated_owned_reads_release_storage() {
    let p = hir(r#"import "std:fs" as fs;
        fs::write_new("empty", "", 0); print(fs::read_text("empty", 0));
        fn read()->string { return fs::read_text("text", 3); }
        mut i: i64 = 0; while i < 10 { a:string=read(); b:string=a; print(a==b); i=i+1; }
        print(false && fs::read_text("missing", 0)=="");
        print(true || fs::read_text("missing", 0)=="");"#);
    for route in 0..4 {
        let mut p = p.clone();
        p.string_heap_limit = 3;
        let memory = Memory::default();
        memory
            .0
            .borrow_mut()
            .files
            .insert("text".into(), b"abc".to_vec());
        let mut files = Files::with_trace_limit(memory.clone(), 32);
        let expected = format!("\n{}false\ntrue\n", "true\n".repeat(10));
        assert_eq!(execute(route, &p, &mut files).unwrap(), expected);
        assert_eq!(files.events().len(), 12);
        assert!(memory.0.borrow().files["empty"].is_empty());
    }
}

#[test]
fn validation_budget_and_unconfigured_connections_stop_before_open() {
    for (body, budget, expected) in [
        (r#"fs::read_text("x", -1)"#, 16, Code::InvalidLimit),
        (r#"fs::read_text("../x", 1)"#, 16, Code::InvalidName),
        (r#"fs::read_text("x", 2)"#, 1, Code::BudgetExceeded),
    ] {
        let mut p = hir(&format!(r#"import "std:fs" as fs; print({body});"#));
        p.string_heap_limit = budget;
        for route in 0..4 {
            let memory = Memory::default();
            let mut files = Files::new(memory.clone());
            assert_eq!(execute(route, &p, &mut files).unwrap_err().0.code, expected);
            assert!(memory.0.borrow().calls.is_empty());
        }
    }
    let p = hir(r#"import "std:fs" as fs; fs::write_new("x", "", 0);"#);
    for route in 0..4 {
        let mut files = Files::<Memory>::unconfigured();
        assert_eq!(
            execute(route, &p, &mut files).unwrap_err().0.code,
            Code::Unconfigured
        );
    }
    assert!(
        ir_executor::run(&p)
            .unwrap_err()
            .to_string()
            .contains("code=unconfigured")
    );
    assert!(
        mir_executor::run(&mir::lower(&p).unwrap())
            .unwrap_err()
            .to_string()
            .contains("code=unconfigured")
    );
    assert!(
        ssa_executor::run(&mir::ssa::construct(&mir::lower(&p).unwrap()).unwrap())
            .unwrap_err()
            .to_string()
            .contains("code=unconfigured")
    );
    assert!(
        matches!(vm::run(&bytecode::lower(&p).unwrap()).unwrap_err().kind(), vm::VmErrorKind::Files(f) if f.code == Code::Unconfigured)
    );
}

#[test]
fn argument_evaluation_order_is_left_to_right_once_and_stops_after_failure() {
    let p = hir(r#"import "std:fs" as fs;
        fn name()->string {print(1); return concat("s", "ecret.txt");}
        fn text()->string {print(2); return concat("a", "b");}
        fn limit()->i64 {print(3); return 2;}
        fs::write_new(name(), text(), limit()); print(4);
        print(fs::read_text(name(), limit())); print(5);"#);
    for route in 0..4 {
        let memory = Memory::default();
        let mut files = Files::with_trace_limit(memory, 32);
        assert_eq!(
            execute(route, &p, &mut files).unwrap(),
            "1\n2\n3\n4\n1\n3\nab\n5\n"
        );
        assert_eq!(files.events().len(), 2);
        assert_eq!(execute(route, &p, &mut files).unwrap_err().1, "1\n2\n3\n");
    }
}

#[test]
fn standard_module_is_virtual_observable_and_internal_calls_are_private() {
    let source = r#"import "std:fs" as a; import "std:fs" as b; a::write_new("x", "", 0); print(b::read_text("x",0));"#;
    let c = modules::from_source(source).unwrap();
    assert_eq!(c.sources.files().count(), 2);
    let standard = c.sources.files().find(|f| f.name() == "std:fs").unwrap();
    assert!(matches!(
        standard.kind(),
        cerune_lang::source::SourceKind::Standard {
            identifier: "std:fs",
            version: 1
        }
    ));
    assert!(standard.text().contains("pub fn write_new"));
    assert!(
        c.source_manifest()
            .contains(r#""module":"std:fs","version":1"#)
    );
    assert!(cerune_lang::compile(source).is_ok());
    for source in [
        r#"import "std:fs" as fs; fs::__cerune_file_read_text("x",0);"#,
        r#"__cerune_file_write_new("x","",0);"#,
        r#"$file_read_text("x",0);"#,
        r#"import "std:fs" as fs; print(fs::read_text(1,0));"#,
        r#"import "std:fs" as fs; fs::write_new("x", "");"#,
        r#"import "std:fs" as fs; x:infer=fs::write_new("x","",0);"#,
    ] {
        assert!(cerune_lang::compile_to_ir(source).is_err(), "{source}");
    }
    let p = c.to_ir().unwrap();
    let mut files = Files::<Memory>::unconfigured();
    let f = execute(0, &p, &mut files).unwrap_err().0;
    assert_eq!(f.details.origin.span.source_id(), standard.id());
}

#[test]
fn generators_reject_file_io_with_source_locations_even_in_unused_functions() {
    use cerune_lang::codegen::{wat, x86_64};
    let p = hir(
        r#"import "std:fs" as fs; fn unused()->string {return fs::read_text("x",0);} print(1);"#,
    );
    let errors = [
        wat::emit_wat(&p).unwrap_err(),
        x86_64::emit_asm(&p, x86_64::Target::X86_64UnknownLinuxGnu).unwrap_err(),
    ];
    for e in errors {
        assert!(e.message().contains("file I/O is not supported yet"));
        assert!(e.primary_span().is_some());
    }
}

#[test]
fn composed_io_arguments_files_and_depth_use_one_ordered_execution() {
    use cerune_lang::runtime::{
        arguments::{Arguments, Limits},
        input::Input,
        output::Output,
    };
    struct Writer(Memory);
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .0
                .borrow_mut()
                .calls
                .push(format!("stdout:{}", String::from_utf8_lossy(bytes)));
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.0.borrow_mut().calls.push("stdout.flush".into());
            Ok(())
        }
    }
    let p = hir(
        r#"import "std:fs" as fs; import "std:env" as env; import "std:io" as io;
        print(env::arg_count()); flush();
        match io::read_line(8) { io::ReadLine::Line{text:text}=>{fs::write_new("x",text,8);},io::ReadLine::End{}=>{} }
        print(fs::read_text("x",8));"#,
    );
    let mut baseline = None;
    for route in 0..4 {
        let memory = Memory::default();
        let mut files = Files::with_trace_limit(memory.clone(), 32);
        let mut input = Input::new(io::Cursor::new(b"abc\n"));
        let mut output = Output::new(Writer(memory.clone()));
        let args = Arguments::capture(&[b"one"], Limits::default()).unwrap();
        let depth = std::num::NonZeroU32::new(128).unwrap();
        match route {
            0 => ir_executor::run_with_io_files_arguments_and_call_limit(
                &p,
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            1 => mir_executor::run_with_io_files_arguments_and_call_limit(
                &mir::lower(&p).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            2 => ssa_executor::run_with_io_files_arguments_and_call_limit(
                &mir::ssa::construct(&mir::lower(&p).unwrap()).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            _ => vm::run_with_io_files_arguments_and_call_limit(
                &bytecode::lower(&p).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
        }
        let calls = memory.0.borrow().calls.clone();
        assert_eq!(calls.first().unwrap(), "stdout:1\n");
        assert_eq!(calls[1], "stdout.flush");
        assert_eq!(calls.last().unwrap(), "stdout:abc\n\n");
        if let Some(expected) = &baseline {
            assert_eq!(&calls, expected);
        } else {
            baseline = Some(calls);
        }
    }
}

#[test]
fn recursive_reads_early_return_and_reassignment_keep_owned_copies_independent() {
    let mut p = hir(r#"import "std:fs" as fs;
        fn read(n:i64)->string {
            text:string=fs::read_text("x",3);
            if n==0 {return text;}
            return read(n-1);
        }
        mut text:string=read(4); original:string=text;
        text=concat(text,"d"); print(original); print(text);
        fs::write_new("copy",original,3);"#);
    p.string_heap_limit = 32;
    for route in 0..4 {
        let memory = Memory::default();
        memory
            .0
            .borrow_mut()
            .files
            .insert("x".into(), b"abc".to_vec());
        let mut files = Files::with_trace_limit(memory.clone(), 32);
        assert_eq!(execute(route, &p, &mut files).unwrap(), "abc\nabcd\n");
        assert_eq!(memory.0.borrow().files["copy"], b"abc");
        assert_eq!(files.events().len(), 6);
    }
}

#[test]
fn cli_observation_does_not_open_files_and_execution_reports_unconfigured() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_cerune");
    for (command, instruction) in [
        ("emit-ir", "file.read_text.utf8"),
        ("emit-mir", "file.write_new.utf8"),
        ("emit-bytecode", "file.write_new.utf8"),
    ] {
        let output = Command::new(binary)
            .args([command, "examples/files/roundtrip.ceru"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains(instruction)
        );
    }
    for command in ["run", "run-vm", "run-mir"] {
        let output = Command::new(binary)
            .args([command, "examples/files/roundtrip.ceru"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains("file-v1")
                && error.contains("code=unconfigured")
                && error.contains("std:fs"),
            "{error}"
        );
    }
}

#[cfg(any(windows, target_os = "linux"))]
mod os;

#[cfg(any(windows, target_os = "linux"))]
mod c;
#[cfg(any(windows, target_os = "linux"))]
mod generated;
