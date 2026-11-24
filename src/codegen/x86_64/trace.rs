//! Native trace calls follow the target C ABI. Program values stay in the caller.
use super::{
    Target,
    abi::integer_argument_register,
    ir::{Module, Origin},
    machine::{EntryKind, Linkage, Opcode, Operand, Section, memory, r64},
    program::{NativeProgram, xmm},
};
use crate::{codegen::trace, ir::NodeId, runtime::FailureCode, source::Span};

fn row(out: &mut NativeProgram, value: String, reg: u8) {
    let id = out
        .trace_rows
        .as_mut()
        .expect("traced program")
        .intern(value);
    let symbol = out.symbol(&format!(".Lcerune_trace_row_{id}"), Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(reg)]);
}
fn call(out: &mut NativeProgram, name: &str) {
    let symbol = out.symbol(name, Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);
}
// 保持するのは整数・浮動小数点の戻り値と分岐条件。先頭32バイトはWindowsの呼出し用。
fn save_values(out: &mut NativeProgram) {
    out.emit(Opcode::Sub64, [Operand::Immediate(48), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), memory(4, None, 32)]);
    out.emit(Opcode::Movsd, [xmm(0), memory(4, None, 40)]);
}
fn restore_values(out: &mut NativeProgram) {
    out.emit(Opcode::Movsd, [memory(4, None, 40), xmm(0)]);
    out.emit(Opcode::Mov64, [memory(4, None, 32), r64(0)]);
    out.emit(Opcode::Add64, [Operand::Immediate(48), r64(4)]);
}

pub(super) fn function(
    event: &str,
    m: &Module,
    id: usize,
    origin: Origin,
    out: &mut NativeProgram,
) {
    if out.trace_rows.is_none() || !m.source_functions[id] {
        return;
    }
    let function = &m.functions[id];
    let (node, span) = match origin {
        Origin::Source { node_id, span } => (Some(node_id), span),
        Origin::Synthetic => {
            let Some(span) = function.declaration else {
                return;
            };
            (None, span)
        }
    };
    let returning = event == "return";
    if returning {
        save_values(out);
    }
    row(
        out,
        trace::function_row(event, &function.name, node, span),
        integer_argument_register(0, m.target),
    );
    call(out, "cerune_trace_v1_record");
    if returning {
        restore_values(out);
    }
}

pub(super) fn branch(m: &Module, out: &mut NativeProgram) {
    if out.trace_rows.is_none() {
        return;
    }
    let Some(site) = out
        .origins()
        .last()
        .and_then(|o| m.if_conditions.get(&(o.function, o.mir?.block.0)))
    else {
        return;
    };
    save_values(out);
    out.emit(
        Opcode::Mov64,
        [r64(0), r64(integer_argument_register(0, m.target))],
    );
    row(
        out,
        trace::branch_row(true, site.node_id, site.span),
        integer_argument_register(1, m.target),
    );
    row(
        out,
        trace::branch_row(false, site.node_id, site.span),
        integer_argument_register(2, m.target),
    );
    call(out, "cerune_trace_v1_branch");
    restore_values(out);
}
pub(super) fn stop(
    code: FailureCode,
    node: NodeId,
    span: Span,
    target: Target,
    out: &mut NativeProgram,
) {
    if out.trace_rows.is_some() {
        row(
            out,
            trace::stop_row(code, node, span),
            integer_argument_register(0, target),
        );
        call(out, "cerune_trace_v1_record");
        call(out, "cerune_trace_v1_save");
    }
}
pub(super) fn finish(out: &mut NativeProgram, arguments: bool, files: bool, process: bool) {
    // References use SymbolId. Rename the entry and the non-returning exit import
    // without changing the instructions or source map of runtime helpers.
    let entry = if arguments && out.program.target == sashimi_assembler::Target::X86_64PcWindowsMsvc
    {
        "wmain"
    } else {
        "main"
    };
    for symbol in &mut out.program.symbols {
        if symbol.linkage == Linkage::Global && symbol.name == entry {
            symbol.name = "cerune_trace_v1_program".into();
        } else if symbol.linkage == Linkage::External && symbol.name == "_Exit" {
            symbol.name = "cerune_trace_v1_exit".into();
        }
    }
    out.synthetic();
    out.push(EntryKind::Section(Section::ReadOnlyData));
    for (name, used) in [
        ("cerune_trace_v1_files", files),
        ("cerune_trace_v1_processes", process),
    ] {
        out.global_label(name);
        out.push(EntryKind::Data(vec![u8::from(used)]));
    }
    let rows = out.trace_rows.take().expect("traced program");
    for (row, id) in rows.entries() {
        out.label(&format!(".Lcerune_trace_row_{id}"));
        out.push(EntryKind::CString(row.clone()));
    }
}
