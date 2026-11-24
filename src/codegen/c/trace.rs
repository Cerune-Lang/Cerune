//! trace-v1の固定フィールドを生成時に決め、実行時には実際に通った行だけ記録します。
use super::ir::{Function, Origin};

use std::fmt::Write;

use crate::codegen::trace;

fn literal(fields: &str) -> String {
    // 明示したUTF-8バイト列をCの文字集合設定に依存させない。
    let mut literal = String::from("\"");
    for byte in fields.bytes() {
        match byte {
            b'"' => literal.push_str("\\\""),
            b'\\' => literal.push_str("\\\\"),
            b'\n' => literal.push_str("\\n"),
            b' '..=b'~' if byte != b'?' => literal.push(byte as char),
            _ => write!(literal, "\\{byte:03o}").unwrap(),
        }
    }
    literal.push('"');
    literal
}

pub(super) fn function_event(event: &str, function: &Function, origin: Option<Origin>) -> String {
    let span = origin.map_or(function.declaration, |o| o.span);
    format!(
        "cerune_trace_record({})",
        literal(&trace::function_row(
            event,
            &function.name,
            origin.map(|o| o.node_id),
            span
        ))
    )
}

pub(super) fn branch(origin: Origin, taken: bool) -> String {
    literal(&trace::branch_row(taken, origin.node_id, origin.span))
}
