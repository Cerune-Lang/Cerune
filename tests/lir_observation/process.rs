use super::*;
use cerune_lang::{ir, modules, types::IntegerType};
fn sample(target: Target) -> Module {
    module(
        include_str!("../../examples/process_execution/unconfigured.ceru"),
        target,
    )
}
fn process_body(m: &mut Module) -> (&mut Function, usize) {
    let f = m
        .functions
        .iter_mut()
        .find(|f| {
            f.instructions
                .iter()
                .any(|i| matches!(i, Instruction::ProcessRun { .. }))
        })
        .unwrap();
    let index = f
        .instructions
        .iter()
        .position(|i| matches!(i, Instruction::ProcessRun { .. }))
        .unwrap();
    (f, index)
}
fn change_operation(m: &mut Module, change: impl FnOnce(&mut Instruction)) {
    let (f, index) = process_body(m);
    change(&mut f.instructions[index]);
}
#[test]
fn keeps_process_operands_types_and_origin_for_baseline_and_ssa() {
    let compiled = modules::from_source(include_str!(
        "../../examples/process_execution/unconfigured.ceru"
    ))
    .unwrap();
    let hir = compiled.to_ir().unwrap();
    let baseline = mir::lower(&hir).unwrap();
    let ssa = mir::ssa::lower(&mir::ssa::construct(&baseline).unwrap()).unwrap();
    for p in [&baseline, &ssa.program] {
        for target in TARGETS {
            let m = x86_64::lower_mir(p, target).unwrap();
            assert_eq!(m.types, p.types);
            let request = m
                .types
                .iter()
                .find(|d| d.fields.iter().any(|(_, n, _)| n == "target"))
                .unwrap();
            assert_eq!(request.fields.len(), 6);
            let text = lir::emit(&m).unwrap();
            assert!(text.contains("ProcessRun { request_offset:"));
            assert!(text.contains("\"target\" String slot=0 byte-displacement=0"));
            assert!(text.contains("\"stdout_limit\" Integer(U64) slot=3 byte-displacement=-24"));
            assert!(text.contains("\"Unconfigured\""));
            let mut seen = 0;
            for (mf, lf) in p.functions.iter().zip(&m.functions).chain(std::iter::once((
                &p.main,
                &Function {
                    id: 0,
                    declaration: None,
                    name: "main".into(),
                    frame_size: m.frame_size,
                    origins: m.origins.clone(),
                    mir_origins: m.mir_origins.clone(),
                    instructions: m.instructions.clone(),
                },
            ))) {
                let expected = mf
                    .blocks
                    .iter()
                    .enumerate()
                    .flat_map(|(bid, b)| {
                        b.instructions.iter().filter_map(move |i| {
                            if let mir::InstructionKind::Assign {
                                value:
                                    mir::Operation::ProcessRun {
                                        request_type,
                                        outcome_type,
                                        ..
                                    },
                                ..
                            } = i.kind
                            {
                                Some((
                                    mir::BlockId(bid),
                                    i.id,
                                    i.origin,
                                    request_type,
                                    outcome_type,
                                ))
                            } else {
                                None
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let actual = lf
                    .instructions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, i)| {
                        if let Instruction::ProcessRun {
                            request_type,
                            outcome_type,
                            request_offset,
                            outcome_offset,
                        } = i
                        {
                            assert_ne!(request_offset, outcome_offset);
                            let origin = lf.mir_origins[index].unwrap();
                            let source = origin.origin.source().unwrap();
                            assert_eq!(
                                lf.origins[index],
                                Origin::Source {
                                    node_id: source.node_id,
                                    span: source.span
                                }
                            );
                            assert_eq!(Some(source.span), hir.first_process_span());
                            Some((
                                origin.block,
                                origin.instruction.unwrap(),
                                origin.origin,
                                *request_type,
                                *outcome_type,
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                seen += actual.len();
                assert_eq!(
                    actual, expected,
                    "MIR operation order/identity must survive lowering"
                );
            }
            assert_eq!(seen, 1);
            assert_eq!(text, lir::emit(&m).unwrap());
            let program = x86_64::build_program_from_lir(&m, true).unwrap();
            let before = program.assemble().unwrap();
            let text = program.render_asm().unwrap();
            assert!(text.contains("cerune_process_v1_prepare"));
            assert!(text.contains("cerune_process_reserve"));
            assert!(
                before.object == program.assemble().unwrap().object,
                "rendering must not change Object bytes"
            );
            assert!(!x86_64::emit_object_from_lir(&m, false).unwrap().is_empty());
            assert!(program.origins().iter().any(|o|matches!(o.source, Origin::Source{span,..} if Some(span)==hir.first_process_span())));
        }
    }
}
#[test]
fn rejects_process_frame_type_and_origin_corruption_before_generation() {
    for target in TARGETS {
        let m = sample(target);
        rejects(
            m.clone(),
            |m| {
                change_operation(m, |i| {
                    if let Instruction::ProcessRun { request_type, .. } = i {
                        *request_type = ir::TypeId(usize::MAX);
                    }
                })
            },
            "missing process request type",
        );
        rejects(
            m.clone(),
            |m| {
                change_operation(m, |i| {
                    if let Instruction::ProcessRun { outcome_type, .. } = i {
                        *outcome_type = ir::TypeId(usize::MAX);
                    }
                })
            },
            "missing process outcome type",
        );
        for offset in [0, isize::MIN, -8_000_000] {
            rejects(
                m.clone(),
                |m| {
                    change_operation(m, |i| {
                        if let Instruction::ProcessRun { request_offset, .. } = i {
                            *request_offset = offset;
                        }
                    })
                },
                "stack operand",
            );
        }
        rejects(
            m.clone(),
            |m| {
                change_operation(m, |i| {
                    if let Instruction::ProcessRun {
                        request_offset,
                        outcome_offset,
                        ..
                    } = i
                    {
                        *request_offset = *outcome_offset;
                    }
                })
            },
            "overlap",
        );
        rejects(
            m.clone(),
            |m| {
                let (f, index) = process_body(m);
                f.origins[index] = Origin::Synthetic;
                f.mir_origins[index] = None;
            },
            "I/O needs a source origin",
        );
        rejects(
            m.clone(),
            |m| m.string_heap_limit = None,
            "string heap runtime is absent",
        );
        rejects(
            m.clone(),
            |m| m.array_heap_limit = None,
            "array runtime is absent",
        );
        rejects(m.clone(), |m| m.types[0].id = ir::TypeId(200), "type IDs");
        rejects(
            m.clone(),
            |m| m.types[0].fields[0].0 = ir::FieldId(200),
            "field IDs",
        );
        rejects(
            m.clone(),
            |m| m.types[0].fields[0].2 = ir::Type::Named(ir::TypeId(999)),
            "missing named type",
        );
        rejects(
            m.clone(),
            |m| m.types[0].fields[0].2 = ir::Type::Named(ir::TypeId(0)),
            "recursive inline type",
        );
        rejects(
            m.clone(),
            |m| {
                m.types[0].fields[0].2 = ir::Type::Array {
                    element: Box::new(ir::Type::Array {
                        element: Box::new(ir::Type::Integer(IntegerType::U64)),
                        length: usize::MAX,
                    }),
                    length: 2,
                }
            },
            "slot count overflow",
        );
        rejects(
            m.clone(),
            |m| {
                m.types
                    .iter_mut()
                    .find(|d| d.variants.is_some())
                    .unwrap()
                    .variants = Some(vec!["same".into(), "same".into()])
            },
            "enum variants",
        );
    }
}

#[test]
fn unused_large_type_does_not_require_an_impossible_native_stack_frame() {
    for target in TARGETS {
        let mut m = module("print(7);", target);
        let before = x86_64::emit_object_from_lir(&m, true).unwrap();
        m.types.push(mir::TypeDefinition {
            id: ir::TypeId(m.types.len()),
            name: "UnusedLarge".into(),
            variants: None,
            fields: vec![
                (
                    ir::FieldId(0),
                    "items".into(),
                    ir::Type::Array {
                        element: Box::new(ir::Type::Integer(IntegerType::I64)),
                        length: i32::MAX as usize,
                    },
                ),
                (ir::FieldId(1), "last".into(), ir::Type::Bool),
            ],
        });
        assert!(
            lir::emit(&m)
                .unwrap()
                .contains("byte-displacement=-17179869176")
        );
        assert_eq!(before, x86_64::emit_object_from_lir(&m, true).unwrap());
    }
}

#[test]
fn rejects_process_field_corruption_before_machine_selection() {
    for target in TARGETS {
        let original = sample(target);
        let mut copy = original.clone();
        let (f, index) = process_body(&mut copy);
        let Instruction::ProcessRun {
            request_type,
            outcome_type,
            ..
        } = f.instructions[index]
        else {
            unreachable!()
        };
        for (id, name, message) in [
            (request_type, "target", "invalid process request fields"),
            (outcome_type, "stdout", "invalid process outcome fields"),
        ] {
            rejects(
                original.clone(),
                |m| {
                    m.types[id.0]
                        .fields
                        .iter_mut()
                        .find(|(_, n, _)| n == name)
                        .unwrap()
                        .2 = ir::Type::Integer(IntegerType::U64);
                },
                message,
            );
        }
        let report = original
            .types
            .iter()
            .find(|t| t.fields.iter().any(|(_, n, _)| n == "progress_known"))
            .unwrap()
            .id;
        rejects(
            original.clone(),
            |m| {
                m.types[report.0]
                    .fields
                    .iter_mut()
                    .find(|(_, n, _)| n == "progress_known")
                    .unwrap()
                    .2 = ir::Type::String;
            },
            "invalid process report fields",
        );
        let started = original
            .types
            .iter()
            .find(|t| {
                t.variants
                    .as_ref()
                    .is_some_and(|v| v.iter().any(|n| n == "Unknown"))
            })
            .unwrap()
            .id;
        rejects(
            original.clone(),
            |m| {
                m.types[started.0].variants.as_mut().unwrap()[0] = "Unexpected".into();
            },
            "invalid process report fields",
        );
        // Enum numbers belong to the actual LIR declaration, not a fixed Rust enum order.
        let mut reordered = original;
        reordered.types[started.0]
            .variants
            .as_mut()
            .unwrap()
            .reverse();
        assert!(x86_64::emit_object_from_lir(&reordered, false).is_ok());
    }
}
