//! 生成したプログラムが使うtrace-v1のJSON行と保存処理。
use crate::{ir::NodeId, source::Span};

pub(super) const RUNTIME: &str = include_str!("trace/runtime.c.inc");
pub(super) const ENTRY: &str = include_str!("trace/entry.c.inc");

#[derive(Default)]
pub(super) struct RowTable(std::collections::BTreeMap<String, usize>);
impl RowTable {
    pub(super) fn intern(&mut self, row: String) -> usize {
        let next = self.0.len();
        *self.0.entry(row).or_insert(next)
    }
    pub(super) fn entries(&self) -> impl Iterator<Item = (&String, &usize)> {
        self.0.iter()
    }
}

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

/// --with-traceを付けたLLVM IR・QBE ILと同じターゲットでリンクするC11ソース。
pub fn runtime_c() -> String {
    let mut out = String::from("#include <stdint.h>\n#include <stdio.h>\n");
    out.push_str(RUNTIME);
    out.push_str(
        r#"
void cerune_trace_v1_record(const char *row) { cerune_trace_record(row); }
void cerune_trace_v1_save(void) { cerune_trace_save(); }
void cerune_trace_v1_branch(int taken, const char *yes, const char *no) {
    cerune_trace_branch(taken, yes, no);
}
void cerune_trace_v1_stop(const char *code, const char *origin) {
    cerune_trace_stop(code, origin);
}
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
