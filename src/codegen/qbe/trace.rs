use super::ir::{FailureOrigin, Function, Module};
use crate::codegen::trace::{self, RowTable};
use std::fmt::Write;

pub(super) fn function(
    rows: &mut RowTable,
    event: &str,
    function: &Function,
    site: Option<FailureOrigin>,
    out: &mut String,
) {
    let span = site.map_or(function.declaration, |o| o.span);
    let id = rows.intern(trace::function_row(
        event,
        &function.name,
        site.map(|o| o.node),
        span,
    ));
    writeln!(
        out,
        "  call $cerune_trace_v1_record(l $cerune_trace_row_{id})"
    )
    .unwrap();
}

pub(super) fn branch(rows: &mut RowTable, site: FailureOrigin, condition: &str, out: &mut String) {
    let yes = rows.intern(trace::branch_row(true, site.node, site.span));
    let no = rows.intern(trace::branch_row(false, site.node, site.span));
    writeln!(out, "  call $cerune_trace_v1_branch(w {condition}, l $cerune_trace_row_{yes}, l $cerune_trace_row_{no})").unwrap();
}

pub(super) fn finish(module: &Module, rows: RowTable, il: &str) -> String {
    let mut out = String::with_capacity(il.len());
    for line in il.split_inclusive('\n') {
        // _Exitとhltは通常終了時の保存処理を通らない。
        if matches!(line.trim(), "call $_Exit(w 1)" | "hlt") {
            out.push_str("  call $cerune_trace_v1_save()\n");
        }
        out.push_str(line);
    }
    writeln!(out, "export data $cerune_trace_v1_files = {{ b {} }}\nexport data $cerune_trace_v1_processes = {{ b {} }}",
        u8::from(super::files::used(module)), u8::from(super::process::used(module))).unwrap();
    for (row, id) in rows.entries() {
        super::failure::data(
            module,
            &format!("cerune_trace_row_{id}"),
            row,
            true,
            &mut out,
        );
    }
    out
}
