//! Explicit scalar exports using the ordinary WAT lowering.
use crate::{
    ast,
    diagnostic::Diagnostic,
    ir,
    modules::{self, Compilation},
    source::Span,
    types::IntegerType,
};
use std::collections::HashSet;

mod bindings;
mod validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    I32,
    F64,
}
impl ScalarType {
    pub fn name(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::F64 => "f64",
        }
    }
    pub fn internal_wat_type(self) -> &'static str {
        match self {
            Self::I32 => "i64",
            Self::F64 => "f64",
        }
    }
    fn descriptor(self) -> String {
        format!(
            "{{\"cerune\":\"{}\",\"wasm\":\"{}\",\"internal\":\"{}\"}}",
            self.name(),
            self.name(),
            self.internal_wat_type()
        )
    }
    fn from_ir(ty: &ir::Type, span: Span) -> Result<Self, Diagnostic> {
        match ty {
            ir::Type::Integer(IntegerType::I32) => Ok(Self::I32),
            ir::Type::F64 => Ok(Self::F64),
            _ => Err(Diagnostic::new(
                "WAT library exports require i32 or f64 values",
                span,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    name: String,
    function_id: ir::FunctionId,
    parameters: Vec<ScalarType>,
    result: ScalarType,
    span: Span,
}
impl Export {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn function_id(&self) -> ir::FunctionId {
        self.function_id
    }
    pub fn parameters(&self) -> &[ScalarType] {
        &self.parameters
    }
    pub fn result(&self) -> ScalarType {
        self.result
    }
    pub fn span(&self) -> Span {
        self.span
    }
}

/// A checked library and its observable, unmodified completed IR.
#[derive(Debug)]
pub struct Library {
    program: ir::Program,
    exports: Vec<Export>,
    wat: String,
}
impl Library {
    pub fn program(&self) -> &ir::Program {
        &self.program
    }
    pub fn exports(&self) -> &[Export] {
        &self.exports
    }
    pub fn wat(&self) -> &str {
        &self.wat
    }

    /// Checked ES module for the accompanying Wasm artifact.
    pub fn javascript(&self) -> String {
        bindings::javascript(self)
    }

    /// TypeScript declarations for the generated ES module.
    pub fn typescript(&self) -> String {
        bindings::typescript(self)
    }

    /// Names, types, wrapper conversions and declaration origins; no source text.
    pub fn export_manifest(&self) -> String {
        let entries = self.exports.iter().enumerate().map(|(i, e)| {
            let params = e.parameters.iter().map(|t| t.descriptor()).collect::<Vec<_>>().join(",");
            let f = self.program.function_definitions.iter().find(|f| f.id == e.function_id).unwrap();
            let result = e.result.descriptor();
            format!(concat!(
                "{{\"name\":{},\"function\":{},\"internal_name\":{},\"wrapper\":\"cerune_export_{}\",",
                "\"parameters\":[{}],\"result\":{},\"origin\":{{\"file\":{},\"start\":{},\"end\":{}}}}}"),
                modules::json(&e.name), e.function_id.0, modules::json(&f.name), i,
                params, result, e.span.source_id().index(), e.span.start(), e.span.end())
        }).collect::<Vec<_>>().join(",");
        format!("{{\"schema\":\"cerune-wat-library-v1\",\"exports\":[{entries}]}}\n")
    }
}

/// Compile and explicitly export entry-module pub functions.
/// This returns a raw Wasm ABI; checked JS bindings are a separate layer.
pub fn emit_library(compilation: &Compilation, selected: &[&str]) -> Result<Library, Diagnostic> {
    if selected.is_empty() {
        return Err(Diagnostic::without_span(
            "select at least one WAT library export",
        ));
    }
    let declarations = compilation.function_declarations();
    let functions = compilation
        .program
        .items
        .iter()
        .filter_map(|i| match i {
            ast::Item::FunctionDefinition(f) => Some(f),
            _ => None,
        })
        .collect::<Vec<_>>();
    if functions.len() != declarations.len()
        || declarations.iter().any(|d| {
            !functions.iter().any(|f| {
                f.name == d.resolved_name()
                    && f.span == d.span()
                    && f.generic_parameters.is_empty() == !d.is_generic()
            })
        })
    {
        return Err(Diagnostic::without_span(
            "function declarations no longer match the resolved program",
        ));
    }
    for item in &compilation.program.items {
        match item {
            ast::Item::Statement(s) => {
                return Err(Diagnostic::new(
                    "WAT libraries cannot contain top-level execution",
                    s.span,
                ));
            }
            ast::Item::TypeDefinition(t) => {
                return Err(Diagnostic::new(
                    "WAT libraries currently require scalar declarations",
                    t.span,
                ));
            }
            _ => {}
        }
    }
    for d in declarations {
        if d.is_generic() {
            return Err(Diagnostic::new(
                "generic declarations are not supported in WAT libraries yet",
                d.name_span(),
            ));
        }
        if d.is_entry() && d.name() == "main" {
            return Err(Diagnostic::new(
                "WAT libraries cannot declare main",
                d.name_span(),
            ));
        }
    }
    let program = compilation.to_ir()?;
    validate::program(&program)?;
    let mut seen = HashSet::new();
    let mut exports = Vec::new();
    for name in selected {
        let d = declarations
            .iter()
            .find(|d| d.is_entry() && d.name() == *name)
            .ok_or_else(|| {
                Diagnostic::without_span(format!("unknown entry-module function `{name}`"))
            })?;
        if !seen.insert(*name) {
            return Err(Diagnostic::new(
                format!("duplicate export `{name}`"),
                d.name_span(),
            ));
        }
        if !d.is_public() {
            return Err(Diagnostic::new(
                format!("function `{name}` is not pub"),
                d.name_span(),
            ));
        }
        let f = program
            .function_definitions
            .iter()
            .find(|f| f.name == d.resolved_name() && f.span == d.span())
            .ok_or_else(|| {
                Diagnostic::new(
                    "resolved declaration is missing from completed IR",
                    d.name_span(),
                )
            })?;
        let parameters = f
            .parameters
            .iter()
            .map(|p| ScalarType::from_ir(&p.ty, p.span))
            .collect::<Result<Vec<_>, _>>()?;
        let result = match &f.return_type {
            ir::ReturnType::Value(ty) => ScalarType::from_ir(ty, d.name_span())?,
            ir::ReturnType::Void => {
                return Err(Diagnostic::new(
                    "WAT library exports require a scalar result",
                    d.name_span(),
                ));
            }
        };
        exports.push(Export {
            name: (*name).into(),
            function_id: f.id,
            parameters,
            result,
            span: d.name_span(),
        });
    }
    let module = super::lower(&program);
    let wat = super::emit::emit_library(&module, &exports);
    Ok(Library {
        program,
        exports,
        wat,
    })
}

#[cfg(test)]
mod tests;
