//! 内部MIRの構造・型・初期化を検査します。heapの寿命証明や外部入力の安全性保証ではありません。
use super::*;
use crate::ir::{BinaryOp as B, Type as T, UnaryOp as U};
use std::collections::HashSet;
const I64: T = T::Integer(IntegerType::I64);
const INTERNAL: Origin = Origin::Synthetic {
    reason: "validation",
};

pub fn validate(p: &Program) -> Result<(), Error> {
    validate_with_block_parameters(p, None)
}

/// SSAの型・slot検査用です。値の支配関係はSSA側で別に検査します。
pub(super) fn validate_with_block_parameters(
    p: &Program,
    block_parameters: Option<&[Vec<Vec<LocalId>>]>,
) -> Result<(), Error> {
    if let Some(args) = block_parameters {
        require(
            args.len() == p.functions.len() + 1,
            "block parameter function count",
            INTERNAL,
        )?;
    }
    for (index, t) in p.types.iter().enumerate() {
        require(t.id.0 == index, "type id/layout mismatch", INTERNAL)?;
        for (i, (id, _, ty)) in t.fields.iter().enumerate() {
            require(id.0 == i, "field id/layout mismatch", INTERNAL)?;
            check_type(p, ty)?;
        }
    }
    for (i, f) in p.functions.iter().enumerate() {
        require(
            f.id == Some(ir::FunctionId(i)),
            "function id/layout mismatch",
            INTERNAL,
        )?;
        function(p, f, block_parameters.map(|a| a[i].as_slice()))?;
    }
    require(
        p.main.id.is_none()
            && p.main.parameters.is_empty()
            && p.main.return_type == ir::ReturnType::Void,
        "invalid main signature",
        INTERNAL,
    )?;
    function(
        p,
        &p.main,
        block_parameters.map(|a| a[p.functions.len()].as_slice()),
    )
}
fn require(ok: bool, message: &str, origin: Origin) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::new(message, origin))
    }
}
fn check_type(p: &Program, ty: &T) -> Result<(), Error> {
    match ty {
        T::Named(id) => require(p.types.get(id.0).is_some(), "unknown type", INTERNAL),
        T::Array { element, .. } | T::DynamicArray { element } => check_type(p, element),
        _ => Ok(()),
    }
}
fn local(f: &Function, id: LocalId, o: Origin) -> Result<&T, Error> {
    f.locals
        .get(id.0)
        .map(|l| &l.ty)
        .ok_or_else(|| Error::new("unknown local", o))
}
fn numeric(t: NumericType) -> T {
    match t {
        NumericType::Integer(t) => T::Integer(t),
        NumericType::F32 => T::F32,
        NumericType::F64 => T::F64,
    }
}
fn is_numeric(t: &T) -> bool {
    matches!(t, T::Integer(_) | T::F32 | T::F64)
}
fn element(t: &T, o: Origin) -> Result<&T, Error> {
    match t {
        T::Array { element, .. } | T::DynamicArray { element } => Ok(element),
        _ => Err(Error::new("expected array", o)),
    }
}
fn dynamic(t: &T, o: Origin) -> Result<&T, Error> {
    match t {
        T::DynamicArray { element } => Ok(element),
        _ => Err(Error::new("expected dynamic array", o)),
    }
}
fn call<'a>(
    p: &'a Program,
    f: &Function,
    id: ir::FunctionId,
    args: &[LocalId],
    ownership: ir::ArgumentOwnership,
    o: Origin,
) -> Result<&'a ir::ReturnType, Error> {
    let callee = p
        .functions
        .get(id.0)
        .ok_or_else(|| Error::new("unknown callee", o))?;
    require(
        callee.parameters.len() == args.len(),
        "call argument count mismatch",
        o,
    )?;
    require(
        callee.argument_ownership == ownership,
        "call ownership mismatch",
        o,
    )?;
    for (&arg, &param) in args.iter().zip(&callee.parameters) {
        require(
            local(f, arg, o)? == local(callee, param, o)?,
            "call argument type mismatch",
            o,
        )?;
    }
    Ok(&callee.return_type)
}
fn operation(p: &Program, f: &Function, op: &Operation, dst: &T, o: Origin) -> Result<(), Error> {
    let ty = |v| local(f, v, o);
    let same = |a: &T, b: &T| require(a == b, "operation type mismatch", o);
    match op {
        Operation::Literal(Literal::Boolean(_)) => same(dst, &T::Bool),
        Operation::Literal(Literal::String(_)) => same(dst, &T::String),
        Operation::Literal(Literal::Integer(v)) => require(
            matches!(dst,T::Integer(t) if t.contains(*v)),
            "integer literal type/range",
            o,
        ),
        Operation::Literal(Literal::Float(v)) => require(
            match dst {
                T::F32 => v.parse::<f32>().is_ok(),
                T::F64 => v.parse::<f64>().is_ok(),
                _ => false,
            },
            "float literal type/text",
            o,
        ),
        Operation::Copy(v) => same(dst, ty(*v)?),
        Operation::Unary { op, value } => {
            let t = ty(*value)?;
            same(dst, t)?;
            require(
                match op {
                    U::Not => *t == T::Bool,
                    U::BitNot => matches!(t, T::Integer(_)),
                    U::Negate => {
                        matches!(t, T::F32 | T::F64) | matches!(t,T::Integer(i) if i.is_signed())
                    }
                },
                "unary operand type",
                o,
            )
        }
        Operation::Binary { op, left, right } => {
            let a = ty(*left)?;
            let b = ty(*right)?;
            same(a, b)?;
            match op {
                B::Equal | B::NotEqual => {
                    same(dst, &T::Bool)?;
                    require(
                        matches!(a, T::Bool | T::String) || is_numeric(a),
                        "unexpanded aggregate comparison",
                        o,
                    )
                }
                B::Less | B::LessEqual | B::Greater | B::GreaterEqual => {
                    same(dst, &T::Bool)?;
                    require(is_numeric(a), "ordered operand type", o)
                }
                B::BitAnd | B::BitOr | B::BitXor | B::ShiftLeft | B::ShiftRight => {
                    same(dst, a)?;
                    require(matches!(a, T::Integer(_)), "bit operand type", o)
                }
                _ => {
                    same(dst, a)?;
                    require(is_numeric(a), "arithmetic operand type", o)
                }
            }
        }
        Operation::ConvertInteger {
            value, from, to, ..
        } => {
            same(ty(*value)?, &T::Integer(*from))?;
            same(dst, &T::Integer(*to))
        }
        Operation::ConvertNumeric {
            value,
            from,
            to,
            mode,
            ..
        } => {
            same(ty(*value)?, &numeric(*from))?;
            same(dst, &numeric(*to))?;
            require(
                *mode == ConversionMode::Exact
                    || matches!(
                        (from, to),
                        (NumericType::F32 | NumericType::F64, NumericType::Integer(_))
                    ),
                "invalid rounded conversion",
                o,
            )
        }
        Operation::Array(values) => {
            let T::Array { element, length } = dst else {
                return Err(Error::new("array result type", o));
            };
            require(*length == values.len(), "array literal length", o)?;
            for v in values {
                same(ty(*v)?, element)?;
            }
            Ok(())
        }
        Operation::Construct {
            ty: id,
            base,
            fields,
        } => {
            same(dst, &T::Named(*id))?;
            let def = p
                .types
                .get(id.0)
                .ok_or_else(|| Error::new("unknown product", o))?;
            if let Some(base) = base {
                same(ty(*base)?, dst)?;
            }
            let mut seen = HashSet::new();
            for (id, v) in fields {
                require(seen.insert(id.0), "duplicate constructor field", o)?;
                let field = def
                    .fields
                    .get(id.0)
                    .ok_or_else(|| Error::new("unknown field", o))?;
                same(ty(*v)?, &field.2)?;
            }
            require(
                base.is_some() || seen.len() == def.fields.len(),
                "incomplete constructor",
                o,
            )
        }
        Operation::Field {
            base,
            ty: id,
            field,
        } => {
            same(ty(*base)?, &T::Named(*id))?;
            let field = p
                .types
                .get(id.0)
                .and_then(|t| t.fields.get(field.0))
                .ok_or_else(|| Error::new("unknown field", o))?;
            same(dst, &field.2)
        }
        Operation::Index { base, index } => {
            same(ty(*index)?, &I64)?;
            same(dst, element(ty(*base)?, o)?)
        }
        Operation::Call {
            function,
            arguments,
            ownership,
        } => match call(p, f, *function, arguments, *ownership, o)? {
            ir::ReturnType::Value(t) => same(dst, t),
            _ => Err(Error::new("void call used as a value", o)),
        },
        Operation::ArrayLength(v) => {
            element(ty(*v)?, o)?;
            same(dst, &I64)
        }
        Operation::StringByteLength(v) => {
            same(ty(*v)?, &T::String)?;
            same(dst, &I64)
        }
        Operation::ArgumentCount => same(dst, &I64),
        Operation::ArgumentGet(value) | Operation::ReadLine(value) => {
            same(ty(*value)?, &I64)?;
            same(dst, &T::String)
        }
        Operation::StringConcat { left, right } => {
            same(ty(*left)?, &T::String)?;
            same(ty(*right)?, &T::String)?;
            same(dst, &T::String)
        }
        Operation::ArrayAllocate { length, .. } => {
            same(ty(*length)?, &I64)?;
            dynamic(dst, o)?;
            Ok(())
        }
        Operation::ArrayReleaseOwner(v) => {
            dynamic(ty(*v)?, o)?;
            same(dst, &T::Bool)
        }
    }
}
fn path_type<'a>(
    f: &'a Function,
    root: LocalId,
    path: &[LocalId],
    o: Origin,
) -> Result<&'a T, Error> {
    let mut ty = local(f, root, o)?;
    for &index in path {
        require(local(f, index, o)? == &I64, "index type mismatch", o)?;
        ty = element(ty, o)?;
    }
    Ok(ty)
}
fn instruction(p: &Program, f: &Function, i: &Instruction) -> Result<(), Error> {
    let o = i.origin;
    // 補助的なcopy以外の操作では停止位置を一意に保ちます。
    let may_fail = !matches!(
        &i.kind,
        InstructionKind::Assign {
            value: Operation::Literal(_) | Operation::Copy(_),
            ..
        }
    );
    let entry_call = matches!((&i.kind, o),
        (InstructionKind::Call { function, arguments, .. },
         Origin::Synthetic { reason: "entry-main-call" })
         if f.id.is_none() && arguments.is_empty()
             && p.functions.get(function.0).is_some_and(|f|
                 f.name == "main" && f.parameters.is_empty() && f.return_type == ir::ReturnType::Void));
    if may_fail && !entry_call {
        require(
            matches!(o, Origin::Source(_)),
            "operation needs primary HIR origin",
            o,
        )?;
    }
    match &i.kind {
        InstructionKind::Assign { destination, value } => {
            operation(p, f, value, local(f, *destination, o)?, o)
        }
        InstructionKind::Call {
            function,
            arguments,
            ownership,
        } => {
            call(p, f, *function, arguments, *ownership, o)?;
            Ok(())
        }
        InstructionKind::CheckIndex { root, path } => {
            require(!path.is_empty(), "empty index check", o)?;
            path_type(f, *root, path, o)?;
            Ok(())
        }
        InstructionKind::Store { root, path, value } => {
            require(
                path_type(f, *root, path, o)? == local(f, *value, o)?,
                "store type mismatch",
                o,
            )?;
            require(
                matches!(
                    f.locals[root.0].kind,
                    LocalKind::Binding { mutable: true, .. }
                ),
                "store to immutable/non-binding local",
                o,
            )
        }
        InstructionKind::Flush => Ok(()),
        InstructionKind::Output { value, quoted, .. } => {
            let t = local(f, *value, o)?;
            require(
                matches!(t, T::Bool | T::String | T::Integer(_) | T::F32 | T::F64),
                "unexpanded aggregate output",
                o,
            )?;
            require(!quoted || *t == T::String, "quoted non-string output", o)
        }
        InstructionKind::ArrayInitialize { array, value } => require(
            dynamic(local(f, *array, o)?, o)? == local(f, *value, o)?,
            "array initialize type mismatch",
            o,
        ),
        InstructionKind::ArrayRetain { value } | InstructionKind::ArrayFree { value } => {
            dynamic(local(f, *value, o)?, o)?;
            Ok(())
        }
        InstructionKind::ArrayRangeCheck { length, start, end } => {
            for &v in [length, start, end] {
                require(local(f, v, o)? == &I64, "range operand type", o)?;
            }
            Ok(())
        }
        InstructionKind::StringManage { value, .. } => {
            require(local(f, *value, o)? == &T::String, "string manage type", o)
        }
    }
}
pub(super) fn operation_reads(op: &Operation) -> Vec<LocalId> {
    match op {
        Operation::ArgumentCount | Operation::Literal(_) => vec![],
        Operation::Copy(v)
        | Operation::ArrayLength(v)
        | Operation::ArgumentGet(v)
        | Operation::ReadLine(v)
        | Operation::StringByteLength(v)
        | Operation::ArrayReleaseOwner(v) => vec![*v],
        Operation::Unary { value, .. }
        | Operation::ConvertInteger { value, .. }
        | Operation::ConvertNumeric { value, .. } => vec![*value],
        Operation::Binary { left, right, .. } | Operation::StringConcat { left, right } => {
            vec![*left, *right]
        }
        Operation::Array(values)
        | Operation::Call {
            arguments: values, ..
        } => values.clone(),
        Operation::Construct { base, fields, .. } => base
            .iter()
            .copied()
            .chain(fields.iter().map(|f| f.1))
            .collect(),
        Operation::Field { base, .. } => vec![*base],
        Operation::Index { base, index } => vec![*base, *index],
        Operation::ArrayAllocate { length, .. } => vec![*length],
    }
}
pub(super) fn reads(i: &InstructionKind) -> Vec<LocalId> {
    match i {
        InstructionKind::Assign { value, .. } => operation_reads(value),
        InstructionKind::Call { arguments, .. } => arguments.clone(),
        InstructionKind::CheckIndex { root, path } => {
            std::iter::once(*root).chain(path.iter().copied()).collect()
        }
        InstructionKind::Store { root, path, value } => std::iter::once(*root)
            .chain(path.iter().copied())
            .chain([*value])
            .collect(),
        InstructionKind::Flush => vec![],
        InstructionKind::Output { value, .. }
        | InstructionKind::ArrayRetain { value }
        | InstructionKind::ArrayFree { value }
        | InstructionKind::StringManage { value, .. } => vec![*value],
        InstructionKind::ArrayInitialize { array, value } => vec![*array, *value],
        InstructionKind::ArrayRangeCheck { length, start, end } => vec![*length, *start, *end],
    }
}
fn terminal_reads(t: &TerminatorKind) -> Vec<LocalId> {
    match t {
        TerminatorKind::Branch { condition, .. } => vec![*condition],
        TerminatorKind::Return(v) => v.iter().copied().collect(),
        _ => vec![],
    }
}
fn function(
    p: &Program,
    f: &Function,
    block_parameters: Option<&[Vec<LocalId>]>,
) -> Result<(), Error> {
    if let Some(args) = block_parameters {
        require(
            args.len() == f.blocks.len(),
            "block parameter count",
            INTERNAL,
        )?;
    }
    require(f.entry.0 < f.blocks.len(), "unknown entry block", INTERNAL)?;
    let mut bindings = HashSet::new();
    for l in &f.locals {
        check_type(p, &l.ty)?;
        if let LocalKind::Binding { id, .. } = &l.kind {
            require(bindings.insert(*id), "duplicate binding", INTERNAL)?;
        }
    }
    if let ir::ReturnType::Value(t) = &f.return_type {
        check_type(p, t)?;
    }
    let mut params = HashSet::new();
    for &id in &f.parameters {
        local(f, id, INTERNAL)?;
        require(params.insert(id), "duplicate parameter", INTERNAL)?;
        require(
            matches!(f.locals[id.0].kind,LocalKind::Binding{mutable:false,borrowed,..}
            if borrowed==(f.argument_ownership==ir::ArgumentOwnership::Borrowed))
                || (block_parameters.is_some()
                    && matches!(f.locals[id.0].kind, LocalKind::Temporary)
                    && super::ssa::is_scalar(&f.locals[id.0].ty)),
            "parameter ownership/kind",
            INTERNAL,
        )?;
    }
    let mut ids = HashSet::new();
    let mut binding_initializers = HashSet::new();
    for b in &f.blocks {
        for i in &b.instructions {
            require(ids.insert(i.id), "duplicate instruction id", i.origin)?;
            instruction(p, f, i)?;
            if let InstructionKind::Assign { destination, .. } = &i.kind
                && matches!(f.locals[destination.0].kind, LocalKind::Binding { .. })
            {
                require(
                    !params.contains(destination) && binding_initializers.insert(*destination),
                    "binding must be initialized once; use store for reassignment",
                    i.origin,
                )?;
            }
        }
        let t = &b.terminator;
        require(ids.insert(t.id), "duplicate instruction id", t.origin)?;
        for next in t.kind.successors() {
            require(next.0 < f.blocks.len(), "unknown successor block", t.origin)?;
        }
        if matches!(
            t.kind,
            TerminatorKind::Branch { .. } | TerminatorKind::Return(Some(_))
        ) {
            require(
                matches!(t.origin, Origin::Source(_)),
                "terminator needs primary HIR origin",
                t.origin,
            )?;
        }
        match &t.kind {
            TerminatorKind::Branch { condition, .. } => require(
                local(f, *condition, t.origin)? == &T::Bool,
                "branch condition type",
                t.origin,
            )?,
            TerminatorKind::Return(v) => match (&f.return_type, v) {
                (ir::ReturnType::Void, None) => {}
                (ir::ReturnType::Value(ty), Some(v)) => require(
                    local(f, *v, t.origin)? == ty,
                    "return type mismatch",
                    t.origin,
                )?,
                _ => return Err(Error::new("return signature mismatch", t.origin)),
            },
            _ => {}
        }
    }
    initialized(f, params, block_parameters)
}
#[derive(Clone, PartialEq, Eq)]
struct Facts {
    initialized: HashSet<LocalId>,
    checked: HashSet<Vec<LocalId>>,
}
fn check_key(root: LocalId, path: &[LocalId]) -> Vec<LocalId> {
    std::iter::once(root).chain(path.iter().copied()).collect()
}
fn transfer(facts: &mut Facts, i: &InstructionKind) {
    match i {
        InstructionKind::Assign { destination, .. } => {
            facts.initialized.insert(*destination);
            facts.checked.retain(|key| !key.contains(destination));
        }
        InstructionKind::CheckIndex { root, path } => {
            facts.checked.insert(check_key(*root, path));
        }
        InstructionKind::Store { root, .. } => {
            facts.checked.retain(|key| !key.contains(root));
        }
        _ => {}
    }
}
/// 到達可能な全前任ブロックの積集合を、不動点まで計算します。
fn initialized(
    f: &Function,
    params: HashSet<LocalId>,
    block_parameters: Option<&[Vec<LocalId>]>,
) -> Result<(), Error> {
    let mut reachable = vec![false; f.blocks.len()];
    let mut stack = vec![f.entry];
    let mut predecessors = vec![vec![]; f.blocks.len()];
    while let Some(id) = stack.pop() {
        if reachable[id.0] {
            continue;
        }
        reachable[id.0] = true;
        for next in f.blocks[id.0].terminator.kind.successors() {
            predecessors[next.0].push(id.0);
            stack.push(next);
        }
    }
    let mut all_checks = HashSet::new();
    for b in &f.blocks {
        for i in &b.instructions {
            if let InstructionKind::CheckIndex { root, path } = &i.kind {
                all_checks.insert(check_key(*root, path));
            }
        }
    }
    let top = Facts {
        initialized: (0..f.locals.len()).map(LocalId).collect(),
        checked: all_checks,
    };
    let entry = Facts {
        initialized: params,
        checked: HashSet::new(),
    };
    let mut incoming = vec![top.clone(); f.blocks.len()];
    let mut outgoing = incoming.clone();
    loop {
        let mut changed = false;
        for (id, b) in f.blocks.iter().enumerate() {
            if !reachable[id] {
                continue;
            }
            let mut facts = if id == f.entry.0 {
                entry.clone()
            } else {
                top.clone()
            };
            for &pred in &predecessors[id] {
                facts
                    .initialized
                    .retain(|v| outgoing[pred].initialized.contains(v));
                facts.checked.retain(|v| outgoing[pred].checked.contains(v));
            }
            // 引数は入口で新しい値になります。前周の添字検査を流用しません。
            if let Some(args) = block_parameters {
                for arg in &args[id] {
                    facts.initialized.insert(*arg);
                    facts.checked.retain(|key| !key.contains(arg));
                }
            }
            incoming[id] = facts.clone();
            for i in &b.instructions {
                transfer(&mut facts, &i.kind);
            }
            if outgoing[id] != facts {
                outgoing[id] = facts;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (id, b) in f.blocks.iter().enumerate() {
        if !reachable[id] {
            continue;
        }
        let mut facts = incoming[id].clone();
        for i in &b.instructions {
            for v in reads(&i.kind) {
                require(
                    facts.initialized.contains(&v),
                    "read before definite initialization",
                    i.origin,
                )?;
            }
            if let InstructionKind::Store { root, path, .. } = &i.kind {
                for n in 1..=path.len() {
                    require(
                        facts.checked.contains(&check_key(*root, &path[..n])),
                        "store without preceding index check",
                        i.origin,
                    )?;
                }
            }
            transfer(&mut facts, &i.kind);
        }
        for v in terminal_reads(&b.terminator.kind) {
            require(
                facts.initialized.contains(&v),
                "terminator reads uninitialized local",
                b.terminator.origin,
            )?;
        }
        require(
            !matches!(b.terminator.kind, TerminatorKind::Unreachable),
            "reachable non-void function end",
            b.terminator.origin,
        )?;
    }
    Ok(())
}
