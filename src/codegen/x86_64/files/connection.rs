//! 明示ターゲットのOS接続。名前の意味検査・転送再試行は親モジュールが担当します。
use super::super::machine::Scale;
use super::*;
mod entry;
mod linux;
mod windows;
pub(in crate::codegen::x86_64) fn append(o: &mut NativeProgram) {
    for name in [
        "cerune_file_host_configured",
        "cerune_file_host_open",
        "cerune_file_host_read",
        "cerune_file_host_write",
        "cerune_file_host_close",
        "cerune_files_parse",
        "cerune_files_connect",
        "cerune_files_disconnect",
        "cerune_files_startup_failed",
        "cerune_files_open_code",
    ] {
        o.symbol(name, Linkage::Local);
    }
    o.push(EntryKind::Section(Section::Data));
    o.push(EntryKind::Align(3));
    for n in ["root", "configured"] {
        o.label(&format!(".Lfile_os_{n}"));
        o.push(EntryKind::Data64(0));
    }
    if !is_linux(o) {
        for name in ["cerune_files_nt_open", "cerune_files_kind"] {
            o.symbol(name, Linkage::Local);
        }
    }
    entry::append(o);
    if is_linux(o) {
        linux::append(o);
    } else {
        windows::append(o);
    }
    o.label("cerune_file_host_configured");
    let p = global(o, "configured");
    mov(o, p, r64(0));
    o.emit(Opcode::Ret, []);
    start(o, "cerune_files_disconnect", 136, 0);
    let p = global(o, "configured");
    compare(o, p, 0);
    branch(o, Opcode::JumpE, "os_disconnect_done");
    let p = global(o, "root");
    mov(o, p, r64(0));
    mov(o, r64(0), slot(128));
    call(o, "cerune_file_host_close", &[slot(128)]);
    let p = global(o, "configured");
    mov(o, imm(0), p);
    label(o, "os_disconnect_done");
    finish(o, 136);
}
fn is_linux(o: &NativeProgram) -> bool {
    o.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu
}
fn global(o: &mut NativeProgram, n: &str) -> Operand {
    local(o, &format!(".Lfile_os_{n}"))
}
fn start(o: &mut NativeProgram, name: &str, frame: i128, args: usize) {
    assert_eq!(frame % 16, 8);
    o.push(EntryKind::Section(Section::Text));
    o.label(name);
    o.emit(Opcode::Sub64, [imm(frame), r64(4)]);
    let regs: &[u8] = if is_linux(o) {
        &[7, 6, 2, 1, 8, 9]
    } else {
        &[1, 2, 8, 9]
    };
    for i in 0..args {
        if i < regs.len() {
            mov(o, r64(regs[i]), slot(128 + 8 * i as i128));
        } else {
            mov(
                o,
                slot(frame + 8 + if is_linux(o) { 0 } else { 32 } + 8 * (i - regs.len()) as i128),
                r64(0),
            );
            mov(o, r64(0), slot(128 + 8 * i as i128));
        }
    }
}
fn finish(o: &mut NativeProgram, frame: i128) {
    o.emit(Opcode::Add64, [imm(frame), r64(4)]);
    o.emit(Opcode::Ret, []);
}
fn result(o: &mut NativeProgram, frame: i128, n: i128) {
    mov(o, imm(n), r64(0));
    finish(o, frame);
}
fn ptr(o: &mut NativeProgram, at: i128, dest: i128) {
    o.emit(Opcode::Lea64, [slot(at), r64(0)]);
    mov(o, r64(0), slot(dest));
}
fn zero(o: &mut NativeProgram, at: i128, n: usize) {
    for i in 0..n {
        mov(o, imm(0), slot(at + 8 * i as i128));
    }
}
fn test(o: &mut NativeProgram) {
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
}
fn startup(o: &mut NativeProgram, phase: &str, code: &str) {
    pointer(o, phase, 128);
    pointer(o, code, 136);
    call(o, "cerune_files_startup_failed", &[slot(128), slot(136)]);
    o.emit(Opcode::Undefined, []);
}
fn open_failure(o: &mut NativeProgram, code: i128) {
    call(o, "cerune_files_open_code", &[slot(code)]);
    mov(o, r64(0), slot(136));
    pointer(o, "open-root", 128);
    call(o, "cerune_files_startup_failed", &[slot(128), slot(136)]);
    o.emit(Opcode::Undefined, []);
}

fn word(o: &mut NativeProgram, base: u8, index: Option<(u8, Scale)>, disp: i128, dest: u8) {
    // 16bit命令を仮定せず、2バイトを明示的に組み立てます。r11は一時値です。
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(base, index, disp), r32(dest)],
    );
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(base, index, disp + 1), r32(11)],
    );
    o.emit(Opcode::ShiftLeft64, [imm(8), r64(11)]);
    o.emit(Opcode::Or32, [r32(11), r32(dest)]);
}
