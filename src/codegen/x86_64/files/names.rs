//! 共通のファイル名規則。データを変更せず、OS接続より前に検査します。
use super::super::machine::Scale;
use super::*;
pub(super) fn append(o: &mut NativeProgram) {
    o.label("cerune_file_valid_name");
    mov(o, address(10, None, 0), r64(1));
    compare(o, r64(1), 1);
    branch(o, Opcode::JumpB, "name_bad");
    compare(o, r64(1), 255);
    branch(o, Opcode::JumpA, "name_bad");
    o.emit(Opcode::Lea64, [address(10, None, 8), r64(2)]);
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(2, Some((1, Scale::One)), -1), r32(0)],
    );
    for b in *b" ." {
        o.emit(Opcode::Cmp32, [imm(b.into()), r32(0)]);
        branch(o, Opcode::JumpE, "name_bad");
    }
    mov(o, r64(1), r64(9));
    mov(o, imm(0), r64(8));
    label(o, "name_scan");
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(2, Some((8, Scale::One)), 0), r32(0)],
    );
    o.emit(Opcode::Cmp32, [imm(32), r32(0)]);
    branch(o, Opcode::JumpB, "name_bad");
    for b in b"<>:\"/\\|?*\x7f" {
        o.emit(Opcode::Cmp32, [imm((*b).into()), r32(0)]);
        branch(o, Opcode::JumpE, "name_bad");
    }
    o.emit(Opcode::Cmp32, [imm(46), r32(0)]);
    branch(o, Opcode::JumpNe, "name_next");
    o.emit(Opcode::Cmp64, [r64(1), r64(9)]);
    branch(o, Opcode::JumpNe, "name_next");
    mov(o, r64(8), r64(9));
    label(o, "name_next");
    o.emit(Opcode::Add64, [imm(1), r64(8)]);
    o.emit(Opcode::Cmp64, [r64(1), r64(8)]);
    branch(o, Opcode::JumpB, "name_scan");
    label(o, "name_trim");
    compare(o, r64(9), 0);
    branch(o, Opcode::JumpE, "name_good");
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(2, Some((9, Scale::One)), -1), r32(0)],
    );
    o.emit(Opcode::Cmp32, [imm(32), r32(0)]);
    branch(o, Opcode::JumpNe, "name_devices");
    o.emit(Opcode::Sub64, [imm(1), r64(9)]);
    branch(o, Opcode::Jump, "name_trim");
    label(o, "name_devices");
    let mut reserved: Vec<String> = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .map(str::to_owned)
        .into();
    for p in ["COM", "LPT"] {
        for n in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"] {
            reserved.push(format!("{p}{n}"));
        }
    }
    for (i, word) in reserved.iter().enumerate() {
        let next = format!("name_device_{i}");
        compare(o, r64(9), word.len() as i128);
        branch(o, Opcode::JumpNe, &next);
        for (j, b) in word.bytes().enumerate() {
            o.emit(
                Opcode::ZeroExtend8To32,
                [address(2, None, j as i128), r32(0)],
            );
            if b.is_ascii_alphabetic() {
                o.emit(Opcode::Or32, [imm(32), r32(0)]);
            }
            o.emit(Opcode::Cmp32, [imm(b.to_ascii_lowercase().into()), r32(0)]);
            branch(o, Opcode::JumpNe, &next);
        }
        branch(o, Opcode::Jump, "name_bad");
        label(o, &next);
    }
    label(o, "name_good");
    mov(o, imm(1), r64(0));
    o.emit(Opcode::Ret, []);
    label(o, "name_bad");
    mov(o, imm(0), r64(0));
    o.emit(Opcode::Ret, []);
}
