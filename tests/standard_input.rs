use cerune_lang::{
    ir, ir_executor, mir, mir_executor, modules,
    runtime::{
        input::{Code, Input},
        output::Output,
    },
    source::SourceKind,
    ssa_executor,
};
use std::{
    cell::RefCell,
    io::{self, Read, Write},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Read,
    Write(Vec<u8>),
    Flush,
}
type Log = Rc<RefCell<Vec<Action>>>;
struct Reader {
    bytes: Vec<u8>,
    position: usize,
    fail_at: Option<usize>,
    invalid: bool,
    interrupt: bool,
    log: Log,
}
impl Read for Reader {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        self.log.borrow_mut().push(Action::Read);
        assert_eq!(dst.len(), 1);
        if std::mem::take(&mut self.interrupt) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.fail_at == Some(self.position) {
            if self.invalid {
                return Ok(2);
            }
            return Err(io::Error::other("SECRET input failure"));
        }
        if self.position == self.bytes.len() {
            return Ok(0);
        }
        dst[0] = self.bytes[self.position];
        self.position += 1;
        Ok(1)
    }
}
struct Writer {
    bytes: Vec<u8>,
    log: Log,
    flush_error: bool,
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        self.log.borrow_mut().push(Action::Write(bytes.into()));
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.log.borrow_mut().push(Action::Flush);
        if self.flush_error {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(())
        }
    }
}
fn execute(
    route: usize,
    hir: &ir::Program,
    input: &mut Input<Reader>,
    output: &mut Output<Writer>,
) -> Result<(), String> {
    match route {
        0 => ir_executor::run_with_io(hir, input, output).map_err(|e| e.to_string()),
        1 => mir_executor::run_with_io(&mir::lower(hir).unwrap(), input, output)
            .map_err(|e| e.to_string()),
        2 => ssa_executor::run_with_io(
            &mir::ssa::construct(&mir::lower(hir).unwrap()).unwrap(),
            input,
            output,
        )
        .map_err(|e| e.to_string()),
        _ => cerune_lang::run_bytecode_with_io(
            &cerune_lang::bytecode::lower(hir).unwrap(),
            input,
            output,
        )
        .map_err(|e| format!("{:?}", e.vm_error().kind())),
    }
}
fn connections(
    bytes: &[u8],
    fail_at: Option<usize>,
    invalid: bool,
    trace: usize,
    flush_error: bool,
) -> (Input<Reader>, Output<Writer>, Log) {
    let log = Log::default();
    let input = Input::with_trace_limit(
        Reader {
            bytes: bytes.into(),
            position: 0,
            fail_at,
            invalid,
            interrupt: true,
            log: log.clone(),
        },
        trace,
    );
    let output = Output::with_trace_limit(
        Writer {
            bytes: vec![],
            log: log.clone(),
            flush_error,
        },
        trace,
    );
    (input, output, log)
}

