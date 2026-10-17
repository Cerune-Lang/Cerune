use crate::{diagnostic::Diagnostic, ir::*, source::Span};

fn scalar(ty: &Type, span: Span) -> Result<(), Diagnostic> {
    if matches!(ty, Type::Bool | Type::Integer(_) | Type::F32 | Type::F64) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            "WAT libraries currently require scalar values",
            span,
        ))
    }
}

pub(super) fn program(p: &Program) -> Result<(), Diagnostic> {
    for c in &p.constant_definitions {
        scalar(&c.ty, c.span)?;
        expr(&c.initializer)?;
        expr(&c.value)?;
    }
    for f in &p.function_definitions {
        for p in &f.parameters {
            scalar(&p.ty, p.span)?;
        }
        if let ReturnType::Value(ty) = &f.return_type {
            scalar(ty, f.span)?;
        }
        statements(&f.body)?;
    }
    statements(&p.statements)
}
fn statements(body: &[Statement]) -> Result<(), Diagnostic> {
    for s in body {
        statement(s)?;
    }
    Ok(())
}
fn statement(s: &Statement) -> Result<(), Diagnostic> {
    match &s.kind {
        StatementKind::Binding { ty, value, .. } => {
            scalar(ty, s.span)?;
            expr(value)?;
        }
        StatementKind::Assignment { target, value } => {
            scalar(&target.root_ty, s.span)?;
            scalar(&target.ty, s.span)?;
            if !target.projections.is_empty() {
                return Err(Diagnostic::new(
                    "aggregate assignment is not supported in WAT libraries",
                    s.span,
                ));
            }
            expr(value)?;
        }
        StatementKind::Call { arguments, .. } => {
            for a in arguments {
                expr(a)?;
            }
        }
        StatementKind::Return { value } => {
            if let Some(e) = value {
                expr(e)?;
            }
        }
        StatementKind::If {
            condition,
            then_body,
            else_body,
        } => {
            expr(condition)?;
            statements(then_body)?;
            statements(else_body)?;
        }
        StatementKind::While { condition, body } => {
            expr(condition)?;
            statements(body)?;
        }
        StatementKind::For {
            initializer,
            condition,
            update,
            body,
        } => {
            statement(initializer)?;
            expr(condition)?;
            statement(update)?;
            statements(body)?;
        }
        StatementKind::Break | StatementKind::Continue => {}
        _ => {
            return Err(Diagnostic::new(
                "I/O and ownership operations are not supported in WAT libraries",
                s.span,
            ));
        }
    }
    Ok(())
}
fn expr(e: &Expr) -> Result<(), Diagnostic> {
    scalar(&e.ty, e.span)?;
    match &e.kind {
        ExprKind::Boolean(_)
        | ExprKind::Integer(_)
        | ExprKind::Float { .. }
        | ExprKind::Variable { .. } => {}
        ExprKind::Let { value, body, .. } => {
            expr(value)?;
            expr(body)?;
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            expr(condition)?;
            expr(then_value)?;
            expr(else_value)?;
        }
        ExprKind::Constant { value, .. }
        | ExprKind::ConvertNumeric { value, .. }
        | ExprKind::ConvertInteger { value, .. }
        | ExprKind::Unary { value, .. } => expr(value)?,
        ExprKind::Logical { left, right, .. } | ExprKind::Binary { left, right, .. } => {
            expr(left)?;
            expr(right)?;
        }
        ExprKind::Call { arguments, .. } => {
            for a in arguments {
                expr(a)?;
            }
        }
        _ => {
            return Err(Diagnostic::new(
                "I/O, allocation and aggregate expressions are not supported in WAT libraries",
                e.span,
            ));
        }
    }
    Ok(())
}
