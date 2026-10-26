//! 明示された型・配列長を通常の関数へ具体化します。実行時の引数は並べ替えません。
use crate::{ast::*, diagnostic::Diagnostic, source::Span, types::IntegerType};
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) struct Lowered {
    pub program: Program,
    pub origins: Vec<(String, GenericOrigin)>,
    pub types: Vec<TypeRef>,
    pub array_length_uses: Vec<crate::array_lengths::LengthUse>,
}
impl Lowered {
    pub fn context(&self, error: Diagnostic) -> Diagnostic {
        let Some(span) = error.primary_span() else {
            return error;
        };
        let notes: Vec<_> = self
            .origins
            .iter()
            .filter(|(_, o)| {
                o.definition.source_id() == span.source_id()
                    && o.definition.start() <= span.start()
                    && span.end() <= o.definition.end()
            })
            .map(|(_, o)| {
                let call = o.calls[0].span;
                format!(
                    "{}::<{}> called at source={} bytes={}..{}",
                    o.template,
                    o.arguments.join(", "),
                    call.source_id().index(),
                    call.start(),
                    call.end()
                )
            })
            .collect();
        if notes.is_empty() {
            error
        } else {
            Diagnostic::new(
                format!(
                    "{}; generic instantiations: {}",
                    error.message(),
                    notes.join("; ")
                ),
                span,
            )
        }
    }
    pub fn validate_arguments(
        &self,
        model: &crate::semantic::SemanticModel,
    ) -> Result<(), Diagnostic> {
        for ty in &self.types {
            model.resolve_type_ref(ty)?;
        }
        Ok(())
    }
}
pub(crate) fn lower(program: &Program) -> Result<Lowered, Diagnostic> {
    let mut templates = HashMap::new();
    let mut functions = HashSet::new();
    let globals: HashSet<_> = program
        .items
        .iter()
        .filter_map(|i| match i {
            Item::FunctionDefinition(d) => Some(d.name.as_str()),
            Item::ConstantDefinition(d) => Some(d.name.as_str()),
            Item::TypeDefinition(d) => Some(d.name.as_str()),
            _ => None,
        })
        .collect();
    for item in &program.items {
        if let Item::FunctionDefinition(d) = item {
            if !functions.insert(d.name.clone()) {
                return Err(Diagnostic::new(
                    format!("duplicate function {}", d.name),
                    d.name_span,
                ));
            }
            if d.generic_parameters.is_empty() {
                continue;
            }
            if matches!(
                d.name.as_str(),
                "main"
                    | "byte_len"
                    | "array_len"
                    | "concat"
                    | "array_copy"
                    | "array_copy_range"
                    | "array_repeat"
            ) || Type::from_name(&d.name).is_some()
            {
                return Err(Diagnostic::new(
                    "this function cannot have generic parameters",
                    d.name_span,
                ));
            }
            let mut seen = HashSet::new();
            for p in &d.generic_parameters {
                if !seen.insert(&p.name)
                    || globals.contains(p.name.as_str())
                    || Type::from_name(&p.name).is_some()
                    || matches!(
                        p.name.as_str(),
                        "infer"
                            | "byte_len"
                            | "array_len"
                            | "concat"
                            | "array_copy"
                            | "array_copy_range"
                            | "array_repeat"
                            | "convert"
                    )
                {
                    return Err(Diagnostic::new(
                        format!("conflicting generic parameter {}", p.name),
                        p.span,
                    ));
                }
            }
            let mut parameters = HashSet::new();
            for p in &d.parameters {
                if seen.contains(&p.name) || !parameters.insert(&p.name) {
                    return Err(Diagnostic::new(
                        format!("conflicting function parameter {}", p.name),
                        p.name_span,
                    ));
                }
            }
            templates.insert(d.name.clone(), d.clone());
        }
    }
    let mut expander = Expander {
        source: program,
        templates,
        instances: Vec::new(),
        origins: Vec::new(),
        types: Vec::new(),
        array_length_uses: Vec::new(),
        keys: HashMap::new(),
        active: Vec::new(),
        pending: VecDeque::new(),
        substitutions: HashMap::new(),
    };
    let mut output = program.clone();
    output
        .items
        .retain(|i| !matches!(i, Item::FunctionDefinition(d) if !d.generic_parameters.is_empty()));
    for item in &mut output.items {
        expander.item(item)?;
    }
    // 本文の具体化はコンパイラの再帰callへ積まず、登録順に処理します。
    while let Some(pending) = expander.pending.pop_front() {
        expander.substitutions = pending.substitutions;
        expander.active = pending.trace;
        let mut definition = pending.definition;
        expander.function(&mut definition).map_err(|mut error| {
            for (name, span) in expander.active.iter().rev() {
                error = Diagnostic::new(
                    format!(
                        "{}; while instantiating {} at source={} bytes={}..{}",
                        error.message(),
                        name,
                        span.source_id().index(),
                        span.start(),
                        span.end()
                    ),
                    error.primary_span().unwrap_or(*span),
                );
            }
            error
        })?;
        expander.instances.push(definition);
    }
    output
        .items
        .extend(expander.instances.into_iter().map(Item::FunctionDefinition));
    Ok(Lowered {
        program: output,
        origins: expander.origins,
        types: expander.types,
        array_length_uses: expander.array_length_uses,
    })
}

