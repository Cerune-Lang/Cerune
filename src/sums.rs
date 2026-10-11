//! 直和型を、タグと独立したペイロード領域を持つ共通の値へ展開します。
use crate::{ast::*, diagnostic::Diagnostic, source::Span};
use std::collections::{HashMap, HashSet};

pub(crate) fn lower(program: &Program) -> Result<Program, Diagnostic> {
    let mut output = program.clone();
    let mut types = HashMap::new();
    for item in &mut output.items {
        if let Item::TypeDefinition(d) = item {
            if let Some(variants) = &d.variants {
                if variants.is_empty() {
                    return Err(Diagnostic::new(
                        "an enum requires at least one variant",
                        d.span,
                    ));
                }
                let mut names = HashSet::new();
                d.fields = vec![FieldDefinition {
                    name: "$tag".into(),
                    name_span: d.name_span,
                    type_ref: named("i64", d.name_span),
                    default: None,
                    span: d.name_span,
                }];
                for variant in variants {
                    if !names.insert(&variant.name) {
                        return Err(Diagnostic::new(
                            format!("duplicate variant {}", variant.name),
                            variant.name_span,
                        ));
                    }
                    let mut fields = HashSet::new();
                    for field in &variant.fields {
                        if !fields.insert(&field.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate payload field {}", field.name),
                                field.name_span,
                            ));
                        }
                        if field.default.is_some() {
                            return Err(Diagnostic::new(
                                "enum payload fields cannot have defaults",
                                field.span,
                            ));
                        }
                        let mut storage = field.clone();
                        storage.name = payload(&variant.name, &field.name);
                        d.fields.push(storage);
                    }
                }
            }
            if types.insert(d.name.clone(), d.clone()).is_some() {
                return Err(Diagnostic::new(
                    format!("duplicate type `{}`", d.name),
                    d.name_span,
                ));
            }
        }
    }
    let mut lowerer = Lowerer {
        types,
        next_match: 0,
    };
    for item in &mut output.items {
        match item {
            Item::TypeDefinition(d) => {
                for field in &mut d.fields {
                    if let Some(value) = &mut field.default {
                        lowerer.expr(value)?;
                    }
                }
            }
            Item::ConstantDefinition(d) => lowerer.expr(&mut d.value)?,
            Item::FunctionDefinition(d) => lowerer.statements(&mut d.body)?,
            Item::Statement(s) => lowerer.statement(s)?,
        }
    }
    Ok(output)
}

