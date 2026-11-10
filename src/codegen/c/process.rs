//! 共通runtimeへの要求変換・予約・実行・返却値変換を生成物に残します。
use super::{
    Target,
    ir::{Module, Type, TypeDefinition},
};
use std::fmt::Write;
mod entry;
pub(super) fn entry(files: bool) -> String {
    entry::emit(files)
}
fn definition(m: &Module, id: usize) -> &TypeDefinition {
    m.type_definitions.iter().find(|d| d.id == id).unwrap()
}
fn field(m: &Module, id: usize, name: &str) -> (String, Type) {
    let name = super::lower::storage_field_name(name);
    let f = definition(m, id)
        .fields
        .iter()
        .find(|f| f.name == name)
        .unwrap();
    (name, f.ty.clone())
}
fn path(m: &Module, id: usize, root: &str, names: &[&str]) -> (String, Type) {
    let mut value = root.to_owned();
    let mut ty = Type::Named(id);
    for name in names {
        let Type::Named(id) = ty else { unreachable!() };
        let (name, next) = field(m, id, name);
        write!(value, ".{name}").unwrap();
        ty = next;
    }
    (value, ty)
}
fn set(m: &Module, id: usize, names: &[&str], value: &str, s: &mut String) {
    writeln!(s, "    {} = {value};", path(m, id, "result", names).0).unwrap();
}
fn tag(m: &Module, id: usize, names: &[&str], value: &str, variants: &[&str], s: &mut String) {
    let (p, Type::Named(t)) = path(m, id, "result", names) else {
        unreachable!()
    };
    let d = definition(m, t);
    let f = super::lower::storage_field_name("$tag");
    writeln!(s, "    switch ((uint64_t)({value})) {{").unwrap();
    for (n, name) in variants.iter().enumerate() {
        let v = d
            .variants
            .as_ref()
            .unwrap()
            .iter()
            .position(|v| v == name)
            .unwrap();
        writeln!(s, "    case {n}: {p}.{f} = {v}; break;").unwrap();
    }
    s.push_str("    default: cerune_process_contract(13, origin);\n    }\n");
}
pub(super) fn support(m: &Module, types: &[(usize, usize)]) -> String {
    let target = m
        .process_target
        .expect("process C requires explicit target");
    let (id, guard) = match target {
        Target::X86_64PcWindowsMsvc => (1, "!defined(_WIN32) || !defined(_M_X64)"),
        Target::X86_64UnknownLinuxGnu => (2, "!defined(__linux__) || !defined(__x86_64__)"),
    };
    let mut s = format!(
        "\n/* Cerune process ABI v1: explicit target; link the matching runtime archive. */\n#if {guard}\n#error Cerune_process_runtime_target_mismatch\n#endif\n"
    );
    s.push_str(include_str!(
        "../../runtime/process/abi/cerune_process_v1.h"
    ));
    s.push_str(
        &include_str!("process/runtime.c.inc")
            .replace("@TARGET@", &id.to_string())
            .replace("@LIMIT@", &m.array_heap_limit.unwrap().to_string()),
    );
    for &(req, out) in types {
        let reqty = super::emit::c_type(&Type::Named(req), m);
        let outty = super::emit::c_type(&Type::Named(out), m);
        writeln!(s,"static {outty} cerune_process_run_{req}_{out}({reqty} request, uint64_t node, uint64_t source, uint64_t start, uint64_t end, const char *origin) {{").unwrap();
        writeln!(s, "    {outty} result = {{0}};").unwrap();
        let get = |name| path(m, req, "request", &[name]).0;
        s.push_str(
            &include_str!("process/run.c.inc")
                .replace("@ARGS@", &get("arguments"))
                .replace("@STDIN@", &get("stdin"))
                .replace("@TARGET@", &get("target"))
                .replace("@OUT_LIMIT@", &get("stdout_limit"))
                .replace("@ERR_LIMIT@", &get("stderr_limit"))
                .replace("@TIME@", &get("timeout_ms"))
                .replace("@OUT@", &path(m, out, "result", &["stdout"]).0)
                .replace("@ERR@", &path(m, out, "result", &["stderr"]).0),
        );
        for (i, name) in ["node_id", "source_id", "start", "end"].iter().enumerate() {
            set(
                m,
                out,
                &["report", "origin", name],
                &format!("w[{}]", i + 2),
                &mut s,
            );
        }
        tag(
            m,
            out,
            &["report", "started"],
            "w[6]",
            &["No", "Yes", "Unknown"],
            &mut s,
        );
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
            "progress_known",
        ]
        .iter()
        .enumerate()
        {
            set(m, out, &["report", name], &format!("w[{}]", i + 7), &mut s);
        }
        tag(
            m,
            out,
            &["report", "exit"],
            "w[17]",
            &["Absent", "Code", "Signal"],
            &mut s,
        );
        set(
            m,
            out,
            &["report", "exit", "$Code$value"],
            "w[17] == 1 ? (int64_t)w[18] : 0",
            &mut s,
        );
        set(
            m,
            out,
            &["report", "exit", "$Signal$value"],
            "w[17] == 2 ? (int64_t)w[18] : 0",
            &mut s,
        );
        tag(
            m,
            out,
            &["report", "failure"],
            "w[19] != 0",
            &["None", "Reason"],
            &mut s,
        );
        s.push_str("    if (w[19]) {\n");
        tag(
            m,
            out,
            &["report", "failure", "$Reason$code"],
            "w[19] - 1",
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
            &mut s,
        );
        tag(
            m,
            out,
            &["report", "failure", "$Reason$phase"],
            "w[20] - 1",
            &[
                "Validate",
                "Reserve",
                "Start",
                "Transfer",
                "CloseStdin",
                "Wait",
                "Cleanup",
            ],
            &mut s,
        );
        s.push_str("    if (w[20] == 4) {\n");
        tag(
            m,
            out,
            &["report", "failure", "$Reason$phase", "$Transfer$channel"],
            "w[21] - 1",
            &["Stdin", "Stdout", "Stderr"],
            &mut s,
        );
        s.push_str("    }\n    }\n");
        for (name, i) in [("cleanup", 22), ("cleanup_reported", 25)] {
            tag(
                m,
                out,
                &["report", name],
                &format!("w[{i}]"),
                &["Absent", "Complete", "Incomplete"],
                &mut s,
            );
            for (f, mask) in [
                ("$Complete$termination_requested", 1),
                ("$Incomplete$child_reaped", 2),
                ("$Incomplete$handles_closed", 4),
                ("$Incomplete$scope_finished", 8),
            ] {
                set(
                    m,
                    out,
                    &["report", name, f],
                    &format!("(w[{}] & {mask}) != 0", i + 1),
                    &mut s,
                );
            }
            writeln!(s, "    if (w[{i}] == 2) {{").unwrap();
            tag(
                m,
                out,
                &["report", name, "$Incomplete$error"],
                &format!("w[{}] - 1", i + 2),
                &["Timeout", "Terminate", "Reap", "Close", "InvalidResponse"],
                &mut s,
            );
            s.push_str("    }\n");
        }
        s.push_str("    cerune_process_check(cerune_process_v1_release(cerune_process_context_value), 0, origin);\n    return result;\n}\n");
    }
    s
}
