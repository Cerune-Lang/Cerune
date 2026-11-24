use super::ir::{Function, Origin};
use crate::codegen::trace;
use std::{collections::BTreeMap, fmt::Write};

/// 各行を静的データに一度だけ置き、通過した命令からその行を指します。
#[derive(Default)]
pub(super) struct Rows {
    symbols: BTreeMap<String, usize>,
}

impl Rows {
    fn symbol(&mut self, row: String) -> usize {
        let next = self.symbols.len();
        *self.symbols.entry(row).or_insert(next)
    }
    pub(super) fn function(
        &mut self,
        event: &str,
        function: &Function,
        origin: Origin,
        out: &mut String,
    ) {
        let (node, span) = match origin {
            Origin::Source { node_id, span } => (Some(node_id), span),
            Origin::Synthetic => (None, function.declaration),
        };
        let id = self.symbol(trace::function_row(event, &function.name, node, span));
        writeln!(
            out,
            "  call void @cerune_trace_v1_record(ptr @cerune.trace.row.{id})"
        )
        .unwrap();
    }
    pub(super) fn branch(
        &mut self,
        (node, span): (crate::ir::NodeId, crate::source::Span),
        condition: &str,
        label: usize,
        out: &mut String,
    ) {
        let yes = self.symbol(trace::branch_row(true, node, span));
        let no = self.symbol(trace::branch_row(false, node, span));
        writeln!(out, "  %trace.branch.{label} = select i1 {condition}, ptr @cerune.trace.row.{yes}, ptr @cerune.trace.row.{no}").unwrap();
        writeln!(
            out,
            "  call void @cerune_trace_v1_record(ptr %trace.branch.{label})"
        )
        .unwrap();
    }
    pub(super) fn emit(&self, out: &mut String) {
        out.push_str(
            "declare void @cerune_trace_v1_record(ptr)\ndeclare void @cerune_trace_v1_save()\n",
        );
        for (row, id) in &self.symbols {
            write!(
                out,
                "@cerune.trace.row.{id} = private unnamed_addr constant [{} x i8] c\"",
                row.len() + 1
            )
            .unwrap();
            for byte in row.bytes() {
                write!(out, "\\{byte:02X}").unwrap();
            }
            out.push_str("\\00\"\n");
        }
    }
}

/// emitした終了命令の直前で、通常終了時のatexitを通らない経路の記録を保存します。
pub(super) fn save_before_immediate_exits(module: &str) -> String {
    let mut result = String::with_capacity(module.len());
    for line in module.split_inclusive('\n') {
        if matches!(
            line.trim(),
            "call void @_Exit(i32 1)" | "call void @llvm.trap()"
        ) {
            result.push_str("  call void @cerune_trace_v1_save()\n");
        }
        result.push_str(line);
    }
    result
}
