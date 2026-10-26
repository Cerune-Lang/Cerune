use cerune_lang::{
    codegen::x86_64::{self, Target, ir::*, lir},
    compile_to_ir, mir,
};
fn module(source: &str, target: Target) -> Module {
    x86_64::lower_mir(
        &mir::lower(&compile_to_ir(source).unwrap()).unwrap(),
        target,
    )
    .unwrap()
}
const TARGETS: [Target; 2] = [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu];

#[test]
fn all_examples_have_valid_deterministic_baseline_and_ssa_lir() {
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
        let hir = cerune_lang::modules::load(&file).unwrap().to_ir().unwrap();
        let original = mir::lower(&hir).unwrap();
        let ssa = mir::ssa::construct(&original).unwrap();
        let lowered = mir::ssa::lower(&ssa).unwrap();
        for p in [&original, &lowered.program] {
            for target in TARGETS {
                if file.parent() == Some(root.join("examples/files").as_path()) {
                    let error = x86_64::lower_mir(p, target).unwrap_err();
                    assert_eq!(error.message(), "Native file I/O is not supported yet");
                    assert!(error.primary_span().is_some());
                    continue;
                }
                let m = x86_64::lower_mir(p, target)
                    .unwrap_or_else(|e| panic!("{} {target:?}: {e:?}", file.display()));
                let before = m.clone();
                let text = lir::emit(&m).unwrap();
                assert_eq!(
                    text,
                    lir::emit(&x86_64::lower_mir(p, target).unwrap()).unwrap()
                );
                assert_eq!(m, before);
                assert_eq!(
                    x86_64::emit_asm_from_lir(&m, true).unwrap(),
                    x86_64::emit_asm_from_mir(p, target, true).unwrap()
                );
            }
        }
        count += 1;
    }
    eprintln!("validated {count} runnable examples, both targets, baseline and SSA");
    assert!(count >= 120);
}

