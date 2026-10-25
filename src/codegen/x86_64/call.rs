use super::{
    ir::{Instruction, Module, Origin},
    machine::*,
    program::NativeProgram,
};
use crate::codegen::call::Site;
use Opcode::*;
use Operand::{Immediate as imm, RipRelative as rip, Symbol as sym};
fn site(function: usize, origin: Origin) -> Site {
    Site {
        function,
        origin: match origin {
            Origin::Source { node_id, span } => Some((node_id, span)),
            Origin::Synthetic => None,
        },
    }
}
fn call(out: &mut NativeProgram, name: &str, external: bool) {
    let s = out.symbol(
        name,
        if external {
            Linkage::External
        } else {
            Linkage::Local
        },
    );
    out.emit(Call, [sym(s)]);
}
fn global(out: &mut NativeProgram, name: &str) -> Operand {
    rip(out.symbol(name, Linkage::Local))
}
fn label(out: &mut NativeProgram, name: &str) {
    out.label(name);
}
fn jump(out: &mut NativeProgram, op: Opcode, name: &str) {
    let s = out.symbol(name, Linkage::Local);
    out.emit(op, [sym(s)]);
}
fn prologue(out: &mut NativeProgram) {
    out.emit(Push, [r64(5)]);
    out.emit(Mov64, [r64(4), r64(5)]);
    out.emit(Sub64, [imm(80), r64(4)]);
}
fn epilogue(out: &mut NativeProgram) {
    out.emit(Mov64, [r64(5), r64(4)]);
    out.emit(Pop, [r64(5)]);
    out.emit(Ret, []);
}
pub(super) fn enter(m: &Module, function: usize, origin: Origin, out: &mut NativeProgram) {
    if m.call_limit.is_none() || !m.source_functions[function] {
        return;
    }
    let (a, b) = if m.target.is_linux() { (7, 6) } else { (1, 2) };
    let s = site(function, origin);
    let name = s.name();
    let p = global(out, &name);
    out.emit(Lea64, [p, r64(a)]);
    let p = global(out, &format!("{name}_alloc"));
    out.emit(Lea64, [p, r64(b)]);
    call(out, "cerune_call_enter", false);
}
pub(super) fn leave(m: &Module, function: usize, out: &mut NativeProgram) {
    if m.call_limit.is_some() && m.source_functions[function] {
        let p = global(out, "cerune_call_depth");
        out.emit(Sub64, [imm(1), p]);
    }
}
fn text(out: &mut NativeProgram, name: &str, value: String) {
    out.label(name);
    out.push(EntryKind::CString(value));
}
pub(super) fn support(m: &Module, out: &mut NativeProgram) {
    let Some(limit) = m.call_limit else { return };
    let linux = m.target.is_linux();
    let (a, b, c) = if linux { (7, 6, 2) } else { (1, 2, 8) };
    out.push(EntryKind::Section(Section::ReadOnlyData));
    let mut sites = Vec::new();
    for (i, origin) in m.instructions.iter().zip(&m.origins).chain(
        m.functions
            .iter()
            .flat_map(|f| f.instructions.iter().zip(&f.origins)),
    ) {
        if let Instruction::Call { function_id, .. } = i
            && m.source_functions[*function_id]
        {
            let s = site(*function_id, *origin);
            if !sites.contains(&s) {
                sites.push(s);
            }
        }
    }
    for s in sites {
        text(out, &s.name(), s.suffix());
        text(out, &format!("{}_alloc", s.name()), s.allocation());
    }
    text(
        out,
        "cerune_call_header",
        format!("call-v1 reason=depth-limit limit={limit}"),
    );
    text(out, "cerune_call_frame_format", "call-v1 frame=%llu".into());
    out.push(EntryKind::Section(Section::Data));
    out.push(EntryKind::Align(3));
    for name in [
        "cerune_call_depth",
        "cerune_call_capacity",
        "cerune_call_frames",
    ] {
        out.label(name);
        out.push(EntryKind::Data64(0));
    }
    out.push(EntryKind::Section(Section::Text));
    // Write diagnostic text using the explicitly selected target's stderr stream.
    label(out, "cerune_call_write");
    prologue(out);
    out.emit(Mov64, [r64(a), memory(5, None, -8)]);
    if linux {
        let s = out.symbol("stderr", Linkage::External);
        out.emit(Mov64, [rip(s), r64(b)]);
    } else {
        out.emit(Mov32, [imm(2), r32(a)]);
        call(out, "__acrt_iob_func", true);
        out.emit(Mov64, [r64(0), r64(b)]);
    }
    out.emit(Mov64, [memory(5, None, -8), r64(a)]);
    call(out, "fputs", true);
    epilogue(out);
    label(out, "cerune_call_enter");
    prologue(out);
    out.emit(Mov64, [r64(a), memory(5, None, -8)]);
    out.emit(Mov64, [r64(b), memory(5, None, -16)]);
    let depth = global(out, "cerune_call_depth");
    out.emit(Mov64, [depth, r64(10)]);
    out.emit(Mov64, [r64(10), memory(5, None, -24)]);
    // u32 limits need a register: cmpq immediate sign-extends its 32-bit operand.
    out.emit(Mov64, [imm(i128::from(limit.get())), r64(11)]);
    out.emit(Cmp64, [r64(11), r64(10)]);
    jump(out, JumpAe, ".Lcall_overflow");
    let cap = global(out, "cerune_call_capacity");
    out.emit(Cmp64, [cap, r64(10)]);
    jump(out, JumpNe, ".Lcall_push");
    out.emit(Test64, [r64(10), r64(10)]);
    jump(out, JumpNe, ".Lcall_double");
    out.emit(Mov64, [imm(4), r64(10)]);
    label(out, ".Lcall_double");
    out.emit(Add64, [r64(10), r64(10)]);
    out.emit(Cmp64, [r64(11), r64(10)]);
    jump(out, JumpBe, ".Lcall_capacity_ready");
    out.emit(Mov64, [r64(11), r64(10)]);
    label(out, ".Lcall_capacity_ready");
    out.emit(Mov64, [r64(10), memory(5, None, -32)]);
    out.emit(ShiftLeft64, [imm(3), r64(10)]);
    let frames = global(out, "cerune_call_frames");
    out.emit(Mov64, [frames, r64(a)]);
    out.emit(Mov64, [r64(10), r64(b)]);
    call(out, "realloc", true);
    out.emit(Test64, [r64(0), r64(0)]);
    jump(out, JumpE, ".Lcall_allocation_failed");
    let frames = global(out, "cerune_call_frames");
    out.emit(Mov64, [r64(0), frames]);
    out.emit(Mov64, [memory(5, None, -32), r64(10)]);
    let cap = global(out, "cerune_call_capacity");
    out.emit(Mov64, [r64(10), cap]);
    label(out, ".Lcall_push");
    out.emit(Mov64, [memory(5, None, -24), r64(10)]);
    out.emit(ShiftLeft64, [imm(3), r64(10)]);
    let frames = global(out, "cerune_call_frames");
    out.emit(Add64, [frames, r64(10)]);
    out.emit(Mov64, [memory(5, None, -8), r64(11)]);
    out.emit(Mov64, [r64(11), memory(10, None, 0)]);
    let depth = global(out, "cerune_call_depth");
    out.emit(Add64, [imm(1), depth]);
    epilogue(out);
    label(out, ".Lcall_allocation_failed");
    out.emit(Xor32, [r32(a), r32(a)]);
    call(out, "fflush", true);
    out.emit(Mov64, [memory(5, None, -16), r64(a)]);
    call(out, "cerune_call_write", false);
    jump(out, Jump, ".Lcall_stop");
    label(out, ".Lcall_overflow");
    out.emit(Xor32, [r32(a), r32(a)]);
    call(out, "fflush", true);
    let p = global(out, "cerune_call_header");
    out.emit(Lea64, [p, r64(a)]);
    call(out, "cerune_call_write", false);
    out.emit(Mov64, [memory(5, None, -8), r64(a)]);
    call(out, "cerune_call_write", false);
    out.emit(Mov64, [imm(0), memory(5, None, -40)]);
    label(out, ".Lcall_history");
    if linux {
        let s = out.symbol("stderr", Linkage::External);
        out.emit(Mov64, [rip(s), r64(a)]);
    } else {
        out.emit(Mov32, [imm(2), r32(a)]);
        call(out, "__acrt_iob_func", true);
        out.emit(Mov64, [r64(0), r64(a)]);
    }
    let p = global(out, "cerune_call_frame_format");
    out.emit(Lea64, [p, r64(b)]);
    out.emit(Mov64, [memory(5, None, -40), r64(c)]);
    if linux {
        out.emit(Xor32, [r32(0), r32(0)]);
    }
    call(out, "fprintf", true);
    out.emit(Mov64, [memory(5, None, -40), r64(10)]);
    out.emit(ShiftLeft64, [imm(3), r64(10)]);
    let frames = global(out, "cerune_call_frames");
    out.emit(Add64, [frames, r64(10)]);
    out.emit(Mov64, [memory(10, None, 0), r64(a)]);
    call(out, "cerune_call_write", false);
    out.emit(Add64, [imm(1), memory(5, None, -40)]);
    out.emit(Mov64, [memory(5, None, -40), r64(10)]);
    out.emit(Cmp64, [memory(5, None, -24), r64(10)]);
    jump(out, JumpB, ".Lcall_history");
    label(out, ".Lcall_stop");
    out.emit(Xor32, [r32(a), r32(a)]);
    call(out, "fflush", true);
    out.emit(Mov32, [imm(1), r32(a)]);
    call(out, "_Exit", true);
    out.emit(Undefined, []);
    label(out, "cerune_call_finish");
    prologue(out);
    let frames = global(out, "cerune_call_frames");
    out.emit(Mov64, [frames, r64(a)]);
    call(out, "free", true);
    let frames = global(out, "cerune_call_frames");
    out.emit(Mov64, [imm(0), frames]);
    let cap = global(out, "cerune_call_capacity");
    out.emit(Mov64, [imm(0), cap]);
    epilogue(out);
}
