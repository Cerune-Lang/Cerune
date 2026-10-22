/// Immutable byte equality. Calling convention and string layout belong to Cerune.
pub(super) fn append(program: &mut super::machine::Program) {
    use super::machine::*;
    use Opcode::*;
    use Operand::{Immediate as imm, Symbol as sym};
    let entry = program.declare("cerune_string_equal", Linkage::Local);
    let different = program.declare(".Lstring_different", Linkage::Local);
    let compare = program.declare(".Lstring_compare", Linkage::Local);
    let equal = program.declare(".Lstring_equal", Linkage::Local);
    let linux = program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    let (left, right, length, index) = if linux { (7, 6, 2, 8) } else { (1, 2, 8, 9) };
    push(program, EntryKind::Section(Section::Text));
    push(program, EntryKind::Align(4));
    push(program, EntryKind::Label(entry));
    instruction(program, Mov64, [memory(left, None, 0), r64(length)]);
    instruction(program, Cmp64, [memory(right, None, 0), r64(length)]);
    instruction(program, JumpNe, [sym(different)]);
    instruction(program, Xor64, [r64(index), r64(index)]);
    push(program, EntryKind::Label(compare));
    instruction(program, Cmp64, [r64(length), r64(index)]);
    instruction(program, JumpE, [sym(equal)]);
    instruction(
        program,
        ZeroExtend8To32,
        [memory(left, Some(index), 8), r32(0)],
    );
    instruction(program, Cmp8, [memory(right, Some(index), 8), r8(0)]);
    instruction(program, JumpNe, [sym(different)]);
    instruction(program, Inc64, [r64(index)]);
    instruction(program, Jump, [sym(compare)]);
    push(program, EntryKind::Label(equal));
    instruction(program, Mov32, [imm(1), r32(0)]);
    instruction(program, Ret, []);
    push(program, EntryKind::Label(different));
    instruction(program, Xor32, [r32(0), r32(0)]);
    instruction(program, Ret, []);
}
