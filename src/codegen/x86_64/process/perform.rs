use super::*;
pub(super) fn append(o: &mut NativeProgram, m: &Module, request: usize, outcome: usize) {
    let prefix = format!("run_{request}_{outcome}");
    let name = format!("cerune_process_{prefix}");
    // 128=result,136=request,144=origin,160/168=owners,192/200=packed requests,
    // 256=ABI request,512=report. Neither borrowed argument survives prepare.
    start(o, &name, 792, 3);
    for n in [160, 168, 192, 200] {
        mov(o, imm(0), slot(n));
    }
    for i in 0..15 {
        mov(o, imm(0), slot(256 + i * 8));
    }
    mov(o, imm(1), slot(256));
    mov(o, imm(120), slot(264));
    mov(o, slot(136), r64(10));
    mov(
        o,
        address(10, None, path(m, request, &["target"]).0),
        r64(0),
    );
    mov(o, address(0, None, 0), r64(11));
    mov(o, r64(11), slot(280));
    o.emit(Opcode::Add64, [imm(8), r64(0)]);
    mov(o, r64(0), slot(272));
    for (n, width, d, len, save) in [
        ("arguments", 16, 288, 296, 192),
        ("stdin", 1, 304, 312, 200),
    ] {
        mov(o, slot(136), r64(10));
        copy(o, address(10, None, path(m, request, &[n]).0), slot(208));
        call(o, "cerune_array_length", &[slot(208)]);
        mov(o, r64(0), slot(len));
        call(
            o,
            "cerune_process_pack",
            &[slot(208), imm(width), slot(144)],
        );
        mov(o, r64(0), slot(d));
        mov(o, r64(0), slot(save));
    }
    for (n, d) in [
        ("stdout_limit", 320),
        ("stderr_limit", 328),
        ("timeout_ms", 336),
    ] {
        mov(o, slot(136), r64(10));
        copy(o, address(10, None, path(m, request, &[n]).0), slot(d));
    }
    mov(o, slot(144), r64(10));
    for i in 0..4 {
        copy(o, address(10, None, i * 8), slot(344 + i * 8));
    }
    ptr(o, 256, 208);
    let ctx = local(o, "cerune_process_context_value");
    call(o, "cerune_process_v1_prepare", &[ctx, slot(208)]);
    mov(o, r64(0), slot(216));
    call(o, "free", &[slot(192)]);
    call(o, "free", &[slot(200)]);
    compare(o, slot(216), 1);
    branch(o, Opcode::JumpNe, &format!("{prefix}_ready"));
    ptr(o, 160, 224);
    ptr(o, 168, 232);
    call(
        o,
        "cerune_process_reserve",
        &[slot(320), slot(328), slot(224), slot(232)],
    );
    mov(o, r64(0), slot(240));
    let ctx = local(o, "cerune_process_context_value");
    call(o, "cerune_process_v1_execute", &[ctx, slot(240)]);
    mov(o, r64(0), slot(216));
    label(o, &format!("{prefix}_ready"));
    mov(o, slot(216), r64(0));
    check(o, 2, slot(144));
    ptr(o, 512, 208);
    let ctx = local(o, "cerune_process_context_value");
    call(o, "cerune_process_v1_report", &[ctx, slot(208), imm(32)]);
    check(o, 0, slot(144));
    let invalid = format!("{prefix}_invalid");
    for (i, expected) in [(0, 1), (1, 32), (28, 0), (29, 0), (30, 0), (31, 0)] {
        compare(o, slot(512 + i * 8), expected);
        branch(o, Opcode::JumpNe, &invalid);
    }
    for (owner, len, channel, n) in [(160, 14, 1, "stdout"), (168, 15, 2, "stderr")] {
        call(
            o,
            "cerune_process_finalize",
            &[slot(owner), slot(512 + len * 8), imm(channel), slot(144)],
        );
        mov(o, slot(128), r64(10));
        copy(o, slot(owner), address(10, None, path(m, outcome, &[n]).0));
    }
    report::append(o, m, outcome, &prefix);
    let ctx = local(o, "cerune_process_context_value");
    call(o, "cerune_process_v1_release", &[ctx]);
    check(o, 0, slot(144));
    finish(o, 792);
    label(o, &invalid);
    call(o, "cerune_process_check", &[imm(13), imm(0), slot(144)]);
    o.emit(Opcode::Undefined, []);
}
