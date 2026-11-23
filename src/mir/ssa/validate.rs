//! SSA固有の定義・支配・辺を検査し、型とslotの規則は既存MIR検証と共有します。
use super::*;
use std::collections::HashSet;
const INTERNAL: Origin = Origin::Synthetic {
    reason: "ssa-validation",
};

fn require(ok: bool, message: &str, origin: Origin) -> Result<(), mir::Error> {
    if ok {
        Ok(())
    } else {
        Err(mir::Error::new(format!("SSA: {message}"), origin))
    }
}
pub fn validate(p: &Program) -> Result<(), mir::Error> {
    mir::validate(&p.original)?;
    require(
        p.functions.len() == p.original.functions.len(),
        "function count",
        INTERNAL,
    )?;
    // これは型・初期化の検査用の参照置換です。実行・公開loweringには使用しません。
    let mut view = p.original.clone();
    let mut arguments = Vec::new();
    for (n, f) in p.functions.iter().chain([&p.main]).enumerate() {
        let original = p.original.functions.get(n).unwrap_or(&p.original.main);
        let (projected, args) = function(f, original)?;
        if n < view.functions.len() {
            view.functions[n] = projected;
        } else {
            view.main = projected;
        }
        arguments.push(args);
    }
    mir::validate::validate_with_block_parameters(&view, Some(&arguments))
}
fn function(
    f: &Function,
    source: &mir::Function,
) -> Result<(mir::Function, Vec<Vec<LocalId>>), mir::Error> {
    require(f.entry.0 < f.blocks.len(), "unknown entry block", INTERNAL)?;
    let base = source.locals.len();
    let value_type = |v: ValueId| -> Result<&ir::Type, mir::Error> {
        f.values
            .get(v.0)
            .map(|v| &v.ty)
            .ok_or_else(|| mir::Error::new("SSA: unknown value", INTERNAL))
    };
    for v in &f.values {
        require(is_scalar(&v.ty), "non-scalar SSA value", INTERNAL)?;
        require(
            source
                .locals
                .get(v.original_local.0)
                .is_some_and(|l| l.ty == v.ty),
            "value/local type mapping",
            INTERNAL,
        )?;
    }
    let operand = |op: Operand| -> Result<LocalId, mir::Error> {
        match op {
            Operand::Value(v) => {
                value_type(v)?;
                Ok(LocalId(base + v.0))
            }
            Operand::Slot(id) => {
                require(
                    source.locals.get(id.0).is_some_and(|l| !is_scalar(&l.ty)),
                    "unknown or scalar slot",
                    INTERNAL,
                )?;
                Ok(id)
            }
        }
    };
    let mut original_reachable = HashSet::new();
    let mut pending = vec![source.entry];
    while let Some(id) = pending.pop() {
        if original_reachable.insert(id) {
            pending.extend(source.blocks[id.0].terminator.kind.successors());
        }
    }
    let mut recorded = HashSet::new();
    for b in &f.blocks {
        require(
            original_reachable.contains(&b.original_block) && recorded.insert(b.original_block),
            "invalid/duplicate block mapping",
            INTERNAL,
        )?;
    }
    require(
        recorded == original_reachable,
        "missing reachable block mapping",
        INTERNAL,
    )?;
    for &id in &f.retained_unreachable {
        require(
            id.0 < source.blocks.len() && !original_reachable.contains(&id) && recorded.insert(id),
            "invalid retained block",
            INTERNAL,
        )?;
    }
    require(
        recorded.len() == source.blocks.len(),
        "missing retained block",
        INTERNAL,
    )?;
    require(
        f.blocks[f.entry.0].original_block == source.entry,
        "entry mapping",
        INTERNAL,
    )?;
    require(
        f.parameters.len() == source.parameters.len(),
        "parameter count",
        INTERNAL,
    )?;
    let mut entry_arguments = vec![];
    for (&param, &old) in f.parameters.iter().zip(&source.parameters) {
        operand(param)?;
        match param {
            Operand::Value(v) => {
                require(
                    f.values[v.0].original_local == old,
                    "parameter local mapping",
                    INTERNAL,
                )?;
                entry_arguments.push(v);
            }
            Operand::Slot(id) => require(id == old, "parameter slot mapping", INTERNAL)?,
        }
    }
    require(
        entry_arguments == f.blocks[f.entry.0].arguments,
        "entry arguments/signature mismatch",
        INTERNAL,
    )?;

    let mut definitions = vec![None; f.values.len()];
    let mut define =
        |v: ValueId, block: usize, position: Option<usize>| -> Result<(), mir::Error> {
            value_type(v)?;
            require(
                definitions[v.0].replace((block, position)).is_none(),
                "duplicate value definition",
                INTERNAL,
            )
        };
    let mut predecessors = vec![vec![]; f.blocks.len()];
    for (id, b) in f.blocks.iter().enumerate() {
        for &arg in &b.arguments {
            define(arg, id, None)?;
        }
        let original = &source.blocks[b.original_block.0];
        require(
            b.instructions.len() == original.instructions.len(),
            "instruction mapping count",
            INTERNAL,
        )?;
        for (position, (i, old)) in b
            .instructions
            .iter()
            .zip(&original.instructions)
            .enumerate()
        {
            require(
                i.original_instruction == old.id && i.origin == old.origin,
                "instruction origin/order mapping",
                i.origin,
            )?;
            let mut invalid = None;
            i.kind.map_operands(|op| {
                if let Err(e) = operand(op) {
                    invalid = Some(e);
                }
            });
            if let Some(e) = invalid {
                return Err(e);
            }
            match &i.kind {
                mir::InstructionKind::Assign {
                    destination: Operand::Value(v),
                    ..
                } => define(*v, id, Some(position))?,
                mir::InstructionKind::Store { root, .. } => require(
                    matches!(root, Operand::Slot(_)),
                    "store into SSA value",
                    i.origin,
                )?,
                _ => {}
            }
        }
        let t = &b.terminator;
        require(
            t.original_instruction == original.terminator.id
                && t.origin == original.terminator.origin,
            "terminator origin mapping",
            t.origin,
        )?;
        for e in t.kind.edges() {
            require(e.target.0 < f.blocks.len(), "unknown edge target", t.origin)?;
            let args = &f.blocks[e.target.0].arguments;
            require(
                e.arguments.len() == args.len(),
                "edge argument count",
                t.origin,
            )?;
            for (&v, &arg) in e.arguments.iter().zip(args) {
                require(
                    value_type(v)? == value_type(arg)?,
                    "edge argument type",
                    t.origin,
                )?;
            }
            predecessors[e.target.0].push(id);
        }
        match t.kind {
            TerminatorKind::Branch { condition, .. } => require(
                value_type(condition)? == &ir::Type::Bool,
                "branch condition type",
                t.origin,
            )?,
            TerminatorKind::Return(Some(v)) => {
                operand(v)?;
            }
            _ => {}
        }
    }
    require(
        definitions.iter().all(Option::is_some),
        "value without definition",
        INTERNAL,
    )?;
    let mut reachable = HashSet::new();
    let mut pending = vec![f.entry.0];
    while let Some(id) = pending.pop() {
        if reachable.insert(id) {
            pending.extend(
                f.blocks[id]
                    .terminator
                    .kind
                    .edges()
                    .iter()
                    .map(|e| e.target.0),
            );
        }
    }
    require(
        reachable.len() == f.blocks.len(),
        "unreachable SSA block; retain original MIR instead",
        INTERNAL,
    )?;
    // 入口の仮想的な前任を保ち、戻り辺があっても支配の初期条件を変えません。
    let mut dom = vec![reachable.clone(); f.blocks.len()];
    dom[f.entry.0] = HashSet::from([f.entry.0]);
    loop {
        let mut changed = false;
        for id in 0..f.blocks.len() {
            if id == f.entry.0 {
                continue;
            }
            let mut next = reachable.clone();
            for pred in &predecessors[id] {
                next.retain(|v| dom[*pred].contains(v));
            }
            next.insert(id);
            if dom[id] != next {
                dom[id] = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let usage =
        |v: ValueId, block: usize, position: usize, origin: Origin| -> Result<(), mir::Error> {
            value_type(v)?;
            let (definition_block, definition_position) = definitions[v.0].unwrap();
            require(
                dom[block].contains(&definition_block)
                    && (definition_block != block
                        || definition_position.is_none_or(|p| p < position)),
                "value definition does not dominate use",
                origin,
            )
        };
    let mut projected = source.clone();
    projected.entry = f.entry;
    projected.locals.extend(f.values.iter().map(|v| mir::Local {
        ty: v.ty.clone(),
        kind: mir::LocalKind::Temporary,
    }));
    projected.parameters = f
        .parameters
        .iter()
        .map(|p| operand(*p))
        .collect::<Result<_, _>>()?;
    projected.blocks.clear();
    let mut block_arguments = vec![];
    for (id, b) in f.blocks.iter().enumerate() {
        let mut instructions = vec![];
        for (position, i) in b.instructions.iter().enumerate() {
            // すべての参照は上で検査済みです。
            let kind = i.kind.map_operands(|op| operand(op).unwrap());
            for v in mir::validate::reads(&kind) {
                if v.0 >= base {
                    usage(ValueId(v.0 - base), id, position, i.origin)?;
                }
            }
            instructions.push(mir::Instruction {
                id: i.original_instruction,
                origin: i.origin,
                kind,
            });
        }
        let t = &b.terminator;
        let position = b.instructions.len();
        for e in t.kind.edges() {
            for &v in &e.arguments {
                usage(v, id, position, t.origin)?;
            }
        }
        let kind = match &t.kind {
            TerminatorKind::Jump(e) => mir::TerminatorKind::Jump(e.target),
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                usage(*condition, id, position, t.origin)?;
                mir::TerminatorKind::Branch {
                    condition: LocalId(base + condition.0),
                    then_block: then_edge.target,
                    else_block: else_edge.target,
                }
            }
            TerminatorKind::Return(v) => {
                if let Some(Operand::Value(v)) = v {
                    usage(*v, id, position, t.origin)?;
                }
                mir::TerminatorKind::Return(v.map(|v| operand(v).unwrap()))
            }
        };
        projected.blocks.push(mir::Block {
            origin: source.blocks[b.original_block.0].origin,
            instructions,
            terminator: mir::Terminator {
                id: t.original_instruction,
                if_condition: source.blocks[b.original_block.0].terminator.if_condition,
                origin: t.origin,
                kind,
            },
        });
        block_arguments.push(b.arguments.iter().map(|v| LocalId(base + v.0)).collect());
    }
    Ok((projected, block_arguments))
}
