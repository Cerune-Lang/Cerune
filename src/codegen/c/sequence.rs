use super::ir::{BinaryOp, Expr, ExprKind, Module, Statement, Type, UnaryOp};

/// Cが順序を保証しない場所で、作用や失敗の順番をCeruneに合わせます。
pub(super) fn lower(module: &mut Module) {
    let counted: Vec<bool> = module
        .functions
        .iter()
        .map(|f| module.call_limit.is_some() && f.source_function)
        .collect();
    for function in &mut module.functions {
        statements(&mut function.body, &mut function.temporaries, &counted);
    }
    statements(&mut module.statements, &mut module.temporaries, &counted);
}

fn statements(body: &mut [Statement], temporaries: &mut Vec<Type>, counted: &[bool]) {
    for statement in body {
        match statement {
            Statement::ArrayInitialize { array, value } => {
                expression(array, temporaries, counted);
                expression(value, temporaries, counted);
            }
            Statement::ArrayRangeCheck {
                length, start, end, ..
            } => {
                for value in [length, start, end] {
                    expression(value, temporaries, counted);
                }
            }
            Statement::ArrayManage { value, .. }
            | Statement::Binding { value, .. }
            | Statement::StringManage { value, .. }
            | Statement::Write { value, .. }
            | Statement::Print { value, .. } => {
                expression(value, temporaries, counted);
            }
            Statement::Assignment { target, value } => {
                for projection in &mut target.projections {
                    expression(&mut projection.index, temporaries, counted);
                }
                expression(value, temporaries, counted);
            }
            Statement::Call {
                evaluation,
                function_id,
                arguments,
                ..
            } => {
                for argument in arguments.iter_mut() {
                    expression(argument, temporaries, counted);
                }
                *evaluation = order(
                    arguments.iter_mut().collect(),
                    temporaries,
                    counted[*function_id],
                );
            }
            Statement::Return(value) => {
                if let Some(value) = value {
                    expression(value, temporaries, counted);
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                expression(condition, temporaries, counted);
                statements(then_body, temporaries, counted);
                statements(else_body, temporaries, counted);
            }
            Statement::While { condition, body } => {
                expression(condition, temporaries, counted);
                statements(body, temporaries, counted);
            }
            Statement::For {
                initializer,
                condition,
                update,
                body,
            } => {
                statements(std::slice::from_mut(initializer), temporaries, counted);
                expression(condition, temporaries, counted);
                statements(std::slice::from_mut(update), temporaries, counted);
                statements(body, temporaries, counted);
            }
            Statement::Flush { .. } | Statement::Break | Statement::Continue => {}
        }
    }
}

fn expression(expr: &mut Expr, temporaries: &mut Vec<Type>, counted: &[bool]) {
    if let ExprKind::Construct {
        base: Some(base),
        fields,
        copy_temp,
        ..
    } = &mut expr.kind
    {
        expression(base, temporaries, counted);
        for field in fields {
            expression(&mut field.value, temporaries, counted);
        }
        // Cのカンマ式でコピー、明示フィールド、結果の順序を固定します。
        *copy_temp = Some(temporaries.len());
        temporaries.push(expr.ty.clone());
        return;
    }
    let guarded = matches!(&expr.kind,ExprKind::Call {function_id,..} if counted[*function_id]);
    let mut children: Vec<&mut Expr> = match &mut expr.kind {
        ExprKind::StringByteAt {
            value: left,
            index: right,
        }
        | ExprKind::StringConcat { left, right }
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Index {
            base: left,
            index: right,
        } => {
            vec![left, right]
        }
        ExprKind::Construct { fields, .. } => {
            fields.iter_mut().map(|field| &mut field.value).collect()
        }
        ExprKind::Array(values)
        | ExprKind::Call {
            arguments: values, ..
        } => values.iter_mut().collect(),
        ExprKind::Logical { left, right, .. } | ExprKind::IntegerBinary { left, right, .. } => {
            // 短絡評価と、既にカンマ式で順序を持つ整数演算は、その位置を保ちます。
            expression(left, temporaries, counted);
            expression(right, temporaries, counted);
            return;
        }
        ExprKind::ArrayAllocate { length: value, .. }
        | ExprKind::ArrayReleaseOwner { value }
        | ExprKind::ArrayLength { value, .. }
        | ExprKind::ArgumentGet { index: value }
        | ExprKind::ReadLine { max_bytes: value }
        | ExprKind::StringByteLength { value }
        | ExprKind::ConvertNumeric { value, .. }
        | ExprKind::CheckIntegerRange { value, .. }
        | ExprKind::Unary { value, .. }
        | ExprKind::FieldAccess { base: value, .. } => {
            expression(value, temporaries, counted);
            return;
        }
        ExprKind::Sequence { .. } => unreachable!("C sequencing runs once after lowering"),
        ExprKind::ArgumentCount
        | ExprKind::Boolean(_)
        | ExprKind::String(_)
        | ExprKind::Integer(_)
        | ExprKind::Float { .. }
        | ExprKind::Variable(_)
        | ExprKind::Temporary(_) => return,
    };
    for child in &mut children {
        expression(child, temporaries, counted);
    }
    let bindings = order(children, temporaries, guarded);
    if guarded {
        let ExprKind::Call { result_temp, .. } = &mut expr.kind else {
            unreachable!()
        };
        *result_temp = Some(temporaries.len());
        temporaries.push(expr.ty.clone());
    }
    if !bindings.is_empty() {
        let value = std::mem::replace(&mut expr.kind, ExprKind::Boolean(false));
        expr.kind = ExprKind::Sequence {
            bindings,
            value: Box::new(Expr {
                origin: expr.origin,
                ty: expr.ty.clone(),
                kind: value,
            }),
        };
    }
}

fn order(children: Vec<&mut Expr>, temporaries: &mut Vec<Type>, force: bool) -> Vec<(usize, Expr)> {
    if !force && children.iter().filter(|child| observable(child)).count() < 2 {
        return Vec::new();
    }
    children
        .into_iter()
        .map(|child| {
            let id = temporaries.len();
            temporaries.push(child.ty.clone());
            let value = std::mem::replace(
                child,
                Expr {
                    origin: child.origin,
                    ty: child.ty.clone(),
                    kind: ExprKind::Temporary(id),
                },
            );
            (id, value)
        })
        .collect()
}

fn observable(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::ArrayAllocate { .. }
        | ExprKind::ArrayReleaseOwner { .. }
        | ExprKind::StringConcat { .. }
        | ExprKind::ArgumentCount
        | ExprKind::ArgumentGet { .. }
        | ExprKind::ReadLine { .. }
        | ExprKind::Call { .. }
        | ExprKind::Index { .. }
        | ExprKind::ConvertNumeric { .. }
        | ExprKind::CheckIntegerRange { .. }
        | ExprKind::IntegerBinary { .. }
        | ExprKind::Sequence { .. } => true,
        ExprKind::Unary { op, value } => *op == UnaryOp::CheckedI64Negate || observable(value),
        ExprKind::Binary { op, left, right } => {
            matches!(
                op,
                BinaryOp::CheckedI64Add
                    | BinaryOp::CheckedI64Subtract
                    | BinaryOp::CheckedI64Multiply
                    | BinaryOp::CheckedI64Divide
            ) || observable(left)
                || observable(right)
        }
        ExprKind::StringByteAt {
            value: left,
            index: right,
        }
        | ExprKind::Logical { left, right, .. } => observable(left) || observable(right),
        ExprKind::ArrayLength { value: base, .. }
        | ExprKind::StringByteLength { value: base }
        | ExprKind::FieldAccess { base, .. } => observable(base),
        ExprKind::Construct { base, fields, .. } => {
            base.is_some() || fields.iter().any(|field| observable(&field.value))
        }
        ExprKind::Array(values) => values.iter().any(observable),
        ExprKind::Boolean(_)
        | ExprKind::String(_)
        | ExprKind::Integer(_)
        | ExprKind::Float { .. }
        | ExprKind::Variable(_)
        | ExprKind::Temporary(_) => false,
    }
}
