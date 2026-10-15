//! v0.1は観測用テキストです。load形式や安定した配布Imageではありません。
use super::*;
use std::fmt::Write;

pub fn emit(p: &Program) -> String {
    let mut out = String::from(
        "; Cerune MIR v0.1\n; non-SSA; no optimization; checked operations stop on failure\n; copy/load stores a value representation, never an implicit ownership copy\n",
    );
    writeln!(
        out,
        "budget string={} array={}\n",
        p.string_heap_limit, p.array_heap_limit
    )
    .unwrap();
    writeln!(out, "source-strings={}\n", p.uses_strings).unwrap();
    for t in &p.types {
        writeln!(out, "type @{} {:?} {{", t.id.0, t.name).unwrap();
        for (id, name, ty) in &t.fields {
            writeln!(out, "  field @{} {:?}: {}", id.0, name, type_name(ty)).unwrap();
        }
        if let Some(variants) = &t.variants {
            writeln!(out, "  variants {variants:?}").unwrap();
        }
        writeln!(out, "}}\n").unwrap();
    }
    for f in p.functions.iter().chain([&p.main]) {
        function(&mut out, f);
    }
    out
}
pub fn type_name(ty: &ir::Type) -> String {
    match ty {
        ir::Type::Bool => "bool".into(),
        ir::Type::String => "string".into(),
        ir::Type::Integer(t) => t.name().into(),
        ir::Type::F32 => "f32".into(),
        ir::Type::F64 => "f64".into(),
        ir::Type::Named(id) => format!("@{}", id.0),
        ir::Type::Array { element, length } => format!("[{}; {length}]", type_name(element)),
        ir::Type::DynamicArray { element } => format!("[{}]", type_name(element)),
    }
}
fn value<R: std::fmt::Display>(v: R) -> String {
    v.to_string()
}
fn values<R: Copy + std::fmt::Display>(v: &[R]) -> String {
    v.iter().map(|v| value(*v)).collect::<Vec<_>>().join(", ")
}
pub(super) fn origin(o: Origin) -> String {
    let source = |s: SourceOrigin| {
        format!(
            "hir=#{} source={} bytes={}..{}",
            s.node_id.0,
            s.span.source_id().index(),
            s.span.start(),
            s.span.end()
        )
    };
    match o {
        Origin::Source(s) => source(s),
        Origin::Derived { source: s, reason } => format!("{} derived={reason}", source(s)),
        Origin::Synthetic { reason } => format!("synthetic={reason}"),
    }
}
fn function(out: &mut String, f: &Function) {
    let id = f.id.map_or("main".into(), |id| format!("@{}", id.0));
    let ret = match &f.return_type {
        ir::ReturnType::Void => "void".into(),
        ir::ReturnType::Value(t) => type_name(t),
    };
    writeln!(
        out,
        "fn {id} {:?} ({}) -> {ret} ownership={:?} lowering={:?} entry=bb{} {{",
        f.name,
        values(&f.parameters),
        f.argument_ownership,
        f.lowering,
        f.entry.0
    )
    .unwrap();
    for (id, l) in f.locals.iter().enumerate() {
        let kind = match &l.kind {
            LocalKind::Temporary => "temporary".into(),
            LocalKind::Binding {
                id,
                name,
                mutable,
                borrowed,
            } => format!(
                "binding=@{} name={name:?} mutable={mutable} borrowed={borrowed}",
                id.0
            ),
        };
        writeln!(out, "  local %{id}: {} ; {kind}", type_name(&l.ty)).unwrap();
    }
    for (id, b) in f.blocks.iter().enumerate() {
        writeln!(out, "  bb{id}: ; {}", origin(b.origin)).unwrap();
        for i in &b.instructions {
            writeln!(
                out,
                "    i{} {} ; {}",
                i.id.0,
                instruction(&i.kind),
                origin(i.origin)
            )
            .unwrap();
        }
        let t = &b.terminator;
        let text = match &t.kind {
            TerminatorKind::Jump(b) => format!("jump bb{}", b.0),
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => format!(
                "branch {} bb{} bb{}",
                value(*condition),
                then_block.0,
                else_block.0
            ),
            TerminatorKind::Return(v) => {
                v.map_or("return".into(), |v| format!("return {}", value(v)))
            }
            TerminatorKind::Unreachable => "unreachable".into(),
        };
        writeln!(out, "    i{} {text} ; {}", t.id.0, origin(t.origin)).unwrap();
    }
    writeln!(out, "}}\n").unwrap();
}
fn call<R: Copy + std::fmt::Display>(
    f: ir::FunctionId,
    args: &[R],
    own: ir::ArgumentOwnership,
) -> String {
    format!("call @{} ({}) ownership={own:?}", f.0, values(args))
}
fn operation<R: Copy + std::fmt::Display>(op: &Operation<R>) -> String {
    match op {
        Operation::Literal(l) => match l {
            Literal::Boolean(v) => v.to_string(),
            Literal::String(v) => format!("{v:?}"),
            Literal::Integer(v) => v.to_string(),
            Literal::Float(v) => v.clone(),
        },
        Operation::Copy(v) => format!("copy {}", value(*v)),
        Operation::Unary { op, value: v } => format!("unary.{op:?} {}", value(*v)),
        Operation::Binary { op, left, right } => {
            format!("binary.{op:?} {}, {}", value(*left), value(*right))
        }
        Operation::ConvertInteger {
            value: v,
            from,
            to,
            syntax,
        } => format!(
            "convert.exact {} {} -> {} syntax={syntax:?}",
            value(*v),
            from.name(),
            to.name()
        ),
        Operation::ConvertNumeric {
            value: v,
            from,
            to,
            mode,
            syntax,
        } => format!(
            "convert.{} {} {from:?} -> {to:?} syntax={syntax:?}",
            mode.name(),
            value(*v)
        ),
        Operation::Array(v) => format!("array [{}]", values(v)),
        Operation::Construct { ty, base, fields } => format!(
            "construct @{} base={} fields=[{}]",
            ty.0,
            base.map_or("none".into(), value),
            fields
                .iter()
                .map(|(id, v)| format!("@{}={}", id.0, value(*v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Operation::Field { base, ty, field } => {
            format!("field {} type=@{} field=@{}", value(*base), ty.0, field.0)
        }
        Operation::Index { base, index } => {
            format!("index.checked {} {}", value(*base), value(*index))
        }
        Operation::Call {
            function,
            arguments,
            ownership,
        } => call(*function, arguments, *ownership),
        Operation::ArrayLength(v) => format!("array.length {}", value(*v)),
        Operation::ReadLine(v) => format!("input.read_line.utf8 {}", value(*v)),
        Operation::StringByteLength(v) => format!("string.byte_length {}", value(*v)),
        Operation::StringConcat { left, right } => {
            format!("string.concat {}, {}", value(*left), value(*right))
        }
        Operation::ArrayAllocate {
            length,
            element_width,
        } => format!(
            "array.allocate {} element_width={element_width}",
            value(*length)
        ),
        Operation::ArrayReleaseOwner(v) => format!("array.release_owner {}", value(*v)),
    }
}
pub(super) fn instruction<R: Copy + std::fmt::Display>(i: &InstructionKind<R>) -> String {
    match i {
        InstructionKind::Assign {
            destination,
            value: v,
        } => format!("{} = {}", value(*destination), operation(v)),
        InstructionKind::Call {
            function,
            arguments,
            ownership,
        } => call(*function, arguments, *ownership),
        InstructionKind::CheckIndex { root, path } => {
            format!("index.check {} [{}]", value(*root), values(path))
        }
        InstructionKind::Store {
            root,
            path,
            value: v,
        } => format!("store {} [{}] = {}", value(*root), values(path), value(*v)),
        InstructionKind::Flush => "flush.stdout".into(),
        InstructionKind::Output {
            value: v,
            newline,
            quoted,
        } => format!("output {} newline={newline} quoted={quoted}", value(*v)),
        InstructionKind::ArrayInitialize { array, value: v } => {
            format!("array.initialize {} {}", value(*array), value(*v))
        }
        InstructionKind::ArrayRetain { value: v } => format!("array.retain {}", value(*v)),
        InstructionKind::ArrayFree { value: v } => format!("array.free {}", value(*v)),
        InstructionKind::ArrayRangeCheck { length, start, end } => format!(
            "array.range_check {} {} {}",
            value(*length),
            value(*start),
            value(*end)
        ),
        InstructionKind::StringManage { value: v, retain } => format!(
            "string.{} {}",
            if *retain { "retain" } else { "release" },
            value(*v)
        ),
    }
}
