//! 配列型の定数長を、既存の定数評価器で確定してから通常の型検査へ渡します。
use crate::{ast::*, diagnostic::Diagnostic, source::Span};
use std::collections::HashMap;

pub(crate) struct LengthUse {
    pub length: usize,
    pub name: String,
    pub span: Span,
}

/// 元のASTは保持し、生成経路へ渡すコピーだけを数値の長さへ解決します。
pub(crate) fn resolve(program: &Program) -> Result<(Program, Vec<LengthUse>), Diagnostic> {
    let mut resolver = Resolver {
        source: program,
        local_names: Vec::new(),
        constants: HashMap::new(),
        types: HashMap::new(),
        lengths: HashMap::new(),
        stack: Vec::new(),
        uses: Vec::new(),
    };
    let mut output = program.clone();
    for item in &mut output.items {
        resolver.item(item)?;
    }
    Ok((output, resolver.uses))
}

pub(crate) fn resolve_length(
    program: &Program,
    name: &str,
    span: Span,
) -> Result<usize, Diagnostic> {
    resolver(program).length(name, span)
}
pub(crate) fn resolve_type(
    program: &Program,
    ty: &mut TypeRef,
) -> Result<Vec<LengthUse>, Diagnostic> {
    let mut resolver = resolver(program);
    resolver.ty(ty, false)?;
    Ok(resolver.uses)
}
fn resolver(program: &Program) -> Resolver<'_> {
    Resolver {
        source: program,
        local_names: Vec::new(),
        constants: HashMap::new(),
        types: HashMap::new(),
        lengths: HashMap::new(),
        stack: Vec::new(),
        uses: Vec::new(),
    }
}

struct Resolver<'a> {
    source: &'a Program,
    local_names: Vec<String>,
    constants: HashMap<String, ConstantDefinition>,
    types: HashMap<String, TypeDefinition>,
    lengths: HashMap<String, usize>,
    stack: Vec<String>,
    uses: Vec<LengthUse>,
}

