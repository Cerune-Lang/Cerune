//! 複合値表示の、改行なし出力とUTF-8の引用処理です。
use crate::ir::{Program, Statement, StatementKind, Type};
pub(super) fn uses_write(p: &Program) -> bool {
    fn body(ss: &[Statement]) -> bool {
        ss.iter().any(|s| match &s.kind {
            StatementKind::Write { .. } => true,
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => body(then_body) || body(else_body),
            StatementKind::For { body: b, .. } | StatementKind::While { body: b, .. } => body(b),
            _ => false,
        })
    }
    body(&p.statements) || p.function_definitions.iter().any(|f| body(&f.body))
}
pub(super) fn kind(ty: &Type, quoted: bool) -> &'static str {
    match ty {
        Type::String => {
            if quoted {
                "quoted"
            } else {
                "string"
            }
        }
        Type::Bool => "bool",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::Integer(crate::types::IntegerType::U64) => "u64",
        Type::Integer(_) => "i64",
        _ => unreachable!(),
    }
}
pub(super) fn escapes() -> Vec<(u8, String)> {
    (0..=127u8)
        .filter_map(|c| {
            let text = match c {
                0 => "\\0".into(),
                9 => "\\t".into(),
                10 => "\\n".into(),
                13 => "\\r".into(),
                34 => "\\\"".into(),
                92 => "\\\\".into(),
                1..=31 | 127 => format!("\\u{{{c:02x}}}"),
                _ => return None,
            };
            Some((c, text))
        })
        .collect()
}
pub(super) fn llvm_type(kind: &str) -> &'static str {
    match kind {
        "string" | "quoted" => "%cerune.string",
        "bool" => "i1",
        "f32" => "float",
        "f64" => "double",
        _ => "i64",
    }
}
pub(super) fn qbe_type(kind: &str) -> &'static str {
    match kind {
        "bool" => "w",
        "f32" => "s",
        "f64" => "d",
        _ => "l",
    }
}
