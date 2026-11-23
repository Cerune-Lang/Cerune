//! 手作りSSAのAPI例です。MIR→SSA変換を実装したものではありません。
use cerune_lang::{
    compile_to_ir, ir,
    mir::{
        self, BlockId, InstructionId, InstructionKind as I, Literal, LocalId, Operation as O,
        Origin, SourceOrigin, ssa::*,
    },
    source::Span,
    types::IntegerType,
};
fn v(id: usize) -> Operand {
    Operand::Value(ValueId(id))
}
fn slot(id: usize) -> Operand {
    Operand::Slot(LocalId(id))
}
fn literal(id: usize, value: Literal) -> I<Operand> {
    I::Assign {
        destination: v(id),
        value: O::Literal(value),
    }
}
fn copy(id: usize, from: usize) -> I<Operand> {
    I::Assign {
        destination: v(id),
        value: O::Copy(v(from)),
    }
}
fn jump(target: usize, args: &[usize]) -> TerminatorKind {
    TerminatorKind::Jump(Edge {
        target: BlockId(target),
        arguments: args.iter().copied().map(ValueId).collect(),
    })
}
fn branch(condition: usize, yes: usize, no: usize) -> TerminatorKind {
    TerminatorKind::Branch {
        condition: ValueId(condition),
        then_edge: Edge {
            target: BlockId(yes),
            arguments: vec![],
        },
        else_edge: Edge {
            target: BlockId(no),
            arguments: vec![],
        },
    }
}
type Body = (Vec<usize>, Vec<I<Operand>>, TerminatorKind);
fn fixture(locals: Vec<mir::Local>, mappings: &[usize], bodies: Vec<Body>) -> Program {
    let mut original = mir::lower(&compile_to_ir("").unwrap()).unwrap();
    original.main.locals = locals;
    original.main.blocks.clear();
    let values: Vec<_> = mappings
        .iter()
        .map(|id| Value {
            ty: original.main.locals[*id].ty.clone(),
            original_local: LocalId(*id),
        })
        .collect();
    let local = |op: Operand| match op {
        Operand::Value(v) => values[v.0].original_local,
        Operand::Slot(id) => id,
    };
    let mut blocks = vec![];
    let mut next = 0;
    for (index, (args, instructions, kind)) in bodies.into_iter().enumerate() {
        let mut mapped = vec![];
        let mut source_instructions = vec![];
        for kind in instructions {
            let id = InstructionId(next);
            next += 1;
            let origin = Origin::Source(SourceOrigin {
                node_id: ir::NodeId(id.0),
                span: Span::new(id.0, id.0 + 1),
            });
            source_instructions.push(mir::Instruction {
                id,
                origin,
                kind: kind.map_operands(local),
            });
            mapped.push(Instruction {
                original_instruction: id,
                origin,
                kind,
            });
        }
        let id = InstructionId(next);
        next += 1;
        let origin = Origin::Source(SourceOrigin {
            node_id: ir::NodeId(id.0),
            span: Span::new(id.0, id.0 + 1),
        });
        let source_kind = match &kind {
            TerminatorKind::Jump(e) => mir::TerminatorKind::Jump(e.target),
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => mir::TerminatorKind::Branch {
                condition: local(v(condition.0)),
                then_block: then_edge.target,
                else_block: else_edge.target,
            },
            TerminatorKind::Return(value) => mir::TerminatorKind::Return(value.map(local)),
        };
        original.main.blocks.push(mir::Block {
            origin,
            instructions: source_instructions,
            terminator: mir::Terminator {
                id,
                origin,
                if_condition: None,
                kind: source_kind,
            },
        });
        blocks.push(Block {
            original_block: BlockId(index),
            arguments: args.into_iter().map(ValueId).collect(),
            instructions: mapped,
            terminator: Terminator {
                original_instruction: id,
                origin,
                kind,
            },
        });
    }
    Program {
        construction: None,
        original,
        functions: vec![],
        main: Function {
            values,
            parameters: vec![],
            entry: BlockId(0),
            blocks,
            retained_unreachable: vec![],
        },
    }
}
fn temporary(ty: ir::Type) -> mir::Local {
    mir::Local {
        ty,
        kind: mir::LocalKind::Temporary,
    }
}
fn u8_type() -> ir::Type {
    ir::Type::Integer(IntegerType::U8)
}
pub fn diamond() -> Program {
    fixture(
        vec![temporary(ir::Type::Bool), temporary(u8_type())],
        &[0, 1, 1, 1, 1],
        vec![
            (
                vec![],
                vec![
                    literal(0, Literal::Boolean(true)),
                    literal(1, Literal::Integer(10)),
                ],
                branch(0, 1, 2),
            ),
            (vec![], vec![copy(2, 1)], jump(3, &[2])),
            (vec![], vec![copy(3, 1)], jump(3, &[3])),
            (
                vec![4],
                vec![I::Output {
                    value: v(4),
                    newline: true,
                    quoted: false,
                }],
                TerminatorKind::Return(None),
            ),
        ],
    )
}
pub fn parallel_loop() -> Program {
    let mut p = fixture(
        vec![
            temporary(ir::Type::Bool),
            temporary(u8_type()),
            temporary(u8_type()),
        ],
        &[0, 1, 2, 1, 2],
        vec![
            (
                vec![],
                vec![
                    literal(0, Literal::Boolean(true)),
                    literal(1, Literal::Integer(10)),
                    literal(2, Literal::Integer(20)),
                ],
                jump(1, &[1, 2]),
            ),
            (
                vec![3, 4],
                vec![],
                TerminatorKind::Branch {
                    condition: ValueId(0),
                    then_edge: Edge {
                        target: BlockId(1),
                        arguments: vec![ValueId(4), ValueId(3)],
                    },
                    else_edge: Edge {
                        target: BlockId(2),
                        arguments: vec![],
                    },
                },
            ),
            (
                vec![],
                vec![
                    I::Output {
                        value: v(3),
                        newline: true,
                        quoted: false,
                    },
                    I::Output {
                        value: v(4),
                        newline: true,
                        quoted: false,
                    },
                ],
                TerminatorKind::Return(None),
            ),
        ],
    );
    // この構造検査例は実行しません。未到達の記録も表示します。
    let id = InstructionId(100);
    p.original.main.blocks.push(mir::Block {
        origin: Origin::Synthetic {
            reason: "retained-example",
        },
        instructions: vec![],
        terminator: mir::Terminator {
            id,
            origin: Origin::Synthetic {
                reason: "retained-example",
            },
            if_condition: None,
            kind: mir::TerminatorKind::Unreachable,
        },
    });
    p.main.retained_unreachable.push(BlockId(3));
    p
}
pub fn residual_slots() -> Program {
    let mut locals = vec![
        temporary(u8_type()),
        temporary(ir::Type::Integer(IntegerType::I64)),
    ];
    locals.push(mir::Local {
        ty: ir::Type::Array {
            element: Box::new(u8_type()),
            length: 1,
        },
        kind: mir::LocalKind::Binding {
            id: ir::BindingId(0),
            name: "values".into(),
            mutable: true,
            borrowed: false,
        },
    });
    locals.push(temporary(ir::Type::String));
    let mut p = fixture(
        locals,
        &[0, 1],
        vec![(
            vec![],
            vec![
                literal(0, Literal::Integer(7)),
                literal(1, Literal::Integer(0)),
                I::Assign {
                    destination: slot(2),
                    value: O::Array(vec![v(0)]),
                },
                I::CheckIndex {
                    root: slot(2),
                    path: vec![v(1)],
                },
                I::Store {
                    root: slot(2),
                    path: vec![v(1)],
                    value: v(0),
                },
                I::Assign {
                    destination: slot(3),
                    value: O::Literal(Literal::String("日本語\0\r\néé".into())),
                },
                I::Output {
                    value: slot(3),
                    newline: true,
                    quoted: false,
                },
            ],
            TerminatorKind::Return(None),
        )],
    );
    p.original.uses_strings = true;
    p
}
