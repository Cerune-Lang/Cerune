use crate::{diagnostic::Diagnostic, source::Span};

mod string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Import,
    As,
    Pub,
    Const,
    Enum,
    Match,
    FatArrow,
    ColonColon,
    Type,
    Fn,
    Return,
    Void,
    Print,
    Flush,
    Mut,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    True,
    False,

    Identifier(String),

    Integer(String),
    Float(String),
    /// エスケープを解釈した値。ソース上の表記はTokenのSpanで保持します。
    String(String),

    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    AndAnd,
    OrOr,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Colon,
    Comma,
    Dot,
    DotDot,

    Plus,
    Minus,
    Arrow,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    ShiftLeft,
    ShiftRight,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// エスケープ処理の失敗を含め、すべての字句位置を元のファイルへ対応付けます。
pub fn lex_source(source: &crate::source::SourceFile) -> Result<Vec<Token>, Diagnostic> {
    lex(source.text())
        .map(|mut tokens| {
            for token in &mut tokens {
                token.span = token.span.with_source(source.id());
            }
            tokens
        })
        .map_err(|error| match error.primary_span() {
            Some(span) => Diagnostic::new(error.message(), span.with_source(source.id())),
            None => error,
        })
}

pub fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comment
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            i += 2;

            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }

            continue;
        }

        let offset = i;

        let kind = match b {
            b'=' if bytes.get(i + 1) == Some(&b'>') => {
                i += 2;
                TokenKind::FatArrow
            }
            b'"' => {
                let (value, end) = string::lex_string(source, offset)?;
                i = end;
                TokenKind::String(value)
            }
            b'&' if bytes.get(i + 1) == Some(&b'&') => {
                i += 2;
                TokenKind::AndAnd
            }
            b'|' if bytes.get(i + 1) == Some(&b'|') => {
                i += 2;
                TokenKind::OrOr
            }
            b'%' => {
                i += 1;
                TokenKind::Percent
            }
            b'&' => {
                i += 1;
                TokenKind::Ampersand
            }
            b'|' => {
                i += 1;
                TokenKind::Pipe
            }
            b'^' => {
                i += 1;
                TokenKind::Caret
            }
            b'~' => {
                i += 1;
                TokenKind::Tilde
            }
            b'<' if bytes.get(i + 1) == Some(&b'<') => {
                i += 2;
                TokenKind::ShiftLeft
            }
            b'>' if bytes.get(i + 1) == Some(&b'>') => {
                i += 2;
                TokenKind::ShiftRight
            }
            b':' if bytes.get(i + 1) == Some(&b':') => {
                i += 2;
                TokenKind::ColonColon
            }
            b':' => {
                i += 1;
                TokenKind::Colon
            }

            b',' => {
                i += 1;
                TokenKind::Comma
            }

            b'.' if bytes.get(i + 1) == Some(&b'.') => {
                i += 2;
                TokenKind::DotDot
            }
            b'.' => {
                i += 1;
                TokenKind::Dot
            }

            b'=' => {
                i += 1;

                if bytes.get(i) == Some(&b'=') {
                    i += 1;
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }

            b'!' => {
                i += 1;

                if bytes.get(i) == Some(&b'=') {
                    i += 1;
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }

            b'<' => {
                i += 1;

                if bytes.get(i) == Some(&b'=') {
                    i += 1;
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }

            b'>' => {
                i += 1;

                if bytes.get(i) == Some(&b'=') {
                    i += 1;
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }

            b'+' => {
                i += 1;
                TokenKind::Plus
            }

            b'-' => {
                i += 1;
                if bytes.get(i) == Some(&b'>') {
                    i += 1;
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }

            b'*' => {
                i += 1;
                TokenKind::Star
            }

            b'/' => {
                i += 1;
                TokenKind::Slash
            }

            b'(' => {
                i += 1;
                TokenKind::LeftParen
            }

            b')' => {
                i += 1;
                TokenKind::RightParen
            }

            b'{' => {
                i += 1;
                TokenKind::LeftBrace
            }

            b'[' => {
                i += 1;
                TokenKind::LeftBracket
            }

            b']' => {
                i += 1;
                TokenKind::RightBracket
            }

            b'}' => {
                i += 1;
                TokenKind::RightBrace
            }

            b';' => {
                i += 1;
                TokenKind::Semicolon
            }

            // Numeric literal
            b'0'..=b'9' => {
                let start = i;
                let mut is_float = false;

                // Integer part
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }

                // Fractional part
                if bytes.get(i) == Some(&b'.') {
                    is_float = true;
                    i += 1;

                    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
                        return Err(Diagnostic::new(
                            "expected digit after decimal point",
                            Span::empty(i),
                        ));
                    }

                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                // Scientific notation
                if matches!(bytes.get(i), Some(b'e') | Some(b'E')) {
                    is_float = true;
                    i += 1;

                    if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
                        i += 1;
                    }

                    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
                        return Err(Diagnostic::new("expected exponent digits", Span::empty(i)));
                    }

                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }

                // Explicit floating-point suffix
                if bytes[i..].starts_with(b"f32") || bytes[i..].starts_with(b"f64") {
                    is_float = true;
                    i += 3;
                }
                if !is_float
                    && let Some(ty) = crate::types::IntegerType::ALL
                        .into_iter()
                        .find(|ty| bytes[i..].starts_with(ty.name().as_bytes()))
                {
                    i += ty.name().len();
                }

                // Prevent things such as:
                //
                //   1foo
                //   0.1f32abc
                //
                // from silently becoming multiple tokens.
                if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                    return Err(Diagnostic::new(
                        "invalid numeric literal",
                        Span::new(start, i),
                    ));
                }

                let text = &source[start..i];

                if is_float {
                    TokenKind::Float(text.to_owned())
                } else {
                    TokenKind::Integer(text.to_owned())
                }
            }

            // Identifier / keyword
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let start = i;
                i += 1;

                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }

                match &source[start..i] {
                    "type" => TokenKind::Type,
                    "fn" => TokenKind::Fn,
                    "import" => TokenKind::Import,
                    "as" => TokenKind::As,
                    "pub" => TokenKind::Pub,
                    "const" => TokenKind::Const,
                    "enum" => TokenKind::Enum,
                    "match" => TokenKind::Match,
                    "return" => TokenKind::Return,
                    "void" => TokenKind::Void,
                    "print" => TokenKind::Print,
                    "flush" => TokenKind::Flush,
                    "mut" => TokenKind::Mut,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    "break" => TokenKind::Break,
                    "continue" => TokenKind::Continue,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    name => TokenKind::Identifier(name.to_owned()),
                }
            }

            _ => {
                let character = source[i..].chars().next().unwrap();

                return Err(Diagnostic::new(
                    format!("unexpected character {character:?}"),
                    Span::new(i, i + character.len_utf8()),
                ));
            }
        };

        tokens.push(Token {
            kind,
            span: Span::new(offset, i),
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::empty(source.len()),
    });

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, lex};
    use crate::source::Span;

    #[test]
    fn lexes_minimal_program() {
        let tokens = lex("x: i64 = 1 + 2; print(x);").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Identifier("x".into()));
        assert_eq!(tokens[1].kind, TokenKind::Colon);
        assert_eq!(tokens[2].kind, TokenKind::Identifier("i64".into()));
        assert_eq!(tokens[3].kind, TokenKind::Equal);
        assert_eq!(tokens[4].kind, TokenKind::Integer("1".into()));
        assert_eq!(tokens[5].kind, TokenKind::Plus);
        assert_eq!(tokens[6].kind, TokenKind::Integer("2".into()));
        assert_eq!(tokens[8].kind, TokenKind::Print);
    }

    #[test]
    fn lexes_product_type_syntax() {
        let tokens =
            lex("type Point { x: f64, } point: Point = Point { x: 1.0, }; print(point.x);")
                .unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Type);
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Comma));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Dot));
    }

    #[test]
    fn lexes_fixed_array_syntax() {
        let tokens = lex("values: [i64; 2] = [1, 2]; print(values[0]);").unwrap();

        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.kind == TokenKind::LeftBracket)
                .count(),
            3
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.kind == TokenKind::RightBracket)
                .count(),
            3
        );
    }

    #[test]
    fn lexes_mutable_binding() {
        let tokens = lex("mut x: i64 = 1;").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Mut);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("x".into()));
    }

    #[test]
    fn lexes_boolean_literals_and_comparisons() {
        let tokens = lex("true == false != !false; 1 < 2 <= 3 > 2 >= 1;").unwrap();

        assert!(tokens.iter().any(|token| token.kind == TokenKind::True));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::False));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::EqualEqual)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::BangEqual)
        );
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Bang));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Less));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::LessEqual)
        );
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Greater));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::GreaterEqual)
        );
    }

    #[test]
    fn lexes_if_else_blocks() {
        let tokens = lex("if true { print(1); } else { print(2); }").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::If);
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Else));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::LeftBrace)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::RightBrace)
        );
    }

    #[test]
    fn lexes_while_block() {
        let tokens = lex("while true { break; continue; }").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::While);
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Break));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Continue));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::LeftBrace)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::RightBrace)
        );
    }

    #[test]
    fn lexes_for_block() {
        let tokens = lex("for (mut i: i64 = 0; i < 3; i = i + 1) { print(i); }").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::For);
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Mut));
    }

    #[test]
    fn lexes_function_definition_and_return() {
        let tokens = lex("fn show(value: i64) -> void { return; }").unwrap();

        for expected in [
            TokenKind::Fn,
            TokenKind::Arrow,
            TokenKind::Void,
            TokenKind::Return,
        ] {
            assert!(tokens.iter().any(|token| token.kind == expected));
        }
    }

    #[test]
    fn lexes_float_literals() {
        let tokens = lex("a: f32 = 0.1f32; \
             b: f64 = 0.2f64; \
             c: infer = 0.3;")
        .unwrap();

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("0.1f32".into()))
        );

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("0.2f64".into()))
        );

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("0.3".into()))
        );
    }

    #[test]
    fn lexes_integer_float_suffixes() {
        let tokens = lex("a: f32 = 1f32; b: f64 = 2f64;").unwrap();

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("1f32".into()))
        );

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("2f64".into()))
        );
    }

    #[test]
    fn keeps_integer_digits_beyond_i64() {
        let tokens = lex("18446744073709551615").unwrap();

        assert_eq!(
            tokens[0].kind,
            TokenKind::Integer("18446744073709551615".into())
        );
    }

    #[test]
    fn lexes_scientific_notation() {
        let tokens = lex("x: f64 = 1.5e-3;").unwrap();

        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Float("1.5e-3".into()))
        );
    }

    #[test]
    fn skips_line_comments() {
        let tokens = lex("x: i64 = 1; // hello\nprint(x);").unwrap();

        assert!(tokens.iter().any(|token| token.kind == TokenKind::Print));
    }

    #[test]
    fn rejects_invalid_numeric_suffix() {
        let error = lex("0.1foo").unwrap_err();

        assert_eq!(error.message(), "invalid numeric literal");
        assert_eq!(error.primary_span(), Some(Span::new(0, 3)));
    }

    #[test]
    fn records_token_spans() {
        let tokens = lex("x: i64").unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 1));
        assert_eq!(tokens[1].span, Span::new(1, 2));
        assert_eq!(tokens[2].span, Span::new(3, 6));
        assert_eq!(tokens[3].span, Span::empty(6));
    }

    #[test]
    fn reports_utf8_character_span() {
        let error = lex("あ").unwrap_err();

        assert_eq!(error.primary_span(), Some(Span::new(0, 3)));
    }

    #[test]
    fn reports_missing_exponent_digits() {
        let error = lex("1e").unwrap_err();

        assert_eq!(error.message(), "expected exponent digits");
        assert_eq!(error.primary_span(), Some(Span::empty(2)));
    }
}
