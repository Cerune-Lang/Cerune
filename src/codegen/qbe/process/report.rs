use super::super::ir::Module;
use super::field;
use std::fmt::Write;
struct Builder<'a> {
    module: &'a Module,
    id: usize,
    text: &'a mut String,
    next: usize,
}
impl Builder<'_> {
    fn value(&mut self, expr: &str) -> String {
        let n = self.next;
        self.next += 1;
        let v = format!("%report.{n}");
        let (ty, expr) = expr.split_once(' ').unwrap();
        if ty == "w" {
            writeln!(self.text, "  {v}.bool =w {expr}\n  {v} =l extuw {v}.bool").unwrap();
        } else {
            writeln!(self.text, "  {v} =l {expr}").unwrap();
        }
        v
    }
    fn path(&self, names: &[&str]) -> (usize, Option<usize>) {
        let mut id = Some(self.id);
        let mut offset = 0;
        for name in names {
            let f = field(self.module, id.unwrap(), name);
            offset += f.offset;
            id = f.named;
        }
        (offset, id)
    }
    fn set(&mut self, names: &[&str], value: &str) {
        let (offset, _) = self.path(names);
        let p = self.value(&format!("l add %result, {offset}"));
        writeln!(self.text, "  storel {value}, {p}").unwrap();
    }
    fn tag(&mut self, names: &[&str], value: &str, variants: &[&str]) {
        let (_, id) = self.path(names);
        let tags = self.module.type_definitions[id.unwrap()]
            .variants
            .as_ref()
            .unwrap();
        let indices: Vec<_> = variants
            .iter()
            .map(|v| tags.iter().position(|t| t == v).unwrap())
            .collect();
        let invalid = self.value(&format!("w cugel {value}, {}", variants.len()));
        let status = self.value(&format!("l mul {invalid}, 13"));
        writeln!(
            self.text,
            "  call $cerune_process_check(w {status}, w 0, l %origin)"
        )
        .unwrap();
        let mut result = String::from("0");
        for (i, tag) in indices.into_iter().enumerate() {
            let yes = self.value(&format!("w ceql {value}, {i}"));
            let selected = self.value(&format!("l mul {yes}, {tag}"));
            result = self.value(&format!("l or {selected}, {result}"));
        }
        let mut fields = names.to_vec();
        fields.push("$tag");
        self.set(&fields, &result);
    }
    fn index_or_zero(&mut self, word: usize) -> String {
        let yes = self.value(&format!("w cnel %w{word}, 0"));
        let index = self.value(&format!("l sub %w{word}, 1"));
        self.value(&format!("l mul {yes}, {index}"))
    }
}
pub(super) fn emit(module: &Module, id: usize, text: &mut String) {
    let mut b = Builder {
        module,
        id,
        text,
        next: 0,
    };
    for i in 0..32 {
        writeln!(
            b.text,
            "  %word.ptr.{i} =l add %words, {}\n  %w{i} =l loadl %word.ptr.{i}",
            i * 8
        )
        .unwrap();
    }
    for (i, name) in ["node_id", "source_id", "start", "end"].iter().enumerate() {
        b.set(&["report", "origin", name], &format!("%w{}", i + 2));
    }
    b.tag(&["report", "started"], "%w6", &["No", "Yes", "Unknown"]);
    for (i, name) in [
        "stdin_requested",
        "stdin_accepted",
        "stdout_limit",
        "stderr_limit",
        "timeout_ms",
        "stdout_read",
        "stderr_read",
        "stdout_retained",
        "stderr_retained",
    ]
    .iter()
    .enumerate()
    {
        b.set(&["report", name], &format!("%w{}", i + 7));
    }
    let known = b.value("w cnel %w16, 0");
    b.set(&["report", "progress_known"], &known);
    b.tag(&["report", "exit"], "%w17", &["Absent", "Code", "Signal"]);
    for (tag, name) in [(1, "$Code$value"), (2, "$Signal$value")] {
        let yes = b.value(&format!("w ceql %w17, {tag}"));
        let value = b.value(&format!("l mul {yes}, %w18"));
        b.set(&["report", "exit", name], &value);
    }
    let failed = b.value("w cnel %w19, 0");
    let tag = failed;
    b.tag(&["report", "failure"], &tag, &["None", "Reason"]);
    let code = b.index_or_zero(19);
    b.tag(
        &["report", "failure", "$Reason$code"],
        &code,
        &[
            "InvalidRequest",
            "RequestLimit",
            "Unconfigured",
            "Unregistered",
            "ConnectionDisabled",
            "OutputBudget",
            "AllocationFailed",
            "NotFound",
            "Denied",
            "Resources",
            "Unsupported",
            "StartFailed",
            "TransferFailed",
            "CloseFailed",
            "WaitFailed",
            "ZeroProgress",
            "InvalidResponse",
            "OutputLimit",
            "Timeout",
            "CleanupFailed",
            "ProgressUnknown",
        ],
    );
    let phase = b.index_or_zero(20);
    b.tag(
        &["report", "failure", "$Reason$phase"],
        &phase,
        &[
            "Validate",
            "Reserve",
            "Start",
            "Transfer",
            "CloseStdin",
            "Wait",
            "Cleanup",
        ],
    );
    let channel = b.index_or_zero(21);
    b.tag(
        &["report", "failure", "$Reason$phase", "$Transfer$channel"],
        &channel,
        &["Stdin", "Stdout", "Stderr"],
    );
    for (name, i) in [("cleanup", 22), ("cleanup_reported", 25)] {
        b.tag(
            &["report", name],
            &format!("%w{i}"),
            &["Absent", "Complete", "Incomplete"],
        );
        for (name2, mask) in [
            ("$Complete$termination_requested", 1),
            ("$Incomplete$child_reaped", 2),
            ("$Incomplete$handles_closed", 4),
            ("$Incomplete$scope_finished", 8),
        ] {
            let masked = b.value(&format!("l and %w{}, {mask}", i + 1));
            let yes = b.value(&format!("w cnel {masked}, 0"));
            b.set(&["report", name, name2], &yes);
        }
        let error = b.index_or_zero(i + 2);
        b.tag(
            &["report", name, "$Incomplete$error"],
            &error,
            &["Timeout", "Terminate", "Reap", "Close", "InvalidResponse"],
        );
    }
}
