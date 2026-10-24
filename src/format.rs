//! Syntax-only source formatting. No imports, name resolution or execution.
use crate::{
    diagnostic::Diagnostic,
    lexer::{self, Token, TokenKind as K},
    parser::{self, BindingSyntax},
    source::SourceFile,
};
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LetStyle {
    #[default]
    Preserve,
    Show,
    Hide,
}

pub fn format(source: &str, style: LetStyle) -> Result<String, Diagnostic> {
    format_tokens(source, lexer::lex(source)?, style)
}
pub fn format_source(source: &SourceFile, style: LetStyle) -> Result<String, Diagnostic> {
    format_tokens(source.text(), lexer::lex_source(source)?, style)
}
fn prefixes(bindings: &[BindingSyntax]) -> BTreeSet<usize> {
    bindings
        .iter()
        .filter_map(|b| b.let_span.map(|s| s.start()))
        .collect()
}
fn meaning(tokens: &[Token], bindings: &[BindingSyntax]) -> Vec<K> {
    let prefixes = prefixes(bindings);
    tokens
        .iter()
        .filter(|t| !prefixes.contains(&t.span.start()))
        .map(|t| t.kind.clone())
        .collect()
}
fn format_tokens(source: &str, tokens: Vec<Token>, style: LetStyle) -> Result<String, Diagnostic> {
    let (_, syntax) = parser::parse_module_with_syntax(tokens.clone())?;
    let removed = if style == LetStyle::Hide {
        prefixes(&syntax.bindings)
    } else {
        BTreeSet::new()
    };
    let added: BTreeSet<_> = syntax
        .bindings
        .iter()
        .filter(|b| style == LetStyle::Show && b.let_span.is_none())
        .map(|b| b.start.start())
        .collect();
    let inline: BTreeSet<_> = syntax.inline_braces.iter().map(|s| s.start()).collect();
    let angles: BTreeSet<_> = syntax.type_angles.iter().map(|s| s.start()).collect();
    let mut writer = Writer::default();
    let mut end = 0;
    for (index, token) in tokens.iter().enumerate() {
        writer.gap(&source[end..token.span.start()]);
        end = token.span.end();
        if token.kind == K::Eof {
            break;
        }
        if removed.contains(&token.span.start()) {
            continue;
        }
        if added.contains(&token.span.start()) {
            writer.token(
                &K::Identifier("let".into()),
                "let",
                Some(&token.kind),
                false,
                false,
            );
        }
        let next = tokens[index + 1..]
            .iter()
            .find(|t| !removed.contains(&t.span.start()))
            .map(|t| &t.kind);
        writer.token(
            &token.kind,
            &source[token.span.start()..end],
            next,
            inline.contains(&token.span.start()),
            angles.contains(&token.span.start()),
        );
    }
    let output = writer.finish();
    let internal_error = |e: Diagnostic| {
        Diagnostic::without_span(format!(
            "formatter produced invalid syntax: {}",
            e.message()
        ))
    };
    let after = lexer::lex(&output).map_err(internal_error)?;
    let (_, after_syntax) =
        parser::parse_module_with_syntax(after.clone()).map_err(internal_error)?;
    if meaning(&tokens, &syntax.bindings) != meaning(&after, &after_syntax.bindings) {
        return Err(Diagnostic::without_span(
            "formatter changed the source token meaning",
        ));
    }
    Ok(output)
}

