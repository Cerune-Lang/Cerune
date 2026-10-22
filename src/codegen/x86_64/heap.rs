//! 同じ確保処理を、明示ターゲットの呼出規約で出力します。
#[cfg(test)]
pub(super) fn support(limit: u64, linux: bool) -> String {
    let target = if linux {
        super::Target::X86_64UnknownLinuxGnu
    } else {
        super::Target::X86_64PcWindowsMsvc
    };
    super::machine::render_support(target, |p| append(p, limit))
}

/// Construct allocation state and helpers with explicit target ABI.
pub(super) fn append(program: &mut super::machine::Program, limit: u64) {
    use super::machine::*;
    use Opcode::*;
    use Operand::{Immediate as imm, RipRelative as rip, Symbol as sym};
    let linux = program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    let (first, second, third) = if linux { (7, 6, 2) } else { (1, 2, 8) };
    let heap_head = program.declare(".Lcerune_heap_head", Linkage::Local);
    let heap_live = program.declare(".Lcerune_heap_live", Linkage::Local);
    let heap_limit = program.declare(".Lcerune_heap_limit", Linkage::Local);
    let heap_max = program.declare(".Lcerune_heap_max", Linkage::Local);
    let heap_empty = program.declare(".Lcerune_heap_empty", Linkage::Local);
    let string_retain = program.declare("cerune_string_retain", Linkage::Local);
    let heap_retain_search = program.declare(".Lcerune_heap_retain_search", Linkage::Local);
    let heap_retain_found = program.declare(".Lcerune_heap_retain_found", Linkage::Local);
    let heap_retain_done = program.declare(".Lcerune_heap_retain_done", Linkage::Local);
    let string_release = program.declare("cerune_string_release", Linkage::Local);
    let heap_release_search = program.declare(".Lcerune_heap_release_search", Linkage::Local);
    let heap_release_found = program.declare(".Lcerune_heap_release_found", Linkage::Local);
    let heap_release_done = program.declare(".Lcerune_heap_release_done", Linkage::Local);
    let string_concat = program.declare("cerune_string_concat", Linkage::Local);
    let heap_concat_empty = program.declare(".Lcerune_heap_concat_empty", Linkage::Local);
    let heap_size_fail = program.declare(".Lcerune_heap_size_fail", Linkage::Local);
    let heap_limit_fail = program.declare(".Lcerune_heap_limit_fail", Linkage::Local);
    let heap_allocation_fail = program.declare(".Lcerune_heap_allocation_fail", Linkage::Local);
    let heap_concat_done = program.declare(".Lcerune_heap_concat_done", Linkage::Local);
    let free = external(program, "free");
    let malloc = external(program, "malloc");
    let memcpy = external(program, "memcpy");
    push(program, EntryKind::Section(Section::Data));
    push(program, EntryKind::Align(3));
    push(program, EntryKind::Label(heap_head));
    push(program, EntryKind::Data64(0));
    push(program, EntryKind::Label(heap_live));
    push(program, EntryKind::Data64(0));
    push(program, EntryKind::Label(heap_limit));
    push(program, EntryKind::Data64(limit));
    push(program, EntryKind::Label(heap_max));
    push(program, EntryKind::Data64(9223372036854775783));
    push(program, EntryKind::Label(heap_empty));
    push(program, EntryKind::Data64(0));
    push(program, EntryKind::Section(Section::Text));
    push(program, EntryKind::Label(string_retain));
    instruction(program, Mov64, [r64(first), r64(10)]);
    instruction(program, Mov64, [rip(heap_head), r64(0)]);
    push(program, EntryKind::Label(heap_retain_search));
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpE, [sym(heap_retain_done)]);
    instruction(program, Lea64, [memory(0, None, 16), r64(11)]);
    instruction(program, Cmp64, [r64(10), r64(11)]);
    instruction(program, JumpE, [sym(heap_retain_found)]);
    instruction(program, Mov64, [memory(0, None, 0), r64(0)]);
    instruction(program, Jump, [sym(heap_retain_search)]);
    push(program, EntryKind::Label(heap_retain_found));
    instruction(program, Add64, [imm(1), memory(0, None, 8)]);
    push(program, EntryKind::Label(heap_retain_done));
    instruction(program, Ret, []);
    push(program, EntryKind::Label(string_release));
    instruction(program, Push, [r64(5)]);
    instruction(program, Mov64, [r64(4), r64(5)]);
    instruction(program, Sub64, [imm(32), r64(4)]);
    instruction(program, Mov64, [r64(first), r64(10)]);
    instruction(program, Lea64, [rip(heap_head), r64(11)]);
    push(program, EntryKind::Label(heap_release_search));
    instruction(program, Mov64, [memory(11, None, 0), r64(0)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpE, [sym(heap_release_done)]);
    instruction(program, Lea64, [memory(0, None, 16), r64(1)]);
    instruction(program, Cmp64, [r64(10), r64(1)]);
    instruction(program, JumpE, [sym(heap_release_found)]);
    instruction(program, Mov64, [r64(0), r64(11)]);
    instruction(program, Jump, [sym(heap_release_search)]);
    push(program, EntryKind::Label(heap_release_found));
    instruction(program, Sub64, [imm(1), memory(0, None, 8)]);
    instruction(program, JumpNe, [sym(heap_release_done)]);
    instruction(program, Mov64, [memory(0, None, 0), r64(1)]);
    instruction(program, Mov64, [r64(1), memory(11, None, 0)]);
    instruction(program, Mov64, [memory(0, None, 16), r64(1)]);
    instruction(program, Sub64, [r64(1), rip(heap_live)]);
    instruction(program, Mov64, [r64(0), r64(first)]);
    instruction(program, Call, [sym(free)]);
    push(program, EntryKind::Label(heap_release_done));
    instruction(program, Add64, [imm(32), r64(4)]);
    instruction(program, Pop, [r64(5)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(string_concat));
    instruction(program, Push, [r64(5)]);
    instruction(program, Mov64, [r64(4), r64(5)]);
    instruction(program, Sub64, [imm(80), r64(4)]);
    instruction(program, Mov64, [r64(first), memory(5, None, -8)]);
    instruction(program, Mov64, [r64(second), memory(5, None, -16)]);
    instruction(program, Mov64, [memory(first, None, 0), r64(0)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -32)]);
    instruction(program, Add64, [memory(second, None, 0), r64(0)]);
    instruction(program, JumpB, [sym(heap_size_fail)]);
    instruction(program, Cmp64, [rip(heap_max), r64(0)]);
    instruction(program, JumpA, [sym(heap_size_fail)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpE, [sym(heap_concat_empty)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -24)]);
    instruction(program, Mov64, [rip(heap_limit), r64(1)]);
    instruction(program, Sub64, [rip(heap_live), r64(1)]);
    instruction(program, Cmp64, [r64(1), r64(0)]);
    instruction(program, JumpA, [sym(heap_limit_fail)]);
    instruction(program, Lea64, [memory(0, None, 24), r64(first)]);
    instruction(program, Call, [sym(malloc)]);
    instruction(program, Test64, [r64(0), r64(0)]);
    instruction(program, JumpE, [sym(heap_allocation_fail)]);
    instruction(program, Mov64, [r64(0), memory(5, None, -40)]);
    instruction(program, Mov64, [rip(heap_head), r64(1)]);
    instruction(program, Mov64, [r64(1), memory(0, None, 0)]);
    instruction(program, Mov64, [imm(1), memory(0, None, 8)]);
    instruction(program, Mov64, [memory(5, None, -24), r64(1)]);
    instruction(program, Mov64, [r64(1), memory(0, None, 16)]);
    instruction(program, Lea64, [memory(0, None, 24), r64(first)]);
    instruction(program, Mov64, [memory(5, None, -8), r64(second)]);
    instruction(program, Add64, [imm(8), r64(second)]);
    instruction(program, Mov64, [memory(5, None, -32), r64(third)]);
    instruction(program, Call, [sym(memcpy)]);
    instruction(program, Mov64, [memory(5, None, -40), r64(first)]);
    instruction(program, Add64, [imm(24), r64(first)]);
    instruction(program, Add64, [memory(5, None, -32), r64(first)]);
    instruction(program, Mov64, [memory(5, None, -16), r64(second)]);
    instruction(program, Mov64, [memory(second, None, 0), r64(third)]);
    instruction(program, Add64, [imm(8), r64(second)]);
    instruction(program, Call, [sym(memcpy)]);
    instruction(program, Mov64, [memory(5, None, -40), r64(0)]);
    instruction(program, Mov64, [r64(0), rip(heap_head)]);
    instruction(program, Mov64, [memory(5, None, -24), r64(1)]);
    instruction(program, Add64, [r64(1), rip(heap_live)]);
    instruction(program, Add64, [imm(16), r64(0)]);
    instruction(program, Xor32, [r32(2), r32(2)]);
    instruction(program, Jump, [sym(heap_concat_done)]);
    push(program, EntryKind::Label(heap_concat_empty));
    instruction(program, Lea64, [rip(heap_empty), r64(0)]);
    instruction(program, Xor32, [r32(2), r32(2)]);
    instruction(program, Jump, [sym(heap_concat_done)]);
    push(program, EntryKind::Label(heap_size_fail));
    instruction(program, Mov32, [imm(1), r32(2)]);
    instruction(program, Jump, [sym(heap_concat_done)]);
    push(program, EntryKind::Label(heap_limit_fail));
    instruction(program, Mov32, [imm(2), r32(2)]);
    instruction(program, Jump, [sym(heap_concat_done)]);
    push(program, EntryKind::Label(heap_allocation_fail));
    instruction(program, Mov32, [imm(3), r32(2)]);
    push(program, EntryKind::Label(heap_concat_done));
    instruction(program, Add64, [imm(80), r64(4)]);
    instruction(program, Pop, [r64(5)]);
    instruction(program, Ret, []);
}

#[cfg(test)]
#[path = "../../../tests/support/typed_native_owners.rs"]
pub(super) mod tests;
