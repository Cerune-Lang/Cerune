use crate::{
    ir::{
        AssignmentProjection, Expr, ExprKind, Program, ReturnType, Statement, StatementKind, Type,
    },
    source::Span,
};

/// 未使用の型や関数も含めて、文字列を必要とする最初の位置を返します。
pub(crate) fn first_string_span(program: &Program) -> Option<Span> {
    // 定数化で文字列式が消えても、ターゲットとバイト出力の契約を保ちます。
    for constant in &program.constant_definitions {
        if let Some(span) = string_expr(&constant.initializer) {
            return Some(span);
        }
    }
    for definition in &program.type_definitions {
        for field in &definition.fields {
            if contains_string(&field.ty) {
                return Some(field.span);
            }
            if let Some(span) = field.default.as_ref().and_then(string_expr) {
                return Some(span);
            }
        }
    }
    for function in &program.function_definitions {
        for parameter in &function.parameters {
            if contains_string(&parameter.ty) {
                return Some(parameter.span);
            }
        }
        if matches!(&function.return_type, ReturnType::Value(ty) if contains_string(ty)) {
            return Some(function.span);
        }
        if let Some(span) = string_statements(&function.body) {
            return Some(span);
        }
    }
    string_statements(&program.statements)
}

fn contains_string(ty: &Type) -> bool {
    match ty {
        Type::String => true,
        Type::Array { element, .. } | Type::DynamicArray { element } => contains_string(element),
        // 名前付き型のフィールドは、未使用の定義も含めて入口で検査します。
        Type::Named(_) | Type::Bool | Type::Integer(_) | Type::F32 | Type::F64 => false,
    }
}

fn string_statements(statements: &[Statement]) -> Option<Span> {
    statements.iter().find_map(string_statement)
}

fn string_statement(statement: &Statement) -> Option<Span> {
    match &statement.kind {
        StatementKind::ArrayInitialize { array, value } => {
            string_expr(array).or_else(|| string_expr(value))
        }
        StatementKind::FileWriteNew {
            name: length,
            text: start,
            max_bytes: end,
        }
        | StatementKind::ArrayRangeCheck { length, start, end } => string_expr(length)
            .or_else(|| string_expr(start))
            .or_else(|| string_expr(end)),
        StatementKind::ArrayRetain { value } | StatementKind::ArrayFree { value } => {
            string_expr(value)
        }
        StatementKind::Binding { ty, value, .. } => {
            string_expr(value).or_else(|| contains_string(ty).then_some(statement.span))
        }
        StatementKind::Assignment { target, value } => string_expr(value)
            .or_else(|| {
                target
                    .projections
                    .iter()
                    .find_map(|projection| match projection {
                        AssignmentProjection::Index {
                            index,
                            element,
                            span,
                            ..
                        }
                        | AssignmentProjection::DynamicIndex {
                            index,
                            element,
                            span,
                        } => {
                            string_expr(index).or_else(|| contains_string(element).then_some(*span))
                        }
                    })
            })
            .or_else(|| {
                (contains_string(&target.ty) || contains_string(&target.root_ty))
                    .then_some(statement.span)
            }),
        StatementKind::StringManage { value, .. }
        | StatementKind::Print { value }
        | StatementKind::Write { value, .. } => string_expr(value),
        StatementKind::Call { arguments, .. } => arguments.iter().find_map(string_expr),
        StatementKind::Return { value } => value.as_ref().and_then(string_expr),
        StatementKind::If {
            condition,
            then_body,
            else_body,
        } => string_expr(condition)
            .or_else(|| string_statements(then_body))
            .or_else(|| string_statements(else_body)),
        StatementKind::While { condition, body } => {
            string_expr(condition).or_else(|| string_statements(body))
        }
        StatementKind::For {
            initializer,
            condition,
            update,
            body,
        } => string_statement(initializer)
            .or_else(|| string_expr(condition))
            .or_else(|| string_statement(update))
            .or_else(|| string_statements(body)),
        StatementKind::Flush | StatementKind::Break | StatementKind::Continue => None,
    }
}

fn string_expr(expr: &Expr) -> Option<Span> {
    if contains_string(&expr.ty) || matches!(expr.kind, ExprKind::String(_)) {
        return Some(expr.span);
    }
    match &expr.kind {
        ExprKind::ArrayCopy { value, range } => string_expr(value).or_else(|| {
            range
                .as_ref()
                .and_then(|(a, b)| string_expr(a).or_else(|| string_expr(b)))
        }),
        ExprKind::ArrayAllocate { length: value, .. } | ExprKind::ArrayReleaseOwner { value } => {
            string_expr(value)
        }
        ExprKind::Let { .. } | ExprKind::Conditional { .. } => {
            unreachable!("match expressions are lowered before code generation")
        }
        ExprKind::Constant { value, .. } => string_expr(value),
        ExprKind::ArrayLength { value }
        | ExprKind::ArgumentGet { index: value }
        | ExprKind::ReadLine { max_bytes: value }
        | ExprKind::StringByteLength { value }
        | ExprKind::ConvertNumeric { value, .. }
        | ExprKind::ConvertInteger { value, .. }
        | ExprKind::Unary { value, .. }
        | ExprKind::FieldAccess { base: value, .. } => string_expr(value),
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
        } => string_expr(left).or_else(|| string_expr(right)),
        ExprKind::Construct { base, fields, .. } => base
            .as_ref()
            .and_then(|base| string_expr(base))
            .or_else(|| fields.iter().find_map(|field| string_expr(&field.value))),
        ExprKind::Array(values)
        | ExprKind::Call {
            arguments: values, ..
        } => values.iter().find_map(string_expr),
        ExprKind::ArgumentCount
        | ExprKind::Boolean(_)
        | ExprKind::Integer(_)
        | ExprKind::Float { .. }
        | ExprKind::Variable { .. }
        | ExprKind::String(_) => None,
    }
}
