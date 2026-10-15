use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

mod constants;

use crate::{
    ast,
    diagnostic::Diagnostic,
    semantic::{self, BindingInfo, Bindings, SemanticModel},
    types::IntegerType,
};

use super::{
    AssignmentProjection, AssignmentTarget, BinaryOp, BindingId, Expr, ExprKind, FieldDefinition,
    FieldId, FieldValue, FieldValueOrigin, FunctionDefinition, FunctionId, NodeId, Parameter,
    Program, ReturnType, Statement, StatementKind, Type, TypeDefinition, TypeId, UnaryOp,
};

pub fn build(program: &ast::Program) -> Result<Program, Diagnostic> {
    let lowered = crate::generics::lower(program)?;
    let (program, uses) =
        crate::array_lengths::resolve(&lowered.program).map_err(|e| lowered.context(e))?;
    let program = crate::sums::lower(&program).map_err(|e| lowered.context(e))?;
    let model = semantic::analyze_lowered(&program).map_err(|e| lowered.context(e))?;
    lowered.validate_arguments(&model)?;
    let mut ir = build_with_model(&program, &model).map_err(|e| lowered.context(e))?;
    for (name, origin) in lowered.origins {
        ir.function_definitions
            .iter_mut()
            .find(|d| d.name == name)
            .unwrap()
            .generic_origin = Some(origin);
    }
    for usage in uses.into_iter().chain(lowered.array_length_uses) {
        let id = model.constants[&usage.name].0;
        ir.constant_definitions[id]
            .array_length_uses
            .push(usage.span);
    }
    super::aggregates::lower(&mut ir);
    super::ownership::lower(&mut ir);
    Ok(ir)
}