#[test]
fn virtual_module_is_unique_observable_and_has_ordinary_enum_identity() {
    let source = r#"import "std:io" as a; import "std:io" as b;
        value:a::ReadLine=b::ReadLine::Line{text:"\n"};
        match value { b::ReadLine::Line{text:text} => {print(byte_len(text));}, b::ReadLine::End{}=>{print(0);} }"#;
    let compilation = modules::from_source(source).unwrap();
    assert_eq!(compilation.sources.files().count(), 2);
    let standard = compilation
        .sources
        .files()
        .find(|f| matches!(f.kind(), SourceKind::Standard { .. }))
        .unwrap();
    assert_eq!(standard.name(), "std:io");
    assert_eq!(
        standard.kind(),
        SourceKind::Standard {
            identifier: "std:io",
            version: 1
        }
    );
    assert!(standard.text().contains("pub enum ReadLine"));
    let manifest = compilation.source_manifest();
    assert!(manifest.contains(r#""kind":"standard","module":"std:io","version":1"#));
    let hir = compilation.to_ir().unwrap();
    assert_eq!(hir.type_definitions.len(), 1);
    assert!(hir.type_definitions[0].variants.is_some());
    assert_eq!(ir_executor::run(&hir).unwrap(), "1\n");
    let text = ir::text::emit(&hir);
    assert!(text.contains("input.read_line.utf8"));
    assert!(cerune_lang::compile(source).is_ok());
    assert!(cerune_lang::compile_to_ir(source).is_ok());
    let ordinary = modules::from_source("print(1);").unwrap();
    assert!(!ordinary.modular);
    assert!(!ordinary.source_manifest().contains(r#""kind""#));
    assert_eq!(
        cerune_lang::compile_to_ir("print(1);").unwrap(),
        ordinary.to_ir().unwrap()
    );
}

#[test]
fn unknown_standard_modules_and_memory_file_imports_are_diagnostics() {
    for name in ["std:unknown", "std:io.ceru", "std:io/../io", "std:IO"] {
        let error = modules::from_source(&format!("import {name:?} as io;")).unwrap_err();
        assert!(
            error
                .diagnostic
                .message()
                .contains("unknown standard module")
        );
    }
    let error = modules::from_source(r#"import "./std/io.ceru" as io;"#).unwrap_err();
    assert!(
        error
            .diagnostic
            .message()
            .contains("relative imports require modules::load")
    );
    assert!(cerune_lang::compile_to_ir("__cerune_read_line_utf8(1);").is_err());
    assert!(cerune_lang::compile_to_ir("$read_line(1);").is_err());
    assert!(
        cerune_lang::compile_to_ir(r#"import "std:io" as io; io::__cerune_read_line_utf8(1);"#)
            .is_err()
    );
}

#[test]
fn real_line_enum_and_output_order_match_all_four_executors() {
    let source = include_str!("fixtures/line_input.ceru");
    let compilation = modules::from_source(source).unwrap();
    let hir = compilation.to_ir().unwrap();
    let bytes = include_bytes!("fixtures/line_input.stdin");
    let expected = include_bytes!("fixtures/line_input.stdout");
    let mut reference = None;
    for route in 0..4 {
        for trace in [0, 1, 100] {
            let (mut input, mut output, log) = connections(bytes, None, false, trace, false);
            execute(route, &hir, &mut input, &mut output).unwrap();
            assert!(input.failure().is_none());
            assert_eq!(output.into_inner().bytes, expected);
            let actions = log.borrow().clone();
            if let Some(reference) = &reference {
                assert_eq!(&actions, reference);
            } else {
                reference = Some(actions);
            }
            assert_eq!(input.into_inner().unwrap().position, bytes.len());
        }
    }
}

#[test]
fn failures_keep_original_origin_consumption_and_prior_output_without_recovery() {
    let source = r#"import "std:io" as io; print("before");flush();
        value:io::ReadLine=io::read_line(4);print("after");"#;
    for (bytes, fail_at, invalid, budget, code, consumed) in [
        (&b"SECRET\n"[..], None, false, 100, Code::LimitExceeded, 5),
        (&b"\xff\n"[..], None, false, 100, Code::InvalidUtf8, 2),
        (&b"ab\n"[..], Some(2), false, 100, Code::ReadFailed, 2),
        (&b"ab\n"[..], Some(1), true, 100, Code::InvalidResponse, 1),
        (&b"\n"[..], None, false, 3, Code::BudgetExceeded, 0),
    ] {
        let compilation = modules::from_source(source).unwrap();
        let mut hir = compilation.to_ir().unwrap();
        hir.string_heap_limit = budget;
        let standard = compilation
            .sources
            .files()
            .find(|f| matches!(f.kind(), SourceKind::Standard { .. }))
            .unwrap()
            .id();
        let mut reference = None;
        for route in 0..4 {
            let (mut input, mut output, log) = connections(bytes, fail_at, invalid, 100, false);
            assert!(execute(route, &hir, &mut input, &mut output).is_err());
            let failure = input.failure().unwrap();
            assert_eq!((failure.code, failure.consumed), (code, consumed));
            assert_eq!(failure.origin.span.source_id(), standard);
            assert!(!failure.record().contains("SECRET"));
            assert!(
                compilation
                    .sources
                    .slice(failure.origin.span)
                    .unwrap()
                    .contains("__cerune_read_line_utf8")
            );
            assert_eq!(output.into_inner().bytes, b"before\n");
            let observed = (failure, log.borrow().clone());
            if let Some(reference) = &reference {
                assert_eq!(&observed, reference);
            } else {
                reference = Some(observed);
            }
        }
    }
}

#[test]
fn flush_failure_and_short_circuit_prevent_input_and_limits_evaluate_once() {
    for route in 0..4 {
        let source = r#"import "std:io" as io; print("before");flush(); value:io::ReadLine=io::read_line(4);"#;
        let hir = modules::from_source(source).unwrap().to_ir().unwrap();
        let (mut input, mut output, log) = connections(b"x\n", None, false, 100, true);
        assert!(execute(route, &hir, &mut input, &mut output).is_err());
        assert!(!log.borrow().contains(&Action::Read));
        assert!(input.events().is_empty());
        let source = r#"import "std:io" as io;
            fn consume()->bool {value:io::ReadLine=io::read_line(4);return true;}
            fn cap()->i64 {print("limit");return 4;}
            if false && consume() {print("bad");}
            print("ready");flush();
            value:io::ReadLine=io::read_line(cap());print("done");"#;
        let hir = modules::from_source(source).unwrap().to_ir().unwrap();
        let (mut input, mut output, log) = connections(b"\n", None, false, 100, false);
        execute(route, &hir, &mut input, &mut output).unwrap();
        assert_eq!(input.events().len(), 1);
        assert_eq!(output.into_inner().bytes, b"ready\nlimit\ndone\n");
        let actions = log.borrow();
        let first_read = actions.iter().position(|a| a == &Action::Read).unwrap();
        assert!(actions[..first_read].contains(&Action::Flush));
    }
}

#[test]
fn copied_line_values_release_storage_between_calls() {
    let source = r#"import "std:io" as io;
        fn show()->void {
            a:io::ReadLine=io::read_line(4);
            b:io::ReadLine=a;
            values:[io::ReadLine;2]=[a,b];print(values);
        }
        for(n:i64 in [1,2,3]) {show();}"#;
    let mut hir = modules::from_source(source).unwrap().to_ir().unwrap();
    hir.string_heap_limit = 4;
    let mut reference = None;
    for route in 0..4 {
        let (mut input, mut output, _) = connections(b"a\nb\nc\n", None, false, 100, false);
        execute(route, &hir, &mut input, &mut output).unwrap();
        let bytes = output.into_inner().bytes;
        if let Some(reference) = &reference {
            assert_eq!(&bytes, reference);
        } else {
            reference = Some(bytes);
        }
    }
}

#[test]
fn native_input_lowers_with_origins_for_both_targets_and_ssa() {
    use cerune_lang::codegen::x86_64 as native;
    let hir =
        modules::from_source(r#"import "std:io" as io; value:io::ReadLine=io::read_line(4);"#)
            .unwrap()
            .to_ir()
            .unwrap();
    let mir = mir::lower(&hir).unwrap();
    let ssa_mir = mir::ssa::lower(&mir::ssa::construct(&mir).unwrap())
        .unwrap()
        .program;
    for target in [
        native::Target::X86_64PcWindowsMsvc,
        native::Target::X86_64UnknownLinuxGnu,
    ] {
        for program in [&mir, &ssa_mir] {
            let lir = native::lower_mir(program, target).unwrap();
            let assembly = native::emit_asm_from_lir(&lir, true).unwrap();
            assert!(assembly.contains("callq cerune_input_read_line"));
            assert!(assembly.contains("io-v1 operation=read-line"));
            assert!(assembly.contains(" file="));
            assert!(!native::emit_object_from_lir(&lir, true).unwrap().is_empty());
        }
    }
}

#[test]
fn file_imports_and_virtual_imports_share_types_without_environment_or_cwd_lookup() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = root
        .join("target")
        .join(format!("standard-module-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("helper.ceru"),
        r#"import "std:io" as io;
        pub fn identity(value:io::ReadLine)->io::ReadLine{return value;}"#,
    )
    .unwrap();
    let entry = directory.join("main.ceru");
    std::fs::write(&entry,r#"import "helper.ceru" as helper;import "std:io" as io;
        value:io::ReadLine=helper::identity(io::ReadLine::Line{text:"same"});
        match value {io::ReadLine::Line{text:text}=>{print(text);},io::ReadLine::End{}=>{print("end");}}"#).unwrap();
    let compilation = modules::load(&entry).unwrap();
    assert_eq!(compilation.sources.files().count(), 3);
    let standard = compilation
        .sources
        .files()
        .find(|f| matches!(f.kind(), SourceKind::Standard { .. }))
        .unwrap();
    assert_eq!(standard.id().index(), 3);
    assert_eq!(
        ir_executor::run(&compilation.to_ir().unwrap()).unwrap(),
        "same\n"
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_cerune"))
        .args(["emit-sources", entry.to_str().unwrap()])
        .current_dir(&root)
        .env("CERUNE_STD_PATH", directory.join("does-not-exist"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, compilation.source_manifest().as_bytes());
    std::fs::write(
        &entry,
        r#"import "std:io" as io;import "helper.ceru" as helper;
        value:io::ReadLine=helper::identity(io::ReadLine::End{});"#,
    )
    .unwrap();
    let reordered = modules::load(&entry).unwrap();
    assert_eq!(reordered.sources.files().count(), 3);
    assert_eq!(
        reordered
            .sources
            .files()
            .find(|f| matches!(f.kind(), SourceKind::Standard { .. }))
            .unwrap()
            .id()
            .index(),
        2
    );
    assert_eq!(reordered.to_ir().unwrap().type_definitions.len(), 1);
}

#[test]
fn input_origin_survives_hir_mir_ssa_and_bytecode_without_implicit_execution() {
    let compilation =
        modules::from_source(r#"import "std:io" as io; value:io::ReadLine=io::read_line(4);"#)
            .unwrap();
    let hir = compilation.to_ir().unwrap();
    let mir = mir::lower(&hir).unwrap();
    let ssa = mir::ssa::construct(&mir).unwrap();
    let bytecode = cerune_lang::bytecode::lower(&hir).unwrap();
    let input_origins: Vec<_> = mir
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.instructions)
        .filter(|i| {
            matches!(
                i.kind,
                mir::InstructionKind::Assign {
                    value: mir::Operation::ReadLine(_),
                    ..
                }
            )
        })
        .map(|i| i.origin.source().unwrap())
        .collect();
    assert_eq!(input_origins.len(), 1);
    let expected = input_origins[0];
    assert!(
        compilation
            .sources
            .slice(expected.span)
            .unwrap()
            .contains("__cerune_read_line_utf8")
    );
    let lowered = mir::ssa::lower(&ssa).unwrap().program;
    let actual = lowered
        .functions
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.instructions)
        .find(|i| {
            matches!(
                i.kind,
                mir::InstructionKind::Assign {
                    value: mir::Operation::ReadLine(_),
                    ..
                }
            )
        })
        .unwrap()
        .origin
        .source()
        .unwrap();
    assert_eq!(actual, expected);
    let bytecode_origin = bytecode
        .functions
        .iter()
        .flat_map(|f| &f.instructions)
        .find(|i| matches!(i.kind, cerune_lang::bytecode::InstructionKind::ReadLine))
        .unwrap()
        .origin;
    assert_eq!(
        bytecode_origin,
        cerune_lang::bytecode::InstructionOrigin::Source {
            node_id: expected.node_id,
            span: expected.span
        }
    );
    let error = ir_executor::run(&hir).unwrap_err();
    let ir_executor::ErrorKind::Input(failure) = error.kind() else {
        panic!("{error:?}")
    };
    assert_eq!(failure.code, Code::Unconfigured);
    assert_eq!(failure.consumed, 0);
    let rendered = compilation.render(&error.diagnostic());
    assert!(rendered.contains("std:io"));
    assert!(rendered.contains("unconfigured"));
}

#[test]
fn public_limit_type_errors_point_to_user_source() {
    for call in [
        "io::read_line()",
        "io::read_line(true)",
        "io::read_line(1u64)",
        "io::read_line(1, 2)",
    ] {
        let source = format!("import \"std:io\" as io; value:io::ReadLine={call};");
        let compilation = modules::from_source(&source).unwrap();
        let error = compilation.to_ir().unwrap_err();
        let span = error.primary_span().unwrap();
        assert_eq!(
            compilation.sources.get(span.source_id()).unwrap().kind(),
            SourceKind::User
        );
    }
}

#[test]
fn known_eof_needs_no_free_string_budget_and_invalid_limits_never_read() {
    for route in 0..4 {
        let source = r#"import "std:io" as io;
            line:io::ReadLine=io::read_line(4);
            end:io::ReadLine=io::read_line(4);
            match end {io::ReadLine::Line{text:text}=>{print("bad");},io::ReadLine::End{}=>{print("end");}}
            print(line);"#;
        let mut hir = modules::from_source(source).unwrap().to_ir().unwrap();
        hir.string_heap_limit = 4;
        let (mut input, mut output, _) = connections(b"last", None, false, 100, false);
        execute(route, &hir, &mut input, &mut output).unwrap();
        assert_eq!(input.events().len(), 2);
        assert_eq!(input.events()[1].read_calls, 0);
        assert_eq!(
            input.events()[1].outcome,
            cerune_lang::runtime::input::Outcome::End
        );
        assert!(output.into_inner().bytes.starts_with(b"end\n"));
        for limit in [0, -1, i64::from(i32::MAX) + 1] {
            let source =
                format!("import \"std:io\" as io; value:io::ReadLine=io::read_line({limit});");
            let hir = modules::from_source(&source).unwrap().to_ir().unwrap();
            let (mut input, mut output, log) = connections(b"unread", None, false, 100, false);
            assert!(execute(route, &hir, &mut input, &mut output).is_err());
            assert_eq!(input.failure().unwrap().code, Code::InvalidLimit);
            assert!(!log.borrow().contains(&Action::Read));
        }
    }
}
