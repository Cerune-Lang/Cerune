//! trace-v1の固定フィールドを生成時に決め、実行時には実際に通った行だけ記録します。
use super::ir::{Function, Origin};
use crate::source::Span;
use std::fmt::Write;

pub(super) const RUNTIME: &str = include_str!("trace/runtime.c.inc");
pub(super) const ENTRY: &str = include_str!("trace/entry.c.inc");

fn suffix(event: &str, origin: Option<Origin>, span: Span) -> String {
    let node = origin.map_or_else(|| "null".to_string(), |o| o.node_id.0.to_string());
    let fields = format!(
        ",{event},\"node\":{node},\"file\":{},\"start\":{},\"end\":{}}}\n",
        span.source_id().index(),
        span.start(),
        span.end()
    );
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
    let name = serde_json::to_string(&function.name).expect("function name is a string");
    let fields = format!(r#""event":"{event}","function":{name}"#);
    let span = origin.map_or(function.declaration, |o| o.span);
    format!("cerune_trace_record({})", suffix(&fields, origin, span))
}

pub(super) fn branch(origin: Origin, taken: bool) -> String {
    suffix(
        &format!(r#""event":"branch","taken":{taken}"#),
        Some(origin),
        origin.span,
    )
}
