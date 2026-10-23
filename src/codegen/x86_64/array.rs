//! 配列の要素は既存Native ABIと同じく、基準アドレスから8byteずつ下へ置きます。
//! 共通IRが要素のコピー/逆順解放を展開し、ここでは保存領域と検査だけを生成します。
use super::machine::{Linkage, Opcode, Operand, r64};
use super::program::{NativeProgram, address, xmm};
use super::{
    Target,
    failure::Reporter,
    ir::{Instruction, Type},
};
use crate::runtime::FailureCode as Failure;

#[cfg(test)]
pub(super) fn support(limit: u64, linux: bool) -> String {
    let target = if linux {
        super::Target::X86_64UnknownLinuxGnu
    } else {
        super::Target::X86_64PcWindowsMsvc
    };
    super::machine::render_support(target, |p| append(p, limit))
}

// owner: data-high/length/refs/logical-bytes/initialized/stride (各8bytes)、続いて要素領域。
// 空配列はnull。確保は一度だけ行い、失敗時は予算を加算しません。
// RAX=owner, RDX=error (0/size/limit/allocation) は処理系内部だけの戻り値規約です。
/// Construct allocation state and helpers with explicit target ABI.
pub(super) fn append(program: &mut super::machine::Program, limit: u64) {
    use super::machine::*;
    use Opcode::*;
    use Operand::{Immediate as imm, RipRelative as rip, Symbol as sym};
    let linux = program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    let (first, second, third) = if linux { (7, 6, 2) } else { (1, 2, 8) };
    let array_live = program.declare(".Lcerune_array_live", Linkage::Local);
    let array_limit = program.declare(".Lcerune_array_limit", Linkage::Local);
    let array_max = program.declare(".Lcerune_array_max", Linkage::Local);
    let array_physical_max = program.declare(".Lcerune_array_physical_max", Linkage::Local);
    let array_allocate = program.declare("cerune_array_allocate", Linkage::Local);
    let array_payload_ready = program.declare(".Lcerune_array_payload_ready", Linkage::Local);
    let array_empty = program.declare(".Lcerune_array_empty", Linkage::Local);
    let array_size_fail = program.declare(".Lcerune_array_size_fail", Linkage::Local);
    let array_limit_fail = program.declare(".Lcerune_array_limit_fail", Linkage::Local);
    let array_allocation_fail = program.declare(".Lcerune_array_allocation_fail", Linkage::Local);
    let array_allocate_done = program.declare(".Lcerune_array_allocate_done", Linkage::Local);
    let array_length = program.declare("cerune_array_length", Linkage::Local);
    let array_length_done = program.declare(".Lcerune_array_length_done", Linkage::Local);
    let array_init_address = program.declare("cerune_array_init_address", Linkage::Local);
    let array_retain = program.declare("cerune_array_retain", Linkage::Local);
    let array_retain_done = program.declare(".Lcerune_array_retain_done", Linkage::Local);
    let array_release_owner = program.declare("cerune_array_release_owner", Linkage::Local);
    let array_release_empty = program.declare(".Lcerune_array_release_empty", Linkage::Local);
    let array_free = program.declare("cerune_array_free", Linkage::Local);
    let array_free_empty = program.declare(".Lcerune_array_free_empty", Linkage::Local);
    let array_invalid_owner = program.declare(".Lcerune_array_invalid_owner", Linkage::Local);
    let malloc = external(program, "malloc");
    let free = external(program, "free");
    push(program, EntryKind::Section(Section::Data));
    push(program, EntryKind::Align(3));
    push(program, EntryKind::Label(array_live));
    push(program, EntryKind::Data64(0));
    push(program, EntryKind::Label(array_limit));
    push(program, EntryKind::Data64(limit));
    push(program, EntryKind::Label(array_max));
    push(program, EntryKind::Data64(9223372036854775807));
    push(program, EntryKind::Label(array_physical_max));
    push(program, EntryKind::Data64(9223372036854775759));
    push(program, EntryKind::Section(Section::Text));
    push(program, EntryKind::Label(array_allocate));
    instruction(program, Push, [r64(5)]);
    instruction(program, Mov64, [r64(4), r64(5)]);
    instruction(program, Sub64, [imm(64), r64(4)]);
    instruction(program, Mov64, [r64(first), r64(0)]);
    instruction(program, Mov64, [r64(second), r64(10)]);
    instruction(program, Mov64, [r64(third), r64(11)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpS, [sym(array_size_fail)]);
    instruction(program, JumpE, [sym(array_empty)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -8)]);
    instruction(program, Mov64, [r64(11), memory(5, None, -32)]);
    instruction(program, Mul64, [r64(10)]);
    instruction(program, Test64, [r64(2), r64(2)]);
    instruction(program, JumpNe, [sym(array_size_fail)]);
    instruction(program, Cmp64, [rip(array_max), r64(0)]);
    instruction(program, JumpA, [sym(array_size_fail)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -16)]);
    instruction(program, Mov64, [memory(5, None, -8), r64(0)]);
    instruction(program, Mul64, [r64(11)]);
    instruction(program, Test64, [r64(2), r64(2)]);
    instruction(program, JumpNe, [sym(array_size_fail)]);
    instruction(program, Cmp64, [rip(array_physical_max), r64(0)]);
    instruction(program, JumpA, [sym(array_size_fail)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -24)]);
    instruction(program, Mov64, [rip(array_limit), r64(10)]);
    instruction(program, Sub64, [rip(array_live), r64(10)]);
    instruction(program, Cmp64, [memory(5, None, -16), r64(10)]);
    instruction(program, JumpB, [sym(array_limit_fail)]);
    instruction(program, Lea64, [memory(0, None, 48), r64(first)]);
    instruction(program, Call, [sym(malloc)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpE, [sym(array_allocation_fail)]);
    instruction(program, Mov64, [memory(5, None, -24), r64(11)]);
    instruction(program, Lea64, [memory(0, None, 48), r64(10)]);
    instruction(program, Test64, [r64(11), r64(11)]);
    instruction(program, JumpE, [sym(array_payload_ready)]);
    instruction(program, Sub64, [imm(8), r64(10)]);
    instruction(program, Add64, [r64(11), r64(10)]);
    push(program, EntryKind::Label(array_payload_ready));
    instruction(program, Mov64, [r64(10), memory(0, None, 0)]);
    instruction(program, Mov64, [memory(5, None, -8), r64(10)]);
    instruction(program, Mov64, [r64(10), memory(0, None, 8)]);
    instruction(program, Mov64, [imm(1), memory(0, None, 16)]);
    instruction(program, Mov64, [memory(5, None, -16), r64(10)]);
    instruction(program, Mov64, [r64(10), memory(0, None, 24)]);
    instruction(program, Add64, [r64(10), rip(array_live)]);
    instruction(program, Mov64, [imm(0), memory(0, None, 32)]);
    instruction(program, Mov64, [memory(5, None, -32), r64(10)]);
    instruction(program, Mov64, [r64(10), memory(0, None, 40)]);
    instruction(program, Xor32, [r32(2), r32(2)]);
    instruction(program, Jump, [sym(array_allocate_done)]);
    push(program, EntryKind::Label(array_empty));
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Xor32, [r32(2), r32(2)]);
    instruction(program, Jump, [sym(array_allocate_done)]);
    push(program, EntryKind::Label(array_size_fail));
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Mov32, [imm(1), r32(2)]);
    instruction(program, Jump, [sym(array_allocate_done)]);
    push(program, EntryKind::Label(array_limit_fail));
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Mov32, [imm(2), r32(2)]);
    instruction(program, Jump, [sym(array_allocate_done)]);
    push(program, EntryKind::Label(array_allocation_fail));
    instruction(program, Mov32, [imm(3), r32(2)]);
    push(program, EntryKind::Label(array_allocate_done));
    instruction(program, Add64, [imm(64), r64(4)]);
    instruction(program, Pop, [r64(5)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_length));
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Test64, [r64(first), r64(first)]);
    instruction(program, JumpE, [sym(array_length_done)]);
    instruction(program, Mov64, [memory(first, None, 8), r64(0)]);
    push(program, EntryKind::Label(array_length_done));
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_init_address));
    instruction(program, Mov64, [r64(first), r64(10)]);
    instruction(program, Test64, [r64(10), r64(10)]);
    instruction(program, JumpE, [sym(array_invalid_owner)]);
    instruction(program, Mov64, [memory(10, None, 32), r64(0)]);
    instruction(program, Cmp64, [memory(10, None, 8), r64(0)]);
    instruction(program, JumpAe, [sym(array_invalid_owner)]);
    instruction(program, SignedMul64, [memory(10, None, 40), r64(0)]);
    instruction(program, Mov64, [memory(10, None, 0), r64(11)]);
    instruction(program, Sub64, [r64(0), r64(11)]);
    instruction(program, Mov64, [r64(11), r64(0)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_retain));
    instruction(program, Mov64, [r64(first), r64(10)]);
    instruction(program, Test64, [r64(10), r64(10)]);
    instruction(program, JumpE, [sym(array_retain_done)]);
    instruction(program, Cmp64, [imm(0), memory(10, None, 16)]);
    instruction(program, JumpE, [sym(array_invalid_owner)]);
    instruction(program, Cmp64, [imm(-1), memory(10, None, 16)]);
    instruction(program, JumpE, [sym(array_invalid_owner)]);
    instruction(program, Add64, [imm(1), memory(10, None, 16)]);
    push(program, EntryKind::Label(array_retain_done));
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_release_owner));
    instruction(program, Mov64, [r64(first), r64(10)]);
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Test64, [r64(10), r64(10)]);
    instruction(program, JumpE, [sym(array_release_empty)]);
    instruction(program, Cmp64, [imm(0), memory(10, None, 16)]);
    instruction(program, JumpE, [sym(array_invalid_owner)]);
    instruction(program, Sub64, [imm(1), memory(10, None, 16)]);
    instruction(program, SetE, [r8(0)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_release_empty));
    instruction(program, Mov32, [imm(1), r32(0)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_free));
    instruction(program, Test64, [r64(first), r64(first)]);
    instruction(program, JumpE, [sym(array_free_empty)]);
    instruction(program, Cmp64, [imm(0), memory(first, None, 16)]);
    instruction(program, JumpNe, [sym(array_invalid_owner)]);
    instruction(program, Push, [r64(5)]);
    instruction(program, Mov64, [r64(4), r64(5)]);
    instruction(program, Sub64, [imm(32), r64(4)]);
    instruction(program, Mov64, [memory(first, None, 24), r64(10)]);
    instruction(program, Sub64, [r64(10), rip(array_live)]);
    instruction(program, Call, [sym(free)]);
    instruction(program, Add64, [imm(32), r64(4)]);
    instruction(program, Pop, [r64(5)]);
    push(program, EntryKind::Label(array_free_empty));
    instruction(program, Ret, []);
    push(program, EntryKind::Label(array_invalid_owner));
    if linux {
        instruction(program, Undefined, []);
    } else {
        instruction(program, Mov32, [imm(7), r32(1)]);
        instruction(program, Interrupt, [imm(0x29)]);
    }
}

