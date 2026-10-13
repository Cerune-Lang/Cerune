use crate::ir as cerune_ir;

use super::ir::{
    ArrayProjection, AssignmentTarget, BinaryOp, Expr, ExprKind, FieldDefinition, FieldValue,
    Function, Module, Parameter, PrintFormat, Statement, Type, TypeDefinition, UnaryOp,
};

pub fn lower(program: &cerune_ir::Program) -> Module {
    let mut module = Module {
        array_heap_limit: program
            .first_dynamic_array_span()
            .map(|_| program.array_heap_limit),
        string_heap_limit: crate::codegen::support::string_heap_limit(program),
        uses_write: crate::codegen::display::uses_write(program),
        uses_strings: crate::codegen::support::first_string_span(program).is_some(),
        temporaries: Vec::new(),
        array_types: collect_array_types(program),
        array_assignment_types: collect_array_assignment_types(program),
        type_definitions: lower_type_definitions(program),
        functions: program
            .function_definitions
            .iter()
            .map(|function| Function {
                temporaries: Vec::new(),
                id: function.id.0,
                name: function.name.clone(),
                parameters: function
                    .parameters
                    .iter()
                    .map(|parameter| Parameter {
                        name: binding_name(parameter.id, &parameter.name),
                        ty: parameter.ty.clone().into(),
                    })
                    .collect(),
                return_type: match &function.return_type {
                    cerune_ir::ReturnType::Void => None,
                    cerune_ir::ReturnType::Value(ty) => Some(ty.clone().into()),
                },
                body: function.body.iter().map(lower_statement).collect(),
            })
            .collect(),
        explicit_main: program
            .function_definitions
            .iter()
            .find(|function| function.name == "main")
            .map(|function| function.id.0),
        statements: program.statements.iter().map(lower_statement).collect(),
    };
    super::sequence::lower(&mut module);
    module
}

fn collect_array_assignment_types(program: &cerune_ir::Program) -> Vec<Type> {
    fn visit(statements: &[cerune_ir::Statement], result: &mut Vec<Type>) {
        for statement in statements {
            match &statement.kind {
                cerune_ir::StatementKind::ArrayInitialize { .. }
                | cerune_ir::StatementKind::ArrayRetain { .. }
                | cerune_ir::StatementKind::ArrayFree { .. }
                | cerune_ir::StatementKind::ArrayRangeCheck { .. } => {}
                cerune_ir::StatementKind::Assignment { target, .. } => {
                    for projection in &target.projections {
                        let ty = match projection {
                            cerune_ir::AssignmentProjection::Index {
                                element, length, ..
                            } => Type::Array {
                                element: Box::new(element.clone().into()),
                                length: *length,
                            },
                            cerune_ir::AssignmentProjection::DynamicIndex { element, .. } => {
                                Type::DynamicArray {
                                    element: Box::new(element.clone().into()),
                                }
                            }
                        };
                        if !result.contains(&ty) {
                            result.push(ty);
                        }
                    }
                }
                cerune_ir::StatementKind::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    visit(then_body, result);
                    visit(else_body, result);
                }
                cerune_ir::StatementKind::While { body, .. } => visit(body, result),
                cerune_ir::StatementKind::For {
                    initializer,
                    update,
                    body,
                    ..
                } => {
                    visit(std::slice::from_ref(initializer), result);
                    visit(std::slice::from_ref(update), result);
                    visit(body, result);
                }
                cerune_ir::StatementKind::Binding { .. }
                | cerune_ir::StatementKind::StringManage { .. }
                | cerune_ir::StatementKind::Write { .. }
                | cerune_ir::StatementKind::Flush
                | cerune_ir::StatementKind::Print { .. }
                | cerune_ir::StatementKind::Call { .. }
                | cerune_ir::StatementKind::Return { .. }
                | cerune_ir::StatementKind::Break
                | cerune_ir::StatementKind::Continue => {}
            }
        }
    }

    let mut result = Vec::new();
    for function in &program.function_definitions {
        visit(&function.body, &mut result);
    }
    visit(&program.statements, &mut result);
    result
}

