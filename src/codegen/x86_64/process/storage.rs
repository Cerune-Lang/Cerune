use super::*;
pub(super) fn append(o: &mut NativeProgram) {
    // Result arrays use the ordinary Native owner (one allocation, descending u8 slots).
    start(o, "cerune_process_result_allocate", 152, 1);
    call(o, "cerune_array_allocate", &[slot(128), imm(8), imm(8)]);
    finish(o, 152);
    start(o, "cerune_process_cancel", 152, 1);
    mov(o, slot(128), r64(0));
    mov(o, address(0, None, 0), r64(0));
    test(o);
    branch(o, Opcode::JumpE, "cancel_done");
    mov(o, imm(0), address(0, None, 16));
    mov(o, r64(0), slot(136));
    call(o, "cerune_array_free", &[slot(136)]);
    mov(o, slot(128), r64(0));
    mov(o, imm(0), address(0, None, 0));
    label(o, "cancel_done");
    finish(o, 152);
    start(o, "cerune_process_reserve", 184, 4);
    for n in [128, 136] {
        mov(o, imm(1152921504606846969), r64(0));
        o.emit(Opcode::Cmp64, [slot(n), r64(0)]);
        branch(o, Opcode::JumpB, "reserve_budget");
    }
    mov(o, slot(128), r64(0));
    o.emit(Opcode::Add64, [slot(136), r64(0)]);
    o.emit(Opcode::ShiftLeft64, [imm(3), r64(0)]);
    let limit = local(o, ".Lcerune_array_limit");
    mov(o, limit, r64(10));
    let live = local(o, ".Lcerune_array_live");
    o.emit(Opcode::Sub64, [live, r64(10)]);
    o.emit(Opcode::Cmp64, [r64(0), r64(10)]);
    branch(o, Opcode::JumpB, "reserve_budget");
    for (n, d) in [(128, 144), (136, 152)] {
        call(o, "cerune_process_result_allocate", &[slot(n)]);
        compare(o, r64(2), 0);
        branch(o, Opcode::JumpNe, "reserve_failed");
        mov(o, slot(d), r64(10));
        mov(o, r64(0), address(10, None, 0));
    }
    mov(o, imm(0), r64(0));
    finish(o, 184);
    label(o, "reserve_failed");
    call(o, "cerune_process_cancel", &[slot(144)]);
    call(o, "cerune_process_cancel", &[slot(152)]);
    mov(o, imm(2), r64(0));
    finish(o, 184);
    label(o, "reserve_budget");
    mov(o, imm(1), r64(0));
    finish(o, 184);
    // No allocation after the child has started. Copy in 1024-byte chunks so this helper needs no stack probe.
    // Arguments: owner, retained length, channel, immutable origin.
    start(o, "cerune_process_finalize", 1320, 4);
    mov(o, imm(0), slot(160));
    compare(o, slot(128), 0);
    branch(o, Opcode::JumpNe, "finalize_owner");
    compare(o, slot(136), 0);
    branch(o, Opcode::JumpNe, "finalize_invalid");
    finish(o, 1320);
    label(o, "finalize_owner");
    mov(o, slot(128), r64(0));
    mov(o, address(0, None, 8), r64(0));
    o.emit(Opcode::Cmp64, [slot(136), r64(0)]);
    branch(o, Opcode::JumpB, "finalize_invalid");
    label(o, "finalize_loop");
    mov(o, slot(136), r64(0));
    o.emit(Opcode::Sub64, [slot(160), r64(0)]);
    branch(o, Opcode::JumpE, "finalize_done");
    compare(o, r64(0), 1024);
    branch(o, Opcode::JumpBe, "finalize_size");
    mov(o, imm(1024), r64(0));
    label(o, "finalize_size");
    mov(o, r64(0), slot(168));
    mov(o, imm(0), slot(176));
    ptr(o, 256, 184);
    ptr(o, 176, 192);
    let ctx = local(o, "cerune_process_context_value");
    call(
        o,
        "cerune_process_v1_copy",
        &[ctx, slot(144), slot(160), slot(184), slot(168), slot(192)],
    );
    check(o, 0, slot(152));
    mov(o, slot(176), r64(0));
    o.emit(Opcode::Cmp64, [slot(168), r64(0)]);
    branch(o, Opcode::JumpNe, "finalize_invalid");
    mov(o, imm(0), slot(200));
    label(o, "finalize_widen");
    mov(o, slot(200), r64(1));
    o.emit(Opcode::Cmp64, [slot(168), r64(1)]);
    branch(o, Opcode::JumpE, "finalize_next");
    mov(o, slot(184), r64(10));
    o.emit(
        Opcode::ZeroExtend8To32,
        [address(10, Some((1, Scale::One)), 0), r32(0)],
    );
    o.emit(Opcode::Add64, [slot(160), r64(1)]);
    o.emit(Opcode::ShiftLeft64, [imm(3), r64(1)]);
    mov(o, slot(128), r64(10));
    mov(o, address(10, None, 0), r64(10));
    o.emit(Opcode::Sub64, [r64(1), r64(10)]);
    mov(o, r64(0), address(10, None, 0));
    o.emit(Opcode::Add64, [imm(1), slot(200)]);
    branch(o, Opcode::Jump, "finalize_widen");
    label(o, "finalize_next");
    mov(o, slot(168), r64(0));
    o.emit(Opcode::Add64, [r64(0), slot(160)]);
    branch(o, Opcode::Jump, "finalize_loop");
    label(o, "finalize_done");
    mov(o, slot(128), r64(10));
    mov(o, slot(136), r64(0));
    mov(o, r64(0), address(10, None, 8));
    mov(o, r64(0), address(10, None, 32));
    finish(o, 1320);
    label(o, "finalize_invalid");
    call(o, "cerune_process_check", &[imm(13), imm(0), slot(152)]);
    o.emit(Opcode::Undefined, []);
    // Request packing: owner, bytes-per-element (1 for stdin, 16 for argument descriptors), origin.
    start(o, "cerune_process_pack", 200, 3);
    call(o, "cerune_array_length", &[slot(128)]);
    mov(o, r64(0), slot(152));
    test(o);
    branch(o, Opcode::JumpE, "pack_empty");
    o.emit(Opcode::Mul64, [slot(136)]);
    compare(o, r64(2), 0);
    branch(o, Opcode::JumpNe, "pack_failed");
    mov(o, r64(0), slot(160));
    call(o, "malloc", &[slot(160)]);
    test(o);
    branch(o, Opcode::JumpE, "pack_failed");
    mov(o, r64(0), slot(160));
    mov(o, imm(0), slot(168));
    label(o, "pack_loop");
    mov(o, slot(168), r64(1));
    o.emit(Opcode::Cmp64, [slot(152), r64(1)]);
    branch(o, Opcode::JumpE, "pack_done");
    mov(o, slot(128), r64(10));
    mov(o, address(10, None, 0), r64(10));
    mov(o, r64(1), r64(0));
    o.emit(Opcode::ShiftLeft64, [imm(3), r64(0)]);
    o.emit(Opcode::Sub64, [r64(0), r64(10)]);
    mov(o, address(10, None, 0), r64(0));
    mov(o, slot(160), r64(11));
    compare(o, slot(136), 1);
    branch(o, Opcode::JumpE, "pack_byte");
    o.emit(Opcode::ShiftLeft64, [imm(4), r64(1)]);
    o.emit(Opcode::Add64, [r64(1), r64(11)]);
    mov(o, address(0, None, 0), r64(10));
    mov(o, r64(10), address(11, None, 8));
    o.emit(Opcode::Add64, [imm(8), r64(0)]);
    mov(o, r64(0), address(11, None, 0));
    branch(o, Opcode::Jump, "pack_next");
    label(o, "pack_byte");
    o.emit(Opcode::Mov8, [r8(0), address(11, Some((1, Scale::One)), 0)]);
    label(o, "pack_next");
    o.emit(Opcode::Add64, [imm(1), slot(168)]);
    branch(o, Opcode::Jump, "pack_loop");
    label(o, "pack_done");
    mov(o, slot(160), r64(0));
    finish(o, 200);
    label(o, "pack_empty");
    mov(o, imm(0), r64(0));
    finish(o, 200);
    label(o, "pack_failed");
    call(o, "cerune_process_pack_failed", &[slot(144)]);
    o.emit(Opcode::Undefined, []);
}
