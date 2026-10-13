use cerune_lang::{
    compile_to_ir, ir, ir_executor, mir, mir_executor,
    runtime::{
        RuntimeFailure,
        output::{Event, Failure, Operation, Origin, Output},
    },
    ssa_executor,
};
use std::io::{self, Write};

#[derive(Debug, PartialEq, Eq)]
struct Stopped {
    io: Option<Failure>,
    runtime: Option<RuntimeFailure>,
    origin: Option<Origin>,
}
fn execute<W: Write>(
    route: usize,
    hir: &ir::Program,
    output: &mut Output<W>,
) -> Result<(), Box<Stopped>> {
    match route {
        0 => ir_executor::run_with_output(hir, output).map_err(|e| {
            assert_eq!(e.output(), "");
            Box::new(Stopped {
                io: match e.kind() {
                    ir_executor::ErrorKind::Output(f) => Some(**f),
                    _ => None,
                },
                runtime: e.runtime_failure(),
                origin: e.origin().map(|o| Origin {
                    node_id: o.node_id,
                    span: o.span,
                }),
            })
        }),
        1 => mir_executor::run_with_output(&mir::lower(hir).unwrap(), output).map_err(|e| {
            assert_eq!(e.output(), "");
            Box::new(Stopped {
                io: match e.kind() {
                    mir_executor::ErrorKind::Output(f) => Some(**f),
                    _ => None,
                },
                runtime: e.runtime_failure(),
                origin: e.origin().map(|o| Origin {
                    node_id: o.node_id,
                    span: o.span,
                }),
            })
        }),
        2 => ssa_executor::run_with_output(
            &mir::ssa::construct(&mir::lower(hir).unwrap()).unwrap(),
            output,
        )
        .map_err(|e| {
            assert_eq!(e.output(), "");
            Box::new(Stopped {
                io: match e.kind() {
                    mir_executor::ErrorKind::Output(f) => Some(**f),
                    _ => None,
                },
                runtime: e.runtime_failure(),
                origin: e.origin().map(|o| Origin {
                    node_id: o.node_id,
                    span: o.span,
                }),
            })
        }),
        3 => cerune_lang::run_bytecode_with_output(
            &cerune_lang::bytecode::lower(hir).unwrap(),
            output,
        )
        .map_err(|e| {
            assert_eq!(e.vm_error().output(), "");
            Box::new(Stopped {
                io: match e.vm_error().kind() {
                    cerune_lang::vm::VmErrorKind::Output(f) => Some(*f),
                    _ => None,
                },
                runtime: e.runtime_failure(),
                origin: e.origin().and_then(|o| match o {
                    cerune_lang::bytecode::InstructionOrigin::Source { node_id, span } => {
                        Some(Origin { node_id, span })
                    }
                    _ => None,
                }),
            })
        }),
        _ => unreachable!(),
    }
}

