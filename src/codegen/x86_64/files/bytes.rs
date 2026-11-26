//! OSには連続するバイトを渡し、Ceruneの配列には末尾アドレスから8バイト刻みで格納します。
use super::super::machine::r8;
use super::*;

pub(super) fn if_binary(o: &mut NativeProgram, target: &str) {
    compare(o, slot(BINARY), 0);
    branch(o, Opcode::JumpNe, target);
}

pub(super) fn append(o: &mut NativeProgram) {
    label(o, "bytes_reserve");
    call(o, "cerune_array_allocate", &[slot(LIMIT), imm(8), imm(8)]);
    compare(o, r64(2), 2);
    branch(o, Opcode::JumpE, "budget");
    compare(o, r64(2), 0);
    branch(o, Opcode::JumpNe, "allocation");
    mov(o, r64(0), slot(ARRAY));
    mov(o, slot(LIMIT), r64(0));
    branch(o, Opcode::Jump, "bytes_buffer");

    label(o, "bytes_pack");
    pointer(o, "reserve", PHASE);
    mov(o, slot(LENGTH), r64(0));
    label(o, "bytes_buffer");
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "open"); // 空の書込み、または読取り上限0（EOF確認のみ）
    mov(o, r64(0), slot(REQUEST));
    call(o, "malloc", &[slot(REQUEST)]);
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "allocation");
    mov(o, r64(0), slot(OWNER));
    mov(o, r64(0), slot(DATA));
    compare(o, slot(MODE), 0);
    branch(o, Opcode::JumpE, "open");

    mov(o, slot(TEXT), r64(10));
    mov(o, address(10, None, 0), r64(10)); // 配列の先頭要素
    mov(o, slot(LENGTH), r64(1));
    mov(o, slot(DATA), r64(11));
    label(o, "bytes_pack_loop");
    mov(o, address(10, None, 0), r64(0));
    o.emit(Opcode::Mov8, [r8(0), address(11, None, 0)]);
    o.emit(Opcode::Sub64, [imm(8), r64(10)]);
    o.emit(Opcode::Add64, [imm(1), r64(11)]);
    o.emit(Opcode::Sub64, [imm(1), r64(1)]);
    branch(o, Opcode::JumpNe, "bytes_pack_loop");
    branch(o, Opcode::Jump, "open");

    label(o, "bytes_finish");
    mov(o, slot(ARRAY), r64(10));
    compare(o, r64(10), 0);
    branch(o, Opcode::JumpE, "bytes_return");
    mov(o, slot(TRANSFER), r64(1));
    mov(o, r64(1), address(10, None, 8)); // 公開する要素数
    mov(o, r64(1), address(10, None, 32)); // 初期化済み要素数。予約時の使用量は解放まで保持。
    compare(o, r64(1), 0);
    branch(o, Opcode::JumpE, "bytes_return");
    mov(o, address(10, None, 0), r64(10));
    mov(o, slot(DATA), r64(11));
    label(o, "bytes_unpack_loop");
    o.emit(Opcode::ZeroExtend8To32, [address(11, None, 0), r32(0)]);
    mov(o, r64(0), address(10, None, 0));
    o.emit(Opcode::Sub64, [imm(8), r64(10)]);
    o.emit(Opcode::Add64, [imm(1), r64(11)]);
    o.emit(Opcode::Sub64, [imm(1), r64(1)]);
    branch(o, Opcode::JumpNe, "bytes_unpack_loop");

    label(o, "bytes_return");
    call(o, "free", &[slot(OWNER)]);
    mov(o, slot(ARRAY), r64(0)); // 新規保存の戻り値は0
    branch(o, Opcode::Jump, "return");

    label(o, "bytes_cleanup");
    call(o, "free", &[slot(OWNER)]);
    call(o, "cerune_array_release_owner", &[slot(ARRAY)]);
    call(o, "cerune_array_free", &[slot(ARRAY)]);
    branch(o, Opcode::Jump, "format");
}
