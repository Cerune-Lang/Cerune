//! 子プロセスの要求・配列・結果を、Nativeの型付き命令から共通C ABIへ渡します。
use super::ir::{Instruction, Module};
use super::machine::{EntryKind, Linkage, Opcode, Operand, Scale, Section, r8, r32, r64};
use super::program::{NativeProgram, address};
mod entry;
mod perform;
mod report;
mod storage;
pub(super) mod validate;
fn linux(o: &NativeProgram) -> bool {
    o.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu
}
fn slot(n: i128) -> Operand {
    address(4, None, n)
}
fn imm(n: i128) -> Operand {
    Operand::Immediate(n)
}
fn mov(o: &mut NativeProgram, a: Operand, b: Operand) {
    o.emit(Opcode::Mov64, [a, b]);
}
fn copy(o: &mut NativeProgram, a: Operand, b: Operand) {
    mov(o, a, r64(0));
    mov(o, r64(0), b);
}
fn ptr(o: &mut NativeProgram, n: i128, d: i128) {
    o.emit(Opcode::Lea64, [slot(n), r64(0)]);
    mov(o, r64(0), slot(d));
}
fn local(o: &mut NativeProgram, n: &str) -> Operand {
    Operand::RipRelative(o.symbol(n, Linkage::Local))
}
fn label(o: &mut NativeProgram, n: &str) {
    o.label(&format!(".Lprocess_{n}"));
}
fn branch(o: &mut NativeProgram, op: Opcode, n: &str) {
    let s = o.symbol(&format!(".Lprocess_{n}"), Linkage::Local);
    o.emit(op, [Operand::Symbol(s)]);
}
fn compare(o: &mut NativeProgram, a: Operand, n: i128) {
    o.emit(Opcode::Cmp64, [imm(n), a]);
}
fn test(o: &mut NativeProgram) {
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
}
fn literal(o: &mut NativeProgram, n: &str, d: i128) {
    let p = local(o, &format!(".Lprocess_text_{}", n.replace('-', "_")));
    o.emit(Opcode::Lea64, [p, r64(0)]);
    mov(o, r64(0), slot(d));
}
fn call(o: &mut NativeProgram, n: &str, args: &[Operand]) {
    let regs: &[u8] = if linux(o) {
        &[7, 6, 2, 1, 8, 9]
    } else {
        &[1, 2, 8, 9]
    };
    for (i, a) in args.iter().enumerate() {
        if i < regs.len() {
            mov(o, a.clone(), r64(regs[i]));
        } else {
            copy(o, a.clone(), slot(32 + 8 * (i - regs.len()) as i128));
        }
    }
    if linux(o) {
        o.emit(Opcode::Xor32, [r32(0), r32(0)]);
    }
    let s = o.symbol(
        n,
        if n.starts_with("cerune_process_v1_") || !n.starts_with("cerune_") {
            Linkage::External
        } else {
            Linkage::Local
        },
    );
    o.emit(Opcode::Call, [Operand::Symbol(s)]);
}
fn start(o: &mut NativeProgram, n: &str, frame: i128, args: usize) {
    assert!(frame < 4096 && frame % 16 == 8);
    o.label(n);
    o.emit(Opcode::Sub64, [imm(frame), r64(4)]);
    let regs: &[u8] = if linux(o) {
        &[7, 6, 2, 1, 8, 9]
    } else {
        &[1, 2, 8, 9]
    };
    for (i, r) in regs.iter().take(args).enumerate() {
        mov(o, r64(*r), slot(128 + 8 * i as i128));
    }
}
fn finish(o: &mut NativeProgram, n: i128) {
    o.emit(Opcode::Add64, [imm(n), r64(4)]);
    o.emit(Opcode::Ret, []);
}
fn check(o: &mut NativeProgram, expected: i128, origin: Operand) {
    mov(o, r64(0), slot(96));
    copy(o, origin, slot(104));
    call(
        o,
        "cerune_process_check",
        &[slot(96), imm(expected), slot(104)],
    );
}
pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| f.instructions.iter()))
        .any(|i| matches!(i, Instruction::ProcessRun { .. }))
}
fn pairs(m: &Module) -> std::collections::BTreeSet<(usize, usize)> {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| f.instructions.iter()))
        .filter_map(|i| {
            if let Instruction::ProcessRun {
                request_type,
                outcome_type,
                ..
            } = i
            {
                Some((request_type.0, outcome_type.0))
            } else {
                None
            }
        })
        .collect()
}
fn path(m: &Module, id: usize, names: &[&str]) -> (i128, Option<usize>) {
    let layouts = super::layout::resolve(&m.types).expect("validated LIR layouts");
    let mut id = Some(id);
    let mut displacement = 0;
    for name in names {
        let t = &m.types[id.unwrap()];
        let index = t
            .fields
            .iter()
            .position(|(_, n, _)| n == name)
            .expect("process field");
        displacement -= 8 * layouts[t.id.0].fields[index] as i128;
        id = if let crate::ir::Type::Named(i) = t.fields[index].2 {
            Some(i.0)
        } else {
            None
        };
    }
    (displacement, id)
}
pub(super) fn append(o: &mut NativeProgram, m: &Module, files: bool) {
    o.push(EntryKind::Section(Section::Data));
    o.push(EntryKind::Align(3));
    for n in [
        "cerune_process_context_value",
        "cerune_process_selected_config",
    ] {
        o.label(n);
        o.push(EntryKind::Data64(0));
    }
    o.push(EntryKind::Section(Section::Text));
    entry::append(o, files);
    storage::append(o);
    for (request, outcome) in pairs(m) {
        perform::append(o, m, request, outcome);
    }
}
pub(super) fn emit(
    o: &mut NativeProgram,
    m: &Module,
    i: &Instruction,
    reporter: &mut super::failure::Reporter,
) {
    let Instruction::ProcessRun {
        request_offset,
        outcome_offset,
        request_type,
        outcome_type,
    } = i
    else {
        unreachable!()
    };
    let super::ir::Origin::Source { node_id, span } = reporter.origin else {
        unreachable!()
    };
    // The call site keeps the exact source origin; helpers do not replace it with a current-location global.
    let regs = if m.target.is_linux() {
        [7, 6, 2]
    } else {
        [1, 2, 8]
    };
    for (off, r) in [(*outcome_offset, regs[0]), (*request_offset, regs[1])] {
        o.emit(Opcode::Lea64, [address(5, None, off as i128), r64(r)]);
    }
    let sequence = o.program.entries.len();
    let name = format!(".Lprocess_origin_{sequence}");
    let s = o.symbol(&name, Linkage::Local);
    o.emit(Opcode::Lea64, [Operand::RipRelative(s), r64(regs[2])]);
    let s = o.symbol(
        &format!("cerune_process_run_{}_{}", request_type.0, outcome_type.0),
        Linkage::Local,
    );
    o.emit(Opcode::Call, [Operand::Symbol(s)]);
    let done = format!("origin_done_{sequence}");
    branch(o, Opcode::Jump, &done);
    // Inline read-only metadata is jumped over, and is shared by request and diagnostics.
    o.label(&name);
    for n in [
        node_id.0 as u64,
        span.source_id().index() as u64,
        span.start() as u64,
        span.end() as u64,
    ] {
        o.push(EntryKind::Data64(n));
    }
    o.push(EntryKind::CString(format!(
        " node={}{} bytes={}..{}\n",
        node_id.0,
        span.source_id().record_field(),
        span.start(),
        span.end()
    )));
    label(o, &done);
}