#[derive(Clone)]
enum Argument {
    Type(TypeRef),
    Length(usize),
}
struct Pending {
    definition: FunctionDefinition,
    substitutions: HashMap<String, Argument>,
    trace: Vec<(String, Span)>,
}
struct Expander<'a> {
    source: &'a Program,
    templates: HashMap<String, FunctionDefinition>,
    instances: Vec<FunctionDefinition>,
    origins: Vec<(String, GenericOrigin)>,
    types: Vec<TypeRef>,
    array_length_uses: Vec<crate::array_lengths::LengthUse>,
    keys: HashMap<String, usize>,
    active: Vec<(String, Span)>,
    pending: VecDeque<Pending>,
    substitutions: HashMap<String, Argument>,
}
fn type_key(ty: &TypeRef) -> String {
    match &ty.kind {
        TypeRefKind::Named(n) => n.clone(),
        TypeRefKind::DynamicArray { element } => format!("[{}]", type_key(element)),
        TypeRefKind::Array { element, length } => format!("[{}; {length}]", type_key(element)),
        TypeRefKind::ArrayConstant { .. } => unreachable!(),
    }
}
impl Expander<'_> {
    fn binding(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if self.substitutions.contains_key(name) {
            return Err(Diagnostic::new(
                format!("binding conflicts with generic parameter {name}"),
                span,
            ));
        }
        Ok(())
    }
    fn ty(&self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        match &mut ty.kind {
            TypeRefKind::Named(n) => match self.substitutions.get(n) {
                Some(Argument::Type(t)) => ty.kind = t.kind.clone(),
                Some(Argument::Length(_)) => {
                    return Err(Diagnostic::new("length parameter used as a type", ty.span));
                }
                None => {}
            },
            TypeRefKind::Array { element, .. } | TypeRefKind::DynamicArray { element } => {
                self.ty(element)?
            }
            TypeRefKind::ArrayConstant { element, constant } => {
                self.ty(element)?;
                match self.substitutions.get(&constant.name) {
                    Some(Argument::Length(length)) => {
                        ty.kind = TypeRefKind::Array {
                            element: element.clone(),
                            length: *length,
                        }
                    }
                    Some(Argument::Type(_)) => {
                        return Err(Diagnostic::new(
                            "type parameter used as an array length",
                            constant.span,
                        ));
                    }
                    None => {}
                }
            }
        }
        Ok(())
    }
    fn instantiate(&mut self, call: &GenericCall, span: Span) -> Result<String, Diagnostic> {
        let mut definition = self.templates.get(&call.name).cloned().ok_or_else(|| {
            Diagnostic::new(
                format!("{} is not a generic function", call.name),
                call.name_span,
            )
        })?;
        if definition.generic_parameters.len() != call.generic_arguments.len() {
            return Err(Diagnostic::new(
                format!(
                    "{} requires {} generic arguments",
                    call.name,
                    definition.generic_parameters.len()
                ),
                span,
            ));
        }
        let mut substitutions = HashMap::new();
        let mut descriptions = Vec::new();
        let mut argument_spans = Vec::new();
        for (parameter, argument) in definition
            .generic_parameters
            .iter()
            .zip(&call.generic_arguments)
        {
            let resolved = match (parameter.length, argument) {
                (true, GenericArgument::Length { value, span }) => {
                    argument_spans.push(*span);
                    Argument::Length(*value)
                }
                (true, GenericArgument::Type(ty)) => {
                    argument_spans.push(ty.span);
                    let TypeRefKind::Named(name) = &ty.kind else {
                        return Err(Diagnostic::new(
                            "generic length requires a positive integer literal or constant name",
                            ty.span,
                        ));
                    };
                    Argument::Length(match self.substitutions.get(name) {
                        Some(Argument::Length(value)) => *value,
                        Some(Argument::Type(_)) => {
                            return Err(Diagnostic::new(
                                "type parameter used as a generic length",
                                ty.span,
                            ));
                        }
                        None => crate::array_lengths::resolve_length(self.source, name, ty.span)?,
                    })
                }
                (false, GenericArgument::Type(ty)) => {
                    argument_spans.push(ty.span);
                    let mut ty = ty.clone();
                    self.ty(&mut ty)?;
                    self.array_length_uses
                        .extend(crate::array_lengths::resolve_type(self.source, &mut ty)?);
                    self.validate_type(&ty)?;
                    self.types.push(ty.clone());
                    Argument::Type(ty)
                }
                (false, GenericArgument::Length { span, .. }) => {
                    return Err(Diagnostic::new(
                        "generic type argument requires a type",
                        *span,
                    ));
                }
            };
            descriptions.push(match &resolved {
                Argument::Type(t) => type_key(t),
                Argument::Length(n) => n.to_string(),
            });
            substitutions.insert(parameter.name.clone(), resolved);
        }
        // 本文の展開前に登録した具体化も再利用します。再帰は通常のcallとして残します。
        let key = format!("{}::<{}>", call.name, descriptions.join(", "));
        if let Some(&index) = self.keys.get(&key) {
            let (name, origin) = &mut self.origins[index];
            origin.calls.push(GenericUse {
                span,
                argument_spans,
            });
            return Ok(name.clone());
        }
        if self.origins.len() >= 256 || self.active.len() >= 64 {
            return Err(Diagnostic::new(
                "generic expansion exceeds 256 instances or depth 64",
                span,
            ));
        }
        let index = self.origins.len();
        let name = format!("$generic_{index}_{}", call.name);
        self.keys.insert(key, index);
        self.origins.push((
            name.clone(),
            GenericOrigin {
                template: call.name.clone(),
                definition: definition.span,
                arguments: descriptions,
                calls: vec![GenericUse {
                    span,
                    argument_spans,
                }],
            },
        ));
        definition.generic_parameters.clear();
        definition.name = name.clone();
        let mut trace = self.active.clone();
        trace.push((call.name.clone(), span));
        self.pending.push_back(Pending {
            definition,
            substitutions,
            trace,
        });
        Ok(name)
    }
    fn validate_type(&self, ty: &TypeRef) -> Result<(), Diagnostic> {
        match &ty.kind {
            TypeRefKind::Named(n)
                if Type::from_name(n).is_some()
                    || self
                        .source
                        .items
                        .iter()
                        .any(|i| matches!(i, Item::TypeDefinition(d) if &d.name == n)) =>
            {
                Ok(())
            }
            TypeRefKind::Array { element, .. } | TypeRefKind::DynamicArray { element } => {
                self.validate_type(element)
            }
            _ => Err(Diagnostic::new(
                "generic argument requires a known concrete type",
                ty.span,
            )),
        }
    }
    fn function(&mut self, d: &mut FunctionDefinition) -> Result<(), Diagnostic> {
        for p in &mut d.parameters {
            self.binding(&p.name, p.name_span)?;
            self.ty(&mut p.type_ref)?;
        }
        if let ReturnTypeRef::Value(ty) = &mut d.return_type {
            self.ty(ty)?;
        }
        self.statements(&mut d.body)
    }
    fn item(&mut self, item: &mut Item) -> Result<(), Diagnostic> {
        match item {
            Item::ConstantDefinition(d) => self.expr(&mut d.value)?,
            Item::TypeDefinition(d) => {
                for field in d
                    .fields
                    .iter_mut()
                    .chain(d.variants.iter_mut().flatten().flat_map(|v| &mut v.fields))
                {
                    if let Some(value) = &mut field.default {
                        self.expr(value)?;
                    }
                }
            }
            Item::FunctionDefinition(d) => self.function(d)?,
            Item::Statement(s) => self.statement(s)?,
        }
        Ok(())
    }
    fn spec(&self, spec: &mut TypeSpec) -> Result<(), Diagnostic> {
        if let TypeSpec::Explicit(ty) = spec {
            self.ty(ty)?;
        }
        Ok(())
    }
    fn statements(&mut self, body: &mut [Stmt]) -> Result<(), Diagnostic> {
        for s in body {
            self.statement(s)?;
        }
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(body) => self.statements(body)?,
            StmtKind::Binding {
                name,
                type_spec,
                value,
                ..
            } => {
                self.binding(name, statement.span)?;
                self.spec(type_spec)?;
                self.expr(value)?;
            }
            StmtKind::Assignment { target, value } => {
                self.binding(&target.name, target.span)?;
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
                    self.binding(&binding.name, binding.span)?;
                    self.spec(&mut binding.type_spec)?;
                }
                self.expr(value)?;
                self.statements(body)?;
            }
            StmtKind::Match { value, arms } => {
                self.expr(value)?;
                for arm in arms {
                    for field in &arm.fields {
                        self.binding(&field.binding, field.binding_span)?;
                    }
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
            ExprKind::GenericCall(call) => {
                for argument in &mut call.arguments {
                    self.expr(argument)?;
                }
                if call.name == "array_repeat" {
                    let [GenericArgument::Type(ty)] = call.generic_arguments.as_mut_slice() else {
                        return Err(Diagnostic::new(
                            "array_repeat requires exactly one explicit element type",
                            expr.span,
                        ));
                    };
                    self.ty(ty)?;
                    self.array_length_uses
                        .extend(crate::array_lengths::resolve_type(self.source, ty)?);
                    self.types.push(ty.clone());
                    return Ok(());
                }
                let name = self.instantiate(call, expr.span)?;
                expr.kind = ExprKind::Call {
                    name,
                    name_span: call.name_span,
                    arguments: std::mem::take(&mut call.arguments),
                };
            }
            ExprKind::Variable(name) => match self.substitutions.get(name) {
                Some(Argument::Length(n)) => {
                    expr.kind = ExprKind::Integer(IntegerLiteral::with_type(
                        n.to_string(),
                        IntegerType::I64,
                    ))
                }
                Some(Argument::Type(_)) => {
                    return Err(Diagnostic::new("type parameter used as a value", expr.span));
                }
                None => {}
            },
            ExprKind::Convert { target, value, .. } => {
                self.ty(target)?;
                self.expr(value)?;
            }
            ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    self.expr(base)?;
                }
                for field in fields {
                    self.expr(&mut field.value)?;
                }
            }
            ExprKind::Call {
                name, arguments, ..
            } => {
                if self.templates.contains_key(name) {
                    return Err(Diagnostic::new(
                        format!("generic function {name} requires explicit ::<...> arguments"),
                        expr.span,
                    ));
                }
                for argument in arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Array(arguments) => {
                for argument in arguments {
                    self.expr(argument)?;
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
