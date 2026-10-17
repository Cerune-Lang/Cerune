use crate::{ast, source::Span};

/// Source declaration retained before module names are flattened.
/// This is visibility metadata, not an external export decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    name: String,
    resolved_name: String,
    span: Span,
    name_span: Span,
    public: bool,
    entry: bool,
    generic: bool,
}

impl FunctionDeclaration {
    pub(super) fn new(
        f: &ast::FunctionDefinition,
        resolved_name: String,
        public: bool,
        entry: bool,
    ) -> Self {
        Self {
            name: f.name.clone(),
            resolved_name,
            span: f.span,
            name_span: f.name_span,
            public,
            entry,
            generic: !f.generic_parameters.is_empty(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn resolved_name(&self) -> &str {
        &self.resolved_name
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn name_span(&self) -> Span {
        self.name_span
    }
    pub fn is_public(&self) -> bool {
        self.public
    }
    pub fn is_entry(&self) -> bool {
        self.entry
    }
    pub fn is_generic(&self) -> bool {
        self.generic
    }
}

pub(super) fn private_functions(program: &ast::Program) -> Vec<FunctionDeclaration> {
    program
        .items
        .iter()
        .filter_map(|item| match item {
            ast::Item::FunctionDefinition(f) => {
                Some(FunctionDeclaration::new(f, f.name.clone(), false, true))
            }
            _ => None,
        })
        .collect()
}