fn rejects(mut m: Module, change: impl FnOnce(&mut Module), expected: &str) {
    change(&mut m);
    let error = lir::emit(&m).unwrap_err();
    assert!(error.message().contains(expected), "{error:?}");
    assert!(x86_64::emit_asm_from_lir(&m, true).is_err());
    assert!(x86_64::emit_object_from_lir(&m, true).is_err());
}
fn append(m: &mut Module, i: Instruction) {
    m.instructions.push(i);
    m.origins.push(Origin::Synthetic);
    m.mir_origins.push(None);
}
#[test]
fn rejects_invalid_references_origins_frames_and_abi_before_emitting() {
    for target in TARGETS {
        let m = module(
            "fn f(a: i64)->i64{return a;} print(f(1)); print(1.5); print(\"日本語\\0\\r\\n\");",
            target,
        );
        rejects(
            m.clone(),
            |m| {
                m.origins.pop();
            },
            "lengths differ",
        );
        rejects(
            m.clone(),
            |m| {
                m.mir_origins.pop();
            },
            "lengths differ",
        );
        rejects(m.clone(), |m| m.functions[0].id = 10, "IDs");
        rejects(m.clone(), |m| m.frame_size = 1, "aligned");
        rejects(
            m.clone(),
            |m| append(m, Instruction::Jump(usize::MAX)),
            "branch",
        );
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::Label {
                        id: 0,
                        name: "duplicate",
                    },
                )
            },
            "duplicate block",
        );
        rejects(
            m.clone(),
            |m| append(m, Instruction::LoadI64FromStack(0)),
            "stack operand",
        );
        rejects(
            m.clone(),
            |m| append(m, Instruction::LoadI64FromStack(isize::MIN)),
            "stack operand",
        );
        rejects(
            m.clone(),
            |m| {
                let o = -(m.frame_size as isize) - 8;
                append(m, Instruction::LoadI64FromStack(o));
            },
            "stack operand",
        );
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::CopyFromPointer {
                        pointer_offset: -8,
                        destination_offset: -8,
                        slots: usize::MAX,
                    },
                )
            },
            "stack operand",
        );
        rejects(
            m.clone(),
            |m| append(m, Instruction::LoadStringConstant(usize::MAX)),
            "string constant",
        );
        rejects(
            m.clone(),
            |m| append(m, Instruction::LoadF32Constant(usize::MAX)),
            "f32 constant",
        );
        rejects(
            m.clone(),
            |m| {
                m.float_constants.push(m.float_constants[0].clone());
            },
            "duplicate float",
        );
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::Call {
                        function_id: 7,
                        arguments: vec![],
                        aggregate_result_offset: None,
                    },
                )
            },
            "callee",
        );
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::StoreParameter {
                        location: ArgumentLocation::Register(99),
                        ty: Type::I64,
                        offset: -8,
                    },
                )
            },
            "ABI",
        );
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::StoreParameter {
                        location: ArgumentLocation::Stack(1),
                        ty: Type::I64,
                        offset: -8,
                    },
                )
            },
            "ABI",
        );
        rejects(
            m.clone(),
            |m| {
                let n = m
                    .mir_origins
                    .iter()
                    .position(|o| o.is_some_and(|o| o.origin.source().is_some()))
                    .unwrap();
                m.origins[n] = Origin::Synthetic;
            },
            "origins disagree",
        );
        rejects(
            m.clone(),
            |m| {
                append(m, Instruction::TrapIfOverflow(999));
                append(m, Instruction::TrapIfOverflow(999));
            },
            "helper",
        );
        rejects(m.clone(), |m| m.uses_strings = false, "string runtime");
        rejects(
            m.clone(),
            |m| {
                append(
                    m,
                    Instruction::Output {
                        kind: "i64",
                        newline: true,
                    },
                )
            },
            "source origin",
        );
        rejects(
            m.clone(),
            |m| {
                m.uses_write = true;
                append(
                    m,
                    Instruction::Output {
                        kind: "invalid",
                        newline: false,
                    },
                );
                *m.origins.last_mut().unwrap() = m
                    .origins
                    .iter()
                    .copied()
                    .find(|o| matches!(o, Origin::Source { .. }))
                    .unwrap();
            },
            "write kind",
        );
        rejects(m.clone(), |m| m.uses_write = false, "write runtime");
        rejects(
            m.clone(),
            |m| append(m, Instruction::ArrayLength),
            "array runtime",
        );
        rejects(
            m.clone(),
            |m| append(m, Instruction::StringManage { retain: true }),
            "string heap runtime",
        );
        if target == Target::X86_64PcWindowsMsvc {
            rejects(
                m.clone(),
                |m| {
                    let offset = -(m.frame_size as isize);
                    append(m, Instruction::StoreI64ToStack(offset));
                },
                "overlaps",
            );
        }
        let text = lir::emit(&m).unwrap();
        assert!(text.contains("日本語\\0\\r\\n"));
        assert!(text.contains("source_id") && text.contains("MirOrigin"));
    }
}

#[test]
fn target_changes_abi_and_lir_snapshot_drives_native_without_relowering() {
    let source = include_str!("../examples/ir_stages/lir_layout.ceru");
    let windows = module(source, TARGETS[0]);
    let linux = module(source, TARGETS[1]);
    let win = lir::emit(&windows).unwrap();
    let lin = lir::emit(&linux).unwrap();
    assert_ne!(win, lin);
    assert!(win.contains("Stack(32)") && lin.contains("Stack(0)"));
    assert!(win.contains("StoreAggregateReturnPointer") && win.contains("CopyToAggregateReturn"));
    for target in TARGETS {
        let mut m = module("print(1);", target);
        *m.instructions
            .iter_mut()
            .find(|i| matches!(i, Instruction::MovI64ImmediateToRax(1)))
            .unwrap() = Instruction::MovI64ImmediateToRax(42);
        assert!(lir::emit(&m).unwrap().contains("MovI64ImmediateToRax(42)"));
        assert!(
            x86_64::emit_asm_from_lir(&m, false)
                .unwrap()
                .contains("movabsq $42, %rax")
        );
        assert!(!x86_64::emit_object_from_lir(&m, true).unwrap().is_empty());
    }
}