fn lower_type_definitions(program: &cerune_ir::Program) -> Vec<TypeDefinition> {
    fn visit(
        id: usize,
        program: &cerune_ir::Program,
        visited: &mut [bool],
        definitions: &mut Vec<TypeDefinition>,
    ) {
        if visited[id] {
            return;
        }
        visited[id] = true;
        let definition = &program.type_definitions[id];
        for field in &definition.fields {
            if let Some(dependency) = named_type_dependency(&field.ty) {
                visit(dependency.0, program, visited, definitions);
            }
        }
        definitions.push(TypeDefinition {
            id,
            name: definition.name.clone(),
            fields: definition
                .fields
                .iter()
                .map(|field| FieldDefinition {
                    name: storage_field_name(&field.name),
                    ty: field.ty.clone().into(),
                })
                .collect(),
        });
    }

    let mut visited = vec![false; program.type_definitions.len()];
    let mut definitions = Vec::new();
    for id in 0..program.type_definitions.len() {
        visit(id, program, &mut visited, &mut definitions);
    }
    definitions
}

fn named_type_dependency(ty: &cerune_ir::Type) -> Option<cerune_ir::TypeId> {
    match ty {
        // 記述子の定義は要素型の完全な定義に依存しません。
        cerune_ir::Type::DynamicArray { .. } => None,
        cerune_ir::Type::String => None,
        cerune_ir::Type::Named(id) => Some(*id),
        cerune_ir::Type::Array { element, .. } => named_type_dependency(element),
        cerune_ir::Type::Bool
        | cerune_ir::Type::Integer(_)
        | cerune_ir::Type::F32
        | cerune_ir::Type::F64 => None,
    }
}

