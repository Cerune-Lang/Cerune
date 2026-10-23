//! Native argument capture and execution. Internal helpers are not a public ABI.
use super::ir::{Instruction, Module};
use super::machine::{EntryKind, Linkage, Opcode, Operand, Scale, Section, r32, r64};
use super::program::{NativeProgram, address};

pub(super) fn used(module: &Module) -> bool {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i, Instruction::ArgumentCount | Instruction::ArgumentGet))
}
pub(super) fn append(out: &mut NativeProgram, module: &Module) {
    let linux = module.target.is_linux();
    out.push(EntryKind::Section(Section::ReadOnlyData));
    for code in [
        "invalid-limit",
        "count-limit-exceeded",
        "byte-limit-exceeded",
        "invalid-utf8",
        "embedded-nul",
        "invalid-unicode",
        "allocation-failed",
        "unconfigured",
        "invalid-index",
        "index-out-of-bounds",
        "budget-exceeded",
    ] {
        out.label(&format!(".Largs_{}", code.replace('-', "_")));
        out.push(EntryKind::CString(code.to_string()));
    }
    for (name, format) in [
        ("capture", "cerune: arguments-v1 phase=capture code=%s\n"),
        (
            "capture_index",
            "cerune: arguments-v1 phase=capture code=%s index=%lld\n",
        ),
        (
            "count",
            "cerune: arguments-v1 phase=execute operation=count code=%s%s",
        ),
        (
            "get",
            "cerune: arguments-v1 phase=execute operation=get code=%s index=%lld%s",
        ),
    ] {
        out.label(&format!(".Largs_format_{name}"));
        out.push(EntryKind::CString(format.to_string()));
    }

    out.push(EntryKind::Align(3));
    out.label(".Largs_max_count");
    out.push(EntryKind::Data64(module.argument_limits.max_count));
    out.label(".Largs_max_bytes");
    out.push(EntryKind::Data64(module.argument_limits.max_bytes));

    snapshot(out, linux);
    get(out, linux);
    failure(out, linux);
    if linux {
        out.emit(Opcode::Lea64, [address(4, None, 80), r64(7)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(512), r32(6)]);
        out.emit(Opcode::Mov64, [address(4, None, 32), r64(2)]);
        out.emit(Opcode::Mov64, [address(4, None, 40), r64(1)]);
        out.emit(Opcode::Mov64, [address(4, None, 48), r64(8)]);
        out.emit(Opcode::Mov64, [address(4, None, 56), r64(9)]);
        out.emit(Opcode::Xor32, [r32(0), r32(0)]);
    } else {
        out.emit(Opcode::Mov64, [address(4, None, 48), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
        out.emit(Opcode::Mov64, [address(4, None, 56), r64(0)]);
        out.emit(Opcode::Mov64, [r64(0), address(4, None, 40)]);
        out.emit(Opcode::Lea64, [address(4, None, 80), r64(1)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(512), r32(2)]);
        out.emit(Opcode::Mov64, [address(4, None, 64), r64(8)]);
        out.emit(Opcode::Mov64, [address(4, None, 72), r64(9)]);
    };

    let symbol = out.symbol("snprintf", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Largs_exit", Linkage::Local);
    out.emit(Opcode::JumpLe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp32, [Operand::Immediate(512), r32(0)]);

    let symbol = out.symbol(".Largs_write", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov32, [Operand::Immediate(511), r32(0)]);
    out.label(".Largs_write");

    if linux {
        out.emit(Opcode::Mov32, [r32(0), r32(2)]);
        out.emit(Opcode::Lea64, [address(4, None, 80), r64(6)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(2), r32(7)]);

        let symbol = out.symbol("write", Linkage::External);
        out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        out.label(".Largs_exit");
        out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(7)]);
    } else {
        out.emit(Opcode::Mov32, [r32(0), r32(8)]);
        out.emit(Opcode::Lea64, [address(4, None, 80), r64(2)]);
        out.emit(Opcode::Mov32, [Operand::Immediate(2), r32(1)]);

        let symbol = out.symbol("_write", Linkage::External);
        out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

        out.label(".Largs_exit");
        out.emit(Opcode::Mov32, [Operand::Immediate(1), r32(1)]);
    };

    let symbol = out.symbol("_Exit", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
// Capture: rax=count, rdx=borrowed [pointer,length] pairs. Validate everything
// before allocating a single private block, then publish count/pointer together.
// Frame: 32 count,40 pairs,48 total,56 index,64 block,72 cursor,80 slot,88 length.
fn snapshot(out: &mut NativeProgram, linux: bool) {
    let (first, second, third) = if linux { (7, 6, 2) } else { (1, 2, 8) };

    out.push(EntryKind::Section(Section::Data));
    out.push(EntryKind::Align(3));
    out.label(".Largs_ready");
    out.push(EntryKind::Data64(0_u64));
    out.label(".Largs_count");
    out.push(EntryKind::Data64(0_u64));
    out.label(".Largs_values");
    out.push(EntryKind::Data64(0_u64));
    out.push(EntryKind::Section(Section::Text));
    out.label("cerune_arguments_capture");
    out.emit(Opcode::Sub64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [r64(2), address(4, None, 40)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 56)]);

    let symbol = out.symbol(".Largs_max_count", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Largs_bad_limit", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_bad_count", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_max_count", Linkage::Local);
    out.emit(Opcode::Cmp64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_bad_count", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    out.label(".Largs_validate");
    out.emit(Opcode::Mov64, [address(4, None, 56), r64(0)]);
    out.emit(Opcode::Cmp64, [address(4, None, 32), r64(0)]);

    let symbol = out.symbol(".Largs_allocate", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);
    out.emit(Opcode::Add64, [address(4, None, 40), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 80)]);
    out.emit(Opcode::Mov64, [address(0, None, 8), r64(1)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 88)]);
    out.emit(Opcode::Add64, [address(4, None, 48), r64(1)]);

    let symbol = out.symbol(".Largs_bad_bytes", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_max_bytes", Linkage::Local);
    out.emit(Opcode::Cmp64, [Operand::RipRelative(symbol), r64(1)]);

    let symbol = out.symbol(".Largs_bad_bytes", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(1), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [address(0, None, 8), r64(1)]);
    out.emit(Opcode::Mov64, [address(0, None, 0), r64(2)]);

    let symbol = out.symbol("cerune_utf8", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Largs_bad_utf8", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 80), r64(0)]);
    out.emit(Opcode::Mov64, [address(0, None, 0), r64(2)]);
    out.emit(Opcode::Xor32, [r32(1), r32(1)]);
    out.label(".Largs_nul_loop");
    out.emit(Opcode::Cmp64, [address(4, None, 88), r64(1)]);

    let symbol = out.symbol(".Largs_valid", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(
        Opcode::Cmp8,
        [Operand::Immediate(0), address(2, Some((1, Scale::One)), 0)],
    );

    let symbol = out.symbol(".Largs_bad_nul", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Add64, [Operand::Immediate(1), r64(1)]);

    let symbol = out.symbol(".Largs_nul_loop", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_valid");
    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 56)]);

    let symbol = out.symbol(".Largs_validate", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_allocate");
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(-1), address(4, None, 56)],
    );
    out.emit(Opcode::Mov64, [address(4, None, 32), r64(0)]);
    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_empty_snapshot", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(
        Opcode::MovAbs64,
        [Operand::Immediate(576460752303423487), r64(1)],
    );
    out.emit(Opcode::Cmp64, [r64(1), r64(0)]);

    let symbol = out.symbol(".Largs_snapshot_allocation", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 72)]);
    out.emit(Opcode::Add64, [address(4, None, 48), r64(0)]);

    let symbol = out.symbol(".Largs_snapshot_allocation", Linkage::Local);
    out.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_snapshot_allocation", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), r64(first)]);

    let symbol = out.symbol("malloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_snapshot_allocation", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);
    out.emit(Opcode::Add64, [r64(0), address(4, None, 72)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 56)]);
    out.label(".Largs_copy");
    out.emit(Opcode::Mov64, [address(4, None, 56), r64(0)]);
    out.emit(Opcode::Cmp64, [address(4, None, 32), r64(0)]);

    let symbol = out.symbol(".Largs_publish", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), r64(10)]);
    out.emit(Opcode::Add64, [address(4, None, 40), r64(0)]);
    out.emit(Opcode::Add64, [address(4, None, 64), r64(10)]);
    out.emit(Opcode::Mov64, [address(0, None, 8), r64(11)]);
    out.emit(Opcode::Mov64, [r64(11), address(4, None, 88)]);
    out.emit(Opcode::Mov64, [r64(11), address(10, None, 8)]);
    out.emit(Opcode::Mov64, [address(4, None, 72), r64(11)]);
    out.emit(Opcode::Mov64, [r64(11), address(10, None, 0)]);
    out.emit(Opcode::Cmp64, [Operand::Immediate(0), address(4, None, 88)]);

    let symbol = out.symbol(".Largs_copy_next", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 72), r64(first)]);
    out.emit(Opcode::Mov64, [address(0, None, 0), r64(second)]);
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(third)]);

    let symbol = out.symbol("memcpy", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.label(".Largs_copy_next");
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(0)]);
    out.emit(Opcode::Add64, [r64(0), address(4, None, 72)]);
    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 56)]);

    let symbol = out.symbol(".Largs_copy", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_empty_snapshot");
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 64)]);
    out.label(".Largs_publish");
    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Largs_values", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 32), r64(0)]);

    let symbol = out.symbol(".Largs_count", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    let symbol = out.symbol(".Largs_ready", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(1), Operand::RipRelative(symbol)],
    );

    out.emit(Opcode::Add64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_bad_limit");

    let symbol = out.symbol(".Largs_invalid_limit", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(-1), address(4, None, 56)],
    );

    let symbol = out.symbol(".Largs_capture_error", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_bad_count");

    let symbol = out.symbol(".Largs_count_limit_exceeded", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(-1), address(4, None, 56)],
    );

    let symbol = out.symbol(".Largs_capture_error", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_bad_bytes");

    let symbol = out.symbol(".Largs_byte_limit_exceeded", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_capture_error", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_bad_utf8");

    let symbol = out.symbol(".Largs_invalid_utf8", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_capture_error", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_bad_nul");

    let symbol = out.symbol(".Largs_embedded_nul", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_capture_error", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_snapshot_allocation");

    let symbol = out.symbol(".Largs_allocation_failed", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.label(".Largs_capture_error");
    out.emit(Opcode::Mov64, [address(4, None, 56), r64(1)]);

    let symbol = out.symbol("cerune_arguments_capture_failed", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
    out.label("cerune_arguments_dispose");
    out.emit(Opcode::Sub64, [Operand::Immediate(40), r64(4)]);

    let symbol = out.symbol(".Largs_values", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(first)]);

    let symbol = out.symbol("free", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_ready", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Largs_values", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Largs_count", Linkage::Local);
    out.emit(
        Opcode::Mov64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    out.emit(Opcode::Add64, [Operand::Immediate(40), r64(4)]);
    out.emit(Opcode::Ret, []);
}
// Execution: r11=immutable origin, rax=get index. Return the existing string
// representation [length,bytes], owned by the ordinary string heap.
fn get(out: &mut NativeProgram, linux: bool) {
    let (first, second, third) = if linux { (7, 6, 2) } else { (1, 2, 8) };

    out.label("cerune_arguments_count");

    let symbol = out.symbol(".Largs_ready", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Largs_count_unconfigured", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_count", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Ret, []);
    out.label(".Largs_count_unconfigured");

    let symbol = out.symbol(".Largs_format_count", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_unconfigured", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    out.emit(Opcode::Mov64, [r64(11), r64(1)]);

    let symbol = out.symbol("cerune_arguments_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label("cerune_arguments_get");
    out.emit(Opcode::Sub64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [r64(11), address(4, None, 40)]);

    let symbol = out.symbol(".Largs_ready", Linkage::Local);
    out.emit(
        Opcode::Cmp64,
        [Operand::Immediate(0), Operand::RipRelative(symbol)],
    );

    let symbol = out.symbol(".Largs_get_unconfigured", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_get_negative", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_count", Linkage::Local);
    out.emit(Opcode::Cmp64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_get_bounds", Linkage::Local);
    out.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);

    let symbol = out.symbol(".Largs_values", Linkage::Local);
    out.emit(Opcode::Add64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Mov64, [address(0, None, 0), r64(1)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [address(0, None, 8), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 56)]);
    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_get_empty", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Lcerune_heap_limit", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(1)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Sub64, [Operand::RipRelative(symbol), r64(1)]);

    out.emit(Opcode::Cmp64, [r64(1), r64(0)]);

    let symbol = out.symbol(".Largs_get_budget", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Lcerune_heap_max", Linkage::Local);
    out.emit(Opcode::Cmp64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_get_allocation", Linkage::Local);
    out.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Lea64, [address(0, None, 24), r64(first)]);

    let symbol = out.symbol("malloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_get_allocation", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);

    let symbol = out.symbol(".Lcerune_heap_head", Linkage::Local);
    out.emit(Opcode::Mov64, [Operand::RipRelative(symbol), r64(1)]);

    out.emit(Opcode::Mov64, [r64(1), address(0, None, 0)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(1), address(0, None, 8)]);
    out.emit(Opcode::Mov64, [address(4, None, 56), r64(1)]);
    out.emit(Opcode::Mov64, [r64(1), address(0, None, 16)]);
    out.emit(Opcode::Lea64, [address(0, None, 24), r64(first)]);
    out.emit(Opcode::Mov64, [address(4, None, 48), r64(second)]);
    out.emit(Opcode::Mov64, [address(4, None, 56), r64(third)]);

    let symbol = out.symbol("memcpy", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Lcerune_heap_head", Linkage::Local);
    out.emit(Opcode::Mov64, [r64(0), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 56), r64(1)]);

    let symbol = out.symbol(".Lcerune_heap_live", Linkage::Local);
    out.emit(Opcode::Add64, [r64(1), Operand::RipRelative(symbol)]);

    out.emit(Opcode::Add64, [Operand::Immediate(16), r64(0)]);
    out.emit(Opcode::Add64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_get_empty");

    let symbol = out.symbol(".Lcerune_heap_empty", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Add64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_get_unconfigured");

    let symbol = out.symbol(".Largs_unconfigured", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    let symbol = out.symbol(".Largs_get_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_get_negative");

    let symbol = out.symbol(".Largs_invalid_index", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    let symbol = out.symbol(".Largs_get_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_get_bounds");

    let symbol = out.symbol(".Largs_index_out_of_bounds", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    let symbol = out.symbol(".Largs_get_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_get_budget");

    let symbol = out.symbol(".Largs_budget_exceeded", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    let symbol = out.symbol(".Largs_get_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_get_allocation");

    let symbol = out.symbol(".Largs_allocation_failed", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(2)]);

    out.label(".Largs_get_failed");

    let symbol = out.symbol(".Largs_format_get", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Mov64, [address(4, None, 32), r64(1)]);
    out.emit(Opcode::Mov64, [address(4, None, 40), r64(11)]);

    let symbol = out.symbol("cerune_arguments_failed", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
// Format/code/argument/origin in rax/rdx/rcx/r11; no input contents.
// The optional origin distinguishes execute failures from capture failures.
fn failure(out: &mut NativeProgram, linux: bool) {
    let first = if linux { 7 } else { 1 };

    out.label("cerune_arguments_capture_failed");
    out.emit(Opcode::Mov64, [r64(0), r64(2)]);

    let symbol = out.symbol(".Largs_format_capture", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Test64, [r64(1), r64(1)]);

    let symbol = out.symbol(".Largs_capture_format_ready", Linkage::Local);
    out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

    let symbol = out.symbol(".Largs_format_capture_index", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.label(".Largs_capture_format_ready");
    out.emit(Opcode::Xor32, [r32(11), r32(11)]);

    let symbol = out.symbol("cerune_arguments_failed", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label("cerune_arguments_failed");
    out.emit(Opcode::Sub64, [Operand::Immediate(600), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [r64(2), address(4, None, 40)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [r64(11), address(4, None, 56)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);
    out.emit(Opcode::Mov64, [r64(2), address(4, None, 72)]);
    out.emit(Opcode::Test64, [r64(11), r64(11)]);

    let symbol = out.symbol(".Largs_failure_format", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [Operand::Immediate(0), r64(first)]);

    let symbol = out.symbol("fflush", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.label(".Largs_failure_format");
}

pub(super) fn append_entry(out: &mut NativeProgram) {
    let linux = out.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    entry(out, linux);
    if linux {
        linux_value(out);
    } else {
        windows_value(out);
    }
}
// OS conversion precedes common capture, as in C/LLVM/QBE. argc excludes argv[0].
// Frame: 64 count,72 argv,80 temporary pairs,88 index,96 result.
fn entry(out: &mut NativeProgram, linux: bool) {
    let (first, argv) = if linux { (7, 6) } else { (1, 2) };

    out.push(EntryKind::Section(Section::Text));
    out.global_label(if linux { "main" } else { "wmain" });
    out.emit(Opcode::Sub64, [Operand::Immediate(120), r64(4)]);
    out.emit(Opcode::Mov64, [r64(argv), address(4, None, 72)]);
    out.emit(Opcode::Mov32, [r32(first), r32(0)]);
    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Largs_start_empty", Linkage::Local);
    out.emit(Opcode::JumpLe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Sub32, [Operand::Immediate(1), r32(0)]);

    let symbol = out.symbol(".Largs_start_size", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_start_empty");
    out.emit(Opcode::Xor32, [r32(0), r32(0)]);
    out.label(".Largs_start_size");
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 80)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 88)]);
    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_start_capture", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), r64(first)]);

    let symbol = out.symbol("malloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_start_allocation", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 80)]);
    out.label(".Largs_start_convert");
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(1)]);
    out.emit(Opcode::Cmp64, [address(4, None, 64), r64(1)]);

    let symbol = out.symbol(".Largs_start_capture", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 72), r64(2)]);
    out.emit(
        Opcode::Mov64,
        [address(2, Some((1, Scale::Eight)), 8), r64(0)],
    );

    let symbol = out.symbol("cerune_arguments_os_value", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 88), r64(2)]);
    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(2)]);
    out.emit(Opcode::Add64, [address(4, None, 80), r64(2)]);
    out.emit(Opcode::Mov64, [r64(0), address(2, None, 0)]);
    out.emit(Opcode::Mov64, [r64(1), address(2, None, 8)]);
    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 88)]);

    let symbol = out.symbol(".Largs_start_convert", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_start_capture");
    out.emit(Opcode::Mov64, [address(4, None, 64), r64(0)]);
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(2)]);

    let symbol = out.symbol("cerune_arguments_capture", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    if !linux {
        free_values(out, linux);
    }
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(first)]);

    let symbol = out.symbol("free", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    let symbol = out.symbol("cerune_program", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 96)]);

    let symbol = out.symbol("cerune_arguments_dispose", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 96), r64(0)]);
    out.emit(Opcode::Add64, [Operand::Immediate(120), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_start_allocation");

    let symbol = out.symbol(".Largs_allocation_failed", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Mov64, [Operand::Immediate(-1), r64(1)]);

    let symbol = out.symbol("cerune_arguments_capture_failed", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
fn free_values(out: &mut NativeProgram, linux: bool) {
    let first = if linux { 7 } else { 1 };

    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 88)]);
    out.label(".Largs_free_os");
    out.emit(Opcode::Mov64, [address(4, None, 88), r64(0)]);
    out.emit(Opcode::Cmp64, [address(4, None, 64), r64(0)]);

    let symbol = out.symbol(".Largs_free_os_done", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::ShiftLeft64, [Operand::Immediate(4), r64(0)]);
    out.emit(Opcode::Add64, [address(4, None, 80), r64(0)]);
    out.emit(Opcode::Cmp64, [Operand::Immediate(0), address(0, None, 8)]);

    let symbol = out.symbol(".Largs_free_os_next", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(0, None, 0), r64(first)]);

    let symbol = out.symbol("free", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.label(".Largs_free_os_next");
    out.emit(Opcode::Add64, [Operand::Immediate(1), address(4, None, 88)]);

    let symbol = out.symbol(".Largs_free_os", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_free_os_done");
}
// Internal OS adapter: rax=text, rcx=index; returns rax=UTF8 bytes, rcx=length.
fn linux_value(out: &mut NativeProgram) {
    out.label("cerune_arguments_os_value");
    out.emit(Opcode::Sub64, [Operand::Immediate(72), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 40)]);
    out.emit(Opcode::Mov64, [r64(0), r64(7)]);

    let symbol = out.symbol("strlen", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [r64(0), r64(1)]);
    out.emit(Opcode::Mov64, [address(4, None, 32), r64(2)]);

    let symbol = out.symbol("cerune_utf8", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Largs_os_unicode", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 32), r64(0)]);
    out.emit(Opcode::Mov64, [address(4, None, 48), r64(1)]);
    out.emit(Opcode::Add64, [Operand::Immediate(72), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_os_unicode");

    let symbol = out.symbol(".Largs_invalid_unicode", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Mov64, [address(4, None, 40), r64(1)]);

    let symbol = out.symbol("cerune_arguments_capture_failed", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
fn windows_value(out: &mut NativeProgram) {
    out.label("cerune_arguments_os_value");
    out.emit(Opcode::Sub64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 64)]);
    out.emit(Opcode::Mov64, [r64(1), address(4, None, 72)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(65001), r32(1)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(128), r32(2)]);
    out.emit(Opcode::Mov64, [r64(0), r64(8)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(-1), r32(9)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 40)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 56)]);

    let symbol = out.symbol("WideCharToMultiByte", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test32, [r32(0), r32(0)]);

    let symbol = out.symbol(".Largs_os_unicode", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 80)]);
    out.emit(Opcode::Cmp32, [Operand::Immediate(1), r32(0)]);

    let symbol = out.symbol(".Largs_os_empty", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), r64(1)]);

    let symbol = out.symbol("malloc", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Test64, [r64(0), r64(0)]);

    let symbol = out.symbol(".Largs_os_allocation", Linkage::Local);
    out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [r64(0), address(4, None, 88)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(65001), r32(1)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(128), r32(2)]);
    out.emit(Opcode::Mov64, [address(4, None, 64), r64(8)]);
    out.emit(Opcode::Mov32, [Operand::Immediate(-1), r32(9)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 32)]);
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(0)]);
    out.emit(Opcode::Mov64, [r64(0), address(4, None, 40)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 48)]);
    out.emit(Opcode::Mov64, [Operand::Immediate(0), address(4, None, 56)]);

    let symbol = out.symbol("WideCharToMultiByte", Linkage::External);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Cmp64, [address(4, None, 80), r64(0)]);

    let symbol = out.symbol(".Largs_os_unicode", Linkage::Local);
    out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Mov64, [address(4, None, 88), r64(0)]);
    out.emit(Opcode::Mov64, [address(4, None, 80), r64(1)]);
    out.emit(Opcode::Sub64, [Operand::Immediate(1), r64(1)]);
    out.emit(Opcode::Add64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_os_empty");

    let symbol = out.symbol(".Lcerune_heap_empty", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.emit(Opcode::Xor32, [r32(1), r32(1)]);
    out.emit(Opcode::Add64, [Operand::Immediate(104), r64(4)]);
    out.emit(Opcode::Ret, []);
    out.label(".Largs_os_allocation");

    let symbol = out.symbol(".Largs_allocation_failed", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    let symbol = out.symbol(".Largs_os_failure", Linkage::Local);
    out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    out.label(".Largs_os_unicode");

    let symbol = out.symbol(".Largs_invalid_unicode", Linkage::Local);
    out.emit(Opcode::Lea64, [Operand::RipRelative(symbol), r64(0)]);

    out.label(".Largs_os_failure");
    out.emit(Opcode::Mov64, [address(4, None, 72), r64(1)]);

    let symbol = out.symbol("cerune_arguments_capture_failed", Linkage::Local);
    out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

    out.emit(Opcode::Undefined, []);
}
