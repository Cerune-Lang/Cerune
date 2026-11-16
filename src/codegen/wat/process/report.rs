use super::*;
pub(super) fn validate(name: &str, s: &mut String) {
    for (i, v) in [(0, 1), (1, 32), (28, 0), (29, 0), (30, 0), (31, 0)] {
        fail_if(
            &format!("(i64.ne (local.get $w{i}) (i64.const {v}))"),
            name,
            s,
        );
    }
    for (i, max) in [
        (6, 2),
        (16, 1),
        (17, 2),
        (18, u32::MAX as u64),
        (19, 21),
        (20, 7),
        (21, 3),
        (22, 2),
        (25, 2),
    ] {
        fail_if(
            &format!("(i64.gt_u (local.get $w{i}) (i64.const {max}))"),
            name,
            s,
        );
    }
    for (a, b) in [(4, 5), (8, 7), (14, 12), (15, 13), (14, 9), (15, 10)] {
        fail_if(
            &format!("(i64.gt_u (local.get $w{a}) (local.get $w{b}))"),
            name,
            s,
        );
    }
    fail_if(
        "(i32.and (i64.eqz (local.get $w17)) (i64.ne (local.get $w18) (i64.const 0)))",
        name,
        s,
    );
    s.push_str("    (if (i64.eqz (local.get $w19)) (then\n");
    fail_if(
        "(i64.ne (i64.or (local.get $w20) (local.get $w21)) (i64.const 0))",
        name,
        s,
    );
    s.push_str("    ) (else\n");
    fail_if("(i64.eqz (local.get $w20))", name, s);
    s.push_str("      (if (i64.eq (local.get $w20) (i64.const 4)) (then\n");
    fail_if("(i64.eqz (local.get $w21))", name, s);
    s.push_str("      ) (else\n");
    fail_if("(i64.ne (local.get $w21) (i64.const 0))", name, s);
    s.push_str("      ))\n    ))\n");
    for (kind, flags, error) in [(22, 23, 24), (25, 26, 27)] {
        writeln!(s, "    (if (i64.eqz (local.get $w{kind})) (then").unwrap();
        fail_if(
            &format!("(i64.ne (i64.or (local.get $w{flags}) (local.get $w{error})) (i64.const 0))"),
            name,
            s,
        );
        writeln!(
            s,
            "    ) (else (if (i64.eq (local.get $w{kind}) (i64.const 1)) (then"
        )
        .unwrap();
        fail_if(
            &format!(
                "(i32.or (i64.gt_u (local.get $w{flags}) (i64.const 1)) (i64.ne (local.get $w{error}) (i64.const 0)))"
            ),
            name,
            s,
        );
        s.push_str("    ) (else\n");
        fail_if(
            &format!("(i64.ne (i64.and (local.get $w{flags}) (i64.const -15)) (i64.const 0))"),
            name,
            s,
        );
        fail_if(
            &format!(
                "(i32.or (i64.eqz (local.get $w{error})) (i64.gt_u (local.get $w{error}) (i64.const 5)))"
            ),
            name,
            s,
        );
        s.push_str("    ))))\n");
    }
}
fn set(m: &Module, id: usize, names: &[&str], value: &str, s: &mut String) {
    let (offset, ty) = path(m, id, names);
    let width = if ty == Type::Bool { "i32" } else { "i64" };
    writeln!(
        s,
        "    ({width}.store offset={offset} (local.get $result) {value})"
    )
    .unwrap();
}
fn tag(m: &Module, id: usize, names: &[&str], value: &str, variants: &[&str], s: &mut String) {
    let (offset, Type::Named(t)) = path(m, id, names) else {
        unreachable!()
    };
    let d = definition(m, t.0);
    let tag_offset = offset + path(m, t.0, &["$tag"]).0;
    for (n, v) in variants.iter().enumerate() {
        let index = d
            .variants
            .as_ref()
            .unwrap()
            .iter()
            .position(|x| x == v)
            .unwrap();
        writeln!(s,"    (if (i64.eq {value} (i64.const {n})) (then (i64.store offset={tag_offset} (local.get $result) (i64.const {index}))))").unwrap();
    }
}
pub(super) fn emit(m: &Module, id: usize, s: &mut String) {
    for (i, field) in ["node_id", "source_id", "start", "end"].iter().enumerate() {
        set(
            m,
            id,
            &["report", "origin", field],
            &format!("(local.get $w{})", i + 2),
            s,
        );
    }
    tag(
        m,
        id,
        &["report", "started"],
        "(local.get $w6)",
        &["No", "Yes", "Unknown"],
        s,
    );
    for (i, field) in [
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
        set(
            m,
            id,
            &["report", field],
            &format!("(local.get $w{})", i + 7),
            s,
        );
    }
    set(
        m,
        id,
        &["report", "progress_known"],
        "(i32.wrap_i64 (local.get $w16))",
        s,
    );
    tag(
        m,
        id,
        &["report", "exit"],
        "(local.get $w17)",
        &["Absent", "Code", "Signal"],
        s,
    );
    for (v, n) in [("$Code$value", 1), ("$Signal$value", 2)] {
        set(
            m,
            id,
            &["report", "exit", v],
            &format!(
                "(select (local.get $w18) (i64.const 0) (i64.eq (local.get $w17) (i64.const {n})))"
            ),
            s,
        );
    }
    tag(
        m,
        id,
        &["report", "failure"],
        "(i64.extend_i32_u (i64.ne (local.get $w19) (i64.const 0)))",
        &["None", "Reason"],
        s,
    );
    s.push_str("    (if (i64.ne (local.get $w19) (i64.const 0)) (then\n");
    tag(
        m,
        id,
        &["report", "failure", "$Reason$code"],
        "(i64.sub (local.get $w19) (i64.const 1))",
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
        s,
    );
    tag(
        m,
        id,
        &["report", "failure", "$Reason$phase"],
        "(i64.sub (local.get $w20) (i64.const 1))",
        &[
            "Validate",
            "Reserve",
            "Start",
            "Transfer",
            "CloseStdin",
            "Wait",
            "Cleanup",
        ],
        s,
    );
    s.push_str("    (if (i64.eq (local.get $w20) (i64.const 4)) (then\n");
    tag(
        m,
        id,
        &["report", "failure", "$Reason$phase", "$Transfer$channel"],
        "(i64.sub (local.get $w21) (i64.const 1))",
        &["Stdin", "Stdout", "Stderr"],
        s,
    );
    s.push_str("    ))\n    ))\n");
    for (field, k, f, e) in [("cleanup", 22, 23, 24), ("cleanup_reported", 25, 26, 27)] {
        tag(
            m,
            id,
            &["report", field],
            &format!("(local.get $w{k})"),
            &["Absent", "Complete", "Incomplete"],
            s,
        );
        for (payload, bit) in [
            ("$Complete$termination_requested", 1),
            ("$Incomplete$child_reaped", 2),
            ("$Incomplete$handles_closed", 4),
            ("$Incomplete$scope_finished", 8),
        ] {
            set(
                m,
                id,
                &["report", field, payload],
                &format!("(i64.ne (i64.and (local.get $w{f}) (i64.const {bit})) (i64.const 0))"),
                s,
            );
        }
        writeln!(s, "    (if (i64.eq (local.get $w{k}) (i64.const 2)) (then").unwrap();
        tag(
            m,
            id,
            &["report", field, "$Incomplete$error"],
            &format!("(i64.sub (local.get $w{e}) (i64.const 1))"),
            &["Timeout", "Terminate", "Reap", "Close", "InvalidResponse"],
            s,
        );
        s.push_str("    ))\n");
    }
}