struct Writer {
    bytes: Vec<u8>,
    chunk: usize,
    fail_at: usize,
    interrupted: bool,
    zero: bool,
    invalid_count: bool,
    flush_error: bool,
    writes: usize,
    flushes: usize,
}
impl Default for Writer {
    fn default() -> Self {
        Self {
            bytes: vec![],
            chunk: usize::MAX,
            fail_at: usize::MAX,
            interrupted: false,
            zero: false,
            invalid_count: false,
            flush_error: false,
            writes: 0,
            flushes: 0,
        }
    }
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if std::mem::take(&mut self.interrupted) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.zero {
            return Ok(0);
        }
        if self.invalid_count {
            return Ok(bytes.len() + 1);
        }
        if self.bytes.len() >= self.fail_at {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let n = bytes
            .len()
            .min(self.chunk)
            .min(self.fail_at - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.flush_error {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn records_and_bytes_match_across_routes_without_observation_controlling_execution() {
    let source = r#"
        fn show(n:i64)->void { print(n); print(["日本\0\r\n", "e\u{301}", "é"]); }
        for (n:i64 in [1,2]) { show(n); }
        if false && 1 / 0 == 0 { print("unreachable"); }
    "#;
    let hir = compile_to_ir(source).unwrap();
    let expected = ir_executor::run(&hir).unwrap();
    let mut reference: Option<Vec<Event>> = None;
    for route in 0..4 {
        for (limit, chunk) in [(0, 1), (1, 1), (1000, 1), (1000, usize::MAX)] {
            let mut output = Output::with_trace_limit(
                Writer {
                    chunk,
                    interrupted: true,
                    ..Writer::default()
                },
                limit,
            );
            execute(route, &hir, &mut output).unwrap();
            output.flush().unwrap();
            if limit == 1000 {
                if let Some(reference) = &reference {
                    assert_eq!(output.events(), reference, "route {route}");
                } else {
                    reference = Some(output.events().to_vec());
                }
                for event in output.events() {
                    if event.operation == Operation::Write {
                        assert!(event.origin.is_some());
                        assert_eq!(event.accepted, event.requested);
                    }
                }
            }
            assert_eq!(output.trace_truncated(), limit == 1);
            if limit == 0 {
                assert!(output.events().is_empty());
            }
            let writer = output.into_inner();
            assert_eq!(writer.bytes, expected.as_bytes());
            assert_eq!(writer.flushes, 1);
        }
    }
}

#[test]
fn partial_utf8_write_failure_keeps_exact_prefix_origin_and_stops_later_operations() {
    let source = r#"fn show()->void { print("日本"); print(1 / 0); } print("before"); show();"#;
    let hir = compile_to_ir(source).unwrap();
    let mut reference = None;
    for route in 0..4 {
        for limit in [0, 1, 100] {
            let mut output = Output::with_trace_limit(
                Writer {
                    chunk: 1,
                    fail_at: 8,
                    ..Writer::default()
                },
                limit,
            );
            let error = execute(route, &hir, &mut output).unwrap_err();
            let failure = error.io.unwrap();
            assert_eq!(failure.kind, io::ErrorKind::BrokenPipe);
            assert_eq!(failure.requested, 7);
            assert_eq!(failure.accepted, 1);
            assert_eq!(failure.origin, error.origin);
            assert!(error.runtime.is_none());
            let span = error.origin.unwrap().span;
            assert_eq!(&source[span.start()..span.end()], r#"print("日本");"#);
            if let Some(reference) = &reference {
                assert_eq!(&error, reference);
            } else {
                reference = Some(error);
            }
            assert_eq!(output.flush(), Err(failure));
            let writer = output.into_inner();
            assert_eq!(writer.bytes, [b"before\n".as_slice(), &[0xe6]].concat());
            assert_eq!(writer.flushes, 0);
        }
    }
}

#[test]
fn zero_progress_and_invalid_writer_counts_are_failures() {
    let hir = compile_to_ir(r#"print("x");print(1/0);"#).unwrap();
    for route in 0..4 {
        for invalid in [false, true] {
            let mut output = Output::new(Writer {
                zero: !invalid,
                invalid_count: invalid,
                ..Writer::default()
            });
            let error = execute(route, &hir, &mut output).unwrap_err();
            let failure = error.io.unwrap();
            assert_eq!(
                failure.kind,
                if invalid {
                    io::ErrorKind::InvalidData
                } else {
                    io::ErrorKind::WriteZero
                }
            );
            assert_eq!(failure.accepted, 0);
            assert_eq!(output.into_inner().writes, 1);
        }
    }
}

#[test]
fn flush_failure_is_explicit_has_no_fabricated_source_and_never_replays_bytes() {
    let hir = compile_to_ir(r#"print("日本\0\r\n");"#).unwrap();
    for route in 0..4 {
        let mut output = Output::with_trace_limit(
            Writer {
                flush_error: true,
                ..Writer::default()
            },
            100,
        );
        execute(route, &hir, &mut output).unwrap();
        assert!(
            output
                .events()
                .iter()
                .all(|e| e.operation == Operation::Write)
        );
        let failure = output.flush().unwrap_err();
        assert_eq!(failure.operation, Operation::Flush);
        assert_eq!(failure.origin, None);
        assert_eq!(output.flush(), Err(failure));
        let writer = output.into_inner();
        assert_eq!(writer.bytes, "日本\0\r\n\n".as_bytes());
        assert_eq!(writer.flushes, 1);
    }
}

#[test]
fn runtime_failure_preserves_prior_output_and_capture_apis() {
    let hir = compile_to_ir(r#"print("before");print(1/0);print("after");"#).unwrap();
    let captured = ir_executor::run(&hir).unwrap_err();
    assert_eq!(captured.output(), "before\n");
    for route in 0..4 {
        let mut output = Output::with_trace_limit(Writer::default(), 100);
        let error = execute(route, &hir, &mut output).unwrap_err();
        assert_eq!(error.runtime, captured.runtime_failure());
        assert_eq!(error.io, None);
        output.flush().unwrap();
        assert_eq!(output.into_inner().bytes, b"before\n");
    }
}

#[test]
fn every_runnable_example_matches_existing_captured_output_in_all_executors() {
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                files(&p, out);
            } else if p.extension().is_some_and(|e| e == "ceru") {
                out.push(p);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = vec![];
    files(&root.join("examples"), &mut paths);
    paths.sort();
    for path in paths {
        if path.parent() == Some(root.join("examples/source_files").as_path()) {
            continue;
        }
        let hir = cerune_lang::modules::load(&path).unwrap().to_ir().unwrap();
        let expected = ir_executor::run(&hir);
        for route in 0..4 {
            let mut output = Output::new(Vec::new());
            let result = execute(route, &hir, &mut output);
            let bytes = match &expected {
                Ok(text) => {
                    assert!(
                        result.is_ok(),
                        "{} route {route}: {result:?}",
                        path.display()
                    );
                    text.as_bytes()
                }
                Err(error) => {
                    let stopped = result.unwrap_err();
                    assert_eq!(stopped.io, None);
                    assert_eq!(
                        stopped.runtime,
                        error.runtime_failure(),
                        "{} route {route}",
                        path.display()
                    );
                    error.output().as_bytes()
                }
            };
            assert_eq!(
                output.into_inner(),
                bytes,
                "{} route {route}",
                path.display()
            );
        }
    }
}

#[test]
fn explicit_flush_retains_order_origin_and_failure_across_executors() {
    let source = "print(\"before\"); flush(); print(\"after\");";
    let hir = compile_to_ir(source).unwrap();
    let mut reference = None;
    for route in 0..4 {
        for limit in [0, 1, 32] {
            let mut output = Output::with_trace_limit(
                Writer {
                    flush_error: true,
                    ..Writer::default()
                },
                limit,
            );
            let error = execute(route, &hir, &mut output).unwrap_err();
            let failure = error.io.unwrap();
            assert!(error.runtime.is_none());
            assert_eq!(failure.operation, Operation::Flush);
            assert_eq!((failure.requested, failure.accepted), (0, 0));
            let origin = failure.origin.unwrap();
            assert_eq!(&source[origin.span.start()..origin.span.end()], "flush();");
            assert_eq!(error.origin, Some(origin));
            if let Some(expected) = reference {
                assert_eq!(failure, expected);
            } else {
                reference = Some(failure);
            }
            if limit == 32 {
                assert_eq!(output.events().last().unwrap().operation, Operation::Flush);
                assert_eq!(output.events().last().unwrap().origin, Some(origin));
            }
            assert_eq!(output.flush(), Err(failure));
            let writer = output.into_inner();
            assert_eq!(writer.bytes, b"before\n");
            assert_eq!(writer.flushes, 1);
        }
    }
}

#[test]
fn explicit_flush_example_has_identical_observed_and_captured_execution() {
    let hir = compile_to_ir(include_str!("../examples/io_protocol/explicit_flush.ceru")).unwrap();
    let expected = include_bytes!("../examples/io_protocol/explicit_flush.stdout");
    for route in 0..4 {
        let mut output = Output::with_trace_limit(Writer::default(), 100);
        execute(route, &hir, &mut output).unwrap();
        let flushes: Vec<_> = output
            .events()
            .iter()
            .filter(|e| e.operation == Operation::Flush)
            .collect();
        assert_eq!(flushes.len(), 4);
        assert!(
            flushes
                .iter()
                .all(|e| e.origin.is_some() && e.failure.is_none())
        );
        assert_eq!(output.into_inner().bytes, expected);
    }
    assert_eq!(ir_executor::run(&hir).unwrap().as_bytes(), expected);
    let mir = mir::lower(&hir).unwrap();
    assert_eq!(mir_executor::run(&mir).unwrap().as_bytes(), expected);
    assert_eq!(
        ssa_executor::run(&mir::ssa::construct(&mir).unwrap())
            .unwrap()
            .as_bytes(),
        expected
    );
    assert_eq!(
        cerune_lang::vm::run(&cerune_lang::bytecode::lower(&hir).unwrap())
            .unwrap()
            .as_bytes(),
        expected
    );
}

#[test]
fn flush_is_a_statement_and_survives_observable_lowering() {
    for invalid in [
        "flush(1);",
        "x: i64 = flush();",
        "flush()",
        "fn flush()->void{}",
    ] {
        assert!(compile_to_ir(invalid).is_err(), "{invalid}");
    }
    let hir = compile_to_ir("flush();").unwrap();
    assert!(
        cerune_lang::compile_to_ir_text("flush();")
            .unwrap()
            .contains("flush.stdout")
    );
    let mir = mir::lower(&hir).unwrap();
    assert!(
        mir.main
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(i.kind, mir::InstructionKind::Flush))
    );
    let ssa = mir::ssa::construct(&mir).unwrap();
    let back = mir::ssa::lower(&ssa).unwrap();
    assert!(
        back.program
            .main
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(i.kind, mir::InstructionKind::Flush))
    );
    assert!(cerune_lang::codegen::llvm::emit_llvm(&hir).is_err());
    assert!(cerune_lang::codegen::qbe::emit_qbe(&hir).is_err());
}

#[test]
fn named_flush_failure_retains_file_origin_and_short_circuit_order() {
    let text = "fn hit() -> bool { flush(); return true; } print(false && hit()); print(true || hit()); flush(); print(99);";
    let mut sources = cerune_lang::source::SourceMap::new();
    let id = sources.add("flush.ceru", text);
    let hir = cerune_lang::compile_source_to_ir(sources.get(id).unwrap()).unwrap();
    for route in 0..4 {
        let mut output = Output::with_trace_limit(
            Writer {
                flush_error: true,
                ..Writer::default()
            },
            32,
        );
        let failure = execute(route, &hir, &mut output).unwrap_err().io.unwrap();
        let origin = failure.origin.unwrap();
        assert_eq!(origin.span.source_id(), id);
        assert_eq!(origin.span.start(), text.rfind("flush();").unwrap());
        assert!(failure.record().contains(&id.record_field()));
        let writer = output.into_inner();
        assert_eq!(writer.bytes, b"false\ntrue\n");
        assert_eq!(writer.flushes, 1);
    }
}
