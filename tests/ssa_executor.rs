use cerune_lang::{
    compile_to_ir, ir, ir_executor,
    mir::{self, InstructionKind as I, Operation as O, ssa},
    mir_executor,
    runtime::input::Code,
    ssa_executor as executor,
    vm::VmErrorKind,
};
#[path = "../experiments/ssa/fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
#[path = "support/runtime_cases.rs"]
#[allow(dead_code)]
mod runtime_cases;
fn program(source: &str) -> ssa::Program {
    ssa::construct(&mir::lower(&compile_to_ir(source).unwrap()).unwrap()).unwrap()
}

fn compare(hir: &ir::Program) {
    let mir = mir::lower(hir).unwrap();
    let p = ssa::construct(&mir).unwrap();
    let before = p.clone();

    let a = cerune_lang::ir_executor::run(hir);
    let b = cerune_lang::mir_executor::run(&mir);
    let c = executor::run(&p);

    let lowered = ssa::lower(&p).unwrap();
    assert_eq!(lowered, ssa::lower(&p).unwrap(), "deterministic lowering");
    assert_eq!(
        cerune_lang::mir_executor::run(&lowered.program),
        b,
        "lowered SSA agrees with original MIR"
    );

    let d = cerune_lang::run_bytecode(&cerune_lang::bytecode::lower(hir).unwrap());

    match (&a, &b, &c, &d) {
        (Ok(a), Ok(b), Ok(c), Ok(d)) => {
            assert_eq!(a, b);
            assert_eq!(a, c);
            assert_eq!(a, d);
        }

        (Err(a), Err(b), Err(c), Err(d)) => {
            match (a.kind(), b.kind(), c.kind(), d.vm_error().kind()) {
                (
                    ir_executor::ErrorKind::Input(ai),
                    mir_executor::ErrorKind::Input(bi),
                    mir_executor::ErrorKind::Input(ci),
                    VmErrorKind::Input(di),
                ) => {
                    assert_eq!(ai.code, Code::Unconfigured);
                    assert_eq!(ai.as_ref(), bi.as_ref());
                    assert_eq!(ai.as_ref(), ci.as_ref());
                    assert_eq!(ai.as_ref(), di.as_ref());
                }

                _ => {
                    assert!(a.runtime_failure().is_some(), "{a:?}");
                    assert_eq!(a.runtime_failure(), b.runtime_failure());
                    assert_eq!(a.runtime_failure(), c.runtime_failure(), "{c:?}");
                    assert_eq!(a.runtime_failure(), d.runtime_failure());
                }
            }

            // 入力エラー・Runtimeエラー両方で出力を比較
            assert_eq!(a.output(), b.output());
            assert_eq!(a.output(), c.output());
            assert_eq!(a.output(), d.vm_error().output());

            // MIRとSSAの命令位置の対応を検証
            let original = b.location().unwrap();
            let loc = c.location().unwrap();

            assert_eq!(loc.function, original.function);
            assert_eq!(loc.original_block, original.block);
            assert_eq!(loc.original_instruction, original.instruction);

            let f = loc.function.map_or(&p.main, |id| &p.functions[id.0]);
            assert_eq!(f.blocks[loc.block.0].original_block, loc.original_block);
        }

        _ => panic!("HIR={a:?}\nMIR={b:?}\nSSA={c:?}\nVM={d:?}"),
    }

    assert_eq!(executor::run(&p), c, "fresh execution");
    assert_eq!(p, before);
    assert_eq!(p.original, mir);
}

