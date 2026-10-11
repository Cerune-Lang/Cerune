use super::*;
use crate::{source::Span, vm::Value};

impl Builder<'_> {
    pub(super) fn prepare_constants(&self) -> Result<(), Diagnostic> {
        let mut dependencies = Vec::new();
        for definition in &self.constant_sources {
            let mut pending = vec![&definition.value];
            let mut references = Vec::new();
            let mut steps = 0;
            while let Some(expr) = pending.pop() {
                steps += 1;
                if steps > 100_000 {
                    return Err(Diagnostic::new(
                        "constant expression expansion exceeds 100000 nodes",
                        expr.span,
                    ));
                }
                match &expr.kind {
                    ast::ExprKind::Let { value, body, .. } => {
                        pending.push(body);
                        pending.push(value);
                    }
                    ast::ExprKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    } => {
                        pending.push(else_value);
                        pending.push(then_value);
                        pending.push(condition);
                    }
                    ast::ExprKind::Match { .. } => {
                        unreachable!("match is expanded before constants")
                    }
                    ast::ExprKind::GenericCall(call) if call.name == "array_repeat" => {
                        return Err(Diagnostic::new(
                            "array_repeat is not allowed in constant expressions",
                            expr.span,
                        ));
                    }
                    ast::ExprKind::GenericCall(_) => {
                        unreachable!("generic calls are lowered before IR")
                    }
                    ast::ExprKind::Variable(name) => {
                        if let Some((id, _)) = self.model.constants.get(name) {
                            references.push((*id, expr.span));
                        }
                    }
                    ast::ExprKind::Call {
                        name, arguments, ..
                    } => {
                        if !matches!(name.as_str(), "byte_len" | "array_len" | "concat") {
                            return Err(Diagnostic::new(
                                "function calls are not allowed in constant expressions",
                                expr.span,
                            ));
                        }
                        pending.extend(arguments.iter().rev());
                    }
                    ast::ExprKind::Construct {
                        type_name,
                        type_name_span,
                        base,
                        fields,
                    } => {
                        if let Some(base) = base {
                            pending.extend(fields.iter().rev().map(|f| &f.value));
                            pending.push(base);
                        } else {
                            let ty = self.model.resolve_type_name(type_name, *type_name_span)?;
                            for field in self.model.type_definition(ty).fields.iter().rev() {
                                if !fields.iter().any(|f| f.name == field.name)
                                    && let Some(default) = &field.default
                                {
                                    pending.push(default);
                                }
                            }
                            pending.extend(fields.iter().rev().map(|f| &f.value));
                        }
                    }
                    ast::ExprKind::Array(values) => pending.extend(values.iter().rev()),
                    ast::ExprKind::Index { base, index } => {
                        pending.push(index);
                        pending.push(base);
                    }
                    ast::ExprKind::Binary { left, right, .. }
                    | ast::ExprKind::Logical { left, right, .. } => {
                        pending.push(right);
                        pending.push(left);
                    }
                    ast::ExprKind::FieldAccess { base: value, .. }
                    | ast::ExprKind::Unary { value, .. }
                    | ast::ExprKind::Convert { value, .. } => pending.push(value),
                    ast::ExprKind::Boolean(_)
                    | ast::ExprKind::String(_)
                    | ast::ExprKind::Integer(_)
                    | ast::ExprKind::Float { .. } => {}
                }
            }
            dependencies.push(references);
        }
        // ホストの呼出しスタックを使わず、参照順に循環と深さを検査します。
        let mut states = vec![0; dependencies.len()];
        let mut order = Vec::new();
        for root in 0..dependencies.len() {
            if states[root] != 0 {
                continue;
            }
            states[root] = 1;
            let mut stack = vec![(root, 0)];
            while let Some(&(id, next)) = stack.last() {
                if next == dependencies[id].len() {
                    stack.pop();
                    states[id] = 2;
                    order.push(id);
                    continue;
                }
                stack.last_mut().unwrap().1 += 1;
                let (dependency, span) = dependencies[id][next];
                if states[dependency] == 1 {
                    return Err(Diagnostic::new(
                        format!(
                            "cyclic constant reference to {}",
                            self.constant_sources[dependency].name
                        ),
                        span,
                    ));
                }
                if states[dependency] == 2 {
                    continue;
                }
                if stack.len() >= 128 {
                    return Err(Diagnostic::new(
                        "constant dependency depth exceeds 128",
                        span,
                    ));
                }
                states[dependency] = 1;
                stack.push((dependency, 0));
            }
        }
        for id in order {
            self.build_constant(id, self.constant_sources[id].name_span)?;
        }
        Ok(())
    }
    pub(super) fn build_constant(&self, id: usize, reference: Span) -> Result<Value, Diagnostic> {
        if let Some((_, value)) = &self.constant_cache.borrow()[id] {
            return Ok(value.clone());
        }
        if self.constant_stack.borrow().contains(&id) {
            return Err(Diagnostic::new(
                format!(
                    "cyclic constant reference to {}",
                    self.constant_sources[id].name
                ),
                reference,
            ));
        }
        if self.constant_stack.borrow().len() >= 128 {
            return Err(Diagnostic::new(
                "constant dependency depth exceeds 128",
                reference,
            ));
        }
        self.constant_stack.borrow_mut().push(id);
        let result = (|| {
            let source = self.constant_sources[id];
            let ty = self.model.constants[&source.name].1.clone();
            let initializer = self.build_expr(&source.value, Some(ty.clone()), &HashMap::new())?;
            let ty = ir_type(ty);
            // 型レイアウトだけを渡します。未使用の既定値を実行・再帰的に構築しません。
            let types = self
                .model
                .type_definitions
                .iter()
                .map(|d| TypeDefinition {
                    variants: d.variants.clone(),
                    id: TypeId(d.id.0),
                    name: d.name.clone(),
                    span: d.span,
                    fields: d
                        .fields
                        .iter()
                        .map(|f| FieldDefinition {
                            id: FieldId(f.id.0),
                            name: f.name.clone(),
                            ty: ir_type(f.ty.clone()),
                            default: None,
                            span: f.span,
                        })
                        .collect(),
                })
                .collect();
            let mut program = Program {
                string_heap_limit: super::super::DEFAULT_STRING_HEAP_LIMIT,
                array_heap_limit: super::super::DEFAULT_ARRAY_HEAP_LIMIT,
                constant_definitions: Vec::new(),
                type_definitions: types,
                function_definitions: vec![FunctionDefinition {
                    lowering: None,
                    generic_origin: None,
                    id: FunctionId(0),
                    name: "constant_evaluation".into(),
                    parameters: Vec::new(),
                    return_type: ReturnType::Value(ty.clone()),
                    span: source.span,
                    body: vec![Statement {
                        id: self.allocate_node_id(),
                        span: source.value.span,
                        kind: StatementKind::Return {
                            value: Some(initializer.clone()),
                        },
                    }],
                }],
                statements: Vec::new(),
            };
            super::super::aggregates::lower(&mut program);
            super::super::ownership::lower(&mut program);
            let bytecode = crate::bytecode::lower(&program)?;
            let evaluated = crate::vm::evaluate_constant(&bytecode).map_err(|error| {
                let function_id = error.function_id().unwrap_or(0);
                let instruction =
                    &bytecode.functions[function_id].instructions[error.instruction_index()];
                let span = match instruction.origin {
                    crate::bytecode::InstructionOrigin::Source { span, .. } => span,
                    _ => source.value.span,
                };
                let reason = crate::runtime::FailureCode::from_vm(error.kind())
                    .map_or_else(|| format!("{:?}", error.kind()), |code| code.name().into());
                Diagnostic::new(
                    format!("constant {} evaluation failed: {reason}", source.name),
                    span,
                )
            })?;
            let value = self.freeze(evaluated.clone(), &ty, source.value.span);
            self.constant_cache.borrow_mut()[id] = Some((
                super::super::ConstantDefinition {
                    array_length_uses: Vec::new(),
                    id,
                    name: source.name.clone(),
                    ty,
                    initializer,
                    value,
                    span: source.span,
                },
                evaluated.clone(),
            ));
            Ok(evaluated)
        })();
        self.constant_stack.borrow_mut().pop();
        result
    }

    /// 値は各使用箇所へ独立して展開し、宣言との対応はConstant参照に残します。
    pub(super) fn freeze(&self, value: Value, ty: &Type, span: Span) -> Expr {
        let id = self.allocate_node_id();
        let kind = match value {
            Value::DynamicArray { .. } => unreachable!("dynamic arrays are not constant values"),
            Value::Bool(v) => ExprKind::Boolean(v),
            Value::String(v) => ExprKind::String(v.text()),
            Value::Integer(v, _) => ExprKind::Integer(v),
            Value::F32(v) => ExprKind::Float {
                text: float_text(f64::from(v)),
            },
            Value::F64(v) => ExprKind::Float {
                text: float_text(v),
            },
            Value::Array { values, .. } => {
                let Type::Array { element, .. } = ty else {
                    unreachable!()
                };
                ExprKind::Array(
                    values
                        .into_iter()
                        .map(|v| self.freeze(v, element, span))
                        .collect(),
                )
            }
            Value::Aggregate { type_id, fields } => {
                let definition = &self.model.type_definitions[type_id];
                ExprKind::Construct {
                    type_id: TypeId(type_id),
                    type_name: definition.name.clone(),
                    base: None,
                    fields: definition
                        .fields
                        .iter()
                        .zip(fields)
                        .map(|(f, v)| FieldValue {
                            id: FieldId(f.id.0),
                            name: f.name.clone(),
                            origin: FieldValueOrigin::Generated { span },
                            value: self.freeze(v, &ir_type(f.ty.clone()), span),
                        })
                        .collect(),
                }
            }
        };
        Expr {
            id,
            kind,
            ty: ty.clone(),
            span,
        }
    }
}

fn float_text(value: f64) -> String {
    if value.is_nan() {
        return "nan".into();
    }
    let text = value.to_string();
    if value.is_finite() && !text.contains(['.', 'e', 'E']) {
        format!("{text}.0")
    } else {
        text
    }
}
