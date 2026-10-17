mod limits;
pub use limits::{MAX_EXPRESSION_DEPTH, MAX_SYNTAX_NESTING};

use crate::ast::{
    AssignmentProjection, AssignmentTarget, BinaryOp, Expr, ExprKind, FieldDefinition, FieldValue,
    FunctionDefinition, Item, IterationBinding, Parameter, Program, ReturnTypeRef, Stmt, StmtKind,
    Type, TypeDefinition, TypeRef, TypeRefKind, TypeSpec, UnaryOp,
};
use crate::ast::{GenericArgument, GenericCall, GenericParameter};
use crate::diagnostic::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::source::{ConversionSyntax, SourceId, Span};
use crate::types::ConversionMode;

type ParseResult<T> = Result<T, Diagnostic>;

/// ファイル単位の宣言。名前解決後にだけ従来の共通ASTへ渡します。
#[derive(Debug, Clone)]
pub struct Module {
    pub program: Program,
    pub imports: Vec<Import>,
    pub exports: Vec<(String, Span)>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, Diagnostic> {
    let module = parse_module(tokens)?;
    if let Some(import) = module.imports.first() {
        return Err(Diagnostic::new(
            "imports require modules::load with an entry file",
            import.span,
        ));
    }
    if let Some((_, span)) = module.exports.first() {
        return Err(Diagnostic::new(
            "public declarations require modules::load with an entry file",
            *span,
        ));
    }
    Ok(module.program)
}

pub fn parse_module(tokens: Vec<Token>) -> Result<Module, Diagnostic> {
    let source_id = tokens
        .first()
        .map_or(SourceId::ANONYMOUS, |token| token.span.source_id());
    if let Some(token) = tokens
        .iter()
        .find(|token| token.span.source_id() != source_id)
    {
        return Err(Diagnostic::new(
            "cannot parse tokens from different source files together",
            token.span,
        ));
    }
    limits::delimiters(&tokens)?;
    Parser {
        source_id,
        tokens,
        current: 0,
        expression_nesting: 0,
        block_nesting: 0,
        allow_construct: true,
    }
    .parse_program()
}

struct Parser {
    source_id: SourceId,
    tokens: Vec<Token>,
    current: usize,
    expression_nesting: usize,
    block_nesting: usize,
    allow_construct: bool,
}

impl Parser {
    fn span(&self, start: usize, end: usize) -> Span {
        Span::in_source(self.source_id, start, end)
    }
    fn parse_program(&mut self) -> ParseResult<Module> {
        let mut items = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        while !matches!(&self.peek().kind, TokenKind::Eof) {
            if matches!(self.peek().kind, TokenKind::Import) {
                if !items.is_empty() {
                    return Err(
                        self.error("imports must precede definitions and statements".into())
                    );
                }
                let start = self.advance().span.start();
                let token = self.advance().clone();
                let TokenKind::String(path) = token.kind else {
                    return Err(Diagnostic::new(
                        "expected a quoted relative import path",
                        token.span,
                    ));
                };
                self.expect_simple(TokenKind::As)?;
                let (alias, _) = self.expect_identifier()?;
                let end = self.expect_simple(TokenKind::Semicolon)?.end();
                imports.push(Import {
                    path,
                    alias,
                    span: self.span(start, end),
                });
                continue;
            }
            let public = matches!(self.peek().kind, TokenKind::Pub);
            if public {
                self.advance();
                if !matches!(
                    self.peek().kind,
                    TokenKind::Fn | TokenKind::Type | TokenKind::Const | TokenKind::Enum
                ) {
                    return Err(self.error(
                        "pub is only supported on functions, types, enums, and constants".into(),
                    ));
                }
                let token = self.peek_next().clone();
                if let TokenKind::Identifier(name) = token.kind {
                    exports.push((name, token.span));
                }
            }
            if matches!(&self.peek().kind, TokenKind::Enum) {
                items.push(Item::TypeDefinition(self.parse_enum_definition()?));
            } else if matches!(&self.peek().kind, TokenKind::Const) {
                items.push(Item::ConstantDefinition(self.parse_constant_definition()?));
            } else if matches!(&self.peek().kind, TokenKind::Type) {
                items.push(Item::TypeDefinition(self.parse_type_definition()?));
            } else if matches!(&self.peek().kind, TokenKind::Fn) {
                items.push(Item::FunctionDefinition(self.parse_function_definition()?));
            } else {
                items.push(Item::Statement(self.parse_statement()?));
            }
        }

        Ok(Module {
            program: Program { items },
            imports,
            exports,
        })
    }

