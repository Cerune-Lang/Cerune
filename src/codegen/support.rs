pub(super) use crate::ir::string_usage::first_string_span;

pub(super) fn string_heap_limit(program: &crate::ir::Program) -> Option<u64> {
    program
        .function_definitions
        .iter()
        .any(|f| {
            matches!(
                f.lowering,
                Some(crate::ir::LoweringKind::OwnershipExpression)
            )
        })
        .then_some(program.string_heap_limit)
}

pub(super) fn first_flush_span(program: &crate::ir::Program) -> Option<crate::source::Span> {
    fn body(ss: &[crate::ir::Statement]) -> Option<crate::source::Span> {
        use crate::ir::StatementKind as S;
        ss.iter().find_map(|s| match &s.kind {
            S::Flush => Some(s.span),
            S::If {
                then_body,
                else_body,
                ..
            } => body(then_body).or_else(|| body(else_body)),
            S::While { body: b, .. } | S::For { body: b, .. } => body(b),
            _ => None,
        })
    }
    body(&program.statements).or_else(|| {
        program
            .function_definitions
            .iter()
            .find_map(|f| body(&f.body))
    })
}