#[derive(Default)]
struct Writer {
    output: String,
    depth: usize,
    delimiters: Vec<(K, bool)>,
    previous: Option<K>,
    pending_break: bool,
    tight_after: bool,
    previous_angle: bool,
}
impl Writer {
    fn at_start(&self) -> bool {
        self.output.is_empty() || self.output.ends_with('\n')
    }
    fn newline(&mut self) {
        if !self.at_start() {
            self.output.push('\n');
        }
        self.pending_break = false;
    }
    fn put(&mut self, text: &str, space: bool) {
        if self.at_start() {
            self.output.push_str(&"    ".repeat(self.depth));
        } else if space && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
        self.output.push_str(text);
    }
    fn gap(&mut self, gap: &str) {
        let mut rest = gap;
        while let Some(at) = rest.find("//") {
            let before = &rest[..at];
            if before.contains('\n') {
                self.newline();
            }
            self.blank_line(before);
            let tail = &rest[at..];
            let end = tail.find('\n').unwrap_or(tail.len());
            let comment = tail[..end].strip_suffix('\r').unwrap_or(&tail[..end]);
            self.put(comment, true);
            self.newline();
            rest = &tail[end..];
        }
        self.blank_line(rest);
    }
    fn blank_line(&mut self, whitespace: &str) {
        if whitespace.bytes().filter(|b| *b == b'\n').count() >= 2 && !self.output.is_empty() {
            self.newline();
            if !self.output.ends_with("\n\n") {
                self.output.push('\n');
            }
        }
    }
    fn token(&mut self, kind: &K, text: &str, next: Option<&K>, inline: bool, angle: bool) {
        if self.pending_break {
            self.newline();
        }
        if *kind == K::RightBrace {
            self.depth = self.depth.saturating_sub(1);
            if self.previous != Some(K::LeftBrace)
                && !self.delimiters.last().is_some_and(|(_, inline)| *inline)
            {
                self.newline();
            }
        }
        let space = self.previous.as_ref().is_some_and(|prev| {
            needs_space(prev, kind, self.tight_after, self.previous_angle, angle)
        });
        self.put(text, space);
        self.tight_after = (angle && *kind == K::Less)
            || matches!(kind, K::Bang | K::Tilde)
            || (*kind == K::Minus && self.previous.as_ref().is_none_or(|p| !ends_expression(p)));
        match kind {
            K::LeftBrace => {
                self.depth += 1;
                self.delimiters.push((kind.clone(), inline));
                self.pending_break = !inline && next != Some(&K::RightBrace);
            }
            K::LeftParen | K::LeftBracket => self.delimiters.push((kind.clone(), false)),
            K::RightBrace => {
                let inline = self.delimiters.pop().unwrap().1;
                self.pending_break = !inline && next.is_some_and(|n| !continues_expression(n));
            }
            K::RightParen | K::RightBracket => {
                self.delimiters.pop();
            }
            K::Semicolon => {
                self.pending_break = self
                    .delimiters
                    .last()
                    .is_none_or(|(k, _)| *k == K::LeftBrace);
            }
            K::Comma => {
                self.pending_break = self.delimiters.last() == Some(&(K::LeftBrace, false));
            }
            _ => {}
        }
        self.previous = Some(kind.clone());
        self.previous_angle = angle;
    }
    fn finish(mut self) -> String {
        while self.output.ends_with("\n\n") {
            self.output.pop();
        }
        self.newline();
        self.output
    }
}
fn ends_expression(kind: &K) -> bool {
    matches!(
        kind,
        K::Identifier(_)
            | K::Integer(_)
            | K::Float(_)
            | K::String(_)
            | K::True
            | K::False
            | K::RightParen
            | K::RightBracket
            | K::RightBrace
    )
}
fn continues_expression(kind: &K) -> bool {
    matches!(
        kind,
        K::Else
            | K::Comma
            | K::Semicolon
            | K::RightParen
            | K::RightBracket
            | K::Dot
            | K::FatArrow
            | K::Plus
            | K::Minus
            | K::Star
            | K::Slash
            | K::Percent
            | K::Ampersand
            | K::Pipe
            | K::Caret
            | K::ShiftLeft
            | K::ShiftRight
            | K::EqualEqual
            | K::BangEqual
            | K::AndAnd
            | K::OrOr
            | K::Less
            | K::Greater
            | K::LessEqual
            | K::GreaterEqual
    )
}
fn needs_space(
    previous: &K,
    current: &K,
    tight_after: bool,
    previous_angle: bool,
    angle: bool,
) -> bool {
    if angle {
        return false;
    }
    if matches!(
        current,
        K::Comma
            | K::Semicolon
            | K::Colon
            | K::ColonColon
            | K::Dot
            | K::RightParen
            | K::RightBracket
    ) {
        return false;
    }
    if *current == K::RightBrace && *previous == K::LeftBrace {
        return false;
    }
    if matches!(
        previous,
        K::LeftParen | K::LeftBracket | K::ColonColon | K::Dot
    ) || tight_after
    {
        return false;
    }
    if *current == K::LeftParen && previous_angle {
        return false;
    }
    if *current == K::LeftParen
        && matches!(
            previous,
            K::Identifier(_) | K::Print | K::Flush | K::RightParen
        )
    {
        return false;
    }
    if *current == K::LeftBracket && ends_expression(previous) {
        return false;
    }
    true
}

#[cfg(test)]
#[path = "format/tests.rs"]
mod tests;