fn named(name: &str, span: Span) -> TypeRef {
    TypeRef {
        kind: TypeRefKind::Named(name.into()),
        span,
    }
}
fn payload(variant: &str, field: &str) -> String {
    format!("${variant}${field}")
}
fn integer(value: usize, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Integer(IntegerLiteral::decimal(value.to_string())),
        span,
    }
}
fn access(name: &str, field: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::FieldAccess {
            base: Box::new(Expr {
                kind: ExprKind::Variable(name.into()),
                span,
            }),
            field_name: field.into(),
            field_name_span: span,
        },
        span,
    }
}
fn field_value(name: String, value: Expr, span: Span) -> FieldValue {
    FieldValue {
        generated: true,
        name,
        value,
        name_span: span,
        span,
    }
}
struct Lowerer {
    types: HashMap<String, TypeDefinition>,
    next_match: usize,
}
impl Lowerer {
    fn variant(&self, path: &str, span: Span) -> Result<(TypeDefinition, usize), Diagnostic> {
        let (name, variant) = path
            .rsplit_once("::")
            .ok_or_else(|| Diagnostic::new("a match arm requires Enum::Variant", span))?;
        let definition = self
            .types
            .get(name)
            .ok_or_else(|| Diagnostic::new(format!("unknown enum {name}"), span))?;
        let variants = definition
            .variants
            .as_ref()
            .ok_or_else(|| Diagnostic::new(format!("{name} is not an enum"), span))?;
        let index = variants
            .iter()
            .position(|v| v.name == variant)
            .ok_or_else(|| Diagnostic::new(format!("unknown variant {path}"), span))?;
        Ok((definition.clone(), index))
    }
    fn statements(&mut self, body: &mut [Stmt]) -> Result<(), Diagnostic> {
        for s in body {
            self.statement(s)?;
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Match { value, arms } => {
                self.expr(value)?;
                let first = arms.first().ok_or_else(|| {
                    Diagnostic::new("match requires every enum variant", statement.span)
                })?;
                let (definition, _) = self.variant(&first.variant, first.variant_span)?;
                let variants = definition.variants.as_ref().unwrap();
                let capture = format!("$match{}", self.next_match);
                self.next_match += 1;
                let mut seen = HashSet::new();
                let mut branches = Vec::new();
                for arm in arms {
                    let (arm_type, index) = self.variant(&arm.variant, arm.variant_span)?;
                    if arm_type.name != definition.name {
                        return Err(Diagnostic::new(
                            "all match arms must use the same enum",
                            arm.variant_span,
                        ));
                    }
                    if seen.contains(&index) {
                        return Err(Diagnostic::new(
                            format!("duplicate match arm {}", arm.variant),
                            arm.variant_span,
                        ));
                    }
                    if arm.guard.is_none() {
                        seen.insert(index);
                    }
                    let variant = &variants[index];
                    let mut fields = HashSet::new();
                    let mut bindings = HashSet::new();
                    let mut body = Vec::new();
                    for field in &arm.fields {
                        let declared = variant
                            .fields
                            .iter()
                            .find(|f| f.name == field.name)
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    format!("unknown payload field {}", field.name),
                                    field.name_span,
                                )
                            })?;
                        if !fields.insert(&field.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate pattern field {}", field.name),
                                field.name_span,
                            ));
                        }
                        if field.binding != "_" {
                            if !bindings.insert(&field.binding) {
                                return Err(Diagnostic::new(
                                    format!("duplicate pattern binding {}", field.binding),
                                    field.binding_span,
                                ));
                            }
                            body.push(Stmt {
                                kind: StmtKind::Binding {
                                    mutable: false,
                                    name: field.binding.clone(),
                                    type_spec: TypeSpec::Explicit(declared.type_ref.clone()),
                                    value: access(
                                        &capture,
                                        &payload(&variant.name, &field.name),
                                        field.binding_span,
                                    ),
                                },
                                span: field.binding_span,
                            });
                        }
                    }
                    if let Some(field) = variant.fields.iter().find(|f| !fields.contains(&f.name)) {
                        return Err(Diagnostic::new(
                            format!(
                                "missing pattern field {}; use _ to discard its value",
                                field.name
                            ),
                            arm.variant_span,
                        ));
                    }
                    let guard = if let Some(guard) = &mut arm.guard {
                        self.expr(guard)?;
                        Some(bind_expression(&body, guard.clone()))
                    } else {
                        None
                    };
                    self.statements(&mut arm.body)?;
                    body.extend(arm.body.clone());
                    branches.push((index, body, guard, arm.variant_span));
                }
                if seen.len() != variants.len() {
                    let missing = variants
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| !seen.contains(i))
                        .map(|(_, v)| v.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(Diagnostic::new(
                        format!("non-exhaustive match; missing variants: {missing}"),
                        statement.span,
                    ));
                }
                let (_, last, _, span) = branches.pop().unwrap();
                let mut tail = vec![Stmt {
                    kind: StmtKind::Block(last),
                    span,
                }];
                for (index, body, guard, span) in branches.into_iter().rev() {
                    let condition = Expr {
                        kind: ExprKind::Binary {
                            op: BinaryOp::Equal,
                            left: Box::new(access(&capture, "$tag", span)),
                            right: Box::new(integer(index, span)),
                        },
                        span,
                    };
                    let condition = if let Some(guard) = guard {
                        Expr {
                            kind: ExprKind::Logical {
                                op: LogicalOp::And,
                                left: Box::new(condition),
                                right: Box::new(guard),
                            },
                            span,
                        }
                    } else {
                        condition
                    };
                    tail = vec![Stmt {
                        kind: StmtKind::If {
                            condition,
                            then_body: body,
                            else_body: tail,
                        },
                        span,
                    }];
                }
                let mut body = vec![Stmt {
                    kind: StmtKind::Binding {
                        mutable: false,
                        name: capture,
                        type_spec: TypeSpec::Explicit(named(&definition.name, value.span)),
                        value: value.clone(),
                    },
                    span: value.span,
                }];
                body.extend(tail);
                statement.kind = StmtKind::Block(body);
            }
            StmtKind::Block(body) => self.statements(body)?,
            StmtKind::Binding { value, .. }
            | StmtKind::Print { value }
            | StmtKind::Call { value } => self.expr(value)?,
            StmtKind::Assignment { target, value } => {
                for AssignmentProjection::Index { index, .. } in &mut target.projections {
                    self.expr(index)?;
                }
                self.expr(value)?;
            }
            StmtKind::Return { value } => {
                if let Some(value) = value {
                    self.expr(value)?;
                }
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.expr(condition)?;
                self.statements(then_body)?;
                self.statements(else_body)?;
            }
            StmtKind::ForEach { value, body, .. } => {
                self.expr(value)?;
                self.statements(body)?;
            }
            StmtKind::While { condition, body } => {
                self.expr(condition)?;
                self.statements(body)?;
            }
            StmtKind::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.statement(initializer)?;
                self.expr(condition)?;
                self.statement(update)?;
                self.statements(body)?;
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
        Ok(())
    }
    fn expr(&mut self, expr: &mut Expr) -> Result<(), Diagnostic> {
        match &mut expr.kind {
            ExprKind::Match { value, arms } => {
                let mut lowered_arms = arms.clone();
                for arm in &mut lowered_arms {
                    let result = *arm.result.take().expect("match expression arm");
                    arm.body = vec![Stmt {
                        span: result.span,
                        kind: StmtKind::Return {
                            value: Some(result),
                        },
                    }];
                }
                let mut statement = Stmt {
                    kind: StmtKind::Match {
                        value: *value.clone(),
                        arms: lowered_arms,
                    },
                    span: expr.span,
                };
                self.statement(&mut statement)?;
                let StmtKind::Block(body) = statement.kind else {
                    unreachable!()
                };
                let span = expr.span;
                *expr = match_expression(&body);
                expr.span = span;
            }

            ExprKind::Construct {
                type_name,
                type_name_span,
                base,
                fields,
            } => {
                if self
                    .types
                    .get(type_name)
                    .is_some_and(|d| d.variants.is_some())
                {
                    return Err(Diagnostic::new(
                        "enum values require an Enum::Variant constructor",
                        *type_name_span,
                    ));
                }
                if type_name.rsplit_once("::").is_some_and(|(name, _)| {
                    self.types.get(name).is_some_and(|d| d.variants.is_some())
                }) {
                    let (definition, index) = self.variant(type_name, *type_name_span)?;
                    let variants = definition.variants.as_ref().unwrap();
                    let variant = &variants[index];
                    if base.is_some() {
                        return Err(Diagnostic::new(
                            "enum constructors do not support product update",
                            expr.span,
                        ));
                    }
                    let mut seen = HashSet::new();
                    let mut values = vec![field_value(
                        "$tag".into(),
                        integer(index, *type_name_span),
                        *type_name_span,
                    )];
                    for field in fields.iter_mut() {
                        if !variant.fields.iter().any(|f| f.name == field.name) {
                            return Err(Diagnostic::new(
                                format!("unknown payload field {}", field.name),
                                field.name_span,
                            ));
                        }
                        if !seen.insert(field.name.clone()) {
                            return Err(Diagnostic::new(
                                format!("duplicate payload field {}", field.name),
                                field.name_span,
                            ));
                        }
                        self.expr(&mut field.value)?;
                        let mut storage = field.clone();
                        storage.name = payload(&variant.name, &field.name);
                        values.push(storage);
                    }
                    if let Some(field) = variant.fields.iter().find(|f| !seen.contains(&f.name)) {
                        return Err(Diagnostic::new(
                            format!("missing payload field {}", field.name),
                            expr.span,
                        ));
                    }
                    // 非選択領域は、既定値やユーザーコードを実行せず初期化します。
                    let mut budget = 100_000;
                    for (other, v) in variants.iter().enumerate() {
                        if other != index {
                            for field in &v.fields {
                                let value = self.zero(
                                    &field.type_ref,
                                    expr.span,
                                    &mut Vec::new(),
                                    &mut budget,
                                )?;
                                values.push(field_value(
                                    payload(&v.name, &field.name),
                                    value,
                                    expr.span,
                                ));
                            }
                        }
                    }
                    *type_name = definition.name;
                    *fields = values;
                } else {
                    if base.is_none() && fields.is_empty() {
                        return Err(Diagnostic::new(
                            "a product constructor requires at least one field",
                            expr.span,
                        ));
                    }
                    if let Some(base) = base {
                        self.expr(base)?;
                    }
                    for field in fields {
                        self.expr(&mut field.value)?;
                    }
                }
            }
            ExprKind::GenericCall(call) if call.name == "array_repeat" => {
                for argument in &mut call.arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Call { arguments, .. } | ExprKind::Array(arguments) => {
                for argument in arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Convert { value, .. }
            | ExprKind::Unary { value, .. }
            | ExprKind::FieldAccess { base: value, .. } => self.expr(value)?,
            ExprKind::Binary { left, right, .. }
            | ExprKind::Logical { left, right, .. }
            | ExprKind::Index {
                base: left,
                index: right,
            } => {
                self.expr(left)?;
                self.expr(right)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn zero(
        &self,
        ty: &TypeRef,
        span: Span,
        path: &mut Vec<String>,
        budget: &mut usize,
    ) -> Result<Expr, Diagnostic> {
        if *budget == 0 || path.len() >= 128 {
            return Err(Diagnostic::new(
                "enum inactive storage exceeds the expansion limit",
                span,
            ));
        }
        *budget -= 1;
        let kind = match &ty.kind {
            TypeRefKind::DynamicArray { element } => {
                let seed = self.zero(element, span, path, budget)?;
                let zero = Expr {
                    kind: ExprKind::Integer(IntegerLiteral::with_type(
                        "0",
                        crate::types::IntegerType::I64,
                    )),
                    span,
                };
                ExprKind::Call {
                    name: "array_copy_range".into(),
                    name_span: span,
                    arguments: vec![
                        Expr {
                            kind: ExprKind::Array(vec![seed]),
                            span,
                        },
                        zero.clone(),
                        zero,
                    ],
                }
            }
            TypeRefKind::ArrayConstant { constant, .. } => {
                return Err(Diagnostic::new("unresolved array length", constant.span));
            }
            TypeRefKind::Array { element, length } => {
                if *length > *budget {
                    return Err(Diagnostic::new(
                        "enum inactive storage exceeds the expansion limit",
                        span,
                    ));
                }
                let mut values = Vec::new();
                for _ in 0..*length {
                    values.push(self.zero(element, span, path, budget)?);
                }
                ExprKind::Array(values)
            }
            TypeRefKind::Named(name) => match Type::from_name(name) {
                Some(Type::Bool) => ExprKind::Boolean(false),
                Some(Type::String) => ExprKind::String(String::new()),
                Some(Type::Integer(ty)) => ExprKind::Integer(IntegerLiteral::with_type("0", ty)),
                Some(ty @ (Type::F32 | Type::F64)) => ExprKind::Float {
                    text: "0.0".into(),
                    explicit_type: Some(ty),
                },
                None => {
                    if path.contains(name) {
                        return Err(Diagnostic::new("enum payload has infinite size", ty.span));
                    }
                    let definition = self
                        .types
                        .get(name)
                        .ok_or_else(|| Diagnostic::new(format!("unknown type {name}"), ty.span))?;
                    path.push(name.clone());
                    let mut fields = Vec::new();
                    for field in &definition.fields {
                        fields.push(field_value(
                            field.name.clone(),
                            self.zero(&field.type_ref, span, path, budget)?,
                            span,
                        ));
                    }
                    path.pop();
                    ExprKind::Construct {
                        type_name: name.clone(),
                        type_name_span: span,
                        base: None,
                        fields,
                    }
                }
            },
        };
        Ok(Expr { kind, span })
    }
}

fn bind_expression(bindings: &[Stmt], mut body: Expr) -> Expr {
    for binding in bindings.iter().rev() {
        let StmtKind::Binding {
            name,
            type_spec: TypeSpec::Explicit(type_ref),
            value,
            ..
        } = &binding.kind
        else {
            unreachable!()
        };
        let span = body.span;
        body = Expr {
            span,
            kind: ExprKind::Let {
                name: name.clone(),
                type_ref: type_ref.clone(),
                value: Box::new(value.clone()),
                body: Box::new(body),
            },
        };
    }
    body
}
fn match_expression(body: &[Stmt]) -> Expr {
    let first = &body[0];
    match &first.kind {
        StmtKind::Binding { .. } => {
            bind_expression(std::slice::from_ref(first), match_expression(&body[1..]))
        }
        StmtKind::Return { value: Some(value) } => value.clone(),
        StmtKind::Block(body) => match_expression(body),
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => Expr {
            span: first.span,
            kind: ExprKind::Conditional {
                condition: Box::new(condition.clone()),
                then_value: Box::new(match_expression(then_body)),
                else_value: Box::new(match_expression(else_body)),
            },
        },
        _ => unreachable!("match expression lowering creates only bindings, returns and branches"),
    }
}
