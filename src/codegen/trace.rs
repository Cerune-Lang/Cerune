//! 生成したプログラムが使うtrace-v1のJSON行と保存処理。
use crate::{ir::NodeId, source::Span};

pub(super) const RUNTIME: &str = include_str!("trace/runtime.c.inc");
pub(super) const ENTRY: &str = include_str!("trace/entry.c.inc");

fn row(event: &str, node: Option<NodeId>, span: Span) -> String {
    let node = node.map_or_else(|| "null".to_owned(), |n| n.0.to_string());
    format!(
        ",{event},\"node\":{node},\"file\":{},\"start\":{},\"end\":{}}}\n",
        span.source_id().index(),
        span.start(),
        span.end()
    )
}

pub(super) fn function_row(event: &str, name: &str, node: Option<NodeId>, span: Span) -> String {
    let name = serde_json::to_string(name).expect("function name is a string");
    row(
        &format!(r#""event":"{event}","function":{name}"#),
        node,
        span,
    )
}
pub(super) fn branch_row(taken: bool, node: NodeId, span: Span) -> String {
    row(
        &format!(r#""event":"branch","taken":{taken}"#),
        Some(node),
        span,
    )
}
pub(super) fn stop_row(code: crate::runtime::FailureCode, node: NodeId, span: Span) -> String {
    row(
        &format!(r#""event":"stop","code":"{}""#, code.name()),
        Some(node),
        span,
    )
}

/// --with-traceを付けたLLVM IRと同じターゲットでコンパイルしてリンクするC11ソース。
pub fn runtime_c() -> String {
    let mut out = String::from("#include <stdint.h>\n#include <stdio.h>\n");
    out.push_str(RUNTIME);
    out.push_str(
        r#"
void cerune_trace_v1_record(const char *row) { cerune_trace_record(row); }
void cerune_trace_v1_save(void) { cerune_trace_save(); }
extern const uint8_t cerune_trace_v1_files, cerune_trace_v1_processes;
#define CERUNE_HAS_FILE_ROOT cerune_trace_v1_files
#define CERUNE_HAS_PROCESS_CONFIG cerune_trace_v1_processes
extern int cerune_trace_v1_program(int argc, cerune_trace_char **argv);
"#,
    );
    out.push_str(&ENTRY.replace(
        "CERUNE_PROGRAM_ENTRY",
        "cerune_trace_v1_program(argc, argv)",
    ));
    out
}
