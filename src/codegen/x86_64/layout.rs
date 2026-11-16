//! Native aggregate fields occupy eight-byte slots in descending stack order.
use crate::{diagnostic::Diagnostic, ir, mir};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Layout {
    pub slots: usize,
    pub fields: Vec<usize>,
}
fn error(message: &str) -> Diagnostic {
    Diagnostic::without_span(format!("LIR types: {message}"))
}
fn references(ty: &ir::Type, count: usize) -> Result<(), Diagnostic> {
    match ty {
        ir::Type::Named(id) if id.0 >= count => Err(error("missing named type")),
        ir::Type::Array { element, .. } | ir::Type::DynamicArray { element } => {
            references(element, count)
        }
        _ => Ok(()),
    }
}
fn slots(ty: &ir::Type, layouts: &[Option<Layout>]) -> Result<Option<usize>, Diagnostic> {
    match ty {
        ir::Type::Named(id) => Ok(layouts[id.0].as_ref().map(|l| l.slots)),
        ir::Type::Array { element, length } => slots(element, layouts)?
            .map(|n| {
                n.checked_mul(*length)
                    .ok_or_else(|| error("slot count overflow"))
            })
            .transpose(),
        _ => Ok(Some(1)),
    }
}
pub(super) fn resolve(types: &[mir::TypeDefinition]) -> Result<Vec<Layout>, Diagnostic> {
    for (index, definition) in types.iter().enumerate() {
        if definition.id.0 != index {
            return Err(error("type IDs must match indices"));
        }
        let mut names = std::collections::BTreeSet::new();
        for (index, (id, name, ty)) in definition.fields.iter().enumerate() {
            if id.0 != index {
                return Err(error("field IDs must match indices"));
            }
            if !names.insert(name) {
                return Err(error("duplicate field name"));
            }
            references(ty, types.len())?;
        }
        if let Some(variants) = &definition.variants {
            let mut names = std::collections::BTreeSet::new();
            if variants.is_empty() || variants.iter().any(|v| !names.insert(v)) {
                return Err(error("empty or duplicate enum variants"));
            }
        }
    }
    let mut layouts: Vec<Option<Layout>> = vec![None; types.len()];
    let mut remaining = types.len();
    while remaining > 0 {
        let before = remaining;
        for (id, definition) in types.iter().enumerate() {
            if layouts[id].is_some() {
                continue;
            }
            let mut layout = Layout {
                slots: 0,
                fields: vec![],
            };
            let mut pending = false;
            for (_, _, ty) in &definition.fields {
                let Some(size) = slots(ty, &layouts)? else {
                    pending = true;
                    break;
                };
                layout.fields.push(layout.slots);
                layout.slots = layout
                    .slots
                    .checked_add(size)
                    .ok_or_else(|| error("slot count overflow"))?;
            }
            if !pending {
                layouts[id] = Some(layout);
                remaining -= 1;
            }
        }
        if remaining == before {
            return Err(error("recursive inline type layout"));
        }
    }
    Ok(layouts.into_iter().map(Option::unwrap).collect())
}
