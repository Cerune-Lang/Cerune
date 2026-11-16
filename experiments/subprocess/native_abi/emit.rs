//! Independent subprocess ABI probe; no Cerune source lowering is claimed here.
use sashimi_assembler::{
    Target,
    program::{self, EntryKind, Linkage, OriginId, Program, Provenance, Section},
    typed::{Instruction, Memory, Opcode, Operand, Register, Width},
};
use std::{error::Error, fs, path::Path};
fn reg(n: u8) -> Operand {
    Register::gpr(n, Width::Qword).unwrap().into()
}
fn mem(base: u8, offset: i32) -> Operand {
    Memory::new(Register::gpr(base, Width::Qword).unwrap(), None, offset)
        .unwrap()
        .into()
}
fn emit(p: &mut Program, origin: u64, op: Opcode, operands: impl Into<Vec<Operand>>) {
    p.push(
        EntryKind::Instruction(Instruction::new(op, operands)),
        Provenance {
            origins: vec![OriginId(origin)],
            synthetic: false,
        },
    );
}
const CALLS: [(&str, usize); 7] = [
    ("create", 5),
    ("prepare", 2),
    ("execute", 2),
    ("report", 3),
    ("copy", 6),
    ("release", 1),
    ("destroy", 1),
];
fn build(target: Target) -> Program {
    let win = target == Target::X86_64PcWindowsMsvc;
    let regs: &[u8] = if win {
        &[1, 2, 8, 9]
    } else {
        &[7, 6, 2, 1, 8, 9]
    };
    let mut p = Program::new(target);
    p.push(EntryKind::Section(Section::Text), Provenance::synthetic());
    for (id, (name, argc)) in CALLS.iter().enumerate() {
        let id = id as u64;
        let wrapper = p.declare(format!("cerune_native_{name}"), Linkage::Global);
        let callee = p.declare(format!("cerune_process_v1_{name}"), Linkage::External);
        p.push(EntryKind::Label(wrapper), Provenance::synthetic());
        // Push RBP aligns RSP to 16 bytes; the 256-byte frame keeps alignment.
        // Outgoing stack args/shadow space: 0..48; saved args: 64..112;
        // local Bytes or Request: 112..232. No callee-saved scratch registers.
        emit(&mut p, id, Opcode::Push, [reg(5)]);
        emit(&mut p, id, Opcode::Mov64, [reg(4), reg(5)]);
        emit(&mut p, id, Opcode::Sub64, [Operand::Immediate(256), reg(4)]);
        for n in 0..*argc {
            let from = if n < regs.len() {
                reg(regs[n])
            } else {
                mem(5, 48 + 8 * (n as i32 - 4))
            };
            emit(&mut p, id, Opcode::Mov64, [from, reg(0)]);
            emit(
                &mut p,
                id,
                Opcode::Mov64,
                [reg(0), mem(4, 64 + n as i32 * 8)],
            );
        }
        if *name == "create" {
            if win {
                // Windows passes the 16-byte Bytes value by pointer to a copy.
                for (from, to) in [(80, 112), (88, 120)] {
                    emit(&mut p, id, Opcode::Mov64, [mem(4, from), reg(0)]);
                    emit(&mut p, id, Opcode::Mov64, [reg(0), mem(4, to)]);
                }
                emit(&mut p, id, Opcode::Mov64, [mem(4, 64), reg(1)]);
                emit(&mut p, id, Opcode::Mov64, [mem(4, 72), reg(2)]);
                emit(&mut p, id, Opcode::Lea64, [mem(4, 112), reg(8)]);
                emit(&mut p, id, Opcode::Mov64, [mem(4, 96), reg(9)]);
            } else {
                // SysV puts the two integer classes of Bytes in RDX and RCX.
                for (n, r) in regs.iter().take(5).enumerate() {
                    emit(
                        &mut p,
                        id,
                        Opcode::Mov64,
                        [mem(4, 64 + n as i32 * 8), reg(*r)],
                    );
                }
            }
        } else if *name == "prepare" {
            emit(&mut p, id, Opcode::Mov64, [mem(4, 72), reg(10)]);
            for offset in (0..120).step_by(8) {
                emit(&mut p, id, Opcode::Mov64, [mem(10, offset), reg(0)]);
                emit(&mut p, id, Opcode::Mov64, [reg(0), mem(4, 112 + offset)]);
            }
            emit(&mut p, id, Opcode::Mov64, [mem(4, 64), reg(regs[0])]);
            emit(&mut p, id, Opcode::Lea64, [mem(4, 112), reg(regs[1])]);
        } else {
            for n in 0..*argc {
                let to = if n < regs.len() {
                    reg(regs[n])
                } else {
                    mem(4, 32 + 8 * (n as i32 - 4))
                };
                emit(
                    &mut p,
                    id,
                    Opcode::Mov64,
                    [mem(4, 64 + n as i32 * 8), reg(0)],
                );
                emit(&mut p, id, Opcode::Mov64, [reg(0), to]);
            }
        }
        emit(&mut p, id, Opcode::Call, [Operand::Symbol(callee)]);
        emit(&mut p, id, Opcode::Add64, [Operand::Immediate(256), reg(4)]);
        emit(&mut p, id, Opcode::Pop, [reg(5)]);
        emit(&mut p, id, Opcode::Ret, []);
    }
    p
}
fn save(target: Target, dir: &Path) -> Result<(), Box<dyn Error>> {
    let p = build(target);
    let artifact = program::assemble_program(&p)?;
    let asm = program::render_asm(&p)?;
    assert_eq!(
        artifact,
        program::assemble_program(&p)?,
        "observation changed object"
    );
    let mut observed = vec![];
    for (id, (name, _)) in CALLS.iter().enumerate() {
        let references: Vec<_> = artifact
            .references
            .iter()
            .filter(|r| {
                artifact.entries[r.entry.0]
                    .provenance
                    .origins
                    .contains(&OriginId(id as u64))
            })
            .collect();
        assert_eq!(references.len(), 1, "one external call per wrapper");
        observed.push(serde_json::json!({"probe_origin":id,"operation":name,
            "external_call":format!("{:?}", references[0])}));
    }
    fs::create_dir(dir)?;
    fs::write(dir.join("probe.s"), asm)?;
    fs::write(dir.join("probe.o"), artifact.object)?;
    fs::write(dir.join("typed.txt"), format!("{p:#?}"))?;
    fs::write(
        dir.join("origins.json"),
        serde_json::to_vec_pretty(&observed)?,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: subprocess_native_abi <existing-output-directory>".into());
    }
    let dir = Path::new(&args[0]);
    save(Target::X86_64PcWindowsMsvc, &dir.join("windows"))?;
    save(Target::X86_64UnknownLinuxGnu, &dir.join("linux"))?;
    Ok(())
}
