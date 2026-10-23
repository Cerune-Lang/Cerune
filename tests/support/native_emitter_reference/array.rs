//! 配列の要素は既存Native ABIと同じく、基準アドレスから8byteずつ下へ置きます。
//! 共通IRが要素のコピー/逆順解放を展開し、ここでは保存領域と検査だけを生成します。
use super::{
    Target,
    failure::Reporter,
    ir::{Instruction, Type},
};
use crate::runtime::FailureCode as Failure;
use std::fmt::Write;

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
    out: &mut String,
) -> bool {
    let (first, second, third) = if target.is_linux() {
        ("%rdi", "%rsi", "%rdx")
    } else {
        ("%rcx", "%rdx", "%r8")
    };
    match i {
        Instruction::ArrayAllocate {
            width,
            stride,
            label,
        } => {
            let length_ok = format!(".L{prefix}_array_length_ok_{label}");
            writeln!(out, "  testq %rax, %rax\n  jns {length_ok}").unwrap();
            reporter.emit(Failure::ArrayLengthOutOfRange, out);
            writeln!(out, "{length_ok}:").unwrap();
            writeln!(out, "  movq %rax, {first}\n  movabsq \u{24}{width}, {second}\n  movabsq \u{24}{stride}, {third}\n  callq cerune_array_allocate").unwrap();
            let done = format!(".L{prefix}_array_allocated_{label}");
            let limit = format!(".L{prefix}_array_limit_{label}");
            let failed = format!(".L{prefix}_array_failed_{label}");
            writeln!(
                out,
                "  testq %rdx, %rdx\n  je {done}\n  cmpq $1, %rdx\n  jne {limit}"
            )
            .unwrap();
            reporter.emit(Failure::AllocationSizeOverflow, out);
            writeln!(out, "{limit}:\n  cmpq $2, %rdx\n  jne {failed}").unwrap();
            reporter.emit(Failure::AllocationLimitExceeded, out);
            writeln!(out, "{failed}:").unwrap();
            reporter.emit(Failure::AllocationFailed, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::ArrayRangeCheck {
            length_offset,
            start_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_array_range_trap_{label}");
            let done = format!(".L{prefix}_array_range_done_{label}");
            writeln!(out, "  movq {start_offset}(%rbp), %r10\n  testq %r10, %r10\n  js {trap}\n  cmpq %r10, %rax\n  jl {trap}\n  cmpq {length_offset}(%rbp), %rax\n  jg {trap}\n  jmp {done}\n{trap}:").unwrap();
            reporter.emit(Failure::ArrayRangeOutOfBounds, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::DynamicArrayAddress {
            base_offset,
            base_is_pointer,
            destination_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_dynamic_index_trap_{label}");
            let done = format!(".L{prefix}_dynamic_index_done_{label}");
            writeln!(out, "  movq {base_offset}(%rbp), %r10").unwrap();
            if *base_is_pointer {
                out.push_str("  movq (%r10), %r10\n");
            }
            writeln!(out, "  testq %rax, %rax\n  js {trap}\n  testq %r10, %r10\n  je {trap}\n  cmpq 8(%r10), %rax\n  jge {trap}\n  cmpq 32(%r10), %rax\n  jae .Lcerune_array_invalid_owner\n  imulq 40(%r10), %rax\n  movq (%r10), %r11\n  subq %rax, %r11\n  movq %r11, {destination_offset}(%rbp)\n  jmp {done}\n{trap}:").unwrap();
            reporter.emit(Failure::ArrayIndexOutOfBounds, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::ArrayInitAddress {
            owner_offset,
            destination_offset,
        } => {
            writeln!(out, "  movq {owner_offset}(%rbp), {first}\n  callq cerune_array_init_address\n  movq %rax, {destination_offset}(%rbp)").unwrap();
        }
        Instruction::ArrayInitialized { owner_offset } => {
            writeln!(
                out,
                "  movq {owner_offset}(%rbp), %r10\n  addq $1, 32(%r10)"
            )
            .unwrap();
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
            writeln!(out, "  movq %rax, {first}\n  callq cerune_array_{name}").unwrap();
        }
        Instruction::LoadFromPointer { ty, pointer_offset } => {
            writeln!(out, "  movq {pointer_offset}(%rbp), %r11").unwrap();
            out.push_str(match ty {
                Type::F32 => "  movss (%r11), %xmm0\n",
                Type::F64 => "  movsd (%r11), %xmm0\n",
                _ => "  movq (%r11), %rax\n",
            });
        }
        Instruction::CopyFromPointer {
            pointer_offset,
            destination_offset,
            slots,
        } => {
            writeln!(out, "  movq {pointer_offset}(%rbp), %r11").unwrap();
            for slot in 0..*slots {
                let source = -8 * slot as isize;
                let destination = destination_offset + source;
                writeln!(
                    out,
                    "  movq {source}(%r11), %r10\n  movq %r10, {destination}(%rbp)"
                )
                .unwrap();
            }
        }
        _ => return false,
    }
    true
}
