use super::*;
use crate::{ir, source::Span, types::IntegerType};
// Public LIR snapshots can be modified. Check field types before using typed loads.
pub(in crate::codegen::x86_64) fn validate(
    m: &Module,
    request: usize,
    outcome: usize,
    span: Span,
) -> Result<(), &'static str> {
    let byte = ir::Type::Integer(IntegerType::U8);
    let u64 = ir::Type::Integer(IntegerType::U64);
    let array = |element| ir::Type::DynamicArray {
        element: Box::new(element),
    };
    let r = &m.types[request];
    let expected = [
        ("target", ir::Type::String),
        ("arguments", array(ir::Type::String)),
        ("stdin", array(byte.clone())),
        ("stdout_limit", u64.clone()),
        ("stderr_limit", u64.clone()),
        ("timeout_ms", u64),
    ];
    if r.variants.is_some()
        || r.fields.len() != expected.len()
        || expected
            .iter()
            .any(|(name, ty)| !r.fields.iter().any(|(_, n, t)| n == name && t == ty))
    {
        return Err("invalid process request fields");
    }
    let d = &m.types[outcome];
    if d.variants.is_some()
        || d.fields.len() != 3
        || ["stdout", "stderr"].iter().any(|name| {
            !d.fields
                .iter()
                .any(|(_, n, t)| n == name && *t == array(byte.clone()))
        })
    {
        return Err("invalid process outcome fields");
    }
    let Some((_, _, ir::Type::Named(report))) = d.fields.iter().find(|(_, n, _)| n == "report")
    else {
        return Err("invalid process report field");
    };
    // Use the runtime's existing fixed Report declaration checks. These temporary spans
    // are the operation's origin, and are never emitted as declaration-source metadata.
    let definitions = m
        .types
        .iter()
        .map(|d| ir::TypeDefinition {
            id: d.id,
            name: d.name.clone(),
            variants: d.variants.clone(),
            span,
            fields: d
                .fields
                .iter()
                .map(|(id, name, ty)| ir::FieldDefinition {
                    id: *id,
                    name: name.clone(),
                    ty: ty.clone(),
                    default: None,
                    span,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    crate::runtime::process::report_value::validate_layout(&definitions, *report)
        .map_err(|_| "invalid process report fields")
}