fn lower_statement(statement: &cerune_ir::Statement) -> Statement {
    match &statement.kind {
        cerune_ir::StatementKind::ArrayInitialize { array, value } => Statement::ArrayInitialize {
            array: lower_expr(array),
            value: lower_expr(value),
        },
        cerune_ir::StatementKind::ArrayRetain { value }
        | cerune_ir::StatementKind::ArrayFree { value } => Statement::ArrayManage {
            value: lower_expr(value),
            retain: matches!(statement.kind, cerune_ir::StatementKind::ArrayRetain { .. }),
        },
        cerune_ir::StatementKind::ArrayRangeCheck { length, start, end } => {
            Statement::ArrayRangeCheck {
                origin: super::ir::Origin {
                    node_id: statement.id,
                    span: statement.span,
                },
                length: lower_expr(length),
                start: lower_expr(start),
                end: lower_expr(end),
            }
        }
        cerune_ir::StatementKind::Binding {
            id,
            name,
            ty,
            value,
            ..
        } => Statement::Binding {
            name: binding_name(*id, name),
            ty: ty.clone().into(),
            value: lower_expr(value),
        },

        cerune_ir::StatementKind::Assignment { target, value } => Statement::Assignment {
            target: AssignmentTarget {
                name: binding_name(target.id, &target.name),
                projections: target
                    .projections
                    .iter()
                    .map(|projection| {
                        let (index, element, length, span) = match projection {
                            cerune_ir::AssignmentProjection::Index {
                                index,
                                element,
                                length,
                                span,
                            } => (index, element, Some(*length), span),
                            cerune_ir::AssignmentProjection::DynamicIndex {
                                index,
                                element,
                                span,
                            } => (index, element, None, span),
                        };
                        ArrayProjection {
                            origin: super::ir::Origin {
                                node_id: statement.id,
                                span: *span,
                            },
                            index: lower_expr(index),
                            element: element.clone().into(),
                            length,
                        }
                    })
                    .collect(),
                ty: target.ty.clone().into(),
            },
            value: lower_expr(value),
        },

        cerune_ir::StatementKind::StringManage { value, retain } => Statement::StringManage {
            value: lower_expr(value),
            retain: *retain,
        },
        cerune_ir::StatementKind::Write { value, quoted } => Statement::Write {
            origin: super::ir::Origin {
                node_id: statement.id,
                span: statement.span,
            },
            kind: crate::codegen::display::kind(&value.ty, *quoted),
            value: lower_expr(value),
        },
        cerune_ir::StatementKind::Flush => Statement::Flush {
            origin: super::ir::Origin {
                node_id: statement.id,
                span: statement.span,
            },
        },
        cerune_ir::StatementKind::Print { value } => Statement::Print {
            origin: super::ir::Origin {
                node_id: statement.id,
                span: statement.span,
            },
            format: print_format(&value.ty),
            value: lower_expr(value),
        },

        cerune_ir::StatementKind::Call {
            function_id,
            function_name,
            arguments,
        } => Statement::Call {
            evaluation: Vec::new(),
            function_id: function_id.0,
            function_name: function_name.clone(),
            arguments: arguments.iter().map(lower_expr).collect(),
        },

        cerune_ir::StatementKind::Return { value } => {
            Statement::Return(value.as_ref().map(lower_expr))
        }

        cerune_ir::StatementKind::If {
            condition,
            then_body,
            else_body,
        } => Statement::If {
            condition: lower_expr(condition),
            then_body: then_body.iter().map(lower_statement).collect(),
            else_body: else_body.iter().map(lower_statement).collect(),
        },

        cerune_ir::StatementKind::While { condition, body } => Statement::While {
            condition: lower_expr(condition),
            body: body.iter().map(lower_statement).collect(),
        },

        cerune_ir::StatementKind::For {
            initializer,
            condition,
            update,
            body,
        } => Statement::For {
            initializer: Box::new(lower_statement(initializer)),
            condition: lower_expr(condition),
            update: Box::new(lower_statement(update)),
            body: body.iter().map(lower_statement).collect(),
        },

        cerune_ir::StatementKind::Break => Statement::Break,
        cerune_ir::StatementKind::Continue => Statement::Continue,
    }
}

// Cの宣言スコープや補助関数名に影響されず、解決済みの束縛を参照します。
fn binding_name(id: cerune_ir::BindingId, name: &str) -> String {
    format!("binding_{}_{}", id.0, name.replace('$', "_"))
}

// enum内部名の区切りとソースのアンダースコアを、衝突しない形で区別します。
fn storage_field_name(name: &str) -> String {
    if name.starts_with('$') {
        name.replace('_', "_u").replace('$', "_d")
    } else {
        name.into()
    }
}

fn lower_expr(expr: &cerune_ir::Expr) -> Expr {
    let value = lower_expr_unchecked(expr);
    if let Some(ty) = super::super::integer_range_check(expr) {
        Expr {
            origin: expr.into(),
            ty: value.ty.clone(),
            kind: ExprKind::CheckIntegerRange {
                value: Box::new(value),
                ty,
                code: match expr.kind {
                    cerune_ir::ExprKind::ConvertInteger { .. } => {
                        crate::runtime::FailureCode::IntegerConversionOutOfRange
                    }
                    cerune_ir::ExprKind::Binary {
                        op: cerune_ir::BinaryOp::Divide,
                        ..
                    } => crate::runtime::FailureCode::DivisionOverflow,
                    _ => crate::runtime::FailureCode::IntegerOverflow,
                },
            },
        }
    } else {
        value
    }
}

