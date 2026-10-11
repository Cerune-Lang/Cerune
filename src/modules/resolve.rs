use super::*;
use crate::ast::{GenericArgument, GenericParameter};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Type,
    Function,
    Constant,
}

#[derive(Clone)]
struct Symbol {
    name: String,
    public: bool,
    is_enum: bool,
    generic_lengths: Vec<bool>,
}
#[derive(Default)]
struct Names {
    functions: HashMap<String, Symbol>,
    types: HashMap<String, Symbol>,
    constants: HashMap<String, Symbol>,
}

pub(super) fn resolve(units: &[Unit]) -> Result<Program, Diagnostic> {
    let mut names = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        let mut scope = Names::default();
        for item in &unit.module.program.items {
            let (name, span, kind) = match item {
                Item::FunctionDefinition(d) => (&d.name, d.name_span, Kind::Function),
                Item::TypeDefinition(d) => (&d.name, d.name_span, Kind::Type),
                Item::ConstantDefinition(d) => (&d.name, d.name_span, Kind::Constant),
                _ => continue,
            };
            if Type::from_name(name).is_some()
                || (kind != Kind::Type
                    && matches!(
                        name.as_str(),
                        "byte_len"
                            | "array_len"
                            | "concat"
                            | "array_copy"
                            | "array_copy_range"
                            | "array_repeat"
                    ))
            {
                return Err(Diagnostic::new(
                    format!("definition name `{name}` is reserved"),
                    span,
                ));
            }
            if unit.imports.contains_key(name) {
                return Err(Diagnostic::new(
                    format!("definition `{name}` conflicts with an import alias"),
                    span,
                ));
            }
            let table = match kind {
                Kind::Function => &mut scope.functions,
                Kind::Constant => &mut scope.constants,
                Kind::Type => &mut scope.types,
            };
            let symbol = Symbol {
                generic_lengths: match item {
                    Item::FunctionDefinition(d) => {
                        d.generic_parameters.iter().map(|p| p.length).collect()
                    }
                    _ => Vec::new(),
                },
                is_enum: matches!(item, Item::TypeDefinition(d) if d.variants.is_some()),
                name: if index == 0 && kind == Kind::Function && name == "main" {
                    name.clone()
                } else {
                    format!("module_{}_{name}", unit.id.index())
                },
                public: unit
                    .module
                    .exports
                    .iter()
                    .any(|(_, exported)| *exported == span),
            };
            if table.insert(name.clone(), symbol).is_some() {
                return Err(Diagnostic::new(
                    format!(
                        "duplicate {} `{name}`",
                        match kind {
                            Kind::Function => "function",
                            Kind::Constant => "constant",
                            Kind::Type => "type",
                        }
                    ),
                    span,
                ));
            }
        }
        names.push(scope);
    }
    let mut program = Program { items: Vec::new() };
    for (index, unit) in units.iter().enumerate() {
        let resolver = Resolver {
            units,
            names: &names,
            current: index,
            generics: &[],
        };
        for mut item in unit.module.program.items.clone() {
            match &mut item {
                Item::ConstantDefinition(d) => {
                    let symbol = &names[index].constants[&d.name];
                    d.name = symbol.name.clone();
                    resolver.ty(&mut d.type_ref, symbol.public)?;
                    resolver.expr(&mut d.value)?;
                }
                Item::TypeDefinition(d) => {
                    let public = names[index].types[&d.name].public;
                    d.name = names[index].types[&d.name].name.clone();
                    if let Some(variants) = &mut d.variants {
                        for variant in variants {
                            for field in &mut variant.fields {
                                resolver.ty(&mut field.type_ref, public)?;
                            }
                        }
                    }
                    for field in &mut d.fields {
                        resolver.ty(&mut field.type_ref, public)?;
                        if let Some(default) = &mut field.default {
                            resolver.expr(default)?;
                        }
                    }
                }
                Item::FunctionDefinition(d) => {
                    resolver.generic_parameters(&d.generic_parameters)?;
                    let generic_parameters = d.generic_parameters.clone();
                    let resolver = Resolver {
                        generics: &generic_parameters,
                        ..resolver
                    };
                    let public = names[index].functions[&d.name].public;
                    d.name = names[index].functions[&d.name].name.clone();
                    for parameter in &mut d.parameters {
                        resolver.binding(&parameter.name, parameter.name_span)?;
                        resolver.ty(&mut parameter.type_ref, public)?;
                    }
                    if let ReturnTypeRef::Value(ty) = &mut d.return_type {
                        resolver.ty(ty, public)?;
                    }
                    resolver.statements(&mut d.body)?;
                }
                Item::Statement(statement) => resolver.statement(statement)?,
            }
            program.items.push(item);
        }
    }
    Ok(program)
}

