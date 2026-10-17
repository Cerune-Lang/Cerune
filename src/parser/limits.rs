//! Bound recursive syntax before descent and expression trees during construction.
use crate::{
    ast::{Expr, ExprKind},
    diagnostic::Diagnostic,
    lexer::{Token, TokenKind},
};

pub const MAX_SYNTAX_NESTING: usize = 16;
pub const MAX_EXPRESSION_DEPTH: usize = 32;

pub(super) fn delimiters(tokens: &[Token]) -> Result<(), Diagnostic> {
    let mut stack = Vec::new();
    for token in tokens {
        let closing = match token.kind {
            TokenKind::LeftParen => Some(TokenKind::RightParen),
            TokenKind::LeftBracket => Some(TokenKind::RightBracket),
            TokenKind::LeftBrace => Some(TokenKind::RightBrace),
            _ => None,
        };
        if let Some(closing) = closing {
            if stack.len() == MAX_SYNTAX_NESTING {
                return Err(Diagnostic::new(
                    format!("syntax nesting exceeds {MAX_SYNTAX_NESTING} delimiters"),
                    token.span,
                ));
            }
            stack.push(closing);
        } else if stack.last() == Some(&token.kind) {
            stack.pop();
        }
    }
    Ok(())
}

pub(super) fn expression(root: &Expr, blocks: usize) -> Result<(), Diagnostic> {
    let mut pending = vec![(root, blocks + 1)];
    while let Some((expr, depth)) = pending.pop() {
        if depth > MAX_EXPRESSION_DEPTH {
            return Err(Diagnostic::new(
                format!(
                    "expression tree depth including blocks exceeds {MAX_EXPRESSION_DEPTH} nodes"
                ),
                root.span,
            ));
        }
        let mut push = |child| pending.push((child, depth + 1));
        match &expr.kind {
            ExprKind::Unary { value, .. } | ExprKind::Convert { value, .. } => push(value),
            ExprKind::FieldAccess { base, .. } => push(base),
            ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
                push(left);
                push(right);
            }
            ExprKind::Index { base, index } => {
                push(base);
                push(index);
            }
            ExprKind::Let { value, body, .. } => {
                push(value);
                push(body);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                push(condition);
                push(then_value);
                push(else_value);
            }
            ExprKind::Array(values)
            | ExprKind::Call {
                arguments: values, ..
            } => {
                for value in values {
                    push(value);
                }
            }
            ExprKind::GenericCall(call) => {
                for value in &call.arguments {
                    push(value);
                }
            }
            ExprKind::Construct { base, fields, .. } => {
                if let Some(base) = base {
                    push(base);
                }
                for field in fields {
                    push(&field.value);
                }
            }
            ExprKind::Match { value, arms } => {
                push(value);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        push(guard);
                    }
                    if let Some(result) = &arm.result {
                        push(result);
                    }
                    // Parser expression arms have results, not statement bodies.
                    debug_assert!(arm.body.is_empty());
                }
            }
            ExprKind::Boolean(_)
            | ExprKind::String(_)
            | ExprKind::Integer(_)
            | ExprKind::Float { .. }
            | ExprKind::Variable(_) => {}
        }
    }
    Ok(())
}
