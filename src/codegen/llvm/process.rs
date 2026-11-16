//! 要求・予約・結果の変換をLLVMへ残し、実行意味は共通runtimeへ委ねます。
pub(super) mod entry;
mod report;
use super::{
    Target,
    ir::{Instruction, Module, Origin, Type},
};
use std::fmt::Write;
pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i.instruction, Instruction::ProcessRun { .. }))
}
pub(super) fn origin_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        unreachable!()
    };
    format!(
        "@cerune.process.origin.{}.{}.{}",
        node_id.0,
        span.start(),
        span.end()
    )
}
pub(super) fn data(out: &mut String, name: &str, value: &str) {
    write!(
        out,
        "{name} = private constant [{} x i8] c\"",
        value.len() + 1
    )
    .unwrap();
    for b in value.bytes().chain([0]) {
        write!(out, "\\{b:02X}").unwrap();
    }
    out.push_str("\"\n");
}
fn definition(m: &Module, id: usize) -> &super::ir::TypeDefinition {
    m.type_definitions.iter().find(|t| t.id == id).unwrap()
}
fn field(m: &Module, id: usize, name: &str) -> usize {
    definition(m, id)
        .field_names
        .iter()
        .position(|f| f == name)
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
            ..
        } = i.instruction
        {
            if !types.contains(&(request_type, outcome_type)) {
                types.push((request_type, outcome_type));
            }
            if !origins.contains(&i.origin) {
                origins.push(i.origin);
            }
        }
    }
    for o in origins {
        let Origin::Source { node_id, span } = o else {
            unreachable!()
        };
        data(
            s,
            &origin_name(o),
            &format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
        );
    }
    data(
        s,
        "@cerune.process.format",
        "cerune-process-abi-v1 status=%u%s",
    );
    data(s, "@cerune.process.host", " origin=host\n");
    s.push_str(include_str!("process/runtime.ll"));
    let windows = m.target == Some(Target::X86_64PcWindowsMsvc);
    // The external create signature follows the explicit target's C ABI.
    s.push_str(if windows {
        include_str!("process/create-windows.ll")
    } else {
        include_str!("process/create-linux.ll")
    });
    s.push_str(
        &include_str!("process/storage.ll")
            .replace("@LIMIT@", &m.array_heap_limit.unwrap().to_string()),
    );
    s.push_str(include_str!("process/perform.ll"));
    entry::support(m, s);
    for (req, out) in types {
        emit_run(m, req, out, s);
    }
}
fn emit_run(m: &Module, req: usize, out: usize, s: &mut String) {
    let reqty = super::emit::type_name(&Type::Named(req), m);
    let outty = super::emit::type_name(&Type::Named(out), m);
    writeln!(s,"define internal {outty} @cerune.process.run.{req}.{out}({reqty} %request, i64 %node, i64 %source, i64 %start, i64 %end, ptr %origin, ptr %failure) {{\nentry:\n  %result = alloca {outty}\n  store {outty} zeroinitializer, ptr %result\n  %words = alloca [32 x i64]").unwrap();
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
            "  %{name} = extractvalue {reqty} %request, {}",
            field(m, req, name)
        )
        .unwrap();
    }
    writeln!(s,"  %outputs = call {{ %cerune.array, %cerune.array }} @cerune.process.perform(%cerune.string %target, %cerune.array %arguments, %cerune.array %stdin, i64 %stdout_limit, i64 %stderr_limit, i64 %timeout_ms, i64 %node, i64 %source, i64 %start, i64 %end, ptr %words, ptr %origin, ptr %failure)").unwrap();
    for (i, name) in ["stdout", "stderr"].iter().enumerate() {
        writeln!(s,"  %{name}.value = extractvalue {{ %cerune.array, %cerune.array }} %outputs, {i}\n  %{name}.slot = getelementptr {outty}, ptr %result, i32 0, i32 {}\n  store %cerune.array %{name}.value, ptr %{name}.slot",field(m,out,name)).unwrap();
    }
    report::emit(m, out, s);
    writeln!(s,"  %context = load ptr, ptr @cerune.process.context\n  %released = call i32 @cerune_process_v1_release(ptr %context)\n  call void @cerune.process.check(i32 %released, i32 0, ptr %origin)\n  %value = load {outty}, ptr %result\n  ret {outty} %value\n}}\n").unwrap();
}
