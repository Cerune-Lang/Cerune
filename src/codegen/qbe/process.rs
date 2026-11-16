//! QBEの要求・予約・結果変換を明示し、子の実行は共通runtimeへ委ねます。
pub(super) mod entry;
mod report;
use super::ir::{FailureOrigin, Instruction, Module};
use std::fmt::Write;
pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i, Instruction::ProcessRun { .. }))
}
pub(super) fn origin_name(o: FailureOrigin) -> String {
    format!(
        "cerune_process_origin_{}_{}_{}_{}",
        o.node.0,
        o.span.source_id().index(),
        o.span.start(),
        o.span.end()
    )
}
pub(super) fn data(m: &Module, s: &mut String, name: &str, value: &str) {
    write!(
        s,
        "section \"{}\" data ${name} = {{ ",
        m.target.unwrap().read_only_section()
    )
    .unwrap();
    for (i, b) in value.bytes().chain([0]).enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        write!(s, "b {b}").unwrap();
    }
    s.push_str(" }\n");
}
fn field<'a>(m: &'a Module, id: usize, name: &str) -> &'a super::ir::Field {
    m.type_definitions[id]
        .fields
        .iter()
        .find(|f| f.name == name)
        .unwrap()
}
pub(super) fn support(m: &Module, s: &mut String) {
    if !used(m) {
        return;
    }
    let mut types = Vec::new();
    let mut origins = Vec::new();
    for i in m
        .instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::ProcessRun {
            request_type,
            outcome_type,
            origin,
            ..
        } = i
        {
            if !types.contains(&(*request_type, *outcome_type)) {
                types.push((*request_type, *outcome_type));
            }
            if !origins.contains(origin) {
                origins.push(*origin);
            }
        }
    }
    for o in origins {
        data(
            m,
            s,
            &origin_name(o),
            &format!(
                " node={}{} bytes={}..{}\n",
                o.node.0,
                o.span.source_id().record_field(),
                o.span.start(),
                o.span.end()
            ),
        );
    }
    data(
        m,
        s,
        "cerune_process_format",
        "cerune-process-abi-v1 status=%u%s",
    );
    data(m, s, "cerune_process_host", " origin=host\n");
    s.push_str(include_str!("process/runtime.ssa"));
    s.push_str(
        &include_str!("process/storage.ssa")
            .replace("@LIMIT@", &m.array_heap_limit.unwrap().to_string()),
    );
    s.push_str(include_str!("process/perform.ssa"));
    entry::support(m, s);
    for (req, out) in types {
        emit_run(m, req, out, s);
    }
}
fn emit_run(m: &Module, req: usize, out: usize, s: &mut String) {
    writeln!(s,"function $cerune_process_run_{req}_{out}(l %result, l %request, l %node, l %source, l %start, l %end, l %origin, l %failure, l %failure_len) {{\n@start\n  %words =l alloc8 256\n  %outputs =l alloc8 16\n  call $memset(l %result, w 0, l {})",m.type_definitions[out].size).unwrap();
    for name in [
        "target",
        "arguments",
        "stdin",
        "stdout_limit",
        "stderr_limit",
        "timeout_ms",
    ] {
        writeln!(
            s,
            "  %{name}_p =l add %request, {}\n  %{name} =l loadl %{name}_p",
            field(m, req, name).offset
        )
        .unwrap();
    }
    s.push_str("  call $cerune_process_perform(l %outputs, l %target, l %arguments, l %stdin, l %stdout_limit, l %stderr_limit, l %timeout_ms, l %node, l %source, l %start, l %end, l %words, l %origin, l %failure, l %failure_len)\n");
    for (i, name) in ["stdout", "stderr"].iter().enumerate() {
        writeln!(s,"  %{name}_p =l add %outputs, {}\n  %{name}_value =l loadl %{name}_p\n  %{name}_slot =l add %result, {}\n  storel %{name}_value, %{name}_slot",i*8,field(m,out,name).offset).unwrap();
    }
    report::emit(m, out, s);
    s.push_str("  %context =l loadl $cerune_process_context\n  %released =w call $cerune_process_v1_release(l %context)\n  call $cerune_process_check(w %released, w 0, l %origin)\n  ret\n}\n");
}