fn lower_expr_unchecked(expr: &cerune_ir::Expr) -> Expr {
    if let Some((value, conversion)) = crate::codegen::u64_integer_conversion(expr) {
        return Expr {
            origin: expr.into(),
            ty: expr.ty.clone().into(),
            kind: ExprKind::ConvertNumeric {
                value: Box::new(lower_expr(value)),
                conversion,
            },
        };
    }

    let kind = match &expr.kind {
        cerune_ir::ExprKind::ArrayCopy { .. } | cerune_ir::ExprKind::ArrayRepeat { .. } => {
            unreachable!("array copies are expanded in common IR")
        }
        cerune_ir::ExprKind::ArrayAllocate {
            length,
            element_width,
        } => ExprKind::ArrayAllocate {
            length: Box::new(lower_expr(length)),
            element_width: *element_width,
        },
        cerune_ir::ExprKind::ArrayReleaseOwner { value } => ExprKind::ArrayReleaseOwner {
            value: Box::new(lower_expr(value)),
        },
        cerune_ir::ExprKind::Let { .. } | cerune_ir::ExprKind::Conditional { .. } => {
            unreachable!("match expressions are lowered before code generation")
        }
        cerune_ir::ExprKind::Constant { value, .. } => lower_expr(value).kind,
        cerune_ir::ExprKind::ArrayLength { value } => {
            let length = match &value.ty {
                cerune_ir::Type::Array { length, .. } => Some(*length),
                cerune_ir::Type::DynamicArray { .. } => None,
                _ => unreachable!(),
            };
            ExprKind::ArrayLength {
                value: Box::new(lower_expr(value)),
                length,
            }
        }
        cerune_ir::ExprKind::StringConcat { left, right } => ExprKind::StringConcat {
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        cerune_ir::ExprKind::StringByteLength { value } => ExprKind::StringByteLength {
            value: Box::new(lower_expr(value)),
        },
        cerune_ir::ExprKind::String(value) => ExprKind::String(value.clone()),
        cerune_ir::ExprKind::Logical { op, left, right } => ExprKind::Logical {
            op: match op {
                cerune_ir::LogicalOp::And => super::ir::LogicalOp::And,
                cerune_ir::LogicalOp::Or => super::ir::LogicalOp::Or,
            },
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        cerune_ir::ExprKind::ConvertNumeric {
            value,
            from,
            to,
            mode,
            ..
        } => {
            if from == to {
                return lower_expr(value);
            }
            ExprKind::ConvertNumeric {
                value: Box::new(lower_expr(value)),
                conversion: crate::codegen::NumericConversion {
                    mode: *mode,
                    from: *from,
                    to: *to,
                },
            }
        }
        cerune_ir::ExprKind::ConvertInteger { value, .. } => return lower_expr(value),
        cerune_ir::ExprKind::Boolean(value) => ExprKind::Boolean(*value),

        cerune_ir::ExprKind::Integer(value) => ExprKind::Integer(*value),

        cerune_ir::ExprKind::Float { text } => ExprKind::Float {
            text: text.clone(),
            suffix_f32: expr.ty == cerune_ir::Type::F32,
        },

        cerune_ir::ExprKind::Variable { id, name } => ExprKind::Variable(binding_name(*id, name)),

        cerune_ir::ExprKind::Unary {
            op: cerune_ir::UnaryOp::BitNot,
            value,
        } => ExprKind::IntegerBinary {
            scratch: expr.id.0,
            op: crate::codegen::IntegerBinaryOp::BitXor,
            ty: crate::codegen::integer_type(&expr.ty),
            left: Box::new(lower_expr(value)),
            right: Box::new(Expr {
                origin: expr.into(),
                ty: expr.ty.clone().into(),
                kind: ExprKind::Integer(crate::codegen::complement_mask(&expr.ty) as i128),
            }),
        },
        cerune_ir::ExprKind::Unary { op, value } => ExprKind::Unary {
            op: lower_unary_op(*op, &expr.ty),
            value: Box::new(lower_expr(value)),
        },

        cerune_ir::ExprKind::Binary { op, left, right }
            if crate::codegen::integer_binary_op(*op, &left.ty).is_some() =>
        {
            ExprKind::IntegerBinary {
                scratch: expr.id.0,
                op: crate::codegen::integer_binary_op(*op, &left.ty).unwrap(),
                ty: crate::codegen::integer_type(&expr.ty),
                left: Box::new(lower_expr(left)),
                right: Box::new(lower_expr(right)),
            }
        }
        cerune_ir::ExprKind::Binary { op, left, right } => ExprKind::Binary {
            op: lower_binary_op(*op, &left.ty),
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },

        cerune_ir::ExprKind::Construct {
            type_id,
            base,
            fields,
            ..
        } => ExprKind::Construct {
            type_id: type_id.0,
            base: base.as_ref().map(|base| Box::new(lower_expr(base))),
            copy_temp: None,
            fields: fields
                .iter()
                .map(|field| FieldValue {
                    name: storage_field_name(&field.name),
                    value: lower_expr(&field.value),
                })
                .collect(),
        },

        cerune_ir::ExprKind::FieldAccess {
            field_name, base, ..
        } => ExprKind::FieldAccess {
            field_name: storage_field_name(field_name),
            base: Box::new(lower_expr(base)),
        },
        cerune_ir::ExprKind::Array(values) => {
            ExprKind::Array(values.iter().map(lower_expr).collect())
        }
        cerune_ir::ExprKind::Index { base, index } => ExprKind::Index {
            base: Box::new(lower_expr(base)),
            index: Box::new(lower_expr(index)),
        },
        cerune_ir::ExprKind::Call {
            function_id,
            function_name,
            arguments,
        } => ExprKind::Call {
            function_id: function_id.0,
            function_name: function_name.clone(),
            arguments: arguments.iter().map(lower_expr).collect(),
        },
    };

    Expr {
        origin: expr.into(),
        ty: expr.ty.clone().into(),
        kind,
    }
}

fn print_format(ty: &cerune_ir::Type) -> PrintFormat {
    match ty {
        cerune_ir::Type::DynamicArray { .. } => {
            unreachable!("dynamic arrays are rejected before backend lowering")
        }
        cerune_ir::Type::String => PrintFormat::String,
        cerune_ir::Type::Bool => PrintFormat::Bool,
        cerune_ir::Type::Integer(crate::types::IntegerType::U64) => PrintFormat::U64,
        cerune_ir::Type::Integer(_) => PrintFormat::I64,
        cerune_ir::Type::F32 => PrintFormat::F32,
        cerune_ir::Type::F64 => PrintFormat::F64,
        cerune_ir::Type::Named(_) | cerune_ir::Type::Array { .. } => {
            unreachable!("semantic analysis rejects aggregate printing")
        }
    }
}

impl From<cerune_ir::Type> for Type {
    fn from(value: cerune_ir::Type) -> Self {
        match value {
            cerune_ir::Type::DynamicArray { element } => Self::DynamicArray {
                element: Box::new((*element).into()),
            },
            cerune_ir::Type::String => Self::String,
            cerune_ir::Type::Bool => Self::Bool,
            cerune_ir::Type::Integer(crate::types::IntegerType::U64) => Self::U64,
            cerune_ir::Type::Integer(_) => Self::I64,
            cerune_ir::Type::F32 => Self::Float,
            cerune_ir::Type::F64 => Self::Double,
            cerune_ir::Type::Named(id) => Self::Named(id.0),
            cerune_ir::Type::Array { element, length } => Self::Array {
                element: Box::new((*element).into()),
                length,
            },
        }
    }
}

fn collect_array_types(program: &cerune_ir::Program) -> Vec<Type> {
    fn add(ty: &cerune_ir::Type, types: &mut Vec<Type>) {
        if let cerune_ir::Type::Array { element, .. } | cerune_ir::Type::DynamicArray { element } =
            ty
        {
            add(element, types);
            let ty = ty.clone().into();
            if !types.contains(&ty) {
                types.push(ty);
            }
        }
    }

    fn visit_expr(expr: &cerune_ir::Expr, types: &mut Vec<Type>) {
        add(&expr.ty, types);
        match &expr.kind {
            cerune_ir::ExprKind::ArrayCopy { .. } | cerune_ir::ExprKind::ArrayRepeat { .. } => {
                unreachable!("array copies are expanded in common IR")
            }
            cerune_ir::ExprKind::ArrayAllocate { length: value, .. }
            | cerune_ir::ExprKind::ArrayReleaseOwner { value } => visit_expr(value, types),
            cerune_ir::ExprKind::Let { .. } | cerune_ir::ExprKind::Conditional { .. } => {
                unreachable!("match expressions are lowered before code generation")
            }
            cerune_ir::ExprKind::Constant { value, .. } => visit_expr(value, types),
            cerune_ir::ExprKind::ArrayLength { value }
            | cerune_ir::ExprKind::StringByteLength { value } => visit_expr(value, types),
            cerune_ir::ExprKind::String(_) => {}
            cerune_ir::ExprKind::Array(values) => {
                for value in values {
                    visit_expr(value, types);
                }
            }
            cerune_ir::ExprKind::StringConcat {
                left: base,
                right: index,
            }
            | cerune_ir::ExprKind::Index { base, index }
            | cerune_ir::ExprKind::Logical {
                left: base,
                right: index,
                ..
            }
            | cerune_ir::ExprKind::Binary {
                left: base,
                right: index,
                ..
            } => {
                visit_expr(base, types);
                visit_expr(index, types);
            }
            cerune_ir::ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    visit_expr(base, types);
                }
                for field in fields {
                    visit_expr(&field.value, types);
                }
            }
            cerune_ir::ExprKind::FieldAccess { base, .. }
            | cerune_ir::ExprKind::ConvertNumeric { value: base, .. }
            | cerune_ir::ExprKind::ConvertInteger { value: base, .. }
            | cerune_ir::ExprKind::Unary { value: base, .. } => visit_expr(base, types),
            cerune_ir::ExprKind::Call { arguments, .. } => {
                for argument in arguments {
                    visit_expr(argument, types);
                }
            }
            cerune_ir::ExprKind::Boolean(_)
            | cerune_ir::ExprKind::Integer(_)
            | cerune_ir::ExprKind::Float { .. }
            | cerune_ir::ExprKind::Variable { .. } => {}
        }
    }

    fn visit_statement(statement: &cerune_ir::Statement, types: &mut Vec<Type>) {
        match &statement.kind {
            cerune_ir::StatementKind::Flush => {}
            cerune_ir::StatementKind::ArrayInitialize { array, value } => {
                visit_expr(array, types);
                visit_expr(value, types);
            }
            cerune_ir::StatementKind::ArrayRetain { value }
            | cerune_ir::StatementKind::ArrayFree { value } => visit_expr(value, types),
            cerune_ir::StatementKind::ArrayRangeCheck { length, start, end } => {
                for value in [length, start, end] {
                    visit_expr(value, types);
                }
            }
            cerune_ir::StatementKind::Binding { ty, value, .. } => {
                add(ty, types);
                visit_expr(value, types);
            }
            cerune_ir::StatementKind::Assignment { target, value } => {
                add(&target.root_ty, types);
                for projection in &target.projections {
                    let (cerune_ir::AssignmentProjection::Index { index, .. }
                    | cerune_ir::AssignmentProjection::DynamicIndex { index, .. }) = projection;
                    visit_expr(index, types);
                }
                visit_expr(value, types);
            }
            cerune_ir::StatementKind::StringManage { value, .. }
            | cerune_ir::StatementKind::Write { value, .. }
            | cerune_ir::StatementKind::Print { value }
            | cerune_ir::StatementKind::Return { value: Some(value) } => visit_expr(value, types),
            cerune_ir::StatementKind::Call { arguments, .. } => {
                for argument in arguments {
                    visit_expr(argument, types);
                }
            }
            cerune_ir::StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                visit_expr(condition, types);
                for statement in then_body.iter().chain(else_body) {
                    visit_statement(statement, types);
                }
            }
            cerune_ir::StatementKind::While { condition, body } => {
                visit_expr(condition, types);
                for statement in body {
                    visit_statement(statement, types);
                }
            }
            cerune_ir::StatementKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                visit_statement(initializer, types);
                visit_expr(condition, types);
                visit_statement(update, types);
                for statement in body {
                    visit_statement(statement, types);
                }
            }
            cerune_ir::StatementKind::Return { value: None }
            | cerune_ir::StatementKind::Break
            | cerune_ir::StatementKind::Continue => {}
        }
    }

    let mut types = Vec::new();
    for definition in &program.type_definitions {
        for field in &definition.fields {
            add(&field.ty, &mut types);
        }
    }
    for function in &program.function_definitions {
        for parameter in &function.parameters {
            add(&parameter.ty, &mut types);
        }
        if let cerune_ir::ReturnType::Value(ty) = &function.return_type {
            add(ty, &mut types);
        }
        for statement in &function.body {
            visit_statement(statement, &mut types);
        }
    }
    for statement in &program.statements {
        visit_statement(statement, &mut types);
    }
    types
}

