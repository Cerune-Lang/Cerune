//! 複合値表示の、改行なし出力とUTF-8の引用処理です。
use crate::ir::{Program, Statement, StatementKind, Type};
use std::fmt::Write;
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
fn escapes() -> Vec<(u8, String)> {
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
pub(super) fn wat_imports() -> String {
    let mut o = String::new();
    for (kind, ty) in [
        ("i64", "i64"),
        ("u64", "i64"),
        ("f32", "f32"),
        ("f64", "f64"),
    ] {
        writeln!(
            o,
            "  (import \"cerune\" \"write_{kind}\" (func $cerune_write_{kind} (param {ty})))"
        )
        .unwrap();
    }
    o
}
pub(super) fn wat() -> String {
    let mut o = String::from("  (func $cerune_write_escaped_byte (param $value i32)\n");
    for (c, text) in escapes() {
        writeln!(
            o,
            "    local.get $value\n    i32.const {c}\n    i32.eq\n    if"
        )
        .unwrap();
        for b in text.bytes() {
            writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
        }
        o.push_str("      return\n    end\n");
    }
    o.push_str("    local.get $value\n    call $write_byte\n  )\n  (func $cerune_write_bool (param $value i32)\n    local.get $value\n    if\n");
    for b in b"true" {
        writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
    }
    o.push_str("    else\n");
    for b in b"false" {
        writeln!(o, "      i32.const {b}\n      call $write_byte").unwrap();
    }
    o.push_str("    end\n  )\n");
    for (kind, quoted) in [("string", false), ("quoted", true)] {
        writeln!(o,"  (func $cerune_write_{kind} (param $value i32)\n    (local $length i32) (local $index i32)\n    local.get $value\n    i32.load\n    local.set $length").unwrap();
        if quoted {
            o.push_str("    i32.const 34\n    call $write_byte\n");
        }
        o.push_str("    block $end\n      loop $loop\n        local.get $index\n        local.get $length\n        i32.eq\n        br_if $end\n        local.get $value\n        local.get $index\n        i32.add\n        i32.load8_u offset=8\n");
        writeln!(
            o,
            "        call ${}",
            if quoted {
                "cerune_write_escaped_byte"
            } else {
                "write_byte"
            }
        )
        .unwrap();
        o.push_str("        local.get $index\n        i32.const 1\n        i32.add\n        local.set $index\n        br $loop\n      end\n    end\n");
        if quoted {
            o.push_str("    i32.const 34\n    call $write_byte\n");
        }
        o.push_str("  )\n");
    }
    o
}