pub(crate) fn build_with_model(
    program: &ast::Program,
    model: &SemanticModel,
) -> Result<Program, Diagnostic> {
    let sources: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let ast::Item::ConstantDefinition(d) = item {
                Some(d)
            } else {
                None
            }
        })
        .collect();
    let mut builder = Builder {
        scopes: vec![HashMap::new()],
        next_binding_id: Cell::new(0),
        expression_bindings: RefCell::new(Vec::new()),
        next_node_id: Cell::new(0),
        current_return_type: None,
        model,
        constant_cache: RefCell::new(vec![None; sources.len()]),
        constant_stack: RefCell::new(Vec::new()),
        constant_sources: sources,
    };

    builder.prepare_constants()?;
    let constant_definitions = builder
        .constant_cache
        .borrow()
        .iter()
        .map(|value| value.as_ref().unwrap().0.clone())
        .collect();
    let type_definitions = model
        .type_definitions
        .iter()
        .map(|definition| builder.build_type_definition(definition))
        .collect::<Result<_, _>>()?;

    let function_definitions = program
        .items
        .iter()
        .filter_map(|item| match item {
            ast::Item::FunctionDefinition(function) => Some(builder.build_function(function)),
            ast::Item::TypeDefinition(_)
            | ast::Item::Statement(_)
            | ast::Item::ConstantDefinition(_) => None,
        })
        .collect::<Result<_, _>>()?;

    let statements = program
        .items
        .iter()
        .filter_map(|item| match item {
            ast::Item::TypeDefinition(_)
            | ast::Item::FunctionDefinition(_)
            | ast::Item::ConstantDefinition(_) => None,
            ast::Item::Statement(statement) => {
                Some(builder.build_statements(std::slice::from_ref(statement)))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let statements = statements.into_iter().flatten().collect();
    Ok(Program {
        string_heap_limit: super::DEFAULT_STRING_HEAP_LIMIT,
        array_heap_limit: super::DEFAULT_ARRAY_HEAP_LIMIT,
        constant_definitions,
        type_definitions,
        function_definitions,
        statements,
    })
}

#[derive(Debug, Clone)]
struct ResolvedBinding {
    id: BindingId,
    info: BindingInfo,
}

struct Builder<'a> {
    constant_sources: Vec<&'a ast::ConstantDefinition>,
    constant_cache: RefCell<Vec<Option<(super::ConstantDefinition, crate::vm::Value)>>>,
    constant_stack: RefCell<Vec<usize>>,
    scopes: Vec<HashMap<String, ResolvedBinding>>,
    next_binding_id: Cell<usize>,
    expression_bindings: RefCell<Vec<HashMap<String, ResolvedBinding>>>,
    next_node_id: Cell<usize>,
    current_return_type: Option<semantic::ReturnType>,
    model: &'a SemanticModel,
}

impl Builder<'_> {
    fn build_function(
        &mut self,
        function: &ast::FunctionDefinition,
    ) -> Result<FunctionDefinition, Diagnostic> {
        let semantic_id = self
            .model
            .resolve_function_name(&function.name, function.name_span)?;
        let definition = self.model.function_definition(semantic_id);
        let previous_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
        let previous_return_type = self
            .current_return_type
            .replace(definition.return_type.clone());

        let result = (|| {
            let mut parameters = Vec::new();
            for parameter in &definition.parameters {
                let id = self.allocate_binding_id();
                self.scopes[0].insert(
                    parameter.name.clone(),
                    ResolvedBinding {
                        id,
                        info: BindingInfo {
                            ty: parameter.ty.clone(),
                            mutable: false,
                        },
                    },
                );
                parameters.push(Parameter {
                    id,
                    name: parameter.name.clone(),
                    ty: ir_type(parameter.ty.clone()),
                    span: parameter.span,
                });
            }
            Ok(FunctionDefinition {
                lowering: None,
                generic_origin: None,
                id: FunctionId(definition.id.0),
                name: definition.name.clone(),
                parameters,
                return_type: match &definition.return_type {
                    semantic::ReturnType::Void => ReturnType::Void,
                    semantic::ReturnType::Value(ty) => ReturnType::Value(ir_type(ty.clone())),
                },
                body: self.build_statements(&function.body)?,
                span: function.span,
            })
        })();

        self.scopes = previous_scopes;
        self.current_return_type = previous_return_type;
        result
    }

    fn build_type_definition(
        &self,
        definition: &semantic::TypeDefinition,
    ) -> Result<TypeDefinition, Diagnostic> {
        let bindings = HashMap::new();
        let fields = definition
            .fields
            .iter()
            .map(|field| {
                Ok(FieldDefinition {
                    id: FieldId(field.id.0),
                    name: field.name.clone(),
                    ty: ir_type(field.ty.clone()),
                    default: field
                        .default
                        .as_ref()
                        .map(|value| self.build_expr(value, Some(field.ty.clone()), &bindings))
                        .transpose()?,
                    span: field.span,
                })
            })
            .collect::<Result<_, Diagnostic>>()?;

        Ok(TypeDefinition {
            variants: definition.variants.clone(),
            id: TypeId(definition.id.0),
            name: definition.name.clone(),
            fields,
            span: definition.span,
        })
    }

    fn build_statements(&mut self, statements: &[ast::Stmt]) -> Result<Vec<Statement>, Diagnostic> {
        let mut result = Vec::new();
        for statement in statements {
            if let ast::StmtKind::Block(body) = &statement.kind {
                result.extend(self.with_scope(|builder| builder.build_statements(body))?);
            } else if matches!(statement.kind, ast::StmtKind::ForEach { .. }) {
                let expanded = crate::iteration::expand(statement);
                result.extend(self.with_scope(|builder| builder.build_statements(&expanded))?);
            } else {
                result.push(self.build_statement(statement)?);
            }
        }
        Ok(result)
    }

    fn build_statement(&mut self, statement: &ast::Stmt) -> Result<Statement, Diagnostic> {
        let id = self.allocate_node_id();
        let bindings = self.visible_bindings();

        let kind = match &statement.kind {
            ast::StmtKind::Block(_)
            | ast::StmtKind::Match { .. }
            | ast::StmtKind::ForEach { .. } => {
                unreachable!("blocks and matches are expanded before building a single statement")
            }
            ast::StmtKind::Binding {
                mutable,
                name,
                type_spec,
                value,
            } => {
                let ty = match type_spec {
                    ast::TypeSpec::Explicit(ty) => self.model.resolve_type_ref(ty)?,
                    ast::TypeSpec::Infer => self.model.type_of_expr(value, &bindings)?,
                };
                let value = self.build_expr(value, Some(ty.clone()), &bindings)?;
                let id = self.allocate_binding_id();

                self.scopes
                    .last_mut()
                    .expect("current scope must exist")
                    .insert(
                        name.clone(),
                        ResolvedBinding {
                            id,
                            info: BindingInfo {
                                ty: ty.clone(),
                                mutable: *mutable,
                            },
                        },
                    );

                StatementKind::Binding {
                    borrowed: false,
                    id,
                    mutable: *mutable,
                    name: name.clone(),
                    ty: ir_type(ty),
                    value,
                }
            }
            ast::StmtKind::Assignment { target, value } => {
                let binding = self.resolve(&target.name).ok_or_else(|| {
                    Diagnostic::without_span(format!("missing resolved binding `{}`", target.name))
                })?;

                let root_ty = binding.info.ty.clone();
                let mut target_ty = root_ty.clone();
                let mut projections = Vec::with_capacity(target.projections.len());
                for projection in &target.projections {
                    let ast::AssignmentProjection::Index { index, span } = projection;
                    let (element, length) = match target_ty {
                        semantic::Type::Array { element, length } => (element, Some(length)),
                        semantic::Type::DynamicArray { element } => (element, None),
                        _ => unreachable!("semantic analysis requires an array assignment target"),
                    };
                    let element_ty = *element;
                    let index = self.build_expr(
                        index,
                        Some(semantic::Type::Integer(IntegerType::I64)),
                        &bindings,
                    )?;
                    let element = ir_type(element_ty.clone());
                    projections.push(match length {
                        Some(length) => AssignmentProjection::Index {
                            index,
                            element,
                            length,
                            span: *span,
                        },
                        None => AssignmentProjection::DynamicIndex {
                            index,
                            element,
                            span: *span,
                        },
                    });
                    target_ty = element_ty;
                }

                StatementKind::Assignment {
                    target: AssignmentTarget {
                        id: binding.id,
                        name: target.name.clone(),
                        root_ty: ir_type(root_ty),
                        projections,
                        ty: ir_type(target_ty.clone()),
                    },
                    value: self.build_expr(value, Some(target_ty), &bindings)?,
                }
            }
            ast::StmtKind::Flush => StatementKind::Flush,
            ast::StmtKind::Print { value } => {
                let ty = self.model.type_of_expr(value, &bindings)?;
                StatementKind::Print {
                    value: self.build_expr(value, Some(ty), &bindings)?,
                }
            }
            ast::StmtKind::Call { value } => {
                let ast::ExprKind::Call {
                    name,
                    name_span,
                    arguments,
                } = &value.kind
                else {
                    unreachable!("parser only creates call statements from calls")
                };
                let (function_id, arguments) =
                    self.build_call(name, *name_span, arguments, &bindings)?;
                StatementKind::Call {
                    function_id,
                    function_name: name.clone(),
                    arguments,
                }
            }
            ast::StmtKind::Return { value } => {
                let value = match (self.current_return_type.clone(), value) {
                    (Some(semantic::ReturnType::Value(ty)), Some(value)) => {
                        Some(self.build_expr(value, Some(ty), &bindings)?)
                    }
                    (_, None) => None,
                    _ => unreachable!("semantic analysis validates return statements"),
                };
                StatementKind::Return { value }
            }
            ast::StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition =
                    self.build_expr(condition, Some(semantic::Type::Bool), &bindings)?;
                let then_body = self.with_scope(|builder| builder.build_statements(then_body))?;
                let else_body = self.with_scope(|builder| builder.build_statements(else_body))?;

                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                }
            }
            ast::StmtKind::While { condition, body } => {
                let condition =
                    self.build_expr(condition, Some(semantic::Type::Bool), &bindings)?;
                let body = self.with_scope(|builder| builder.build_statements(body))?;

                StatementKind::While { condition, body }
            }
            ast::StmtKind::For {
                initializer,
                condition,
                update,
                body,
            } => self.with_scope(|builder| {
                let initializer = Box::new(builder.build_statement(initializer)?);
                let bindings = builder.visible_bindings();
                let condition =
                    builder.build_expr(condition, Some(semantic::Type::Bool), &bindings)?;
                let body = builder.with_scope(|builder| builder.build_statements(body))?;
                let update = Box::new(builder.build_statement(update)?);

                Ok(StatementKind::For {
                    initializer,
                    condition,
                    update,
                    body,
                })
            })?,
            ast::StmtKind::Break => StatementKind::Break,
            ast::StmtKind::Continue => StatementKind::Continue,
        };

        Ok(Statement {
            id,
            kind,
            span: statement.span,
        })
    }

    fn build_expr(
        &self,
        expr: &ast::Expr,
        expected: Option<semantic::Type>,
        bindings: &Bindings,
    ) -> Result<Expr, Diagnostic> {
        if !self.constant_stack.borrow().is_empty()
            && matches!(&expr.kind, ast::ExprKind::Call { name, .. } if !matches!(name.as_str(), "byte_len" | "array_len" | "concat"))
        {
            return Err(Diagnostic::new(
                "function calls are not allowed in constant expressions",
                expr.span,
            ));
        }
        if !self.constant_stack.borrow().is_empty()
            && matches!(&expr.kind, ast::ExprKind::GenericCall(call) if call.name == "array_repeat")
        {
            return Err(Diagnostic::new(
                "array_repeat is not allowed in constant expressions",
                expr.span,
            ));
        }
        let id = self.allocate_node_id();
        let ty = self.model.type_of_expr_expected(expr, bindings, expected)?;

        let kind = match &expr.kind {
            ast::ExprKind::Call {
                name, arguments, ..
            } if name == "$read_line" => ExprKind::ReadLine {
                max_bytes: Box::new(self.build_expr(
                    &arguments[0],
                    Some(semantic::Type::Integer(IntegerType::I64)),
                    bindings,
                )?),
            },
            ast::ExprKind::Call {
                name, arguments, ..
            } if name == "concat" => ExprKind::StringConcat {
                left: Box::new(self.build_expr(
                    &arguments[0],
                    Some(semantic::Type::String),
                    bindings,
                )?),
                right: Box::new(self.build_expr(
                    &arguments[1],
                    Some(semantic::Type::String),
                    bindings,
                )?),
            },
            ast::ExprKind::Call {
                name, arguments, ..
            } if matches!(name.as_str(), "array_copy" | "array_copy_range") => {
                ExprKind::ArrayCopy {
                    value: Box::new(self.build_expr(&arguments[0], None, bindings)?),
                    range: if name == "array_copy_range" {
                        Some((
                            Box::new(self.build_expr(
                                &arguments[1],
                                Some(semantic::Type::Integer(IntegerType::I64)),
                                bindings,
                            )?),
                            Box::new(self.build_expr(
                                &arguments[2],
                                Some(semantic::Type::Integer(IntegerType::I64)),
                                bindings,
                            )?),
                        ))
                    } else {
                        None
                    },
                }
            }
            ast::ExprKind::Call {
                name, arguments, ..
            } if name == "array_len" => ExprKind::ArrayLength {
                value: Box::new(self.build_expr(&arguments[0], None, bindings)?),
            },
            ast::ExprKind::Call {
                name, arguments, ..
            } if name == "byte_len" => ExprKind::StringByteLength {
                value: Box::new(self.build_expr(
                    &arguments[0],
                    Some(semantic::Type::String),
                    bindings,
                )?),
            },
            ast::ExprKind::Convert {
                value,
                syntax,
                mode,
                ..
            } => {
                let value = self.build_expr(value, None, bindings)?;
                if let (Type::Integer(from), semantic::Type::Integer(to)) = (&value.ty, &ty) {
                    ExprKind::ConvertInteger {
                        from: *from,
                        to: *to,
                        value: Box::new(value),
                        syntax: *syntax,
                    }
                } else {
                    ExprKind::ConvertNumeric {
                        mode: *mode,
                        from: numeric_type(&value.ty),
                        to: numeric_type(&ir_type(ty.clone())),
                        value: Box::new(value),
                        syntax: *syntax,
                    }
                }
            }
            ast::ExprKind::Match { .. } => unreachable!("match is expanded before IR"),
            ast::ExprKind::Let {
                name,
                type_ref,
                value,
                body,
            } => {
                let binding_ty = self.model.resolve_type_ref(type_ref)?;
                let value = self.build_expr(value, Some(binding_ty.clone()), bindings)?;
                let binding = self.allocate_binding_id();
                let info = BindingInfo {
                    ty: binding_ty,
                    mutable: false,
                };
                let mut locals = bindings.clone();
                locals.insert(name.clone(), info.clone());
                self.expression_bindings.borrow_mut().push(HashMap::from([(
                    name.clone(),
                    ResolvedBinding { id: binding, info },
                )]));
                let result = self.build_expr(body, Some(ty.clone()), &locals);
                self.expression_bindings.borrow_mut().pop();
                ExprKind::Let {
                    binding,
                    name: name.clone(),
                    value: Box::new(value),
                    body: Box::new(result?),
                }
            }
            ast::ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => ExprKind::Conditional {
                condition: Box::new(self.build_expr(
                    condition,
                    Some(semantic::Type::Bool),
                    bindings,
                )?),
                then_value: Box::new(self.build_expr(then_value, Some(ty.clone()), bindings)?),
                else_value: Box::new(self.build_expr(else_value, Some(ty.clone()), bindings)?),
            },
            ast::ExprKind::GenericCall(call) if call.name == "array_repeat" => {
                let semantic::Type::DynamicArray { element } = &ty else {
                    unreachable!()
                };
                ExprKind::ArrayRepeat {
                    value: Box::new(self.build_expr(
                        &call.arguments[0],
                        Some(*element.clone()),
                        bindings,
                    )?),
                    count: Box::new(self.build_expr(
                        &call.arguments[1],
                        Some(semantic::Type::Integer(IntegerType::I64)),
                        bindings,
                    )?),
                }
            }
            ast::ExprKind::GenericCall(_) => unreachable!("generic calls are lowered before IR"),
            ast::ExprKind::Boolean(value) => ExprKind::Boolean(*value),
            ast::ExprKind::String(value) => ExprKind::String(value.clone()),
            ast::ExprKind::Integer(literal) => {
                let semantic::Type::Integer(integer) = ty else {
                    unreachable!()
                };
                ExprKind::Integer(semantic::resolve_integer_literal(
                    literal, integer, false, expr.span,
                )?)
            }
            ast::ExprKind::Float { text, .. } => ExprKind::Float { text: text.clone() },
            ast::ExprKind::Variable(name) if self.model.constants.contains_key(name) => {
                let constant_id = self.model.constants[name].0;
                let value = self.build_constant(constant_id, expr.span)?;
                ExprKind::Constant {
                    id: constant_id,
                    value: Box::new(self.freeze(value, &ir_type(ty.clone()), expr.span)),
                }
            }
            ast::ExprKind::Variable(name) => {
                let binding = self.resolve(name).ok_or_else(|| {
                    Diagnostic::without_span(format!("missing resolved binding `{name}`"))
                })?;
                ExprKind::Variable {
                    id: binding.id,
                    name: name.clone(),
                }
            }
            ast::ExprKind::Construct {
                type_name,
                type_name_span,
                base,
                fields,
            } => {
                let type_id = self.model.resolve_type_name(type_name, *type_name_span)?;
                let definition = self.model.type_definition(type_id);
                let mut supplied = vec![false; definition.fields.len()];
                let mut values = Vec::with_capacity(definition.fields.len());
                let base = base
                    .as_ref()
                    .map(|base| self.build_expr(base, None, bindings).map(Box::new))
                    .transpose()?;

                // 明示値は、ソースに書かれた順番のまま評価順として保存します。
                for field_value in fields {
                    let field = definition
                        .fields
                        .iter()
                        .find(|field| field.name == field_value.name)
                        .expect("semantic analysis must resolve aggregate fields");
                    supplied[field.id.0] = true;
                    values.push(FieldValue {
                        id: FieldId(field.id.0),
                        name: field.name.clone(),
                        value: self.build_expr(
                            &field_value.value,
                            Some(field.ty.clone()),
                            bindings,
                        )?,
                        origin: if field_value.generated {
                            FieldValueOrigin::Generated {
                                span: field_value.span,
                            }
                        } else {
                            FieldValueOrigin::Explicit {
                                span: field_value.span,
                            }
                        },
                    });
                }

                // 省略された既定値は、その後に型定義順で評価します。
                let default_bindings = HashMap::new();
                for field in &definition.fields {
                    if base.is_some() || supplied[field.id.0] {
                        continue;
                    }
                    let default = field
                        .default
                        .as_ref()
                        .expect("semantic analysis requires every omitted field to have a default");
                    values.push(FieldValue {
                        id: FieldId(field.id.0),
                        name: field.name.clone(),
                        value: self.build_expr(
                            default,
                            Some(field.ty.clone()),
                            &default_bindings,
                        )?,
                        origin: FieldValueOrigin::Default {
                            definition_span: field.span,
                        },
                    });
                }

                ExprKind::Construct {
                    type_id: TypeId(type_id.0),
                    type_name: type_name.clone(),
                    base,
                    fields: values,
                }
            }
            ast::ExprKind::FieldAccess {
                base, field_name, ..
            } => {
                let semantic::Type::Named(type_id) = self.model.type_of_expr(base, bindings)?
                else {
                    unreachable!("semantic analysis must reject field access on scalar values")
                };
                let field = self
                    .model
                    .type_definition(type_id)
                    .fields
                    .iter()
                    .find(|field| field.name == *field_name)
                    .expect("semantic analysis must resolve field access");
                ExprKind::FieldAccess {
                    type_id: TypeId(type_id.0),
                    field_id: FieldId(field.id.0),
                    field_name: field.name.clone(),
                    base: Box::new(self.build_expr(base, None, bindings)?),
                }
            }
            ast::ExprKind::Array(values) => {
                let semantic::Type::Array { element, .. } = &ty else {
                    unreachable!("semantic analysis must assign an array type")
                };
                let expected_element = (**element).clone();
                ExprKind::Array(
                    values
                        .iter()
                        .map(|value| {
                            self.build_expr(value, Some(expected_element.clone()), bindings)
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            ast::ExprKind::Index { base, index } => ExprKind::Index {
                base: Box::new(self.build_expr(base, None, bindings)?),
                index: Box::new(self.build_expr(
                    index,
                    Some(semantic::Type::Integer(IntegerType::I64)),
                    bindings,
                )?),
            },
            ast::ExprKind::Call {
                name,
                name_span,
                arguments,
            } => {
                let (function_id, arguments) =
                    self.build_call(name, *name_span, arguments, bindings)?;
                ExprKind::Call {
                    function_id,
                    function_name: name.clone(),
                    arguments,
                }
            }
            ast::ExprKind::Unary { op, value } => {
                let is_minimum_literal = if *op == ast::UnaryOp::Negate {
                    if let ast::ExprKind::Integer(literal) = &value.kind {
                        let semantic::Type::Integer(integer) = ty else {
                            unreachable!()
                        };
                        semantic::resolve_integer_literal(literal, integer, true, expr.span)?
                            == integer.minimum()
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_minimum_literal {
                    let semantic::Type::Integer(integer) = ty else {
                        unreachable!()
                    };
                    ExprKind::Integer(integer.minimum())
                } else {
                    ExprKind::Unary {
                        op: (*op).into(),
                        value: Box::new(self.build_expr(value, Some(ty.clone()), bindings)?),
                    }
                }
            }
            ast::ExprKind::Logical { op, left, right } => ExprKind::Logical {
                op: match op {
                    ast::LogicalOp::And => super::LogicalOp::And,
                    ast::LogicalOp::Or => super::LogicalOp::Or,
                },
                left: Box::new(self.build_expr(left, Some(semantic::Type::Bool), bindings)?),
                right: Box::new(self.build_expr(right, Some(semantic::Type::Bool), bindings)?),
            },
            ast::ExprKind::Binary { op, left, right } => {
                let (left_expected, right_expected) = if is_comparison(*op) {
                    semantic::comparison_operand_types(left, right, bindings, self.model)?
                } else {
                    (ty.clone(), ty.clone())
                };

                ExprKind::Binary {
                    op: (*op).into(),
                    left: Box::new(self.build_expr(left, Some(left_expected), bindings)?),
                    right: Box::new(self.build_expr(right, Some(right_expected), bindings)?),
                }
            }
        };

        Ok(Expr {
            id,
            ty: ir_type(ty),
            kind,
            span: expr.span,
        })
    }

    fn allocate_node_id(&self) -> NodeId {
        let id = self.next_node_id.get();
        self.next_node_id.set(id + 1);
        NodeId(id)
    }

    fn allocate_binding_id(&self) -> BindingId {
        let id = self.next_binding_id.get();
        self.next_binding_id.set(id + 1);
        BindingId(id)
    }
    fn resolve(&self, name: &str) -> Option<ResolvedBinding> {
        if let Some(value) = self
            .expression_bindings
            .borrow()
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
        {
            return Some(value);
        }
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn build_call(
        &self,
        name: &str,
        name_span: crate::source::Span,
        arguments: &[ast::Expr],
        bindings: &Bindings,
    ) -> Result<(FunctionId, Vec<Expr>), Diagnostic> {
        let semantic_id = self.model.resolve_function_name(name, name_span)?;
        let function = self.model.function_definition(semantic_id);
        let arguments = arguments
            .iter()
            .zip(&function.parameters)
            .map(|(argument, parameter)| {
                self.build_expr(argument, Some(parameter.ty.clone()), bindings)
            })
            .collect::<Result<_, _>>()?;
        Ok((FunctionId(semantic_id.0), arguments))
    }

    fn visible_bindings(&self) -> Bindings {
        let mut bindings = HashMap::new();
        for scope in &self.scopes {
            bindings.extend(
                scope
                    .iter()
                    .map(|(name, binding)| (name.clone(), binding.info.clone())),
            );
        }
        bindings
    }

    fn with_scope<T>(
        &mut self,
        build: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        self.scopes.push(HashMap::new());
        let result = build(self);
        self.scopes.pop();
        result
    }
}

fn ir_type(value: semantic::Type) -> Type {
    match value {
        semantic::Type::Bool => Type::Bool,
        semantic::Type::String => Type::String,
        semantic::Type::Integer(integer) => Type::Integer(integer),
        semantic::Type::F32 => Type::F32,
        semantic::Type::F64 => Type::F64,
        semantic::Type::Named(id) => Type::Named(TypeId(id.0)),
        semantic::Type::DynamicArray { element } => Type::DynamicArray {
            element: Box::new(ir_type(*element)),
        },
        semantic::Type::Array { element, length } => Type::Array {
            element: Box::new(ir_type(*element)),
            length,
        },
    }
}

impl From<ast::UnaryOp> for UnaryOp {
    fn from(value: ast::UnaryOp) -> Self {
        match value {
            ast::UnaryOp::Negate => Self::Negate,
            ast::UnaryOp::Not => Self::Not,
            ast::UnaryOp::BitNot => Self::BitNot,
        }
    }
}

impl From<ast::BinaryOp> for BinaryOp {
    fn from(value: ast::BinaryOp) -> Self {
        match value {
            ast::BinaryOp::Add => Self::Add,
            ast::BinaryOp::Subtract => Self::Subtract,
            ast::BinaryOp::Multiply => Self::Multiply,
            ast::BinaryOp::Divide => Self::Divide,
            ast::BinaryOp::Remainder => Self::Remainder,
            ast::BinaryOp::BitAnd => Self::BitAnd,
            ast::BinaryOp::BitOr => Self::BitOr,
            ast::BinaryOp::BitXor => Self::BitXor,
            ast::BinaryOp::ShiftLeft => Self::ShiftLeft,
            ast::BinaryOp::ShiftRight => Self::ShiftRight,
            ast::BinaryOp::Equal => Self::Equal,
            ast::BinaryOp::NotEqual => Self::NotEqual,
            ast::BinaryOp::Less => Self::Less,
            ast::BinaryOp::LessEqual => Self::LessEqual,
            ast::BinaryOp::Greater => Self::Greater,
            ast::BinaryOp::GreaterEqual => Self::GreaterEqual,
        }
    }
}

fn numeric_type(ty: &Type) -> crate::types::NumericType {
    use crate::types::NumericType;
    match ty {
        Type::Integer(ty) => NumericType::Integer(*ty),
        Type::F32 => NumericType::F32,
        Type::F64 => NumericType::F64,
        _ => unreachable!("semantic analysis checked numeric conversion types"),
    }
}

const fn is_comparison(op: ast::BinaryOp) -> bool {
    matches!(
        op,
        ast::BinaryOp::Equal
            | ast::BinaryOp::NotEqual
            | ast::BinaryOp::Less
            | ast::BinaryOp::LessEqual
            | ast::BinaryOp::Greater
            | ast::BinaryOp::GreaterEqual
    )
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        BinaryOp as AstBinaryOp, Expr as AstExpr, ExprKind as AstExprKind, Item as AstItem,
        Program as AstProgram, Stmt, StmtKind as AstStmtKind, TypeRef, TypeRefKind, TypeSpec,
    };
    use crate::source::Span;
    use crate::{lexer::lex, parser::parse};

    use super::*;

    fn ast_expr(kind: AstExprKind) -> AstExpr {
        AstExpr {
            kind,
            span: Span::empty(0),
        }
    }

    fn ast_stmt(kind: AstStmtKind) -> Stmt {
        Stmt {
            kind,
            span: Span::empty(0),
        }
    }

    fn ast_item(kind: AstStmtKind) -> AstItem {
        AstItem::Statement(ast_stmt(kind))
    }

    fn explicit_type(name: &str) -> TypeSpec {
        TypeSpec::Explicit(TypeRef {
            kind: TypeRefKind::Named(name.into()),
            span: Span::empty(0),
        })
    }

    #[test]
    fn resolves_contextual_f32_literals() {
        let ast = AstProgram {
            items: vec![ast_item(AstStmtKind::Binding {
                name: "x".into(),
                mutable: false,
                type_spec: explicit_type("f32"),
                value: ast_expr(AstExprKind::Binary {
                    op: AstBinaryOp::Add,
                    left: Box::new(ast_expr(AstExprKind::Float {
                        text: "0.1".into(),
                        explicit_type: None,
                    })),
                    right: Box::new(ast_expr(AstExprKind::Float {
                        text: "0.2".into(),
                        explicit_type: None,
                    })),
                }),
            })],
        };

        let ir = build(&ast).unwrap();
        let StatementKind::Binding { ty, value, .. } = &ir.statements[0].kind else {
            panic!("expected binding")
        };
        assert_eq!(*ty, Type::F32);
        assert_eq!(value.ty, Type::F32);

        let ExprKind::Binary { left, right, .. } = &value.kind else {
            panic!("expected binary")
        };
        assert_eq!(left.ty, Type::F32);
        assert_eq!(right.ty, Type::F32);
    }

    #[test]
    fn infer_defaults_unsuffixed_float_to_f64() {
        let ast = AstProgram {
            items: vec![ast_item(AstStmtKind::Binding {
                name: "x".into(),
                mutable: false,
                type_spec: TypeSpec::Infer,
                value: ast_expr(AstExprKind::Float {
                    text: "0.1".into(),
                    explicit_type: None,
                }),
            })],
        };

        let ir = build(&ast).unwrap();
        let StatementKind::Binding { ty, value, .. } = &ir.statements[0].kind else {
            panic!("expected binding")
        };
        assert_eq!(*ty, Type::F64);
        assert_eq!(value.ty, Type::F64);
    }

    #[test]
    fn resolves_the_minimum_i64_after_parsing_its_sign() {
        let ast = parse(lex("x: i64 = -9223372036854775808;").unwrap()).unwrap();

        let ir = build(&ast).unwrap();
        let StatementKind::Binding { value, .. } = &ir.statements[0].kind else {
            panic!("expected binding")
        };

        assert_eq!(value.kind, ExprKind::Integer(i64::MIN as i128));
        assert_eq!(value.span, Span::new(9, 29));
    }

    #[test]
    fn preserves_expression_spans() {
        let ast = AstProgram {
            items: vec![ast_item(AstStmtKind::Print {
                value: AstExpr {
                    kind: AstExprKind::Binary {
                        op: AstBinaryOp::Add,
                        left: Box::new(AstExpr {
                            kind: AstExprKind::Integer(ast::IntegerLiteral::decimal("1")),
                            span: Span::new(0, 1),
                        }),
                        right: Box::new(AstExpr {
                            kind: AstExprKind::Integer(ast::IntegerLiteral::decimal("2")),
                            span: Span::new(4, 5),
                        }),
                    },
                    span: Span::new(0, 5),
                },
            })],
        };

        let ir = build(&ast).unwrap();
        let StatementKind::Print { value } = &ir.statements[0].kind else {
            panic!("expected print");
        };

        assert_eq!(value.span, Span::new(0, 5));

        let ExprKind::Binary { left, right, .. } = &value.kind else {
            panic!("expected binary expression");
        };

        assert_eq!(left.span, Span::new(0, 1));
        assert_eq!(right.span, Span::new(4, 5));
    }

    #[test]
    fn preserves_statement_spans() {
        let statement_span = Span::new(0, 9);
        let ast = AstProgram {
            items: vec![AstItem::Statement(Stmt {
                kind: AstStmtKind::Print {
                    value: ast_expr(AstExprKind::Integer(ast::IntegerLiteral::decimal("1"))),
                },
                span: statement_span,
            })],
        };

        let ir = build(&ast).unwrap();

        assert_eq!(ir.statements[0].span, statement_span);
    }

    #[test]
    fn assigns_distinct_deterministic_node_ids() {
        let ast = AstProgram {
            items: vec![ast_item(AstStmtKind::Print {
                value: ast_expr(AstExprKind::Binary {
                    op: AstBinaryOp::Add,
                    left: Box::new(ast_expr(AstExprKind::Integer(
                        ast::IntegerLiteral::decimal("1"),
                    ))),
                    right: Box::new(ast_expr(AstExprKind::Integer(
                        ast::IntegerLiteral::decimal("2"),
                    ))),
                }),
            })],
        };

        let first = build(&ast).unwrap();
        let second = build(&ast).unwrap();
        assert_eq!(first, second);

        let statement = &first.statements[0];
        assert_eq!(statement.id, NodeId(0));
        let StatementKind::Print { value } = &statement.kind else {
            panic!("expected print");
        };
        assert_eq!(value.id, NodeId(1));

        let ExprKind::Binary { left, right, .. } = &value.kind else {
            panic!("expected binary expression");
        };
        assert_eq!(left.span, right.span);
        assert_eq!(left.id, NodeId(2));
        assert_eq!(right.id, NodeId(3));
    }
}
