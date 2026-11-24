use super::ir::{Module, Origin};
use crate::codegen::trace::{self, RowTable};
use std::{
    fmt::Write,
    ops::{Deref, DerefMut},
};

pub(super) struct WatText {
    text: String,
    rows: Option<RowTable>,
}
impl Deref for WatText {
    type Target = String;
    fn deref(&self) -> &String {
        &self.text
    }
}
impl DerefMut for WatText {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.text
    }
}
impl WatText {
    pub fn new(enabled: bool) -> Self {
        Self {
            text: String::new(),
            rows: enabled.then(RowTable::default),
        }
    }
    pub fn tracing(&self) -> bool {
        self.rows.is_some()
    }
    pub fn event(&mut self, row: String, indent: &str) {
        let id = self.rows.as_mut().expect("traced module").intern(row);
        writeln!(self.text, "{indent}(call $trace_event_v1 (i32.const {id}))").unwrap();
    }
    pub fn branch(&mut self, taken: bool, site: Origin, indent: &str) {
        self.event(trace::branch_row(taken, site.node_id, site.span), indent);
    }
    pub fn function(
        &mut self,
        event: &str,
        module: &Module,
        id: usize,
        site: Option<Origin>,
        indent: &str,
    ) {
        if self.tracing() && module.source_functions[id] {
            let f = &module.functions[id];
            self.event(
                trace::function_row(
                    event,
                    &f.name,
                    site.map(|s| s.node_id),
                    site.map_or(f.declaration, |s| s.span),
                ),
                indent,
            );
        }
    }
    pub fn finish(mut self) -> String {
        if let Some(rows) = self.rows {
            self.text.push_str("  (@custom \"cerune.trace.v1\"\n");
            let mut entries: Vec<_> = rows.entries().collect();
            entries.sort_unstable_by_key(|(_, id)| **id);
            for (row, _) in entries {
                // 1行が1箇所。ゲストのメモリやヒープ上限を使わずにホストが読める。
                self.text.push_str("    \"{");
                let fields = row
                    .strip_prefix(',')
                    .expect("trace row follows the sequence field");
                for b in fields.bytes() {
                    match b {
                        b' '..=b'~' if b != b'"' && b != b'\\' => self.text.push(char::from(b)),
                        _ => write!(self.text, "\\{b:02x}").unwrap(),
                    }
                }
                self.text.push_str("\"\n");
            }
            self.text.push_str("  )\n");
        }
        self.text.push_str(")\n");
        self.text
    }
}
