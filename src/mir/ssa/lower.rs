//! SSAを明示的な一時領域と辺上の並列copyへ戻します。元MIRの実行へのfallbackではありません。
use super::*;
use std::fmt::Write;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lowered {
    pub program: mir::Program,
    /// 入出力snapshotに対する観測用の対応記録です。load形式ではありません。
    pub mapping: String,
}
pub fn lower(p: &Program) -> Result<Lowered, mir::Error> {
    validate(p)?;
    let mut program = p.original.clone();
    let mut mapping = String::from(
        "; Cerune SSA lowering mapping v0.1\n; pass=ssa-lower-v1 options=none; no optimization\n; function-local IDs; original MIR blocks/instruction IDs retained\n",
    );
    for ((f, original), out) in p
        .functions
        .iter()
        .chain([&p.main])
        .zip(p.original.functions.iter().chain([&p.original.main]))
        .zip(program.functions.iter_mut().chain([&mut program.main]))
    {
        function(f, original, out, &mut mapping)?;
    }
    mir::validate(&program)?;
    Ok(Lowered { program, mapping })
}
struct Builder<'a> {
    output: &'a mut mir::Function,
    mapping: &'a mut String,
    next: usize,
    base: usize,
}
impl Builder<'_> {
    fn id(&mut self) -> Result<InstructionId, mir::Error> {
        let id = self.next;
        self.next = id.checked_add(1).ok_or_else(|| {
            mir::Error::new(
                "SSA lowering instruction id overflow",
                Origin::Synthetic {
                    reason: "ssa-lowering",
                },
            )
        })?;
        Ok(InstructionId(id))
    }
    fn copy(
        &mut self,
        to: LocalId,
        from: LocalId,
        origin: Origin,
    ) -> Result<mir::Instruction, mir::Error> {
        Ok(mir::Instruction {
            id: self.id()?,
            origin,
            kind: mir::InstructionKind::Assign {
                destination: to,
                value: mir::Operation::Copy(from),
            },
        })
    }
    fn edge(
        &mut self,
        f: &Function,
        source: usize,
        name: &str,
        edge: &Edge,
        origin: Origin,
    ) -> Result<BlockId, mir::Error> {
        let target = f.blocks[edge.target.0].original_block;
        if edge.arguments.is_empty() {
            writeln!(
                self.mapping,
                "  edge ssa-bb{source}/{name} -> ssa-bb{} direct mir-bb{}",
                edge.target.0, target.0
            )
            .unwrap();
            return Ok(target);
        }
        let helper = BlockId(self.output.blocks.len());
        let origin = derived(origin, "ssa-edge-copy");
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        writeln!(
            self.mapping,
            "  edge ssa-bb{source}/{name} -> ssa-bb{} helper=mir-bb{} target=mir-bb{} {{",
            edge.target.0, helper.0, target.0
        )
        .unwrap();
        for (&from, &to) in edge
            .arguments
            .iter()
            .zip(&f.blocks[edge.target.0].arguments)
        {
            let temporary = LocalId(self.output.locals.len());
            self.output.locals.push(mir::Local {
                ty: f.values[from.0].ty.clone(),
                kind: mir::LocalKind::Temporary,
            });
            let read = self.copy(temporary, LocalId(self.base + from.0), origin)?;
            let write = self.copy(LocalId(self.base + to.0), temporary, origin)?;
            writeln!(
                self.mapping,
                "    v{} -> %{} -> v{} read=mir-i{} write=mir-i{}",
                from.0, temporary.0, to.0, read.id.0, write.id.0
            )
            .unwrap();
            reads.push(read);
            writes.push(write);
        }
        // 必ず全readの後にwriteを置きます。自己ループや循環する受渡しも同じ規則です。
        reads.extend(writes);
        let id = self.id()?;
        self.mapping.push_str("  }\n");
        self.output.blocks.push(mir::Block {
            origin,
            instructions: reads,
            terminator: mir::Terminator {
                id,
                origin,
                if_condition: None,
                kind: mir::TerminatorKind::Jump(target),
            },
        });
        Ok(helper)
    }
}
fn derived(origin: Origin, reason: &'static str) -> Origin {
    origin
        .source()
        .map_or(Origin::Synthetic { reason }, |source| Origin::Derived {
            source,
            reason,
        })
}
fn function(
    f: &Function,
    original: &mir::Function,
    output: &mut mir::Function,
    mapping: &mut String,
) -> Result<(), mir::Error> {
    let name = original.id.map_or("main".into(), |id| format!("@{}", id.0));
    writeln!(mapping, "fn {name} {{").unwrap();
    let base = original.locals.len();
    output.locals.extend(f.values.iter().map(|v| mir::Local {
        ty: v.ty.clone(),
        kind: mir::LocalKind::Temporary,
    }));
    let operand = |op| match op {
        Operand::Value(v) => LocalId(base + v.0),
        Operand::Slot(id) => id,
    };
    for (n, v) in f.values.iter().enumerate() {
        writeln!(
            mapping,
            "  v{n} -> %{} original-local=%{}",
            base + n,
            v.original_local.0
        )
        .unwrap();
    }
    for (n, _) in original
        .locals
        .iter()
        .enumerate()
        .filter(|(_, l)| !is_scalar(&l.ty))
    {
        writeln!(mapping, "  slot{n} -> %{n}").unwrap();
    }
    // 元blockの番号を保持するので、未到達blockからの参照もそのまま有効です。
    let last = original
        .blocks
        .iter()
        .flat_map(|b| {
            b.instructions
                .iter()
                .map(|i| i.id.0)
                .chain([b.terminator.id.0])
        })
        .max()
        .unwrap_or(0);
    let next = last.checked_add(1).ok_or_else(|| {
        mir::Error::new(
            "SSA lowering instruction id overflow",
            Origin::Synthetic {
                reason: "ssa-lowering",
            },
        )
    })?;
    let mut builder = Builder {
        output,
        mapping,
        next,
        base,
    };
    for (index, b) in f.blocks.iter().enumerate() {
        writeln!(
            builder.mapping,
            "  ssa-bb{index} -> mir-bb{} original-block=bb{}",
            b.original_block.0, b.original_block.0
        )
        .unwrap();
        let instructions = b
            .instructions
            .iter()
            .map(|i| mir::Instruction {
                id: i.original_instruction,
                origin: i.origin,
                kind: i.kind.map_operands(operand),
            })
            .collect();
        let t = &b.terminator;
        let kind = match &t.kind {
            TerminatorKind::Jump(e) => {
                mir::TerminatorKind::Jump(builder.edge(f, index, "jump", e, t.origin)?)
            }
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => mir::TerminatorKind::Branch {
                condition: operand(Operand::Value(*condition)),
                then_block: builder.edge(f, index, "then", then_edge, t.origin)?,
                else_block: builder.edge(f, index, "else", else_edge, t.origin)?,
            },
            TerminatorKind::Return(v) => mir::TerminatorKind::Return(v.map(operand)),
        };
        builder.output.blocks[b.original_block.0] = mir::Block {
            origin: original.blocks[b.original_block.0].origin,
            instructions,
            terminator: mir::Terminator {
                id: t.original_instruction,
                if_condition: original.blocks[b.original_block.0].terminator.if_condition,
                origin: t.origin,
                kind,
            },
        };
    }
    builder.output.entry = f.blocks[f.entry.0].original_block;
    let mut parameters = Vec::new();
    let origin = Origin::Synthetic {
        reason: "ssa-parameter-copy",
    };
    // 関数signatureは元のBindingを保ち、入口でSSAのTemporaryへ明示copyします。
    for (&param, &from) in f.parameters.iter().zip(&original.parameters) {
        if let Operand::Value(v) = param {
            let copy = builder.copy(operand(param), from, origin)?;
            writeln!(
                builder.mapping,
                "  parameter %{} -> v{} mir-i{}",
                from.0, v.0, copy.id.0
            )
            .unwrap();
            parameters.push(copy);
        }
    }
    if !parameters.is_empty() {
        let entry = BlockId(builder.output.blocks.len());
        let id = builder.id()?;
        let target = builder.output.entry;
        writeln!(
            builder.mapping,
            "  parameter-entry=mir-bb{} target=mir-bb{}",
            entry.0, target.0
        )
        .unwrap();
        builder.output.blocks.push(mir::Block {
            origin,
            instructions: parameters,
            terminator: mir::Terminator {
                id,
                origin,
                if_condition: None,
                kind: mir::TerminatorKind::Jump(target),
            },
        });
        builder.output.entry = entry;
    }
    for b in &f.retained_unreachable {
        writeln!(
            builder.mapping,
            "  retained original-block=bb{} -> mir-bb{} reason=unreachable",
            b.0, b.0
        )
        .unwrap();
    }
    builder.mapping.push_str("}\n");
    Ok(())
}