fn lower_unary_op(op: cerune_ir::UnaryOp, ty: &cerune_ir::Type) -> UnaryOp {
    match (op, ty) {
        (cerune_ir::UnaryOp::BitNot, _) => unreachable!("bit complement uses integer lowering"),
        (cerune_ir::UnaryOp::Negate, cerune_ir::Type::Integer(_)) => UnaryOp::CheckedI64Negate,
        (cerune_ir::UnaryOp::Negate, _) => UnaryOp::Negate,
        (cerune_ir::UnaryOp::Not, _) => UnaryOp::Not,
    }
}

fn lower_binary_op(op: cerune_ir::BinaryOp, operand_ty: &cerune_ir::Type) -> BinaryOp {
    match (op, operand_ty) {
        (
            cerune_ir::BinaryOp::Remainder
            | cerune_ir::BinaryOp::BitAnd
            | cerune_ir::BinaryOp::BitOr
            | cerune_ir::BinaryOp::BitXor
            | cerune_ir::BinaryOp::ShiftLeft
            | cerune_ir::BinaryOp::ShiftRight,
            _,
        ) => unreachable!("integer operation uses separate lowering"),
        (cerune_ir::BinaryOp::Add, cerune_ir::Type::Integer(_)) => BinaryOp::CheckedI64Add,
        (cerune_ir::BinaryOp::Subtract, cerune_ir::Type::Integer(_)) => {
            BinaryOp::CheckedI64Subtract
        }
        (cerune_ir::BinaryOp::Multiply, cerune_ir::Type::Integer(_)) => {
            BinaryOp::CheckedI64Multiply
        }
        (cerune_ir::BinaryOp::Divide, cerune_ir::Type::Integer(_)) => BinaryOp::CheckedI64Divide,
        (cerune_ir::BinaryOp::Add, _) => BinaryOp::Add,
        (cerune_ir::BinaryOp::Subtract, _) => BinaryOp::Subtract,
        (cerune_ir::BinaryOp::Multiply, _) => BinaryOp::Multiply,
        (cerune_ir::BinaryOp::Divide, _) => BinaryOp::Divide,
        (cerune_ir::BinaryOp::Equal, _) => BinaryOp::Equal,
        (cerune_ir::BinaryOp::NotEqual, _) => BinaryOp::NotEqual,
        (cerune_ir::BinaryOp::Less, _) => BinaryOp::Less,
        (cerune_ir::BinaryOp::LessEqual, _) => BinaryOp::LessEqual,
        (cerune_ir::BinaryOp::Greater, _) => BinaryOp::Greater,
        (cerune_ir::BinaryOp::GreaterEqual, _) => BinaryOp::GreaterEqual,
    }
}