impl Resolver<'_> {
    fn enter(&mut self, key: String, span: Span) -> Result<(), Diagnostic> {
        if self.stack.contains(&key) {
            return Err(Diagnostic::new(
                format!("cyclic array length dependency through {key}"),
                span,
            ));
        }
        if self.stack.len() >= 128 {
            return Err(Diagnostic::new(
                "array length dependency depth exceeds 128",
                span,
            ));
        }
        self.stack.push(key);
        Ok(())
    }

    fn length(&mut self, name: &str, span: Span) -> Result<usize, Diagnostic> {
        if let Some(value) = self.lengths.get(name) {
            return Ok(*value);
        }
        self.constant(name, span)?;
        // 完了済みの依存だけを宣言順に並べます。実行文・関数は評価器へ渡しません。
        let mut items = Vec::new();
        for item in &self.source.items {
            match item {
                Item::ConstantDefinition(d) => {
                    if let Some(d) = self.constants.get(&d.name) {
                        items.push(Item::ConstantDefinition(d.clone()));
                    }
                }
                Item::TypeDefinition(d) => {
                    if let Some(d) = self.types.get(&d.name) {
                        items.push(Item::TypeDefinition(d.clone()));
                    }
                }
                _ => {}
            }
        }
        let ir = crate::ir::builder::build(&Program { items })?;
        let definition = ir
            .constant_definitions
            .iter()
            .find(|d| d.name == name)
            .unwrap();
        let crate::ir::ExprKind::Integer(value) = definition.value.kind else {
            return Err(Diagnostic::new(
                "array length constant must have an integer type",
                span,
            ));
        };
        if value <= 0 {
            return Err(Diagnostic::new(
                "array length must be greater than zero",
                span,
            ));
        }
        if value > i64::MAX as i128 {
            return Err(Diagnostic::new("array length is too large", span));
        }
        let length = usize::try_from(value)
            .map_err(|_| Diagnostic::new("array length is too large", span))?;
        self.lengths.insert(name.to_owned(), length);
        Ok(length)
    }

    fn constant(&mut self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if self.constants.contains_key(name) {
            return Ok(());
        }
        let mut matches = self.source.items.iter().filter_map(|item| match item {
            Item::ConstantDefinition(d) if d.name == name => Some(d.clone()),
            _ => None,
        });
        let mut definition = matches.next().ok_or_else(|| {
            Diagnostic::new(
                format!("array length requires a constant: unknown constant {name}"),
                span,
            )
        })?;
        if matches.next().is_some() {
            return Err(Diagnostic::new(
                format!("duplicate constant {name}"),
                definition.name_span,
            ));
        }
        self.enter(format!("constant {name}"), span)?;
        self.ty(&mut definition.type_ref, true)?;
        let mut budget = 100_000;
        let local_names = std::mem::take(&mut self.local_names);
        let result = self.constant_expr(&mut definition.value, &mut budget, 0);
        self.local_names = local_names;
        result?;
        self.stack.pop();
        self.constants.insert(name.to_owned(), definition);
        Ok(())
    }

    fn named_type(&mut self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if Type::from_name(name).is_some() || self.types.contains_key(name) {
            return Ok(());
        }
        let mut matches = self.source.items.iter().filter_map(|item| match item {
            Item::TypeDefinition(d) if d.name == name => Some(d.clone()),
            _ => None,
        });
        let mut definition = matches
            .next()
            .ok_or_else(|| Diagnostic::new(format!("unknown type {name}"), span))?;
        if matches.next().is_some() {
            return Err(Diagnostic::new(
                format!("duplicate type {name}"),
                definition.name_span,
            ));
        }
        self.enter(format!("type {name}"), span)?;
        for field in definition.fields.iter_mut().chain(
            definition
                .variants
                .iter_mut()
                .flatten()
                .flat_map(|v| &mut v.fields),
        ) {
            self.ty(&mut field.type_ref, true)?;
            // 既定値は実際の構築で必要になったものだけを下で展開します。
            field.default = None;
        }
        self.stack.pop();
        self.types.insert(name.to_owned(), definition);
        Ok(())
    }

    fn ty(&mut self, ty: &mut TypeRef, dependencies: bool) -> Result<(), Diagnostic> {
        match &mut ty.kind {
            TypeRefKind::Named(name) if dependencies => self.named_type(name, ty.span)?,
            TypeRefKind::Array { element, .. } | TypeRefKind::DynamicArray { element } => {
                self.ty(element, dependencies)?
            }
            TypeRefKind::ArrayConstant { element, constant } => {
                self.ty(element, dependencies)?;
                let length = self.length(&constant.name, constant.span)?;
                if !dependencies {
                    self.uses.push(LengthUse {
                        length,
                        name: constant.name.clone(),
                        span: constant.span,
                    });
                }
                ty.kind = TypeRefKind::Array {
                    element: element.clone(),
                    length,
                };
            }
            _ => {}
        }
        Ok(())
    }

    /// 定数に実際に使う既定値を明示した評価専用AST。元の式とそのSpanは変えません。
    fn constant_expr(
        &mut self,
        expr: &mut Expr,
        budget: &mut usize,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if *budget == 0 || depth >= 128 {
            return Err(Diagnostic::new(
                "array length constant expansion exceeds 100000 nodes or depth 128",
                expr.span,
            ));
        }
        *budget -= 1;
        match &mut expr.kind {
            ExprKind::GenericCall(_) => {
                return Err(Diagnostic::new(
                    "function calls are not allowed in constant expressions",
                    expr.span,
                ));
            }
            ExprKind::Variable(name) => {
                if !self.local_names.contains(name) {
                    self.constant(name, expr.span)?;
                }
            }
            ExprKind::Match { value, arms } => {
                self.constant_expr(value, budget, depth + 1)?;
                for arm in arms {
                    if let Some((name, _)) = arm.variant.rsplit_once("::") {
                        self.ty(
                            &mut TypeRef {
                                kind: TypeRefKind::Named(name.into()),
                                span: arm.variant_span,
                            },
                            true,
                        )?;
                    }
                    let previous = self.local_names.len();
                    self.local_names
                        .extend(arm.fields.iter().map(|f| f.binding.clone()));
                    if let Some(guard) = &mut arm.guard {
                        self.constant_expr(guard, budget, depth + 1)?;
                    }
                    if let Some(result) = &mut arm.result {
                        self.constant_expr(result, budget, depth + 1)?;
                    }
                    self.local_names.truncate(previous);
                }
            }
            ExprKind::Let {
                name, value, body, ..
            } => {
                self.constant_expr(value, budget, depth + 1)?;
                self.local_names.push(name.clone());
                self.constant_expr(body, budget, depth + 1)?;
                self.local_names.pop();
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.constant_expr(condition, budget, depth + 1)?;
                self.constant_expr(then_value, budget, depth + 1)?;
                self.constant_expr(else_value, budget, depth + 1)?;
            }
            ExprKind::Call {
                name, arguments, ..
            } => {
                if !matches!(name.as_str(), "byte_len" | "array_len" | "concat") {
                    return Err(Diagnostic::new(
                        "function calls are not allowed in constant expressions",
                        expr.span,
                    ));
                }
                for arg in arguments {
                    self.constant_expr(arg, budget, depth + 1)?;
                }
            }
            ExprKind::Construct {
                type_name,
                type_name_span,
                base,
                fields,
            } => {
                let (root, variant) = self
                    .source
                    .items
                    .iter()
                    .find_map(|item| {
                        let Item::TypeDefinition(d) = item else {
                            return None;
                        };
                        if d.name == *type_name {
                            return Some((d.clone(), None));
                        }
                        let suffix = type_name.strip_prefix(&format!("{}::", d.name))?;
                        d.variants
                            .as_ref()?
                            .iter()
                            .find(|v| v.name == suffix)
                            .map(|_| (d.clone(), Some(suffix.to_owned())))
                    })
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!("unknown type or variant {type_name}"),
                            *type_name_span,
                        )
                    })?;
                self.named_type(&root.name, *type_name_span)?;
                if let Some(base) = base {
                    self.constant_expr(base, budget, depth + 1)?;
                } else if variant.is_none() {
                    for field in &root.fields {
                        if !fields.iter().any(|f| f.name == field.name)
                            && let Some(default) = &field.default
                        {
                            fields.push(FieldValue {
                                generated: false,
                                name: field.name.clone(),
                                name_span: field.name_span,
                                value: default.clone(),
                                span: field.span,
                            });
                        }
                    }
                }
                for field in fields {
                    self.constant_expr(&mut field.value, budget, depth + 1)?;
                }
            }
            ExprKind::Convert { target, value, .. } => {
                self.ty(target, true)?;
                self.constant_expr(value, budget, depth + 1)?;
            }
            ExprKind::Array(values) => {
                for value in values {
                    self.constant_expr(value, budget, depth + 1)?;
                }
            }
            ExprKind::Unary { value, .. } | ExprKind::FieldAccess { base: value, .. } => {
                self.constant_expr(value, budget, depth + 1)?
            }
            ExprKind::Binary { left, right, .. }
            | ExprKind::Logical { left, right, .. }
            | ExprKind::Index {
                base: left,
                index: right,
            } => {
                self.constant_expr(left, budget, depth + 1)?;
                self.constant_expr(right, budget, depth + 1)?;
            }
            ExprKind::Boolean(_)
            | ExprKind::String(_)
            | ExprKind::Integer(_)
            | ExprKind::Float { .. } => {}
        }
        Ok(())
    }

    fn item(&mut self, item: &mut Item) -> Result<(), Diagnostic> {
        match item {
            Item::ConstantDefinition(d) => {
                self.ty(&mut d.type_ref, false)?;
                self.expr(&mut d.value)?;
            }
            Item::TypeDefinition(d) => {
                for field in d
                    .fields
                    .iter_mut()
                    .chain(d.variants.iter_mut().flatten().flat_map(|v| &mut v.fields))
                {
                    self.ty(&mut field.type_ref, false)?;
                    if let Some(value) = &mut field.default {
                        self.expr(value)?;
                    }
                }
            }
            Item::FunctionDefinition(d) => {
                for p in &mut d.parameters {
                    self.ty(&mut p.type_ref, false)?;
                }
                if let ReturnTypeRef::Value(ty) = &mut d.return_type {
                    self.ty(ty, false)?;
                }
                self.statements(&mut d.body)?;
            }
            Item::Statement(s) => self.statement(s)?,
        }
        Ok(())
    }
    fn spec(&mut self, spec: &mut TypeSpec) -> Result<(), Diagnostic> {
        if let TypeSpec::Explicit(ty) = spec {
            self.ty(ty, false)?;
        }
        Ok(())
    }
    fn statements(&mut self, statements: &mut [Stmt]) -> Result<(), Diagnostic> {
        for s in statements {
            self.statement(s)?;
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(body) => self.statements(body)?,
            StmtKind::Binding {
                type_spec, value, ..
            } => {
                self.spec(type_spec)?;
                self.expr(value)?;
            }
            StmtKind::Assignment { target, value } => {
                for AssignmentProjection::Index { index, .. } in &mut target.projections {
                    self.expr(index)?;
                }
                self.expr(value)?;
            }
            StmtKind::Print { value } | StmtKind::Call { value } => self.expr(value)?,
            StmtKind::Return { value } => {
                if let Some(v) = value {
                    self.expr(v)?;
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
            StmtKind::ForEach {
                index,
                element,
                value,
                body,
                ..
            } => {
                for binding in index.iter_mut().chain(std::iter::once(element)) {
                    self.spec(&mut binding.type_spec)?;
                }
                self.expr(value)?;
                self.statements(body)?;
            }
            StmtKind::Match { value, arms } => {
                self.expr(value)?;
                for arm in arms {
                    if let Some(guard) = &mut arm.guard {
                        self.expr(guard)?;
                    }
                    if let Some(result) = &mut arm.result {
                        self.expr(result)?;
                    }
                    self.statements(&mut arm.body)?;
                }
            }
            StmtKind::Flush | StmtKind::Break | StmtKind::Continue => {}
        }
        Ok(())
    }
    fn expr(&mut self, expr: &mut Expr) -> Result<(), Diagnostic> {
        match &mut expr.kind {
            ExprKind::Match { value, arms } => {
                let mut statement = Stmt {
                    kind: StmtKind::Match {
                        value: *value.clone(),
                        arms: arms.clone(),
                    },
                    span: expr.span,
                };
                self.statement(&mut statement)?;
                let StmtKind::Match {
                    value: new_value,
                    arms: new_arms,
                } = statement.kind
                else {
                    unreachable!()
                };
                **value = new_value;
                *arms = new_arms;
            }

            ExprKind::Convert { target, value, .. } => {
                self.ty(target, false)?;
                self.expr(value)?;
            }
            ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    self.expr(base)?;
                }
                for f in fields {
                    self.expr(&mut f.value)?;
                }
            }
            ExprKind::GenericCall(call) if call.name == "array_repeat" => {
                for argument in &mut call.arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Call { arguments, .. } | ExprKind::Array(arguments) => {
                for arg in arguments {
                    self.expr(arg)?;
                }
            }
            ExprKind::Unary { value, .. } | ExprKind::FieldAccess { base: value, .. } => {
                self.expr(value)?
            }
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
}
