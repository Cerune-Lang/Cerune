//! 完成済みHIRの choose をKの入力項に写します。
use cerune_lang::{
    ir::{
        self, BinaryOp, Expr, ExprKind, LogicalOp, NodeId, ReturnType, Statement, StatementKind,
        Type,
    },
    source::Span,
    types::IntegerType,
};

fn site(id: NodeId, span: Span) -> String {
    format!(
        "site({},{},{},{})",
        id.0,
        span.source_id().index(),
        span.start(),
        span.end()
    )
}

fn expression(expr: &Expr, param: ir::BindingId) -> Result<String, String> {
    let u8_type = Type::Integer(IntegerType::U8);
    match (&expr.kind, &expr.ty) {
        (ExprKind::Integer(n), ty) if *ty == u8_type && (0..=255).contains(n) => {
            Ok(format!("num({n})"))
        }
        (ExprKind::Variable { id, .. }, ty) if *ty == u8_type && *id == param => {
            Ok("argument".into())
        }
        (
            ExprKind::Binary {
                op: BinaryOp::Add,
                left,
                right,
            },
            ty,
        ) if *ty == u8_type => Ok(format!(
            "add({}, {}, {})",
            site(expr.id, expr.span),
            expression(left, param)?,
            expression(right, param)?
        )),
        (
            ExprKind::Binary {
                op: BinaryOp::Less,
                left,
                right,
            },
            Type::Bool,
        ) => Ok(format!(
            "lt({}, {})",
            expression(left, param)?,
            expression(right, param)?
        )),
        (
            ExprKind::Logical {
                op: LogicalOp::And,
                left,
                right,
            },
            Type::Bool,
        ) => Ok(format!(
            "andThen({}, {})",
            expression(left, param)?,
            expression(right, param)?
        )),
        _ => Err(format!(
            "unsupported choose expression at node {}",
            expr.id.0
        )),
    }
}

fn body(statements: &[Statement], param: ir::BindingId) -> Result<String, String> {
    let [stmt] = statements else {
        return Err("expected one if or return".into());
    };
    match &stmt.kind {
        StatementKind::If {
            condition,
            then_body,
            else_body,
        } => Ok(format!(
            "when({}, {}, {}, {})",
            site(condition.id, condition.span),
            expression(condition, param)?,
            body(then_body, param)?,
            body(else_body, param)?
        )),
        StatementKind::Return { value: Some(value) } => Ok(format!(
            "ret({}, {})",
            site(stmt.id, stmt.span),
            expression(value, param)?
        )),
        _ => Err(format!(
            "unsupported choose statement at node {}",
            stmt.id.0
        )),
    }
}

fn main() -> Result<(), String> {
    let hir = cerune_lang::compile_to_ir(include_str!("../lean/branch.ceru"))
        .map_err(|e| e.message().to_string())?;
    let f = hir
        .function_definitions
        .iter()
        .find(|f| f.name == "choose")
        .ok_or("missing choose")?;
    let [param] = f.parameters.as_slice() else {
        return Err("expected one parameter".into());
    };
    let u8_type = Type::Integer(IntegerType::U8);
    if f.lowering.is_some() || param.ty != u8_type || f.return_type != ReturnType::Value(u8_type) {
        return Err("expected a source function choose(u8) -> u8".into());
    }
    println!("{}", body(&f.body, param.id)?);
    Ok(())
}
