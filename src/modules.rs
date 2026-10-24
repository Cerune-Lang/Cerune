//! 明示した入口からファイルを読み、公開範囲を解決して共通ASTへ渡します。
use crate::{
    ast::*,
    diagnostic::{Diagnostic, render},
    lexer, parser,
    source::{SourceId, SourceKind, SourceMap, Span},
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

mod declarations;
pub use declarations::FunctionDeclaration;
mod limits;
mod resolve;
pub use limits::LoadLimits;
use limits::SourceBudget;

#[derive(Debug)]
pub struct Compilation {
    pub sources: SourceMap,
    pub program: Program,
    entry: SourceId,
    declarations: Vec<FunctionDeclaration>,
    pub modular: bool,
}

impl Compilation {
    pub fn function_declarations(&self) -> &[FunctionDeclaration] {
        &self.declarations
    }

    pub fn to_ir(&self) -> Result<crate::ir::Program, Diagnostic> {
        crate::ir::builder::build(&self.program)
    }

    pub fn render(&self, diagnostic: &Diagnostic) -> String {
        render_diagnostic(diagnostic, &self.sources, self.entry)
    }

    pub fn render_execution(&self, error: &crate::ExecutionError) -> String {
        match error.origin() {
            Some(crate::bytecode::InstructionOrigin::Source { span, .. })
                if span.source_id() != SourceId::ANONYMOUS =>
            {
                crate::vm::render::render_compact_with_sources(
                    error.vm_error(),
                    &self.sources,
                    span,
                )
            }
            Some(crate::bytecode::InstructionOrigin::Source { span, .. }) => {
                crate::vm::render::render_compact_with_source(
                    error.vm_error(),
                    self.sources.get(self.entry).unwrap().text(),
                    span,
                )
            }
            _ => crate::vm::render::render_compact(error.vm_error()),
        }
    }

    /// JSONの依存ファイル一覧。呼び出し側が選んだときだけ名前と本文を公開します。
    /// 本文は正確なバイト範囲の照合用であり、生成プログラムの状態ではありません。
    pub fn source_manifest(&self) -> String {
        let files = self
            .sources
            .files()
            .map(|file| {
                format!(
                    "{{\"id\":{},\"name\":{},\"text\":{}{}}}",
                    if self.modular { file.id().index() } else { 0 },
                    json(file.name()),
                    json(file.text()),
                    match file.kind() {
                        SourceKind::User => String::new(),
                        SourceKind::Standard {
                            identifier,
                            version,
                        } => format!(
                            ",\"kind\":\"standard\",\"module\":{},\"version\":{version}",
                            json(identifier)
                        ),
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"schema\":\"cerune-sources-v1\",\"files\":[{files}]}}\n")
    }
}

#[derive(Debug)]
pub struct LoadError {
    pub diagnostic: Diagnostic,
    pub sources: SourceMap,
    entry: Option<SourceId>,
}

impl LoadError {
    pub fn render(&self) -> String {
        match self.entry {
            Some(entry) => render_diagnostic(&self.diagnostic, &self.sources, entry),
            None => render::render_compact(&self.diagnostic, ""),
        }
    }
}

fn render_diagnostic(error: &Diagnostic, sources: &SourceMap, entry: SourceId) -> String {
    if error
        .primary_span()
        .is_some_and(|span| span.source_id() != SourceId::ANONYMOUS)
    {
        render::render_compact_with_sources(error, sources)
    } else {
        render::render_compact(error, sources.get(entry).unwrap().text())
    }
}

struct Unit {
    id: SourceId,
    path: PathBuf,
    module: parser::Module,
    imports: HashMap<String, usize>,
    complete: bool,
}

/// 相対importは宣言元のディレクトリを基準にします。CWDや検索パスへフォールバックしません。
pub fn load(entry: &Path) -> Result<Compilation, LoadError> {
    load_with_limits(entry, LoadLimits::default())
}

/// Load with explicit filesystem-source budgets; path resolution is unchanged.
pub fn load_with_limits(entry: &Path, limits: LoadLimits) -> Result<Compilation, LoadError> {
    let mut budget = SourceBudget::new(limits);
    let mut sources = SourceMap::new();
    let mut entry_id = None;
    let result = (|| {
        let path = fs::canonicalize(entry).map_err(|e| {
            Diagnostic::without_span(format!("failed to read {}: {e}", entry.display()))
        })?;
        let text = budget.read(&path).map_err(|e| {
            Diagnostic::without_span(format!("failed to read {}: {e}", entry.display()))
        })?;
        let id = sources.add(entry.to_string_lossy(), text);
        entry_id = Some(id);
        let file = sources.get(id).unwrap();
        // 単一ファイルのCLI成果物と診断は既存の匿名ソースの契約を保ちます。
        let mut tokens = lexer::lex(file.text())?;
        let modular = tokens
            .iter()
            .any(|token| matches!(token.kind, lexer::TokenKind::Import | lexer::TokenKind::Pub));
        if modular {
            for token in &mut tokens {
                token.span = token.span.with_source(id);
            }
        }
        let module = parser::parse_module(tokens)?;
        if !modular {
            let declarations = declarations::private_functions(&module.program);
            return Ok((module.program, declarations, false));
        }
        let mut units = vec![Unit {
            id,
            path,
            module,
            imports: HashMap::new(),
            complete: false,
        }];
        let mut paths = HashMap::from([(units[0].path.clone(), 0)]);
        visit(
            0,
            &mut units,
            &mut paths,
            &mut sources,
            0,
            true,
            &mut budget,
        )?;
        let (program, declarations) = resolve::resolve(&units)?;
        Ok((program, declarations, true))
    })();
    match result {
        Ok((program, declarations, modular)) => Ok(Compilation {
            declarations,
            sources,
            program,
            entry: entry_id.unwrap(),
            modular,
        }),
        Err(diagnostic) => Err(LoadError {
            diagnostic,
            sources,
            entry: entry_id,
        }),
    }
}

fn visit(
    index: usize,
    units: &mut Vec<Unit>,
    paths: &mut HashMap<PathBuf, usize>,
    sources: &mut SourceMap,
    depth: usize,
    allow_files: bool,
    budget: &mut SourceBudget,
) -> Result<(), Diagnostic> {
    // Keep the existing depth-first registration order without nesting parser
    // calls underneath every ancestor import's Rust stack frame.
    let mut pending = vec![(index, 0usize, depth)];
    while let Some(&(index, next, depth)) = pending.last() {
        let Some(import) = units[index].module.imports.get(next).cloned() else {
            units[index].complete = true;
            pending.pop();
            continue;
        };
        pending.last_mut().unwrap().1 += 1;
        if depth >= 128 {
            return Err(Diagnostic::new(
                "import nesting exceeds 128 files",
                import.span,
            ));
        }
        if units[index].imports.contains_key(&import.alias) {
            return Err(Diagnostic::new(
                format!("duplicate import alias `{}`", import.alias),
                import.span,
            ));
        }
        if Type::from_name(&import.alias).is_some()
            || matches!(
                import.alias.as_str(),
                "infer" | "convert" | "byte_len" | "array_len"
            )
        {
            return Err(Diagnostic::new(
                format!("import alias `{}` is reserved", import.alias),
                import.span,
            ));
        }
        if import.path.starts_with("std:") {
            if !matches!(import.path.as_str(), "std:io" | "std:env" | "std:num") {
                return Err(Diagnostic::new(
                    format!("unknown standard module `{}`", import.path),
                    import.span,
                ));
            }
            let target = if let Some(target) = units.iter().position(|u| {
                matches!(
                    sources.get(u.id).unwrap().kind(),
                    SourceKind::Standard { identifier, .. } if *identifier == import.path
                )
            }) {
                target
            } else {
                let (identifier, text) = match import.path.as_str() {
                    "std:io" => ("std:io", include_str!("standard/io.ceru")),
                    "std:env" => ("std:env", include_str!("standard/env.ceru")),
                    "std:num" => ("std:num", include_str!("standard/num.ceru")),
                    _ => unreachable!(),
                };
                let id = sources.add_standard(identifier, 1, text);
                let mut tokens = lexer::lex_source(sources.get(id).unwrap())?;
                for token in &mut tokens {
                    if let lexer::TokenKind::Identifier(name) = &mut token.kind {
                        let internal = match (identifier, name.as_str()) {
                            ("std:io", "__cerune_read_line_utf8") => Some("$read_line"),
                            ("std:env", "__cerune_arg_count") => Some("$arg_count"),
                            ("std:env", "__cerune_arg_get") => Some("$arg_get"),
                            ("std:num", "__cerune_string_byte_at") => Some("$string_byte_at"),
                            _ => None,
                        };
                        if let Some(internal) = internal {
                            *name = internal.into();
                        }
                    }
                }
                let module = parser::parse_module(tokens)?;
                let target = units.len();
                units.push(Unit {
                    id,
                    path: PathBuf::new(),
                    module,
                    imports: HashMap::new(),
                    complete: true,
                });
                target
            };
            units[index].imports.insert(import.alias, target);
            continue;
        }
        if !allow_files {
            return Err(Diagnostic::new(
                "relative imports require modules::load with an entry file",
                import.span,
            ));
        }
        if import.path.is_empty()
            || import.path.starts_with('/')
            || import.path.contains(['\\', ':'])
            || import.path.chars().any(char::is_control)
            || !import.path.ends_with(".ceru")
        {
            return Err(Diagnostic::new(
                "import path must be a relative .ceru path using forward slashes",
                import.span,
            ));
        }
        let requested = units[index].path.parent().unwrap().join(&import.path);
        let path = fs::canonicalize(&requested).map_err(|e| {
            Diagnostic::new(format!("cannot import `{}`: {e}", import.path), import.span)
        })?;
        let mut descend = None;
        let target = if let Some(&target) = paths.get(&path) {
            if !units[target].complete {
                return Err(Diagnostic::new(
                    format!("cyclic import of `{}`", import.path),
                    import.span,
                ));
            }
            target
        } else {
            let text = budget.read(&path).map_err(|e| {
                Diagnostic::new(format!("cannot import `{}`: {e}", import.path), import.span)
            })?;
            // リンク名側の親へ結び付けず、診断も実際に読んだファイルを指します。
            let name = path.to_string_lossy().into_owned();
            let id = sources.add(name, text);
            let module = parser::parse_module(lexer::lex_source(sources.get(id).unwrap())?)?;
            if let Some(statement) = module.program.items.iter().find_map(|item| {
                if let Item::Statement(s) = item {
                    Some(s)
                } else {
                    None
                }
            }) {
                return Err(Diagnostic::new(
                    "imported files may contain only imports, types, functions, and constants; call initialization explicitly",
                    statement.span,
                ));
            }
            let target = units.len();
            paths.insert(path.clone(), target);
            units.push(Unit {
                id,
                path,
                module,
                imports: HashMap::new(),
                complete: false,
            });
            descend = Some(target);
            target
        };
        units[index].imports.insert(import.alias, target);
        if let Some(target) = descend {
            pending.push((target, 0, depth + 1));
        }
    }
    Ok(())
}

pub(crate) fn json(text: &str) -> String {
    use std::fmt::Write;
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c <= '\u{1f}' => write!(result, "\\u{:04x}", c as u32).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

/// メモリ中の入口と同梱標準モジュールだけを解決し、ファイルI/Oを行いません。
pub fn from_source(text: &str) -> Result<Compilation, LoadError> {
    let mut sources = SourceMap::new();
    let id = sources.add("<memory>", text);
    let result = (|| {
        let mut tokens = lexer::lex(text)?;
        let modular = tokens
            .iter()
            .any(|t| matches!(t.kind, lexer::TokenKind::Import | lexer::TokenKind::Pub));
        if modular {
            for token in &mut tokens {
                token.span = token.span.with_source(id);
            }
        }
        let module = parser::parse_module(tokens)?;
        if !modular {
            let declarations = declarations::private_functions(&module.program);
            return Ok((module.program, declarations, false));
        }
        let mut units = vec![Unit {
            id,
            path: PathBuf::new(),
            module,
            imports: HashMap::new(),
            complete: false,
        }];
        visit(
            0,
            &mut units,
            &mut HashMap::new(),
            &mut sources,
            0,
            false,
            &mut SourceBudget::new(LoadLimits::default()),
        )?;
        let (program, declarations) = resolve::resolve(&units)?;
        Ok((program, declarations, true))
    })();
    match result {
        Ok((program, declarations, modular)) => Ok(Compilation {
            declarations,
            sources,
            program,
            entry: id,
            modular,
        }),
        Err(diagnostic) => Err(LoadError {
            diagnostic,
            sources,
            entry: Some(id),
        }),
    }
}

pub(crate) fn parse_inline(text: &str) -> Result<Program, Diagnostic> {
    let tokens = lexer::lex(text)?;
    if tokens
        .iter()
        .any(|t| matches!(t.kind, lexer::TokenKind::Import))
    {
        from_source(text)
            .map(|c| c.program)
            .map_err(|e| e.diagnostic)
    } else {
        parser::parse(tokens)
    }
}