struct Resolver<'a> {
    units: &'a [Unit],
    names: &'a [Names],
    current: usize,
    generics: &'a [GenericParameter],
}
impl Resolver<'_> {
    fn generic_parameters(&self, parameters: &[GenericParameter]) -> Result<(), Diagnostic> {
        for p in parameters {
            if self.units[self.current].imports.contains_key(&p.name)
                || self.names[self.current].types.contains_key(&p.name)
                || self.names[self.current].constants.contains_key(&p.name)
                || self.names[self.current].functions.contains_key(&p.name)
            {
                return Err(Diagnostic::new(
                    format!("conflicting generic parameter {}", p.name),
                    p.span,
                ));
            }
        }
        Ok(())
    }
    fn generic(&self, name: &str, length: bool) -> bool {
        self.generics
            .iter()
            .any(|p| p.name == name && p.length == length)
    }

    fn name(&self, name: &str, span: Span, kind: Kind, public: bool) -> Result<String, Diagnostic> {
        let (target, member, qualified) = if let Some((alias, member)) = name.split_once("::") {
            (
                *self.units[self.current].imports.get(alias).ok_or_else(|| {
                    Diagnostic::new(format!("unknown import alias `{alias}`"), span)
                })?,
                member,
                true,
            )
        } else {
            (self.current, name, false)
        };
        let table = match kind {
            Kind::Function => &self.names[target].functions,
            Kind::Constant => &self.names[target].constants,
            Kind::Type => &self.names[target].types,
        };
        let kind = match kind {
            Kind::Function => "function",
            Kind::Constant => "constant",
            Kind::Type => "type",
        };
        let symbol = table
            .get(member)
            .ok_or_else(|| Diagnostic::new(format!("unknown {kind} `{name}`"), span))?;
        if (qualified || public) && !symbol.public {
            return Err(Diagnostic::new(
                format!(
                    "private {kind} `{name}` cannot be used {}",
                    if qualified {
                        "through an import"
                    } else {
                        "in a public signature"
                    }
                ),
                span,
            ));
        }
        Ok(symbol.name.clone())
    }

    fn constructor_name(&self, path: &str, span: Span) -> Result<String, Diagnostic> {
        if let Some((prefix, variant)) = path.rsplit_once("::")
            && (prefix.contains("::")
                || self.names[self.current]
                    .types
                    .get(prefix)
                    .is_some_and(|s| s.is_enum))
        {
            return Ok(format!(
                "{}::{variant}",
                self.name(prefix, span, Kind::Type, false)?
            ));
        }
        self.name(path, span, Kind::Type, false)
    }

    fn ty(&self, ty: &mut TypeRef, public: bool) -> Result<(), Diagnostic> {
        match &mut ty.kind {
            TypeRefKind::Named(name)
                if Type::from_name(name).is_none()
                    && name != "infer"
                    && !self.generic(name, false) =>
            {
                *name = self.name(name, ty.span, Kind::Type, public)?
            }
            TypeRefKind::Array { element, .. } | TypeRefKind::DynamicArray { element } => {
                self.ty(element, public)?
            }
            TypeRefKind::ArrayConstant { element, constant } => {
                self.ty(element, public)?;
                if !self.generic(&constant.name, true) {
                    constant.name =
                        self.name(&constant.name, constant.span, Kind::Constant, false)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn binding(&self, name: &str, span: Span) -> Result<(), Diagnostic> {
        if self.generics.iter().any(|p| p.name == name) {
            return Err(Diagnostic::new(
                format!("binding conflicts with generic parameter {name}"),
                span,
            ));
        }
        if self.names[self.current].constants.contains_key(name) {
            return Err(Diagnostic::new(
                format!("binding {name} conflicts with a constant"),
                span,
            ));
        }
        if self.units[self.current].imports.contains_key(name) {
            return Err(Diagnostic::new(
                format!("binding `{name}` conflicts with an import alias"),
                span,
            ));
        }
        Ok(())
    }

    fn statements(&self, body: &mut [Stmt]) -> Result<(), Diagnostic> {
        for statement in body {
            self.statement(statement)?;
        }
        Ok(())
    }

    fn statement(&self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Block(body) => self.statements(body)?,
            StmtKind::Match { value, arms } => {
                self.expr(value)?;
                for arm in arms {
                    let (name, variant) = arm.variant.rsplit_once("::").ok_or_else(|| {
                        Diagnostic::new("a match arm requires Enum::Variant", arm.variant_span)
                    })?;
                    arm.variant = format!(
                        "{}::{variant}",
                        self.name(name, arm.variant_span, Kind::Type, false)?
                    );
                    for field in &arm.fields {
                        if field.binding != "_" {
                            self.binding(&field.binding, field.binding_span)?;
                        }
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
            StmtKind::Binding {
                name,
                type_spec,
                value,
                ..
            } => {
                self.binding(name, statement.span)?;
                if let TypeSpec::Explicit(ty) = type_spec {
                    self.ty(ty, false)?;
                }
                self.expr(value)?;
            }
            StmtKind::Assignment { target, value } => {
                if self.names[self.current]
                    .constants
                    .contains_key(&target.name)
                {
                    return Err(Diagnostic::new(
                        "cannot assign to a constant",
                        target.name_span,
                    ));
                }
                for AssignmentProjection::Index { index, .. } in &mut target.projections {
                    self.expr(index)?;
                }
                self.expr(value)?;
            }
            StmtKind::Print { value } | StmtKind::Call { value } => self.expr(value)?,
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
            StmtKind::ForEach {
                index,
                element,
                value,
                body,
                ..
            } => {
                self.expr(value)?;
                for binding in index.iter_mut().chain(std::iter::once(element)) {
                    self.binding(&binding.name, binding.span)?;
                    if let TypeSpec::Explicit(ty) = &mut binding.type_spec {
                        self.ty(ty, false)?;
                    }
                }
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

    fn expr(&self, expr: &mut Expr) -> Result<(), Diagnostic> {
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
                if call.name == "array_repeat" {
                    for argument in &mut call.generic_arguments {
                        if let GenericArgument::Type(ty) = argument {
                            self.ty(ty, false)?;
                        }
                    }
                    for argument in &mut call.arguments {
                        self.expr(argument)?;
                    }
                    return Ok(());
                }
                call.name = self.name(&call.name, call.name_span, Kind::Function, false)?;
                let parameters = &self
                    .names
                    .iter()
                    .flat_map(|n| n.functions.values())
                    .find(|s| s.name == call.name)
                    .unwrap()
                    .generic_lengths;
                if parameters.len() != call.generic_arguments.len() {
                    return Err(Diagnostic::new(
                        format!(
                            "{} requires {} generic arguments",
                            call.name,
                            parameters.len()
                        ),
                        expr.span,
                    ));
                }
                for (length, argument) in parameters.iter().zip(&mut call.generic_arguments) {
                    if let GenericArgument::Type(ty) = argument {
                        if *length {
                            if let TypeRefKind::Named(name) = &mut ty.kind
                                && !self.generic(name, true)
                            {
                                *name = self.name(name, ty.span, Kind::Constant, false)?;
                            }
                        } else {
                            self.ty(ty, false)?;
                        }
                    }
                }
                for argument in &mut call.arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Call {
                name,
                name_span,
                arguments,
            } => {
                if !matches!(
                    name.as_str(),
                    "byte_len"
                        | "array_len"
                        | "concat"
                        | "array_copy"
                        | "array_copy_range"
                        | "array_repeat"
                ) {
                    *name = self.name(name, *name_span, Kind::Function, false)?;
                }
                for argument in arguments {
                    self.expr(argument)?;
                }
            }
            ExprKind::Construct {
                type_name,
                type_name_span,
                base,
                fields,
            } => {
                *type_name = self.constructor_name(type_name, *type_name_span)?;
                if let Some(base) = base {
                    self.expr(base)?;
                }
                for field in fields {
                    self.expr(&mut field.value)?;
                }
            }
            ExprKind::Convert { target, value, .. } => {
                self.ty(target, false)?;
                self.expr(value)?;
            }
            ExprKind::Array(values) => {
                for value in values {
                    self.expr(value)?;
                }
            }
            ExprKind::Index { base, index } => {
                self.expr(base)?;
                self.expr(index)?;
            }
            ExprKind::FieldAccess { base, .. } | ExprKind::Unary { value: base, .. } => {
                self.expr(base)?
            }
            ExprKind::Logical { left, right, .. } | ExprKind::Binary { left, right, .. } => {
                self.expr(left)?;
                self.expr(right)?;
            }
            ExprKind::Variable(name)
                if name.contains("::") || self.names[self.current].constants.contains_key(name) =>
            {
                *name = self.name(name, expr.span, Kind::Constant, false)?;
            }
            _ => {}
        }
        Ok(())
    }
}