pub(super) fn emit(
    i: &Instruction,
    prefix: &str,
    target: Target,
    reporter: &mut Reporter,
    out: &mut NativeProgram,
) -> bool {
    let (first, second, third) = if target.is_linux() {
        (7, 6, 2)
    } else {
        (1, 2, 8)
    };
    match i {
        Instruction::ArrayAllocate {
            width,
            stride,
            label,
        } => {
            let length_ok = format!(".L{prefix}_array_length_ok_{label}");

            out.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = out.symbol(&length_ok, Linkage::Local);
            out.emit(Opcode::JumpNs, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::ArrayLengthOutOfRange, out);

            out.label(&length_ok);

            out.emit(Opcode::Mov64, [r64(0), r64(first)]);
            out.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((width).to_owned() as i128), r64(second)],
            );
            out.emit(
                Opcode::MovAbs64,
                [Operand::Immediate((stride).to_owned() as i128), r64(third)],
            );

            let symbol = out.symbol("cerune_array_allocate", Linkage::Local);
            out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            let done = format!(".L{prefix}_array_allocated_{label}");
            let limit = format!(".L{prefix}_array_limit_{label}");
            let failed = format!(".L{prefix}_array_failed_{label}");

            out.emit(Opcode::Test64, [r64(2), r64(2)]);

            let symbol = out.symbol(&done, Linkage::Local);
            out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            out.emit(Opcode::Cmp64, [Operand::Immediate(1), r64(2)]);

            let symbol = out.symbol(&limit, Linkage::Local);
            out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::AllocationSizeOverflow, out);

            out.label(&limit);
            out.emit(Opcode::Cmp64, [Operand::Immediate(2), r64(2)]);

            let symbol = out.symbol(&failed, Linkage::Local);
            out.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

            reporter.emit(Failure::AllocationLimitExceeded, out);

            out.label(&failed);

            reporter.emit(Failure::AllocationFailed, out);

            out.label(&done);
        }
        Instruction::ArrayRangeCheck {
            length_offset,
            start_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_array_range_trap_{label}");
            let done = format!(".L{prefix}_array_range_done_{label}");

            out.emit(
                Opcode::Mov64,
                [address(5, None, (start_offset).to_owned() as i128), r64(10)],
            );
            out.emit(Opcode::Test64, [r64(10), r64(10)]);

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

            out.emit(Opcode::Cmp64, [r64(10), r64(0)]);

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpL, [Operand::Symbol(symbol)]);

            out.emit(
                Opcode::Cmp64,
                [address(5, None, (length_offset).to_owned() as i128), r64(0)],
            );

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpG, [Operand::Symbol(symbol)]);

            let symbol = out.symbol(&done, Linkage::Local);
            out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            out.label(&trap);

            reporter.emit(Failure::ArrayRangeOutOfBounds, out);

            out.label(&done);
        }
        Instruction::DynamicArrayAddress {
            base_offset,
            base_is_pointer,
            destination_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_dynamic_index_trap_{label}");
            let done = format!(".L{prefix}_dynamic_index_done_{label}");

            out.emit(
                Opcode::Mov64,
                [address(5, None, (base_offset).to_owned() as i128), r64(10)],
            );

            if *base_is_pointer {
                out.emit(Opcode::Mov64, [address(10, None, 0), r64(10)]);
            }

            out.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);

            out.emit(Opcode::Test64, [r64(10), r64(10)]);

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            out.emit(Opcode::Cmp64, [address(10, None, 8), r64(0)]);

            let symbol = out.symbol(&trap, Linkage::Local);
            out.emit(Opcode::JumpGe, [Operand::Symbol(symbol)]);

            out.emit(Opcode::Cmp64, [address(10, None, 32), r64(0)]);

            let symbol = out.symbol(".Lcerune_array_invalid_owner", Linkage::Local);
            out.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            out.emit(Opcode::SignedMul64, [address(10, None, 40), r64(0)]);
            out.emit(Opcode::Mov64, [address(10, None, 0), r64(11)]);
            out.emit(Opcode::Sub64, [r64(0), r64(11)]);
            out.emit(
                Opcode::Mov64,
                [
                    r64(11),
                    address(5, None, (destination_offset).to_owned() as i128),
                ],
            );

            let symbol = out.symbol(&done, Linkage::Local);
            out.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            out.label(&trap);

            reporter.emit(Failure::ArrayIndexOutOfBounds, out);

            out.label(&done);
        }
        Instruction::ArrayInitAddress {
            owner_offset,
            destination_offset,
        } => {
            out.emit(
                Opcode::Mov64,
                [
                    address(5, None, (owner_offset).to_owned() as i128),
                    r64(first),
                ],
            );

            let symbol = out.symbol("cerune_array_init_address", Linkage::Local);
            out.emit(Opcode::Call, [Operand::Symbol(symbol)]);

            out.emit(
                Opcode::Mov64,
                [
                    r64(0),
                    address(5, None, (destination_offset).to_owned() as i128),
                ],
            );
        }
        Instruction::ArrayInitialized { owner_offset } => {
            out.emit(
                Opcode::Mov64,
                [address(5, None, (owner_offset).to_owned() as i128), r64(10)],
            );
            out.emit(
                Opcode::Add64,
                [Operand::Immediate(1), address(10, None, 32)],
            );
        }
        Instruction::ArrayLength
        | Instruction::ArrayRetain
        | Instruction::ArrayReleaseOwner
        | Instruction::ArrayFree => {
            let name = match i {
                Instruction::ArrayLength => "length",
                Instruction::ArrayRetain => "retain",
                Instruction::ArrayReleaseOwner => "release_owner",
                Instruction::ArrayFree => "free",
                _ => unreachable!(),
            };

            out.emit(Opcode::Mov64, [r64(0), r64(first)]);

            let symbol = out.symbol(&format!("cerune_array_{name}"), Linkage::Local);
            out.emit(Opcode::Call, [Operand::Symbol(symbol)]);
        }
        Instruction::LoadFromPointer { ty, pointer_offset } => {
            out.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(11),
                ],
            );

            match ty {
                Type::F32 => {
                    out.emit(Opcode::Movss, [address(11, None, 0), xmm(0)]);
                }
                Type::F64 => {
                    out.emit(Opcode::Movsd, [address(11, None, 0), xmm(0)]);
                }
                _ => {
                    out.emit(Opcode::Mov64, [address(11, None, 0), r64(0)]);
                }
            };
        }
        Instruction::CopyFromPointer {
            pointer_offset,
            destination_offset,
            slots,
        } => {
            out.emit(
                Opcode::Mov64,
                [
                    address(5, None, (pointer_offset).to_owned() as i128),
                    r64(11),
                ],
            );

            for slot in 0..*slots {
                let source = -8 * slot as isize;
                let destination = destination_offset + source;

                out.emit(
                    Opcode::Mov64,
                    [address(11, None, (source).to_owned() as i128), r64(10)],
                );
                out.emit(
                    Opcode::Mov64,
                    [r64(10), address(5, None, (destination).to_owned() as i128)],
                );
            }
        }
        _ => return false,
    }
    true
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
#[path = "array_tests.rs"]
mod tests;
