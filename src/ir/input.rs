use super::*;

pub(super) fn first_span(program: &Program) -> Option<Span> {
    let mut span = None;
    visit_program(
        program,
        &mut |e| {
            if span.is_none() && matches!(e.kind, ExprKind::ReadLine { .. }) {
                span = Some(e.span);
            }
        },
        &mut |_| {},
    );
    span
}
pub(super) fn visit_program(
    p: &Program,
    expr: &mut impl FnMut(&Expr),
    stmt: &mut impl FnMut(&Statement),
) {
    for c in &p.constant_definitions {
        visit_expr(&c.initializer, expr);
        visit_expr(&c.value, expr);
    }
    for t in &p.type_definitions {
        for f in &t.fields {
            if let Some(v) = &f.default {
                visit_expr(v, expr);
            }
        }
    }
    for f in &p.function_definitions {
        visit_body(&f.body, expr, stmt);
    }
    visit_body(&p.statements, expr, stmt);
}
pub(super) fn visit_body(
    body: &[Statement],
    f: &mut impl FnMut(&Expr),
    s: &mut impl FnMut(&Statement),
) {
    for stmt in body {
        match &stmt.kind {
            StatementKind::ArrayInitialize { array, value } => {
                visit_expr(array, f);
                visit_expr(value, f);
            }
            StatementKind::FileWriteNew {
                name: length,
                text: start,
                max_bytes: end,
            }
            | StatementKind::ArrayRangeCheck { length, start, end } => {
                visit_expr(length, f);
                visit_expr(start, f);
                visit_expr(end, f);
            }
            StatementKind::ArrayRetain { value }
            | StatementKind::ArrayFree { value }
            | StatementKind::StringManage { value, .. }
            | StatementKind::Binding { value, .. }
            | StatementKind::Print { value }
            | StatementKind::Write { value, .. } => visit_expr(value, f),
            StatementKind::Assignment { target, value } => {
                for AssignmentProjection::Index { index, .. }
                | AssignmentProjection::DynamicIndex { index, .. } in &target.projections
                {
                    visit_expr(index, f);
                }
                visit_expr(value, f);
            }
            StatementKind::Call { arguments, .. } => {
                for e in arguments {
                    visit_expr(e, f);
                }
            }
            StatementKind::Return { value } => {
                if let Some(v) = value {
                    visit_expr(v, f);
                }
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                visit_expr(condition, f);
                visit_body(then_body, f, s);
                visit_body(else_body, f, s);
            }
            StatementKind::While { condition, body } => {
                visit_expr(condition, f);
                visit_body(body, f, s);
            }
            StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                visit_body(std::slice::from_ref(initializer), f, s);
                visit_expr(condition, f);
                visit_body(std::slice::from_ref(update), f, s);
                visit_body(body, f, s);
            }
            StatementKind::Flush | StatementKind::Break | StatementKind::Continue => {}
        }
        s(stmt);
    }
}
pub(super) fn visit_expr(expr: &Expr, f: &mut impl FnMut(&Expr)) {
    match &expr.kind {
        ExprKind::Let { value, body, .. } => {
            visit_expr(value, f);
            visit_expr(body, f);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            visit_expr(condition, f);
            visit_expr(then_value, f);
            visit_expr(else_value, f);
        }
        ExprKind::ArrayCopy { value, range } => {
            visit_expr(value, f);
            if let Some((start, end)) = range {
                visit_expr(start, f);
                visit_expr(end, f);
            }
        }
        ExprKind::ArrayAllocate { length: value, .. }
        | ExprKind::ArrayReleaseOwner { value }
        | ExprKind::Constant { value, .. }
        | ExprKind::ArrayLength { value }
        | ExprKind::ArgumentGet { index: value }
        | ExprKind::ReadLine { max_bytes: value }
        | ExprKind::StringByteLength { value }
        | ExprKind::ConvertNumeric { value, .. }
        | ExprKind::ConvertInteger { value, .. }
        | ExprKind::Unary { value, .. }
        | ExprKind::FieldAccess { base: value, .. } => visit_expr(value, f),
        ExprKind::ArrayRepeat {
            value: left,
            count: right,
        }
        | ExprKind::StringByteAt {
            value: left,
            index: right,
        }
        | ExprKind::FileReadText {
            name: left,
            max_bytes: right,
        }
        | ExprKind::StringConcat { left, right }
        | ExprKind::Logical { left, right, .. }
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Index {
            base: left,
            index: right,
        } => {
            visit_expr(left, f);
            visit_expr(right, f);
        }
        ExprKind::Construct { base, fields, .. } => {
            if let Some(v) = base {
                visit_expr(v, f);
            }
            for field in fields {
                visit_expr(&field.value, f);
            }
        }
        ExprKind::Array(values)
        | ExprKind::Call {
            arguments: values, ..
        } => {
            for v in values {
                visit_expr(v, f);
            }
        }
        _ => {}
    }
    f(expr);
}
