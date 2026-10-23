//! MIR由来の行入力。ASMと自前Objectへ同じ処理を出力します。
use super::ir::{Instruction, Module};
use super::machine::{EntryKind, Linkage, Opcode, Operand, Scale, Section, r8, r32, r64};
use super::program::{NativeProgram, address};

pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i, Instruction::ReadLine))
}
pub(super) fn append(out: &mut NativeProgram) {
    let linux = out.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    out.push(EntryKind::Section(Section::ReadOnlyData));
    for code in [
        "invalid-limit",
        "unsupported-connection",
        "budget-exceeded",
        "allocation-failed",
        "read-failed",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
        "known",
        "unknown",
    ] {
        let suffix = if matches!(code, "invalid-limit" | "read-failed" | "invalid-response") {
            "_text"
        } else {
            ""
        };
        let label = code.replace('-', "_");

        out.label(&format!(".Linput_{label}{suffix}"));
        out.push(EntryKind::CString(code.to_string()));
    }

    out.label(".Linput_format");
    out.push(EntryKind::CString(
        "cerune: io-v1 operation=read-line code=%s limit=%lld consumed=%llu consumption=%s%s"
            .to_owned(),
    ));

    common(out, linux);
    if linux {
        linux_runtime(out);
    } else {
        windows(out);
    }
    // 呼出し時のrspは16byte整列。Windowsのshadow spaceとstack引数は0..64です。

    out.label(".Linput_format_failure");

    if linux {
        out.emit(Opcode::Lea64, [address(4, None, 160), r64(7)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(512), r32(6)]);

        let symbol = out.symbol(".Linput_format", Linkage::Local);
        out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

        out.emit(Opcode::Mov64, [address(4, None, 112), r64(1)]);
        out.emit(Opcode::Mov64, [address(4, None, 64), r64(8)]);
        out.emit(Opcode::Mov64, [address(4, None, 96), r64(9)]);
        out.emit(Opcode::Mov64, [address(4, None, 104), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 0)]);
        out.emit(Opcode::Mov64, [address(4, None, 72), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 8)]);
        out.emit(Opcode::Xor32, [r32(0), r32(0)]);
    } else {
        out.emit(Opcode::Lea64, [address(4, None, 160), r64(1)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(512), r32(2)]);

        let symbol = out.symbol(".Linput_format", Linkage::Local);
        out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(8)]);

        out.emit(Opcode::Mov64, [address(4, None, 112), r64(9)]);
        out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
        out.emit(Opcode::Mov64, [address(4, None, 96), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 40)]);
        out.emit(Opcode::Mov64, [address(4, None, 104), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 48)]);
        out.emit(Opcode::Mov64, [address(4, None, 72), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 56)]);
    };

    let symbol = out.symbol("snprintf", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Linput_exit", Linkage::Local);
    out.emit(Opcode::JumpLe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(512), r32(0)]);

    let symbol = out.symbol(".Linput_diagnostic", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov32, [Operand::Immediate(511), r32(0)]);
    out.label(".Linput_diagnostic");

    if linux {
        out.emit(Opcode::Mov32, [r32(0), r32(2)]);
        out.emit(Opcode::Lea64, [address(4, None, 160), r64(6)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(2), r32(7)]);

        let symbol = out.symbol("write", Linkage::External);
        out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        out.label(".Linput_exit");
        out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(7)]);
    } else {
        out.emit(Opcode::Mov32, [r32(0), r32(8)]);
        out.emit(Opcode::Lea64, [address(4, None, 160), r64(2)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(2), r32(1)]);

        let symbol = out.symbol("_write", Linkage::External);
        out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        out.label(".Linput_exit");
        out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(1)]);
    };

    let symbol = out.symbol("_Exit", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
// 入口rax=limit、r11=不変の出自。戻り値rax=既存string表現。
// frame: 64 limit,72 origin,80 owner,88 length,96 consumed,104 known,112 code,
// 120 byte,128 UTF8残数,136下限,144上限,160..672診断。所有権は成功時だけ公開します。
fn common(out: &mut NativeProgram, linux: bool) {
    let (first, second) = if linux { (7, 6) } else { (1, 2) };

    out.push(EntryKind::Section(Section::Data));
    out.push(EntryKind::Align(3));
    out.label(".Linput_eof");
    out.push(EntryKind::Data64(0_u64));
    out.push(EntryKind::Section(Section::Text));
    out.label("cerune_input_read_line");
    out.emit(Opcode::Sub64, [Operand::Immediate(696), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);
    out.emit(Opcode::Mov64, [r64(11), address(4, None, 72)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 80)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 88)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 96)]);

    let symbol = out.symbol(".Linput_known", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(1)]);

    out.emit(Opcode::Mov64, [r64(1), address(4, None, 104)]);
    out.emit(Opcode::Sub64, [Operand::Immediate(1), r64(0)]);
    out.emit(Opcode::Cmp64, [Operand::Immediate(2147483647), r64(0)]);

    let symbol = out.symbol(".Linput_invalid_limit", Linkage::Local);
    out.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol("cerune_input_connection", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_eof", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Linput_empty", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Lcerune_heap_limit", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Sub64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Cmp64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Linput_budget", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(first)]);
    out.emit(Opcode::Add64, [Operand::Immediate(24), r64(first)]);

    let symbol = out.symbol("malloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_allocation", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 80)]);
    out.emit(Opcode::Mov64, [address(4, None, 64), r64(1)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Add64, [r64(1), Operand::RipRelative(symbol)]);

    out.label(".Linput_read");
    out.emit(Opcode::Lea64, [address(4, None, 120), r64(first)]);

    let symbol = out.symbol("cerune_input_read_byte", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp64, [Operand::Immediate(-2), r64(0)]);

    let symbol = out.symbol(".Linput_read", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_read_failed", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp64, [Operand::Immediate(1), r64(0)]);

    let symbol = out.symbol(".Linput_invalid_response", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_eof_seen", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 96)]);
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(1)]);
    out.emit(Opcode::Cmp64, [address(4, None, 64), r64(1)]);

    let symbol = out.symbol(".Linput_limit", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 80), r64(2)]);
    out.emit(Opcode::Add64, [Operand::Immediate(24), r64(2)]);
    out.emit(Opcode::ZeroExtend8To32, [address(4, None, 120), r32(0)]);
    out.emit(Opcode::Mov8, [r8(0), address(2, Some((1, Scale::One)), 0)]);
    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 88)]);
    out.emit(Opcode::Cmp32, [Operand::Immediate(10), r32(0)]);

    let symbol = out.symbol(".Linput_read", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_validate", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_eof_seen");

    let symbol = out.symbol(".Linput_eof", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(1), Operand::RipRelative(symbol)],
    );

    out.label(".Linput_validate");
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(1)]);
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(2)]);
    out.emit(Opcode::Add64, [Operand::Immediate(24), r64(2)]);

    let symbol = out.symbol("cerune_utf8", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Linput_utf8_failed", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Sub64, [r64(0), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Cmp64, [Operand::Immediate(0), address(4, None, 88)]);

    let symbol = out.symbol(".Linput_free_empty", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);
    out.emit(Opcode::ShiftRight64, [Operand::Immediate(1), r64(0)]);
    out.emit(Opcode::Cmp64, [address(4, None, 88), r64(0)]);

    let symbol = out.symbol(".Linput_publish", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);
    out.emit(Opcode::Sub64, [address(4, None, 88), r64(0)]);
    out.emit(Opcode::Cmp64, [Operand::Immediate(4096), r64(0)]);

    let symbol = out.symbol(".Linput_publish", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 80), r64(first)]);
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(second)]);
    out.emit(Opcode::Add64, [Operand::Immediate(24), r64(second)]);

    let symbol = out.symbol("realloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_publish", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 80)]);
    out.label(".Linput_publish");
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(0)]);

    let symbol = out.symbol(".Lcerune_heap_head", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(1)]);

    out.emit(Opcode::Mov64, [r64(1), address(0, None, 0)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(1), address(0, None, 8)]);
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(1)]);
    out.emit(Opcode::Mov64, [r64(1), address(0, None, 16)]);

    let symbol = out.symbol(".Lcerune_heap_head", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Add64, [r64(1), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Add64, [Operand::Immediate(16), r64(0)]);
    out.emit(Opcode::Add64, [Operand::Immediate(696), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Linput_free_empty");
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(first)]);

    let symbol = out.symbol("free", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.label(".Linput_empty");

    let symbol = out.symbol(".Lcerune_heap_empty", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Add64, [Operand::Immediate(696), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Linput_invalid_limit");

    let symbol = out.symbol(".Linput_invalid_limit_text", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_budget");

    let symbol = out.symbol(".Linput_budget_exceeded", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_allocation");

    let symbol = out.symbol(".Linput_allocation_failed", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_read_failed");

    let symbol = out.symbol(".Linput_read_failed_text", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_invalid_response");

    let symbol = out.symbol(".Linput_unknown", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 104)]);

    let symbol = out.symbol(".Linput_invalid_response_text", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_limit");

    let symbol = out.symbol(".Linput_limit_exceeded", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_utf8_failed");

    let symbol = out.symbol(".Linput_invalid_utf8", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.label(".Linput_failed");
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 112)]);
    out.emit(Opcode::Cmp64, [Operand::Immediate(0), address(4, None, 80)]);

    let symbol = out.symbol(".Linput_format_failure", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Sub64, [r64(0), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 80), r64(first)]);

    let symbol = out.symbol("free", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_format_failure", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);
}
fn linux_runtime(out: &mut NativeProgram) {
    out.label("cerune_input_connection");
    out.emit(Opcode::Xor32, [r32(0), r32(0)]);
    out.emit(Opcode::Ret, []);
    out.label("cerune_input_read_byte");
    out.emit(Opcode::Sub64, [Operand::Immediate(40), r64(4)]);
    out.emit(Opcode::Mov64, [r64(7), r64(6)]);
    out.emit(Opcode::Xor32, [r32(7), r32(7)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(2)]);

    let symbol = out.symbol("read", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_linux_done", Linkage::Local);
    out.emit(Opcode::JumpNs, [Operand::Symbol(symbol)]);

    let symbol = out.symbol("__errno_location", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(4), address(0, None, 0)]);

    let symbol = out.symbol(".Linput_linux_error", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [Operand::Immediate(-2), r64(0)]);

    let symbol = out.symbol(".Linput_linux_done", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_linux_error");
    out.emit(Opcode::Mov64, [Operand::Immediate(-1), r64(0)]);
    out.label(".Linput_linux_done");
    out.emit(Opcode::Add64, [Operand::Immediate(40), r64(4)]);
    out.emit(Opcode::Ret, []);
}
fn windows(out: &mut NativeProgram) {
    out.push(EntryKind::Section(Section::Data));
    out.push(EntryKind::Align(3));
    out.label(".Linput_initialized");
    out.push(EntryKind::Data64(0_u64));
    out.label(".Linput_handle");
    out.push(EntryKind::Data64(0_u64));
    out.label(".Linput_pipe");
    out.push(EntryKind::Data64(0_u64));
    out.label(".Linput_connection_error");
    out.push(EntryKind::Data64(0_u64));
    out.push(EntryKind::Section(Section::Text));
    out.label("cerune_input_connection");
    out.emit(Opcode::Sub64, [Operand::Immediate(56), r64(4)]);

    let symbol = out.symbol(".Linput_initialized", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Linput_connection_done", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_initialized", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(1), Operand::RipRelative(symbol)],
    );

    out.emit(Opcode::Mov32, [Operand::Immediate(-10), r32(1)]);

    let symbol = out.symbol("GetStdHandle", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_handle", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Linput_connection_failed", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp64, [Operand::Immediate(-1), r64(0)]);

    let symbol = out.symbol(".Linput_connection_failed", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), r64(1)]);
    out.emit(Opcode::Lea64, [address(4, None, 40), r64(2)]);

    let symbol = out.symbol("GetConsoleMode", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Linput_console", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_handle", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(1)]);

    let symbol = out.symbol("GetFileType", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(3), r32(0)]);

    let symbol = out.symbol(".Linput_connection_done", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_pipe", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(1), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Linput_connection_done", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_console");

    let symbol = out.symbol(".Linput_unsupported_connection", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Linput_connection_store", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_connection_failed");

    let symbol = out.symbol(".Linput_read_failed_text", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.label(".Linput_connection_store");

    let symbol = out.symbol(".Linput_connection_error", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    out.label(".Linput_connection_done");

    let symbol = out.symbol(".Linput_connection_error", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Add64, [Operand::Immediate(56), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label("cerune_input_read_byte");
    out.emit(Opcode::Sub64, [Operand::Immediate(72), r64(4)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 48)]);
    out.label(".Linput_windows_read");
    out.emit(Opcode::Mov32, [Operand::Immediate(0), address(4, None, 56)]);

    let symbol = out.symbol(".Linput_handle", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(1)]);

    out.emit(Opcode::Mov64, [address(4, None, 48), r64(2)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(8)]);
    out.emit(Opcode::Lea64, [address(4, None, 56), r64(9)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 32)]);

    let symbol = out.symbol("ReadFile", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Linput_windows_error", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov32, [address(4, None, 56), r32(0)]);
    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Linput_windows_done", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_pipe", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Linput_windows_read", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Linput_windows_done", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_windows_error");

    let symbol = out.symbol("GetLastError", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(234), r32(0)]);

    let symbol = out.symbol(".Linput_windows_eof_check", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(0), address(4, None, 56)]);

    let symbol = out.symbol(".Linput_windows_eof_check", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov32, [address(4, None, 56), r32(0)]);

    let symbol = out.symbol(".Linput_windows_done", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_windows_eof_check");
    out.emit(Opcode::Cmp32, [Operand::Immediate(109), r32(0)]);

    let symbol = out.symbol(".Linput_windows_eof", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(38), r32(0)]);

    let symbol = out.symbol(".Linput_windows_eof", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [Operand::Immediate(-1), r64(0)]);

    let symbol = out.symbol(".Linput_windows_done", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Linput_windows_eof");
    out.emit(Opcode::Xor32, [r32(0), r32(0)]);
    out.label(".Linput_windows_done");
    out.emit(Opcode::Add64, [Operand::Immediate(72), r64(4)]);
    out.emit(Opcode::Ret, []);
}