#[test]
fn all_runnable_examples_match_hir_mir_and_vm() {
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                files(&p, out);
            } else if p.extension().is_some_and(|e| e == "ceru") {
                out.push(p);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut all = vec![];
    files(&root.join("examples"), &mut all);
    all.sort();
    let mut count = 0;
    for file in all {
        if file.parent() == Some(root.join("examples/source_files").as_path()) {
            continue;
        }
        eprintln!("SSA comparison: {}", file.display());
        compare(&cerune_lang::modules::load(&file).unwrap().to_ir().unwrap());
        count += 1;
    }
    eprintln!("compared {count} runnable examples");
    assert!(count >= 115, "coverage: {count}");
}
#[test]
fn ssa_bodies_drive_execution_without_running_original_mir() {
    for source in ["print(1);", "fn f()->i64{return 1;} print(f());"] {
        let mut p = program(source);
        let original = p.original.clone();
        for f in p.functions.iter_mut().chain([&mut p.main]) {
            for i in f.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
                if let I::Assign {
                    value: O::Literal(mir::Literal::Integer(v)),
                    ..
                } = &mut i.kind
                {
                    *v = 42;
                }
            }
        }
        assert_eq!(executor::run(&p).unwrap(), "42\n");
        assert_eq!(
            cerune_lang::mir_executor::run(&ssa::lower(&p).unwrap().program).unwrap(),
            "42\n"
        );
        assert_eq!(cerune_lang::mir_executor::run(&p.original).unwrap(), "1\n");
        assert_eq!(p.original, original);
    }
}
#[test]
fn known_outputs_and_heap_budget_boundaries() {
    let source = include_str!("../examples/ir_stages/ssa_values.ceru");
    assert_eq!(executor::run(&program(source)).unwrap(), "14\n16\n20\n10\n");
    let constants = include_str!("../examples/ir_stages/constant_candidates.ceru");
    compare(&compile_to_ir(constants).unwrap());
    assert_eq!(
        executor::run(&program(constants)).unwrap(),
        "42\n18446744073709551615\ntrue\nfalse\ntrue\n4\n10\n7\n"
    );
    let failure = include_str!("../examples/runtime_failures/constant_overflow.ceru");
    compare(&compile_to_ir(failure).unwrap());
    let error = executor::run(&program(failure)).unwrap_err();
    assert_eq!(error.output(), "7\n");
    let stop = error.runtime_failure().unwrap();
    assert_eq!(stop.code.name(), "integer-overflow");
    assert_eq!(&failure[stop.span.start()..stop.span.end()], "limit + 1");
    let owned = include_str!("../examples/ir_stages/owned_values.ceru");
    assert_eq!(
        executor::run(&program(owned)).unwrap(),
        "[\"こんにちは\", \"Cerune\"]\n[\"こんにちは！\", \"Cerune\"]\n[\"反復\"]\n"
    );
    for source in [
        owned,
        include_str!("../examples/dynamic_arrays/lowering_order.ceru"),
    ] {
        for arrays in [0, 16, 47, 48, 96, 1024] {
            for strings in [0, 5, 6, 1024] {
                let mut hir = compile_to_ir(source).unwrap();
                hir.array_heap_limit = arrays;
                hir.string_heap_limit = strings;
                compare(&hir);
            }
        }
    }
}
#[test]
fn runtime_failure_matrix_preserves_codes_origins_and_prior_output() {
    for &(source, code) in runtime_cases::FAILURES {
        let source = format!("print(7); {source}");
        let hir = compile_to_ir(&source).unwrap();
        compare(&hir);
        let e = executor::run(&program(&source)).unwrap_err();
        assert_eq!(e.runtime_failure().unwrap().code.name(), code);
        assert!(e.output().starts_with("7\n"));
    }
    compare(&compile_to_ir(include_str!("../experiments/lean/for_update_failure.ceru")).unwrap());
}
#[test]
fn invalid_ssa_is_rejected_before_output_and_recursive_calls_are_internal_errors() {
    let mut p = program("print(1);");
    p.main.blocks[0].terminator.kind = ssa::TerminatorKind::Jump(ssa::Edge {
        target: mir::BlockId(999),
        arguments: vec![],
    });
    let e = executor::run(&p).unwrap_err();
    assert!(matches!(e.kind(), executor::ErrorKind::Validation(_)));
    assert!(e.output().is_empty());
    assert!(e.runtime_failure().is_none());
    let mut p = program("fn f()->i64{return 1;} print(f());");
    for i in p.functions[0]
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
    {
        if let I::Assign { value, .. } = &mut i.kind
            && matches!(value, O::Literal(mir::Literal::Integer(1)))
        {
            *value = O::Call {
                function: ir::FunctionId(0),
                arguments: vec![],
                ownership: ir::ArgumentOwnership::Owned,
            };
        }
    }
    let e = executor::run(&p).unwrap_err();
    assert!(matches!(e.kind(), executor::ErrorKind::InvalidMir(_)));
    assert!(e.runtime_failure().is_none());
}
#[test]
fn branch_to_same_block_uses_only_selected_edge_arguments() {
    use ssa::{Edge, Operand, TerminatorKind, ValueId};
    let mut p = fixtures::parallel_loop();
    // 観測用の無限loopを終了するCFGに変更し、同じ行き先の二つの辺を区別します。
    p.main.blocks[1].terminator.kind = TerminatorKind::Branch {
        condition: ValueId(0),
        then_edge: Edge {
            target: mir::BlockId(2),
            arguments: vec![ValueId(3), ValueId(4)],
        },
        else_edge: Edge {
            target: mir::BlockId(2),
            arguments: vec![ValueId(4), ValueId(3)],
        },
    };
    p.main
        .values
        .extend([p.main.values[3].clone(), p.main.values[4].clone()]);
    p.main.blocks[2].arguments = vec![ValueId(5), ValueId(6)];
    for i in &mut p.main.blocks[2].instructions {
        i.kind = i.kind.map_operands(|op| match op {
            Operand::Value(ValueId(3)) => Operand::Value(ValueId(5)),
            Operand::Value(ValueId(4)) => Operand::Value(ValueId(6)),
            other => other,
        });
    }
    for (condition, expected) in [(true, "10\n20\n"), (false, "20\n10\n")] {
        let I::Assign {
            value: O::Literal(mir::Literal::Boolean(v)),
            ..
        } = &mut p.main.blocks[0].instructions[0].kind
        else {
            panic!()
        };
        *v = condition;
        assert_eq!(executor::run(&p).unwrap(), expected);
        assert_eq!(
            cerune_lang::mir_executor::run(&ssa::lower(&p).unwrap().program).unwrap(),
            expected
        );
    }
    assert_eq!(
        executor::run(&fixtures::residual_slots()).unwrap(),
        "日本語\0\r\néé\n"
    );
}
#[test]
fn missing_cleanup_is_not_reported_as_success() {
    let mut p = program("a:[i64]=array_copy([1]); print(a);");
    let mut changed = false;
    for i in p
        .functions
        .iter_mut()
        .chain([&mut p.main])
        .flat_map(|f| &mut f.blocks)
        .flat_map(|b| &mut b.instructions)
    {
        if let I::ArrayFree { value } = i.kind {
            i.kind = I::ArrayRetain { value };
            changed = true;
        }
    }
    assert!(changed);
    ssa::validate(&p).unwrap();
    let e = executor::run(&p).unwrap_err();
    assert!(matches!(e.kind(), executor::ErrorKind::InvalidMir(_)));
    assert_eq!(e.output(), "[1]\n");
    assert!(e.runtime_failure().is_none());
}
#[test]
fn source_map_and_nested_calls_keep_the_failing_callee() {
    use cerune_lang::{ast, lexer, parser, source::SourceMap};
    let mut map = SourceMap::new();
    let a = map.add(
        "values.ceru",
        "fn inner(x:i64)->i64{return 1/x;} fn outer()->i64{return inner(0);}",
    );
    let b = map.add("entry.ceru", "print(7); print(outer());");
    let mut ast = ast::Program { items: vec![] };
    for id in [a, b] {
        ast.items.extend(
            parser::parse(lexer::lex_source(map.get(id).unwrap()).unwrap())
                .unwrap()
                .items,
        );
    }
    let hir = ir::builder::build(&ast).unwrap();
    compare(&hir);
    let p = ssa::construct(&mir::lower(&hir).unwrap()).unwrap();
    let e = executor::run(&p).unwrap_err();
    assert_eq!(e.location().unwrap().function, Some(ir::FunctionId(0)));
    assert_eq!(e.origin().unwrap().span.source_id(), a);
}