    fn parse_enum_definition(&mut self) -> ParseResult<TypeDefinition> {
        let start = self.advance().span.start();
        let (name, name_span) = self.expect_identifier()?;
        self.expect_simple(TokenKind::LeftBrace)?;
        let mut variants = Vec::new();
        while !matches!(self.peek().kind, TokenKind::RightBrace) {
            let (variant, variant_span) = self.expect_identifier()?;
            let mut fields = Vec::new();
            let mut end = variant_span.end();
            if matches!(self.peek().kind, TokenKind::LeftBrace) {
                self.advance();
                while !matches!(self.peek().kind, TokenKind::RightBrace) {
                    let (field, span) = self.expect_identifier()?;
                    self.expect_simple(TokenKind::Colon)?;
                    let type_ref = self.parse_type_ref()?;
                    if type_ref.is_named("infer") {
                        return Err(Diagnostic::new(
                            "variant fields require an explicit type",
                            type_ref.span,
                        ));
                    }
                    fields.push(FieldDefinition {
                        name: field,
                        name_span: span,
                        span: self.span(span.start(), type_ref.span.end()),
                        type_ref,
                        default: None,
                    });
                    if !matches!(self.peek().kind, TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                end = self.expect_simple(TokenKind::RightBrace)?.end();
            }
            variants.push(crate::ast::VariantDefinition {
                name: variant,
                name_span: variant_span,
                fields,
                span: self.span(variant_span.start(), end),
            });
            if !matches!(self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        let end = self.expect_simple(TokenKind::RightBrace)?.end();
        if variants.is_empty() {
            return Err(Diagnostic::new(
                "enum must have at least one variant",
                self.span(start, end),
            ));
        }
        Ok(TypeDefinition {
            name,
            name_span,
            fields: Vec::new(),
            variants: Some(variants),
            span: self.span(start, end),
        })
    }

    fn parse_match(&mut self) -> ParseResult<Stmt> {
        let (start, value, arms, end) = self.parse_match_parts(false)?;
        Ok(Stmt {
            kind: StmtKind::Match { value, arms },
            span: self.span(start, end),
        })
    }
    fn parse_match_parts(
        &mut self,
        expression: bool,
    ) -> ParseResult<(usize, Expr, Vec<crate::ast::MatchArm>, usize)> {
        let start = self.advance().span.start();
        let value = self.parse_block_condition()?;
        self.expect_simple(TokenKind::LeftBrace)?;
        let mut arms = Vec::new();
        while !matches!(self.peek().kind, TokenKind::RightBrace) {
            let (variant, variant_span) = self.expect_path()?;
            self.expect_simple(TokenKind::LeftBrace)?;
            let mut fields = Vec::new();
            while !matches!(self.peek().kind, TokenKind::RightBrace) {
                let (name, name_span) = self.expect_identifier()?;
                self.expect_simple(TokenKind::Colon)?;
                let (binding, binding_span) = self.expect_identifier()?;
                fields.push(crate::ast::PatternField {
                    name,
                    name_span,
                    binding,
                    binding_span,
                });
                if !matches!(self.peek().kind, TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect_simple(TokenKind::RightBrace)?;
            let guard = if matches!(self.peek().kind, TokenKind::If) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect_simple(TokenKind::FatArrow)?;
            let (body, result, end) = if expression {
                let value = self.parse_expression()?;
                let end = value.span.end();
                (Vec::new(), Some(Box::new(value)), end)
            } else {
                let (body, end) = self.parse_block()?;
                (body, None, end)
            };
            arms.push(crate::ast::MatchArm {
                guard,
                result,
                variant,
                variant_span,
                fields,
                body,
                span: self.span(variant_span.start(), end),
            });
            if !matches!(self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
        }
        let end = self.expect_simple(TokenKind::RightBrace)?.end();
        Ok((start, value, arms, end))
    }

    fn parse_constant_definition(&mut self) -> ParseResult<crate::ast::ConstantDefinition> {
        let start = self.advance().span.start();
        let (name, name_span) = self.expect_identifier()?;
        self.expect_simple(TokenKind::Colon)?;
        let type_ref = self.parse_type_ref()?;
        if type_ref.is_named("infer") {
            return Err(Diagnostic::new(
                "constants require an explicit type",
                type_ref.span,
            ));
        }
        self.expect_simple(TokenKind::Equal)?;
        let value = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::Semicolon)?.end();
        Ok(crate::ast::ConstantDefinition {
            name,
            name_span,
            type_ref,
            value,
            span: self.span(start, end),
        })
    }

    fn parse_type_definition(&mut self) -> ParseResult<TypeDefinition> {
        let start = self.advance().span.start();
        let (name, name_span) = self.expect_identifier()?;
        let opening = self.expect_simple(TokenKind::LeftBrace)?;

        if matches!(&self.peek().kind, TokenKind::RightBrace) {
            let closing = self.advance().span;
            return Err(Diagnostic::new(
                "product type must have at least one field",
                self.span(opening.start(), closing.end()),
            ));
        }

        let mut fields = Vec::new();

        loop {
            let (field_name, field_name_span) = self.expect_identifier()?;
            self.expect_simple(TokenKind::Colon)?;
            let type_ref = self.parse_type_ref()?;

            if type_ref.is_named("infer") {
                return Err(Diagnostic::new(
                    "product type fields require an explicit type",
                    type_ref.span,
                ));
            }

            let default = if matches!(&self.peek().kind, TokenKind::Equal) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };
            let end = default
                .as_ref()
                .map_or(type_ref.span.end(), |value| value.span.end());

            fields.push(FieldDefinition {
                name: field_name,
                name_span: field_name_span,
                type_ref,
                default,
                span: self.span(field_name_span.start(), end),
            });

            if matches!(&self.peek().kind, TokenKind::Comma) {
                self.advance();
                if matches!(&self.peek().kind, TokenKind::RightBrace) {
                    break;
                }
            } else {
                break;
            }
        }

        let closing = self.expect_simple(TokenKind::RightBrace)?;

        Ok(TypeDefinition {
            variants: None,
            name,
            name_span,
            fields,
            span: self.span(start, closing.end()),
        })
    }

    fn parse_function_definition(&mut self) -> ParseResult<FunctionDefinition> {
        let start = self.advance().span.start();
        let (name, name_span) = self.expect_identifier()?;
        let generic_parameters = self.parse_generic_parameters()?;
        self.expect_simple(TokenKind::LeftParen)?;
        let mut parameters = Vec::new();

        while !matches!(&self.peek().kind, TokenKind::RightParen) {
            let (parameter_name, parameter_name_span) = self.expect_identifier()?;
            self.expect_simple(TokenKind::Colon)?;
            let type_ref = self.parse_type_ref()?;
            if type_ref.is_named("infer") {
                return Err(Diagnostic::new(
                    "function parameters require an explicit type",
                    type_ref.span,
                ));
            }
            parameters.push(Parameter {
                name: parameter_name,
                name_span: parameter_name_span,
                span: self.span(parameter_name_span.start(), type_ref.span.end()),
                type_ref,
            });

            if matches!(&self.peek().kind, TokenKind::Comma) {
                self.advance();
                if matches!(&self.peek().kind, TokenKind::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }

        self.expect_simple(TokenKind::RightParen)?;
        self.expect_simple(TokenKind::Arrow)?;
        let return_type = if matches!(&self.peek().kind, TokenKind::Void) {
            ReturnTypeRef::Void(self.advance().span)
        } else {
            let type_ref = self.parse_type_ref()?;
            if type_ref.is_named("infer") {
                return Err(Diagnostic::new(
                    "function return types require an explicit type",
                    type_ref.span,
                ));
            }
            ReturnTypeRef::Value(type_ref)
        };
        let (body, end) = self.parse_block()?;

        Ok(FunctionDefinition {
            generic_parameters,
            name,
            name_span,
            parameters,
            return_type,
            body,
            span: self.span(start, end),
        })
    }

    fn parse_generic_parameters(&mut self) -> ParseResult<Vec<GenericParameter>> {
        let mut parameters = Vec::new();
        if !matches!(self.peek().kind, TokenKind::Less) {
            return Ok(parameters);
        }
        self.advance();
        loop {
            let length = matches!(self.peek().kind, TokenKind::Const);
            if length {
                self.advance();
            }
            let (name, span) = self.expect_identifier()?;
            if length {
                self.expect_simple(TokenKind::Colon)?;
                let ty = self.parse_type_ref()?;
                if !ty.is_named("i64") {
                    return Err(Diagnostic::new(
                        "generic lengths require const N: i64",
                        ty.span,
                    ));
                }
            }
            parameters.push(GenericParameter { name, span, length });
            if !matches!(self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
            if matches!(self.peek().kind, TokenKind::Greater) {
                break;
            }
        }
        self.expect_simple(TokenKind::Greater)?;
        Ok(parameters)
    }
    fn parse_generic_call(&mut self, name: String, name_span: Span) -> ParseResult<Expr> {
        self.expect_simple(TokenKind::ColonColon)?;
        self.expect_simple(TokenKind::Less)?;
        let mut generic_arguments = Vec::new();
        loop {
            let argument = if let TokenKind::Integer(digits) = &self.peek().kind {
                let value = parse_array_length(digits, self.peek().span)?;
                GenericArgument::Length {
                    value,
                    span: self.advance().span,
                }
            } else {
                GenericArgument::Type(self.parse_type_ref()?)
            };
            generic_arguments.push(argument);
            if !matches!(self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
            if matches!(self.peek().kind, TokenKind::Greater) {
                break;
            }
        }
        self.expect_simple(TokenKind::Greater)?;
        let expr = self.parse_call(name, name_span)?;
        let ExprKind::Call {
            name,
            name_span,
            arguments,
        } = expr.kind
        else {
            unreachable!()
        };
        Ok(Expr {
            span: expr.span,
            kind: ExprKind::GenericCall(Box::new(GenericCall {
                name,
                name_span,
                generic_arguments,
                arguments,
            })),
        })
    }

    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        match &self.peek().kind {
            TokenKind::Match => self.parse_match(),
            TokenKind::Mut => self.parse_binding(),
            TokenKind::Identifier(_) => match &self.peek_next().kind {
                TokenKind::Colon => self.parse_binding(),
                TokenKind::ColonColon => self.parse_call_statement(),
                TokenKind::Equal | TokenKind::LeftBracket => self.parse_assignment(),
                TokenKind::LeftParen => self.parse_call_statement(),
                TokenKind::Dot => Err(Diagnostic::new(
                    "fields cannot be assigned directly; construct a new value and reassign the whole mutable binding",
                    self.peek_next().span,
                )),
                other => Err(Diagnostic::new(
                    format!("expected `:` or `=` after identifier, found {other:?}"),
                    self.peek_next().span,
                )),
            },
            TokenKind::Flush => {
                let start = self.advance().span.start();
                self.expect_simple(TokenKind::LeftParen)?;
                self.expect_simple(TokenKind::RightParen)?;
                let end = self.expect_simple(TokenKind::Semicolon)?.end();
                Ok(Stmt {
                    kind: StmtKind::Flush,
                    span: self.span(start, end),
                })
            }
            TokenKind::Print => self.parse_print(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Break => self.parse_loop_control(StmtKind::Break),
            TokenKind::Continue => self.parse_loop_control(StmtKind::Continue),
            other => Err(self.error(format!("expected statement, found {other:?}"))),
        }
    }

    fn parse_binding(&mut self) -> ParseResult<Stmt> {
        let (mutable, start) = if matches!(&self.peek().kind, TokenKind::Mut) {
            (true, self.advance().span.start())
        } else {
            (false, self.peek().span.start())
        };

        let token = self.advance().clone();

        let name = match token.kind {
            TokenKind::Identifier(name) => name,
            other => {
                return Err(Diagnostic::new(
                    format!("expected identifier, found {other:?}"),
                    token.span,
                ));
            }
        };

        self.expect_simple(TokenKind::Colon)?;
        let type_spec = self.parse_type_spec()?;
        self.expect_simple(TokenKind::Equal)?;
        let value = self.parse_expression()?;
        let semicolon = self.expect_simple(TokenKind::Semicolon)?;

        Ok(Stmt {
            kind: StmtKind::Binding {
                mutable,
                name,
                type_spec,
                value,
            },
            span: self.span(start, semicolon.end()),
        })
    }

    fn parse_assignment(&mut self) -> ParseResult<Stmt> {
        let mut statement = self.parse_assignment_clause()?;
        let semicolon = self.expect_simple(TokenKind::Semicolon)?;
        statement.span = self.span(statement.span.start(), semicolon.end());
        Ok(statement)
    }

    fn parse_assignment_clause(&mut self) -> ParseResult<Stmt> {
        let token = self.advance().clone();
        let start = token.span.start();
        let name_span = token.span;

        let name = match token.kind {
            TokenKind::Identifier(name) => name,
            other => {
                return Err(Diagnostic::new(
                    format!("expected identifier, found {other:?}"),
                    token.span,
                ));
            }
        };

        let mut projections = Vec::new();
        let mut target_end = name_span.end();
        while matches!(&self.peek().kind, TokenKind::LeftBracket) {
            let projection_start = self.advance().span.start();
            let index = self.parse_expression()?;
            let end = self.expect_simple(TokenKind::RightBracket)?.end();
            projections.push(AssignmentProjection::Index {
                index,
                span: self.span(projection_start, end),
            });
            target_end = end;
        }

        if matches!(&self.peek().kind, TokenKind::Dot) {
            return Err(Diagnostic::new(
                "fields cannot be assigned directly; construct a new value and reassign the whole mutable binding",
                self.peek().span,
            ));
        }

        self.expect_simple(TokenKind::Equal)?;
        let value = self.parse_expression()?;
        let end = value.span.end();

        Ok(Stmt {
            kind: StmtKind::Assignment {
                target: AssignmentTarget {
                    name,
                    name_span,
                    projections,
                    span: self.span(start, target_end),
                },
                value,
            },
            span: self.span(start, end),
        })
    }

    fn parse_type_spec(&mut self) -> ParseResult<TypeSpec> {
        let type_ref = self.parse_type_ref()?;

        if type_ref.is_named("infer") {
            Ok(TypeSpec::Infer)
        } else {
            Ok(TypeSpec::Explicit(type_ref))
        }
    }

    fn parse_type_ref(&mut self) -> ParseResult<TypeRef> {
        if matches!(&self.peek().kind, TokenKind::LeftBracket) {
            let start = self.advance().span.start();
            let element = self.parse_type_ref()?;
            if element.is_named("infer") {
                return Err(Diagnostic::new(
                    "array element type must be written explicitly",
                    element.span,
                ));
            }
            if matches!(self.peek().kind, TokenKind::RightBracket) {
                let end = self.advance().span.end();
                return Ok(TypeRef {
                    kind: TypeRefKind::DynamicArray {
                        element: Box::new(element),
                    },
                    span: self.span(start, end),
                });
            }
            self.expect_simple(TokenKind::Semicolon)?;
            if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                let (name, name_span) = self.expect_path()?;
                let end = self.expect_simple(TokenKind::RightBracket)?.end();
                return Ok(TypeRef {
                    kind: TypeRefKind::ArrayConstant {
                        element: Box::new(element),
                        constant: Box::new(crate::ast::ArrayLengthRef {
                            name,
                            span: name_span,
                        }),
                    },
                    span: self.span(start, end),
                });
            }
            let length_token = self.advance().clone();
            let length = match length_token.kind {
                TokenKind::Integer(digits) => parse_array_length(&digits, length_token.span)?,
                other => {
                    return Err(Diagnostic::new(
                        format!("expected positive integer array length, found {other:?}"),
                        length_token.span,
                    ));
                }
            };
            let end = self.expect_simple(TokenKind::RightBracket)?.end();
            Ok(TypeRef {
                kind: TypeRefKind::Array {
                    element: Box::new(element),
                    length,
                },
                span: self.span(start, end),
            })
        } else {
            let (name, span) = self.expect_path()?;
            Ok(TypeRef {
                kind: TypeRefKind::Named(name),
                span,
            })
        }
    }

    fn parse_print(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start();

        self.expect_simple(TokenKind::LeftParen)?;
        let value = self.parse_expression()?;
        self.expect_simple(TokenKind::RightParen)?;
        let semicolon = self.expect_simple(TokenKind::Semicolon)?;

        Ok(Stmt {
            kind: StmtKind::Print { value },
            span: self.span(start, semicolon.end()),
        })
    }

    fn parse_call_statement(&mut self) -> ParseResult<Stmt> {
        let value = self.parse_expression()?;
        let semicolon = self.expect_simple(TokenKind::Semicolon)?;
        let span = self.span(value.span.start(), semicolon.end());
        if !matches!(value.kind, ExprKind::Call { .. } | ExprKind::GenericCall(_)) {
            return Err(Diagnostic::new(
                "only a function call can be used as an expression statement",
                value.span,
            ));
        }
        Ok(Stmt {
            kind: StmtKind::Call { value },
            span,
        })
    }

    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start();
        let value = if matches!(&self.peek().kind, TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        let end = self.expect_simple(TokenKind::Semicolon)?.end();
        Ok(Stmt {
            kind: StmtKind::Return { value },
            span: self.span(start, end),
        })
    }

    fn parse_expression(&mut self) -> ParseResult<Expr> {
        if self.expression_nesting + self.block_nesting == MAX_SYNTAX_NESTING {
            return Err(self.error(format!("expression nesting exceeds {MAX_SYNTAX_NESTING}")));
        }
        self.expression_nesting += 1;
        let result = self.parse_logical_or();
        self.expression_nesting -= 1;
        let expr = result?;
        limits::expression(&expr, self.block_nesting)?;
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_logical_and()?;
        while matches!(self.peek().kind, TokenKind::OrOr) {
            self.advance();
            let right = self.parse_logical_and()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Logical {
                    op: crate::ast::LogicalOp::Or,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bit_or()?;
        while matches!(self.peek().kind, TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_bit_or()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Logical {
                    op: crate::ast::LogicalOp::And,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_block_condition(&mut self) -> ParseResult<Expr> {
        let previous = self.allow_construct;
        self.allow_construct = false;
        let result = self.parse_expression();
        self.allow_construct = previous;
        result
    }

    fn parse_if(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start();
        let condition = self.parse_block_condition()?;
        let (then_body, then_end) = self.parse_block()?;

        let (else_body, end) = if matches!(&self.peek().kind, TokenKind::Else) {
            self.advance();
            let (body, end) = self.parse_block()?;
            (body, end)
        } else {
            (Vec::new(), then_end)
        };

        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_body,
                else_body,
            },
            span: self.span(start, end),
        })
    }

    fn parse_while(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start();
        let condition = self.parse_block_condition()?;
        let (body, end) = self.parse_block()?;

        Ok(Stmt {
            kind: StmtKind::While { condition, body },
            span: self.span(start, end),
        })
    }

    fn parse_for(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start();
        if !matches!(&self.peek().kind, TokenKind::LeftParen) {
            return Err(Diagnostic::new(
                "expected `(` after `for`",
                self.peek().span,
            ));
        }
        self.advance();

        let initializer = match (&self.peek().kind, &self.peek_next().kind) {
            (TokenKind::Mut, _) | (TokenKind::Identifier(_), TokenKind::Colon) => {
                let header = self.parse_iteration_binding()?;
                if matches!(self.peek().kind, TokenKind::In | TokenKind::Comma) {
                    return self.parse_for_each(start, header);
                }
                self.expect_simple(TokenKind::Equal)?;
                let value = self.parse_expression()?;
                let end = self.expect_simple(TokenKind::Semicolon)?.end();
                Stmt {
                    kind: StmtKind::Binding {
                        mutable: header.mutable,
                        name: header.name,
                        type_spec: header.type_spec,
                        value,
                    },
                    span: self.span(header.span.start(), end),
                }
            }
            (TokenKind::Identifier(_), TokenKind::Equal) => self.parse_assignment()?,
            _ => {
                return Err(Diagnostic::new(
                    "expected a binding or assignment at the start of `for`",
                    self.peek().span,
                ));
            }
        };

        let condition = self.parse_expression()?;
        self.expect_simple(TokenKind::Semicolon)?;

        let update = match (&self.peek().kind, &self.peek_next().kind) {
            (TokenKind::Identifier(_), TokenKind::Equal) => self.parse_assignment_clause()?,
            _ => {
                return Err(Diagnostic::new(
                    "for update must be an assignment",
                    self.peek().span,
                ));
            }
        };
        if !matches!(&self.peek().kind, TokenKind::RightParen) {
            return Err(Diagnostic::new(
                "expected `)` after the `for` update",
                self.peek().span,
            ));
        }
        self.advance();

        let (body, end) = self.parse_block()?;

        Ok(Stmt {
            kind: StmtKind::For {
                initializer: Box::new(initializer),
                condition,
                update: Box::new(update),
                body,
            },
            span: self.span(start, end),
        })
    }

    fn parse_iteration_binding(&mut self) -> ParseResult<IterationBinding> {
        let start = self.peek().span.start();
        let mutable = matches!(self.peek().kind, TokenKind::Mut);
        if mutable {
            self.advance();
        }
        let (name, _) = self.expect_identifier()?;
        self.expect_simple(TokenKind::Colon)?;
        let type_spec = self.parse_type_spec()?;
        let span = self.span(start, self.tokens[self.current - 1].span.end());
        Ok(IterationBinding {
            mutable,
            name,
            type_spec,
            span,
        })
    }

    fn parse_for_each(&mut self, start: usize, first: IterationBinding) -> ParseResult<Stmt> {
        let (index, element) = if matches!(self.peek().kind, TokenKind::Comma) {
            self.advance();
            (Some(first), self.parse_iteration_binding()?)
        } else {
            (None, first)
        };
        self.expect_simple(TokenKind::In)?;
        let value = self.parse_expression()?;
        let close = self.expect_simple(TokenKind::RightParen)?;
        let header_span = self.span(start, close.end());
        let (body, end) = self.parse_block()?;
        Ok(Stmt {
            kind: StmtKind::ForEach {
                index,
                element,
                value,
                body,
                header_span,
            },
            span: self.span(start, end),
        })
    }

    fn parse_loop_control(&mut self, kind: StmtKind) -> ParseResult<Stmt> {
        let start = self.advance().span.start();
        let end = self.expect_simple(TokenKind::Semicolon)?.end();

        Ok(Stmt {
            kind,
            span: self.span(start, end),
        })
    }

    fn parse_block(&mut self) -> ParseResult<(Vec<Stmt>, usize)> {
        self.block_nesting += 1;
        let result = self.parse_block_inner();
        self.block_nesting -= 1;
        result
    }

    fn parse_block_inner(&mut self) -> ParseResult<(Vec<Stmt>, usize)> {
        self.expect_simple(TokenKind::LeftBrace)?;
        let mut statements = Vec::new();

        while !matches!(&self.peek().kind, TokenKind::RightBrace | TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }

        let closing = self.expect_simple(TokenKind::RightBrace)?;
        Ok((statements, closing.end()))
    }

    fn parse_bit_or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bit_xor()?;
        while matches!(&self.peek().kind, TokenKind::Pipe) {
            let op = BinaryOp::BitOr;
            self.advance();
            let right = self.parse_bit_xor()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_bit_xor(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bit_and()?;
        while matches!(&self.peek().kind, TokenKind::Caret) {
            let op = BinaryOp::BitXor;
            self.advance();
            let right = self.parse_bit_and()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_bit_and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_equality()?;
        while matches!(&self.peek().kind, TokenKind::Ampersand) {
            let op = BinaryOp::BitAnd;
            self.advance();
            let right = self.parse_equality()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_shift(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::ShiftLeft => BinaryOp::ShiftLeft,
                TokenKind::ShiftRight => BinaryOp::ShiftRight,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            let span = self.span(expr.span.start(), right.span.end());
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::BangEqual => BinaryOp::NotEqual,
                _ => break,
            };

            self.advance();

            let right = self.parse_comparison()?;
            let span = self.span(expr.span.start(), right.span.end());

            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_shift()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                _ => break,
            };

            self.advance();

            let right = self.parse_shift()?;
            let span = self.span(expr.span.start(), right.span.end());

            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => break,
            };

            self.advance();

            let right = self.parse_multiplicative()?;

            let span = self.span(expr.span.start(), right.span.end());

            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Percent => BinaryOp::Remainder,
                _ => break,
            };

            self.advance();

            let right = self.parse_unary()?;
            let span = self.span(expr.span.start(), right.span.end());

            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let mut operators = Vec::new();
        loop {
            let op = match self.peek().kind {
                TokenKind::Minus => UnaryOp::Negate,
                TokenKind::Bang => UnaryOp::Not,
                TokenKind::Tilde => UnaryOp::BitNot,
                _ => break,
            };
            if operators.len() == MAX_EXPRESSION_DEPTH - 1 {
                return Err(self.error(format!(
                    "expression tree depth exceeds {MAX_EXPRESSION_DEPTH} nodes"
                )));
            }
            operators.push((op, self.advance().span));
        }
        let mut expr = self.parse_postfix()?;
        for (op, operator_span) in operators.into_iter().rev() {
            let span = self.span(operator_span.start(), expr.span.end());
            expr = Expr {
                kind: ExprKind::Unary {
                    op,
                    value: Box::new(expr),
                },
                span,
            };
            limits::expression(&expr, self.block_nesting)?;
        }
        Ok(expr)
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if matches!(&self.peek().kind, TokenKind::Dot) {
                self.advance();
                let (field_name, field_name_span) = self.expect_identifier()?;
                let span = self.span(expr.span.start(), field_name_span.end());
                expr = Expr {
                    kind: ExprKind::FieldAccess {
                        base: Box::new(expr),
                        field_name,
                        field_name_span,
                    },
                    span,
                };
                limits::expression(&expr, self.block_nesting)?;
            } else if matches!(&self.peek().kind, TokenKind::LeftBracket) {
                self.advance();
                let index = self.parse_expression()?;
                let end = self.expect_simple(TokenKind::RightBracket)?.end();
                let span = self.span(expr.span.start(), end);
                expr = Expr {
                    kind: ExprKind::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                };
                limits::expression(&expr, self.block_nesting)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        if matches!(self.peek().kind, TokenKind::Match) {
            let (start, value, arms, end) = self.parse_match_parts(true)?;
            return Ok(Expr {
                kind: ExprKind::Match {
                    value: Box::new(value),
                    arms,
                },
                span: self.span(start, end),
            });
        }
        let token = self.advance().clone();
        let span = token.span;

        match token.kind {
            TokenKind::True => Ok(Expr {
                kind: ExprKind::Boolean(true),
                span,
            }),

            TokenKind::False => Ok(Expr {
                kind: ExprKind::Boolean(false),
                span,
            }),

            TokenKind::Integer(text) => {
                let literal = crate::types::IntegerType::ALL
                    .into_iter()
                    .find_map(|ty| {
                        text.strip_suffix(ty.name())
                            .map(|digits| crate::ast::IntegerLiteral::with_type(digits, ty))
                    })
                    .unwrap_or_else(|| crate::ast::IntegerLiteral::decimal(text));
                Ok(Expr {
                    kind: ExprKind::Integer(literal),
                    span,
                })
            }

            TokenKind::Float(text) => Ok(parse_float_literal(text, span)),

            TokenKind::String(value) => Ok(Expr {
                kind: ExprKind::String(value),
                span,
            }),

            TokenKind::Identifier(name) => {
                let (name, span) = self.finish_path(name, span)?;
                if matches!(&self.peek().kind, TokenKind::ColonColon) {
                    self.parse_generic_call(name, span)
                } else if ConversionMode::from_name(&name).is_some()
                    && self.starts_explicit_conversion()
                {
                    self.expect_simple(TokenKind::Less)?;
                    let target = self.parse_type_ref()?;
                    self.expect_simple(TokenKind::Greater)?;
                    self.parse_conversion(
                        target,
                        ConversionSyntax::Explicit,
                        ConversionMode::from_name(&name).unwrap(),
                        span.start(),
                    )
                } else if Type::from_name(&name).is_some()
                    && matches!(&self.peek().kind, TokenKind::LeftParen)
                {
                    let target = TypeRef {
                        kind: TypeRefKind::Named(name),
                        span,
                    };
                    self.parse_conversion(
                        target,
                        ConversionSyntax::Compact,
                        ConversionMode::Exact,
                        span.start(),
                    )
                } else if matches!(&self.peek().kind, TokenKind::LeftParen) {
                    self.parse_call(name, span)
                } else if self.starts_construct() {
                    self.parse_construct(name, span)
                } else {
                    Ok(Expr {
                        kind: ExprKind::Variable(name),
                        span,
                    })
                }
            }

            TokenKind::LeftParen => {
                let previous = self.allow_construct;
                self.allow_construct = true;
                let result = self.parse_expression();
                self.allow_construct = previous;
                let expr = result?;
                let closing_span = self.expect_simple(TokenKind::RightParen)?;
                let span = self.span(span.start(), closing_span.end());

                Ok(Expr { span, ..expr })
            }

            TokenKind::LeftBracket => self.parse_array(span),

            other => Err(Diagnostic::new(
                format!("expected expression, found {other:?}"),
                token.span,
            )),
        }
    }

    fn starts_explicit_conversion(&mut self) -> bool {
        if !matches!(&self.peek().kind, TokenKind::Less) {
            return false;
        }
        // 通常の`convert < limit`を比較として残すため、型引数の形を先読みします。
        let saved = self.current;
        self.advance();
        let is_conversion = self.parse_type_ref().is_ok()
            && matches!(&self.peek().kind, TokenKind::Greater)
            && matches!(&self.peek_next().kind, TokenKind::LeftParen);
        self.current = saved;
        is_conversion
    }

    fn parse_conversion(
        &mut self,
        target: TypeRef,
        syntax: ConversionSyntax,
        mode: ConversionMode,
        start: usize,
    ) -> ParseResult<Expr> {
        self.expect_simple(TokenKind::LeftParen)?;
        if matches!(&self.peek().kind, TokenKind::RightParen) {
            return Err(Diagnostic::new(
                "conversion requires exactly one value",
                self.peek().span,
            ));
        }
        let value = self.parse_expression()?;
        if matches!(&self.peek().kind, TokenKind::Comma) {
            self.advance();
            if !matches!(&self.peek().kind, TokenKind::RightParen) {
                return Err(Diagnostic::new(
                    "conversion requires exactly one value",
                    self.peek().span,
                ));
            }
        }
        let end = self.expect_simple(TokenKind::RightParen)?.end();
        Ok(Expr {
            kind: ExprKind::Convert {
                mode,
                target,
                value: Box::new(value),
                syntax,
            },
            span: self.span(start, end),
        })
    }

    fn parse_call(&mut self, name: String, name_span: Span) -> ParseResult<Expr> {
        self.expect_simple(TokenKind::LeftParen)?;
        let mut arguments = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::RightParen) {
            arguments.push(self.parse_expression()?);
            if matches!(&self.peek().kind, TokenKind::Comma) {
                self.advance();
                if matches!(&self.peek().kind, TokenKind::RightParen) {
                    break;
                }
            } else {
                break;
            }
        }
        let closing = self.expect_simple(TokenKind::RightParen)?;
        Ok(Expr {
            kind: ExprKind::Call {
                name,
                name_span,
                arguments,
            },
            span: self.span(name_span.start(), closing.end()),
        })
    }

    fn parse_array(&mut self, opening_span: Span) -> ParseResult<Expr> {
        if matches!(&self.peek().kind, TokenKind::RightBracket) {
            let closing = self.advance().span;
            return Err(Diagnostic::new(
                "array literal must contain at least one value",
                self.span(opening_span.start(), closing.end()),
            ));
        }

        let mut values = Vec::new();
        loop {
            values.push(self.parse_expression()?);
            if matches!(&self.peek().kind, TokenKind::Comma) {
                self.advance();
                if matches!(&self.peek().kind, TokenKind::RightBracket) {
                    break;
                }
            } else {
                break;
            }
        }
        let closing = self.expect_simple(TokenKind::RightBracket)?;
        Ok(Expr {
            kind: ExprKind::Array(values),
            span: self.span(opening_span.start(), closing.end()),
        })
    }

    fn parse_construct(&mut self, type_name: String, type_name_span: Span) -> ParseResult<Expr> {
        let opening = self.expect_simple(TokenKind::LeftBrace)?;

        if matches!(&self.peek().kind, TokenKind::RightBrace) && !type_name.contains("::") {
            let closing = self.advance().span;
            return Err(Diagnostic::new(
                "aggregate literal must have at least one field",
                self.span(opening.start(), closing.end()),
            ));
        }

        // 元の値を先頭に限定し、表記順と評価順を一致させます。
        let base = if matches!(&self.peek().kind, TokenKind::DotDot) {
            self.advance();
            let value = self.parse_expression()?;
            if !matches!(&self.peek().kind, TokenKind::RightBrace) {
                self.expect_simple(TokenKind::Comma)?;
            }
            Some(Box::new(value))
        } else {
            None
        };
        let mut fields = Vec::new();

        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::DotDot) {
                return Err(Diagnostic::new(
                    "update base must appear once, before all fields",
                    self.peek().span,
                ));
            }
            let (name, name_span) = self.expect_identifier()?;
            self.expect_simple(TokenKind::Colon)?;
            let value = self.parse_expression()?;
            let span = self.span(name_span.start(), value.span.end());
            fields.push(FieldValue {
                generated: false,
                name,
                name_span,
                value,
                span,
            });

            if matches!(&self.peek().kind, TokenKind::Comma) {
                self.advance();
                if matches!(&self.peek().kind, TokenKind::RightBrace) {
                    break;
                }
            } else {
                break;
            }
        }

        let closing = self.expect_simple(TokenKind::RightBrace)?;

        Ok(Expr {
            kind: ExprKind::Construct {
                type_name,
                type_name_span,
                base,
                fields,
            },
            span: self.span(type_name_span.start(), closing.end()),
        })
    }

    fn expect_simple(&mut self, expected: TokenKind) -> ParseResult<Span> {
        let token = self.advance().clone();

        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&expected) {
            Ok(token.span)
        } else {
            Err(Diagnostic::new(
                format!("expected {expected:?}, found {:?}", token.kind),
                token.span,
            ))
        }
    }

    fn expect_identifier(&mut self) -> ParseResult<(String, Span)> {
        let token = self.advance().clone();

        match token.kind {
            TokenKind::Identifier(name) => Ok((name, token.span)),
            other => Err(Diagnostic::new(
                format!("expected identifier, found {other:?}"),
                token.span,
            )),
        }
    }

    fn expect_path(&mut self) -> ParseResult<(String, Span)> {
        let (name, span) = self.expect_identifier()?;
        self.finish_path(name, span)
    }

    fn finish_path(&mut self, mut name: String, mut span: Span) -> ParseResult<(String, Span)> {
        while matches!(self.peek().kind, TokenKind::ColonColon)
            && matches!(self.peek_next().kind, TokenKind::Identifier(_))
        {
            self.advance();
            let (member, end) = self.expect_identifier()?;
            name.push_str("::");
            name.push_str(&member);
            span = self.span(span.start(), end.end());
        }
        Ok((name, span))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> &Token {
        self.peek_n(1)
    }

    fn peek_n(&self, distance: usize) -> &Token {
        self.tokens
            .get(self.current + distance)
            .unwrap_or_else(|| self.peek())
    }

    fn starts_construct(&self) -> bool {
        self.allow_construct
            && matches!(&self.peek().kind, TokenKind::LeftBrace)
            && (matches!(
                &self.peek_n(1).kind,
                TokenKind::DotDot | TokenKind::RightBrace
            ) || (matches!(&self.peek_n(1).kind, TokenKind::Identifier(_))
                && matches!(&self.peek_n(2).kind, TokenKind::Colon)))
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;

        if !matches!(&self.tokens[self.current].kind, TokenKind::Eof) {
            self.current += 1;
        }

        &self.tokens[index]
    }

    fn error(&self, message: String) -> Diagnostic {
        Diagnostic::new(message, self.peek().span)
    }
}

fn parse_float_literal(text: String, span: Span) -> Expr {
    let kind = if let Some(value) = text.strip_suffix("f32") {
        ExprKind::Float {
            text: value.to_owned(),
            explicit_type: Some(Type::F32),
        }
    } else if let Some(value) = text.strip_suffix("f64") {
        ExprKind::Float {
            text: value.to_owned(),
            explicit_type: Some(Type::F64),
        }
    } else {
        ExprKind::Float {
            text,
            explicit_type: None,
        }
    };

    Expr { kind, span }
}

fn parse_array_length(digits: &str, span: Span) -> ParseResult<usize> {
    let value = digits
        .parse::<u64>()
        .map_err(|_| Diagnostic::new("array length is too large", span))?;

    if value == 0 {
        return Err(Diagnostic::new(
            "array length must be greater than zero",
            span,
        ));
    }

    if value > i64::MAX as u64 {
        return Err(Diagnostic::new("array length is too large", span));
    }

    usize::try_from(value).map_err(|_| Diagnostic::new("array length is too large", span))
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        BinaryOp, ExprKind, IntegerLiteral, Item, ReturnTypeRef, StmtKind, TypeRef, TypeRefKind,
        TypeSpec, UnaryOp,
    };
    use crate::lexer::lex;
    use crate::source::Span;

    use super::parse;

    #[test]
    fn parses_binding() {
        let program = parse(lex("x: i64 = 42;").unwrap()).unwrap();

        let StmtKind::Binding {
            mutable,
            name,
            type_spec,
            value,
        } = &program.statement(0).kind
        else {
            panic!("expected binding");
        };

        assert!(!mutable);
        assert_eq!(name, "x");
        assert_eq!(
            *type_spec,
            TypeSpec::Explicit(TypeRef {
                kind: TypeRefKind::Named("i64".into()),
                span: Span::new(3, 6),
            })
        );
        assert_eq!(value.kind, ExprKind::Integer(IntegerLiteral::decimal("42")));
        assert_eq!(value.span, Span::new(9, 11));
        assert_eq!(program.statement(0).span, Span::new(0, 12));
    }

    #[test]
    fn parses_product_type_construction_and_field_access() {
        let source = "type Point { x: f64, y: f64 = 0.0, } point: Point = Point { x: 1.0, }; print(point.y);";
        let program = parse(lex(source).unwrap()).unwrap();

        let Item::TypeDefinition(definition) = &program.items[0] else {
            panic!("expected type definition");
        };
        assert_eq!(definition.name, "Point");
        assert_eq!(definition.fields.len(), 2);
        assert!(definition.fields[0].default.is_none());
        assert!(definition.fields[1].default.is_some());

        let StmtKind::Binding {
            type_spec, value, ..
        } = &program.statement(0).kind
        else {
            panic!("expected binding");
        };
        assert!(matches!(
            type_spec,
            TypeSpec::Explicit(TypeRef {
                kind: TypeRefKind::Named(name),
                ..
            }) if name == "Point"
        ));
        assert!(matches!(value.kind, ExprKind::Construct { .. }));

        let StmtKind::Print { value } = &program.statement(1).kind else {
            panic!("expected print");
        };
        assert!(matches!(value.kind, ExprKind::FieldAccess { .. }));
    }

    #[test]
    fn parses_fixed_array_type_literal_and_index() {
        let program =
            parse(lex("values: [i64; 2] = [10, 20]; print(values[1]);").unwrap()).unwrap();

        let StmtKind::Binding {
            type_spec, value, ..
        } = &program.statement(0).kind
        else {
            panic!("expected binding");
        };
        assert!(matches!(
            type_spec,
            TypeSpec::Explicit(TypeRef {
                kind: TypeRefKind::Array { element, length: 2 },
                ..
            }) if element.is_named("i64")
        ));
        assert!(matches!(&value.kind, ExprKind::Array(values) if values.len() == 2));

        let StmtKind::Print { value } = &program.statement(1).kind else {
            panic!("expected print");
        };
        assert!(matches!(value.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn rejects_empty_array_literal() {
        let error = parse(lex("values: [i64; 1] = [];").unwrap()).unwrap_err();

        assert_eq!(
            error.message(),
            "array literal must contain at least one value"
        );
    }

    #[test]
    fn rejects_zero_length_array_type() {
        let error = parse(lex("values: [i64; 0] = [1];").unwrap()).unwrap_err();

        assert_eq!(error.message(), "array length must be greater than zero");
    }

    #[test]
    fn rejects_array_length_beyond_the_language_limit() {
        let error = parse(lex("values: [i64; 9223372036854775808] = [1];").unwrap()).unwrap_err();

        assert_eq!(error.message(), "array length is too large");
        assert_eq!(error.primary_span(), Some(Span::new(14, 33)));
    }

    #[test]
    fn rejects_inferred_array_element_type() {
        let error = parse(lex("values: [infer; 1] = [1];").unwrap()).unwrap_err();

        assert_eq!(
            error.message(),
            "array element type must be written explicitly"
        );
    }

    #[test]
    fn preserves_nested_array_type_structure() {
        let program =
            parse(lex("values: [[i64; 2]; 3] = [[1, 2], [3, 4], [5, 6]];").unwrap()).unwrap();

        let StmtKind::Binding { type_spec, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };
        let TypeSpec::Explicit(TypeRef {
            kind: TypeRefKind::Array { element, length: 3 },
            ..
        }) = type_spec
        else {
            panic!("expected outer array type");
        };
        assert!(matches!(
            &element.kind,
            TypeRefKind::Array {
                element: inner,
                length: 2,
            } if inner.is_named("i64")
        ));
    }

    #[test]
    fn parses_function_definition_call_and_return() {
        let program = parse(
            lex("fn identity(value: i64) -> i64 { return value; }
                 answer: i64 = identity(42);")
            .unwrap(),
        )
        .unwrap();

        let Item::FunctionDefinition(function) = &program.items[0] else {
            panic!("expected function definition");
        };
        assert_eq!(function.name, "identity");
        assert_eq!(function.parameters.len(), 1);
        assert_eq!(function.parameters[0].name, "value");
        assert!(matches!(function.return_type, ReturnTypeRef::Value(ref ty) if ty.is_named("i64")));
        assert!(matches!(function.body[0].kind, StmtKind::Return { .. }));

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };
        assert!(matches!(value.kind, ExprKind::Call { ref name, .. } if name == "identity"));
    }

    #[test]
    fn rejects_empty_product_type() {
        let error = parse(lex("type Empty {}").unwrap()).unwrap_err();

        assert_eq!(error.message(), "product type must have at least one field");
        assert_eq!(error.primary_span(), Some(Span::new(11, 13)));
    }

    #[test]
    fn parses_mutable_binding() {
        let program = parse(lex("mut x: i64 = 1;").unwrap()).unwrap();

        let StmtKind::Binding { mutable, name, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert!(*mutable);
        assert_eq!(name, "x");
        assert_eq!(program.statement(0).span, Span::new(0, 15));
    }

    #[test]
    fn parses_assignment() {
        let program = parse(lex("x = x + 1;").unwrap()).unwrap();

        let StmtKind::Assignment { target, value } = &program.statement(0).kind else {
            panic!("expected assignment");
        };

        assert_eq!(target.name, "x");
        assert_eq!(target.name_span, Span::new(0, 1));
        assert!(target.projections.is_empty());
        assert_eq!(value.span, Span::new(4, 9));
        assert_eq!(program.statement(0).span, Span::new(0, 10));
    }

    #[test]
    fn parses_nested_array_element_assignment() {
        let program = parse(lex("matrix[row][column] = 42;").unwrap()).unwrap();
        let StmtKind::Assignment { target, value } = &program.statement(0).kind else {
            panic!("expected assignment");
        };

        assert_eq!(target.name, "matrix");
        assert_eq!(target.projections.len(), 2);
        assert_eq!(target.span, Span::new(0, 19));
        assert_eq!(value.span, Span::new(22, 24));
    }

    #[test]
    fn explains_that_fields_cannot_be_assigned_directly() {
        let tokens = lex("type Point { x: f64, }
             point: Point = Point { x: 1.0, };
             point.x = 2.0;")
        .unwrap();
        let error = parse(tokens).unwrap_err();

        assert_eq!(
            error.message(),
            "fields cannot be assigned directly; construct a new value and reassign the whole mutable binding"
        );
    }

    #[test]
    fn parses_infer() {
        let program = parse(lex("x: infer = 1 + 2;").unwrap()).unwrap();

        let StmtKind::Binding { type_spec, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert_eq!(*type_spec, TypeSpec::Infer);
    }

    #[test]
    fn parses_float_without_explicit_type() {
        let program = parse(lex("x: f32 = 0.1;").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert_eq!(
            value.kind,
            ExprKind::Float {
                text: "0.1".into(),
                explicit_type: None
            }
        );
    }

    #[test]
    fn multiplication_has_higher_precedence() {
        let program = parse(lex("x: i64 = 1 + 2 * 3;").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        let ExprKind::Binary { op, right, .. } = &value.kind else {
            panic!("expected binary expression");
        };

        assert_eq!(*op, BinaryOp::Add);
        assert_eq!(value.span, Span::new(9, 18));

        assert!(matches!(
            &right.kind,
            ExprKind::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
        assert_eq!(right.span, Span::new(13, 18));
    }

    #[test]
    fn comparison_has_lower_precedence_than_arithmetic() {
        let program = parse(lex("x: bool = 1 + 2 < 4;").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        let ExprKind::Binary { op, left, .. } = &value.kind else {
            panic!("expected comparison");
        };

        assert_eq!(*op, BinaryOp::Less);
        assert!(matches!(
            &left.kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn parses_boolean_literal_and_not() {
        let program = parse(lex("x: bool = !false;").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert!(matches!(
            &value.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn parses_if_else_blocks() {
        let program = parse(lex("if true { print(1); } else { print(2); }").unwrap()).unwrap();

        let StmtKind::If {
            condition,
            then_body,
            else_body,
        } = &program.statement(0).kind
        else {
            panic!("expected if statement");
        };

        assert_eq!(condition.kind, ExprKind::Boolean(true));
        assert_eq!(then_body.len(), 1);
        assert_eq!(else_body.len(), 1);
        assert_eq!(program.statement(0).span, Span::new(0, 40));
    }

    #[test]
    fn parses_while_block() {
        let program = parse(lex("while true { print(1); }").unwrap()).unwrap();

        let StmtKind::While { condition, body } = &program.statement(0).kind else {
            panic!("expected while statement");
        };

        assert_eq!(condition.kind, ExprKind::Boolean(true));
        assert_eq!(body.len(), 1);
        assert_eq!(program.statement(0).span, Span::new(0, 24));
    }

    #[test]
    fn parses_break_and_continue() {
        let program = parse(lex("while true { continue; break; }").unwrap()).unwrap();

        let StmtKind::While { body, .. } = &program.statement(0).kind else {
            panic!("expected while statement");
        };

        assert_eq!(body[0].kind, StmtKind::Continue);
        assert_eq!(body[1].kind, StmtKind::Break);
        assert_eq!(body[0].span, Span::new(13, 22));
        assert_eq!(body[1].span, Span::new(23, 29));
    }

    #[test]
    fn parses_for_block() {
        let source = "for (mut i: i64 = 0; i < 3; i = i + 1) { print(i); }";
        let program = parse(lex(source).unwrap()).unwrap();

        let StmtKind::For {
            initializer,
            condition,
            update,
            body,
        } = &program.statement(0).kind
        else {
            panic!("expected for statement");
        };

        assert!(matches!(initializer.kind, StmtKind::Binding { .. }));
        assert!(matches!(condition.kind, ExprKind::Binary { .. }));
        assert!(matches!(update.kind, StmtKind::Assignment { .. }));
        assert_eq!(body.len(), 1);
        assert_eq!(program.statement(0).span, Span::new(0, source.len()));
    }

    #[test]
    fn parses_for_block_with_assignment_as_start() {
        let source = "mut i: i64 = 3; for (i = 0; i < 3; i = i + 1) { print(i); }";
        let program = parse(lex(source).unwrap()).unwrap();

        let StmtKind::For { initializer, .. } = &program.statement(1).kind else {
            panic!("expected for statement");
        };

        assert!(matches!(initializer.kind, StmtKind::Assignment { .. }));
    }

    #[test]
    fn requires_parentheses_around_for_header() {
        let error =
            parse(lex("for mut i: i64 = 0; i < 3; i = i + 1 { print(i); }").unwrap()).unwrap_err();

        assert_eq!(error.message(), "expected `(` after `for`");
    }

    #[test]
    fn reports_missing_closing_parenthesis_in_for_header() {
        let error =
            parse(lex("for (mut i: i64 = 0; i < 3; i = i + 1 { print(i); }").unwrap()).unwrap_err();

        assert_eq!(error.message(), "expected `)` after the `for` update");
    }

    #[test]
    fn unary_span_includes_operator() {
        let program = parse(lex("x: i64 = -1;").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert_eq!(value.span, Span::new(9, 11));
    }

    #[test]
    fn parenthesized_span_includes_parentheses() {
        let program = parse(lex("x: i64 = (1 + 2);").unwrap()).unwrap();

        let StmtKind::Binding { value, .. } = &program.statement(0).kind else {
            panic!("expected binding");
        };

        assert_eq!(value.span, Span::new(9, 16));
    }

    #[test]
    fn reports_missing_semicolon_at_end_of_input() {
        let error = parse(lex("x: i64 = 1").unwrap()).unwrap_err();

        assert_eq!(error.message(), "expected Semicolon, found Eof");
        assert_eq!(error.primary_span(), Some(Span::empty(10)));
    }

    #[test]
    fn print_span_covers_keyword_through_semicolon() {
        let program = parse(lex("  print(1);").unwrap()).unwrap();
        let statement = program.statement(0);

        let StmtKind::Print { value } = &statement.kind else {
            panic!("expected print");
        };

        assert_eq!(statement.span, Span::new(2, 11));
        assert_eq!(value.span, Span::new(8, 9));
    }
}
