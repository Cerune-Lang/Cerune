use super::super::{
    emit::type_name,
    ir::{Module, Type},
};
use super::{definition, field};
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
        writeln!(self.text, "  {v} = {expr}").unwrap();
        v
    }
    fn path(&self, names: &[&str]) -> (String, Type) {
        let mut ty = Type::Named(self.id);
        let mut indices = String::from("i32 0");
        for name in names {
            let Type::Named(id) = ty else { unreachable!() };
            let index = field(self.module, id, name);
            write!(indices, ", i32 {index}").unwrap();
            ty = definition(self.module, id).fields[index].clone();
        }
        (indices, ty)
    }
    fn set(&mut self, names: &[&str], value: &str) {
        let (indices, ty) = self.path(names);
        let root = type_name(&Type::Named(self.id), self.module);
        let p = self.value(&format!("getelementptr {root}, ptr %result, {indices}"));
        writeln!(
            self.text,
            "  store {} {value}, ptr {p}",
            type_name(&ty, self.module)
        )
        .unwrap();
    }
    fn tag(&mut self, names: &[&str], value: &str, variants: &[&str]) {
        let (_, Type::Named(id)) = self.path(names) else {
            unreachable!()
        };
        let tags = definition(self.module, id).variants.as_ref().unwrap();
        let indices: Vec<_> = variants
            .iter()
            .map(|v| tags.iter().position(|t| t == v).unwrap())
            .collect();
        let valid = self.value(&format!("icmp ult i64 {value}, {}", variants.len()));
        let status = self.value(&format!("select i1 {valid}, i32 0, i32 13"));
        writeln!(
            self.text,
            "  call void @cerune.process.check(i32 {status}, i32 0, ptr %origin)"
        )
        .unwrap();
        let mut result = String::from("0");
        for (i, tag) in indices.into_iter().enumerate() {
            let yes = self.value(&format!("icmp eq i64 {value}, {i}"));
            result = self.value(&format!("select i1 {yes}, i64 {tag}, i64 {result}"));
        }
        let mut fields = names.to_vec();
        fields.push("$tag");
        self.set(&fields, &result);
    }
    fn index_or_zero(&mut self, word: usize) -> String {
        let nonzero = self.value(&format!("icmp ne i64 %w{word}, 0"));
        let index = self.value(&format!("sub i64 %w{word}, 1"));
        self.value(&format!("select i1 {nonzero}, i64 {index}, i64 0"))
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
        writeln!(b.text,"  %word.ptr.{i} = getelementptr i64, ptr %words, i64 {i}\n  %w{i} = load i64, ptr %word.ptr.{i}").unwrap();
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
    let known = b.value("icmp ne i64 %w16, 0");
    b.set(&["report", "progress_known"], &known);
    b.tag(&["report", "exit"], "%w17", &["Absent", "Code", "Signal"]);
    for (tag, name) in [(1, "$Code$value"), (2, "$Signal$value")] {
        let yes = b.value(&format!("icmp eq i64 %w17, {tag}"));
        let value = b.value(&format!("select i1 {yes}, i64 %w18, i64 0"));
        b.set(&["report", "exit", name], &value);
    }
    let failed = b.value("icmp ne i64 %w19, 0");
    let tag = b.value(&format!("zext i1 {failed} to i64"));
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
            let masked = b.value(&format!("and i64 %w{}, {mask}", i + 1));
            let yes = b.value(&format!("icmp ne i64 {masked}, 0"));
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
