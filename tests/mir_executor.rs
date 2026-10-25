use cerune_lang::{compile_to_ir, ir, mir, mir_executor as executor};
use mir::{InstructionKind as I, Operation as O};
fn program(source: &str) -> mir::Program {
    mir::lower(&compile_to_ir(source).unwrap()).unwrap()
}
fn compare(hir: &ir::Program) {
    let p = mir::lower(hir).unwrap();
    let before = p.clone();
    let a = cerune_lang::ir_executor::run(hir);
    let b = executor::run(&p);
    let vm = cerune_lang::run_bytecode(&cerune_lang::bytecode::lower(hir).unwrap());
    match (&a, &b, &vm) {
        (Ok(a), Ok(b), Ok(c)) => {
            assert_eq!(a, b);
            assert_eq!(a, c);
        }
        (Err(a), Err(b), Err(c)) => {
            assert!(a.runtime_failure().is_some());
            assert_eq!(a.runtime_failure(), b.runtime_failure(), "{b:?}");
            assert_eq!(a.runtime_failure(), c.runtime_failure());
            assert_eq!(a.output(), b.output());
            assert_eq!(a.output(), c.vm_error().output());
            let loc = b.location().unwrap();
            let f = loc.function.map_or(&p.main, |id| &p.functions[id.0]);
            let block = &f.blocks[loc.block.0];
            let origin = block
                .instructions
                .iter()
                .find(|i| i.id == loc.instruction)
                .map_or(block.terminator.origin, |i| i.origin);
            assert_eq!(origin.source(), b.origin());
        }
        _ => panic!("HIR={a:?}\nMIR={b:?}\nVM={vm:?}"),
    }
    assert_eq!(p, before);
}
#[test]
fn independent_mir_snapshot_drives_execution() {
    let hir = compile_to_ir("print(1);").unwrap();
    let mut p = mir::lower(&hir).unwrap();
    for i in p.main.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
        if let I::Assign {
            value: O::Literal(mir::Literal::Integer(v)),
            ..
        } = &mut i.kind
        {
            *v = 42;
        }
    }
    assert_eq!(executor::run(&p).unwrap(), "42\n");
    assert_eq!(cerune_lang::ir_executor::run(&hir).unwrap(), "1\n");
}
#[test]
fn main_entry_is_an_explicit_instruction_and_child_failure_keeps_its_location() {
    let p = program("fn main()->void{print(7);print(1/0);}");
    assert!(
        p.main
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.origin,
                mir::Origin::Synthetic {
                    reason: "entry-main-call"
                }
            ))
    );
    let e = executor::run(&p).unwrap_err();
    assert_eq!(e.output(), "7\n");
    assert_eq!(e.location().unwrap().function, Some(ir::FunctionId(0)));
    assert_eq!(e.runtime_failure().unwrap().code.name(), "division-by-zero");
    let src = "fn inner(x:i64)->i64{return 1/x;} fn outer()->i64{return inner(0);} print(outer());";
    let p = program(src);
    let e = executor::run(&p).unwrap_err();
    assert_eq!(e.location().unwrap().function, Some(ir::FunctionId(0)));
    let span = e.origin().unwrap().span;
    assert_eq!(&src[span.start()..span.end()], "1/x");
}
#[test]
fn malformed_mir_is_rejected_before_any_output() {
    let mut p = program("print(1);");
    p.main.blocks[0].terminator.kind = mir::TerminatorKind::Jump(mir::BlockId(999));
    let e = executor::run(&p).unwrap_err();
    assert!(matches!(e.kind(), executor::ErrorKind::Validation(_)));
    assert!(e.output().is_empty());
    assert!(e.runtime_failure().is_none());
}
#[test]
fn missing_cleanup_is_an_internal_failure_not_a_success() {
    let mut p = program("a:[i64]=array_copy([1]); print(a);");
    for f in p.functions.iter_mut().chain([&mut p.main]) {
        for b in &mut f.blocks {
            b.instructions
                .retain(|i| !matches!(i.kind, I::ArrayFree { .. }));
        }
    }
    mir::validate(&p).unwrap();
    let e = executor::run(&p).unwrap_err();
    assert!(matches!(e.kind(), executor::ErrorKind::InvalidMir(_)));
    assert_eq!(e.output(), "[1]\n");
    assert_eq!(e.runtime_failure(), None);
}
#[test]
fn test_ir_reentry_stops_at_explicit_depth_limit() {
    let mut p = program("fn f()->i64{return 1;} print(f());");
    for i in p.functions[0]
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
    {
        if let I::Assign { value: v, .. } = &mut i.kind
            && matches!(v, O::Literal(mir::Literal::Integer(1)))
        {
            *v = O::Call {
                function: ir::FunctionId(0),
                arguments: vec![],
                ownership: ir::ArgumentOwnership::Owned,
            };
        }
    }
    let e = executor::run_with_call_limit(&p, std::num::NonZeroU32::new(8).unwrap()).unwrap_err();
    assert_eq!(e.call_failure().unwrap().active().len(), 8);
    assert_eq!(
        e.call_failure().unwrap().attempted().function,
        ir::FunctionId(0)
    );
    assert!(e.output().is_empty());
    assert!(e.location().is_some());
    assert!(e.origin().is_some());
    assert_eq!(e.runtime_failure(), None);
}
#[test]
fn owned_example_and_budget_failures_match_in_fresh_runs() {
    let src = include_str!("../examples/ir_stages/owned_values.ceru");
    let expected = "[\"こんにちは\", \"Cerune\"]\n[\"こんにちは！\", \"Cerune\"]\n[\"反復\"]\n";
    assert_eq!(cerune_lang::run_mir(src).unwrap(), expected);
    for source in [
        src,
        include_str!("../examples/dynamic_arrays/lowering_order.ceru"),
    ] {
        for limit in [0, 16, 47, 48, 96, 1024] {
            let mut hir = compile_to_ir(source).unwrap();
            hir.array_heap_limit = limit;
            for strings in [0, 5, 6, 1024] {
                hir.string_heap_limit = strings;
                compare(&hir);
                compare(&hir);
            }
        }
    }
}
#[test]
fn source_map_entry_and_failure_keep_file_ids() {
    use cerune_lang::{ast, lexer, parser, source::SourceMap};
    for body in [
        include_str!("../examples/source_files/main.ceru"),
        include_str!("../examples/source_files/failure.ceru"),
    ] {
        let mut map = SourceMap::new();
        let a = map.add(
            "values.ceru",
            include_str!("../examples/source_files/values.ceru"),
        );
        let b = map.add("entry.ceru", body);
        let mut ast = ast::Program { items: vec![] };
        for id in [a, b] {
            ast.items.extend(
                parser::parse(lexer::lex_source(map.get(id).unwrap()).unwrap())
                    .unwrap()
                    .items,
            );
        }
        compare(&ir::builder::build(&ast).unwrap());
    }
}
#[test]
fn source_api_distinguishes_construction_and_execution() {
    use cerune_lang::{MirRunError, run_mir};
    assert!(matches!(
        run_mir("print(missing);"),
        Err(MirRunError::Compilation(_))
    ));
    let Err(MirRunError::Execution(e)) = run_mir("print(1);print(1/0);") else {
        panic!()
    };
    assert_eq!(e.output(), "1\n");
    assert_eq!(e.runtime_failure().unwrap().code.name(), "division-by-zero");
    assert!(e.location().is_some());
}
#[test]
fn cli_diagnostics_and_options_match_the_explicit_route() {
    use std::process::Command;
    let exe = env!("CARGO_BIN_EXE_cerune");
    for file in [
        "examples/ir_stages/owned_values.ceru",
        "examples/dynamic_arrays/lowering_order.ceru",
        "examples/modules/failure.ceru",
    ] {
        for budget in ["0", "47", "48", "1024"] {
            let args = [
                file,
                "--array-heap-limit",
                budget,
                "--diagnostic-format",
                "runtime-v1",
            ];
            let a = Command::new(exe).arg("run-ir").args(args).output().unwrap();
            let b = Command::new(exe)
                .arg("run-mir")
                .args(args)
                .output()
                .unwrap();
            assert_eq!(a.status, b.status);
            assert_eq!(a.stdout, b.stdout);
            assert_eq!(a.stderr, b.stderr);
        }
    }
    let a = Command::new(exe)
        .args(["run-ir", "examples/modules/failure.ceru"])
        .output()
        .unwrap();
    let b = Command::new(exe)
        .args(["run-mir", "examples/modules/failure.ceru"])
        .output()
        .unwrap();
    assert_eq!(a.stderr, b.stderr);
    for option in ["--target", "--ssa", "-o"] {
        let r = Command::new(exe)
            .args([
                "run-mir",
                "examples/ir_stages/owned_values.ceru",
                option,
                "unused",
            ])
            .output()
            .unwrap();
        assert!(!r.status.success());
        assert!(r.stdout.is_empty());
    }
}
