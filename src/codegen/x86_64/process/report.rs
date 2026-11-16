use super::*;
struct Builder<'a> {
    o: &'a mut NativeProgram,
    m: &'a Module,
    id: usize,
    next: usize,
    prefix: String,
}
impl Builder<'_> {
    fn set(&mut self, n: &[&str]) {
        let (d, _) = path(self.m, self.id, n);
        mov(self.o, slot(128), r64(10));
        mov(self.o, r64(0), address(10, None, d));
    }
    fn word(&mut self, i: usize) {
        mov(self.o, slot(512 + 8 * i as i128), r64(0));
    }
    fn unique(&mut self) -> String {
        self.next += 1;
        format!("{}_{}", self.prefix, self.next)
    }
    fn boolean(&mut self) {
        test(self.o);
        self.o.emit(Opcode::SetNe, [r8(0)]);
        self.o.emit(Opcode::ZeroExtend8To32, [r8(0), r32(0)]);
    }
    fn index(&mut self, i: usize) {
        self.word(i);
        test(self.o);
        let l = self.unique();
        branch(self.o, Opcode::JumpE, &l);
        self.o.emit(Opcode::Sub64, [imm(1), r64(0)]);
        label(self.o, &l);
    }
    fn tag(&mut self, n: &[&str], variants: &[&str]) {
        let (_, id) = path(self.m, self.id, n);
        let tags = self.m.types[id.unwrap()].variants.as_ref().unwrap();
        let done = self.unique();
        let bad = format!("{}_invalid", self.prefix);
        compare(self.o, r64(0), variants.len() as i128);
        branch(self.o, Opcode::JumpAe, &bad);
        for (i, v) in variants.iter().enumerate() {
            let next = self.unique();
            compare(self.o, r64(0), i as i128);
            branch(self.o, Opcode::JumpNe, &next);
            mov(
                self.o,
                imm(tags.iter().position(|x| x == v).unwrap() as i128),
                r64(0),
            );
            branch(self.o, Opcode::Jump, &done);
            label(self.o, &next);
        }
        label(self.o, &done);
        let mut n = n.to_vec();
        n.push("$tag");
        self.set(&n);
    }
}
pub(super) fn append(o: &mut NativeProgram, m: &Module, id: usize, prefix: &str) {
    let mut b = Builder {
        o,
        m,
        id,
        next: 0,
        prefix: prefix.into(),
    };
    for (i, n) in ["node_id", "source_id", "start", "end"].iter().enumerate() {
        b.word(i + 2);
        b.set(&["report", "origin", n]);
    }
    b.word(6);
    b.tag(&["report", "started"], &["No", "Yes", "Unknown"]);
    for (i, n) in [
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
        b.word(i + 7);
        b.set(&["report", n]);
    }
    b.word(16);
    b.boolean();
    b.set(&["report", "progress_known"]);
    b.word(17);
    b.tag(&["report", "exit"], &["Absent", "Code", "Signal"]);
    for (tag, n) in [(1, "$Code$value"), (2, "$Signal$value")] {
        b.word(17);
        compare(b.o, r64(0), tag);
        b.o.emit(Opcode::SetE, [r8(0)]);
        b.o.emit(Opcode::ZeroExtend8To32, [r8(0), r32(0)]);
        b.o.emit(Opcode::SignedMul64, [slot(512 + 18 * 8), r64(0)]);
        b.set(&["report", "exit", n]);
    }
    b.word(19);
    b.boolean();
    b.tag(&["report", "failure"], &["None", "Reason"]);
    b.index(19);
    b.tag(
        &["report", "failure", "$Reason$code"],
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
    b.index(20);
    b.tag(
        &["report", "failure", "$Reason$phase"],
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
    b.index(21);
    b.tag(
        &["report", "failure", "$Reason$phase", "$Transfer$channel"],
        &["Stdin", "Stdout", "Stderr"],
    );
    for (n, i) in [("cleanup", 22), ("cleanup_reported", 25)] {
        b.word(i);
        b.tag(&["report", n], &["Absent", "Complete", "Incomplete"]);
        for (n2, mask) in [
            ("$Complete$termination_requested", 1),
            ("$Incomplete$child_reaped", 2),
            ("$Incomplete$handles_closed", 4),
            ("$Incomplete$scope_finished", 8),
        ] {
            b.word(i + 1);
            b.o.emit(Opcode::And64, [imm(mask), r64(0)]);
            b.boolean();
            b.set(&["report", n, n2]);
        }
        b.index(i + 2);
        b.tag(
            &["report", n, "$Incomplete$error"],
            &["Timeout", "Terminate", "Reap", "Close", "InvalidResponse"],
        );
    }
}
