//! Shared typed construction for migrated Native runtime helpers.
//! Instruction selection and ABI remain in Cerune.
pub(super) use sashimi_assembler::program::{EntryKind, Linkage, Program, Provenance, Section};
pub(super) use sashimi_assembler::typed::{
    Instruction, Memory, Opcode, Operand, Register, Scale, Width,
};

pub(super) fn target(target: super::Target) -> sashimi_assembler::Target {
    match target {
        super::Target::X86_64PcWindowsMsvc => sashimi_assembler::Target::X86_64PcWindowsMsvc,
        super::Target::X86_64UnknownLinuxGnu => sashimi_assembler::Target::X86_64UnknownLinuxGnu,
    }
}
/// Runtime helpers share explicit external declarations when appended together.
pub(super) fn external(program: &mut Program, name: &str) -> sashimi_assembler::program::SymbolId {
    if let Some((index, symbol)) = program
        .symbols
        .iter()
        .enumerate()
        .find(|(_, s)| s.name == name)
    {
        assert_eq!(
            symbol.linkage,
            Linkage::External,
            "runtime external conflicts with local symbol"
        );
        sashimi_assembler::program::SymbolId(index)
    } else {
        program.declare(name, Linkage::External)
    }
}

pub(super) fn push(program: &mut Program, kind: EntryKind) {
    program.push(kind, Provenance::synthetic());
}
pub(super) fn instruction(
    program: &mut Program,
    opcode: Opcode,
    operands: impl Into<Vec<Operand>>,
) {
    push(
        program,
        EntryKind::Instruction(Instruction::new(opcode, operands)),
    );
}
pub(super) fn r64(n: u8) -> Operand {
    Register::gpr(n, Width::Qword)
        .expect("fixed x86-64 register")
        .into()
}
pub(super) fn r32(n: u8) -> Operand {
    Register::gpr(n, Width::Dword)
        .expect("fixed x86-64 register")
        .into()
}
pub(super) fn r8(n: u8) -> Operand {
    Register::gpr(n, Width::Byte)
        .expect("fixed x86-64 register")
        .into()
}
pub(super) fn memory(base: u8, index: Option<u8>, displacement: i32) -> Operand {
    Memory::new(
        Register::gpr(base, Width::Qword).expect("fixed base register"),
        index.map(|n| {
            (
                Register::gpr(n, Width::Qword).expect("fixed index register"),
                Scale::One,
            )
        }),
        displacement,
    )
    .expect("fixed runtime address")
    .into()
}

/// Temporary rendering adapter while other runtime helpers still emit assembly.
/// It only formats validated Program output; it never parses instructions or operands.
pub(super) fn render_support(target: super::Target, emit: impl FnOnce(&mut Program)) -> String {
    let mut program = Program::new(self::target(target));
    emit(&mut program);
    let asm = sashimi_assembler::program::render_asm(&program)
        .expect("compiler-generated runtime Program must be valid");
    let mut output = String::new();
    for line in asm.lines() {
        // The legacy text Object reader expects plain names and already emits PLT32
        // for external calls. Retire this formatting bridge with whole-Program output.
        let line = line.strip_suffix("@PLT").unwrap_or(line);
        // The enclosing emitter owns the final stack-note section and annotations.
        if line.starts_with("# entry ")
            || line.starts_with(".section .note.GNU-stack,")
            || line.starts_with(".extern ")
        {
            continue;
        }
        if !line.starts_with('.') && !line.ends_with(':') {
            output.push_str("  ");
        }
        output.push_str(line);
        output.push('\n');
    }
    output
}
#[cfg(test)]
#[path = "../../../tests/support/typed_native_runtime.rs"]
pub(super) mod tests;
