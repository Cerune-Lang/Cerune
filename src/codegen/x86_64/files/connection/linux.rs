//! x86-64 GNU: stat=144、mode=24、dev/ino=0/8。O_PATHで対象を固定します。
use super::*;
pub(super) fn append(o: &mut NativeProgram) {
    o.symbol("cerune_files_errno", Linkage::Local);
    o.push(EntryKind::Section(Section::ReadOnlyData));
    o.label(".Lfile_text_proc");
    o.push(EntryKind::CString("/proc/self/fd/%d".into()));
    start(o, "cerune_files_errno", 152, 1);
    call(o, "__errno_location", &[]);
    o.emit(Opcode::Mov32, [address(0, None, 0), r32(0)]);
    for (n, code) in [(2, 1), (17, 2), (20, 3), (40, 3)] {
        o.emit(Opcode::Cmp32, [imm(n), r32(0)]);
        branch(o, Opcode::JumpE, &format!("os_errno_{code}"));
    }
    o.emit(Opcode::Cmp32, [imm(21), r32(0)]);
    branch(o, Opcode::JumpNe, "os_errno_4");
    compare(o, slot(128), 0);
    branch(o, Opcode::JumpNe, "os_errno_2");
    branch(o, Opcode::Jump, "os_errno_3");
    for n in 1..=4 {
        label(o, &format!("os_errno_{n}"));
        result(o, 152, n);
    }
    start(o, "cerune_files_connect", 360, 1);
    compare(o, slot(128), 0);
    branch(o, Opcode::JumpE, "os_root_done");
    call(o, "strlen", &[slot(128)]);
    mov(o, r64(0), slot(136));
    o.emit(Opcode::Add64, [imm(1), r64(0)]);
    mov(o, r64(0), slot(144));
    call(o, "malloc", &[slot(144)]);
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpE, "os_root_alloc");
    mov(o, r64(0), slot(152));
    call(o, "memcpy", &[slot(152), slot(128), slot(144)]);
    label(o, "os_root_trim");
    mov(o, slot(136), r64(1));
    compare(o, r64(1), 1);
    branch(o, Opcode::JumpBe, "os_root_open");
    mov(o, slot(152), r64(0));
    o.emit(
        Opcode::Cmp8,
        [imm(47), address(0, Some((1, Scale::One)), -1)],
    );
    branch(o, Opcode::JumpNe, "os_root_open");
    o.emit(
        Opcode::Mov8,
        [imm(0), address(0, Some((1, Scale::One)), -1)],
    );
    o.emit(Opcode::Sub64, [imm(1), slot(136)]);
    branch(o, Opcode::Jump, "os_root_trim");
    label(o, "os_root_open");
    call(o, "open", &[slot(152), imm(2752512)]);
    test(o);
    branch(o, Opcode::JumpS, "os_root_error");
    mov(o, r64(0), slot(160));
    call(o, "free", &[slot(152)]);
    ptr(o, 192, 168);
    call(o, "fstat", &[slot(160), slot(168)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_root_stat");
    o.emit(Opcode::Mov32, [slot(216), r32(0)]);
    o.emit(Opcode::And32, [imm(61440), r32(0)]);
    o.emit(Opcode::Cmp32, [imm(16384), r32(0)]);
    branch(o, Opcode::JumpNe, "os_root_kind");
    mov(o, slot(160), r64(0));
    let p = global(o, "root");
    mov(o, r64(0), p);
    let p = global(o, "configured");
    mov(o, imm(1), p);
    label(o, "os_root_done");
    finish(o, 360);
    label(o, "os_root_error");
    call(o, "cerune_files_errno", &[imm(0)]);
    mov(o, r64(0), slot(176));
    call(o, "free", &[slot(152)]);
    open_failure(o, 176);
    label(o, "os_root_stat");
    mov(o, imm(4), slot(176));
    branch(o, Opcode::Jump, "os_root_close");
    label(o, "os_root_kind");
    mov(o, imm(3), slot(176));
    label(o, "os_root_close");
    call(o, "close", &[slot(160)]);
    open_failure(o, 176);
    label(o, "os_root_alloc");
    startup(o, "open-root", "allocation-failed");
    // 128 name,136 length,144 write,152 result,160 fd,168 readable,176 ptr,184 scratch.
    // path=208..464, before=464..608, after=608..752, proc=752..816.
    start(o, "cerune_file_host_open", 824, 4);
    ptr(o, 208, 176);
    call(o, "memcpy", &[slot(176), slot(128), slot(136)]);
    mov(o, slot(136), r64(1));
    o.emit(Opcode::Lea64, [slot(208), r64(0)]);
    o.emit(Opcode::Mov8, [imm(0), address(0, Some((1, Scale::One)), 0)]);
    let p = global(o, "root");
    mov(o, p, r64(0));
    mov(o, r64(0), slot(184));
    mov(o, imm(2752512), slot(192));
    compare(o, slot(144), 0);
    branch(o, Opcode::JumpE, "os_open_call");
    mov(o, imm(655553), slot(192));
    label(o, "os_open_call");
    call(o, "openat", &[slot(184), slot(176), slot(192), imm(438)]);
    test(o);
    branch(o, Opcode::JumpS, "os_open_error");
    mov(o, r64(0), slot(160));
    compare(o, slot(144), 0);
    branch(o, Opcode::JumpNe, "os_open_ready");
    ptr(o, 464, 176);
    call(o, "fstat", &[slot(160), slot(176)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_open_stat");
    o.emit(Opcode::Mov32, [slot(488), r32(0)]);
    o.emit(Opcode::And32, [imm(61440), r32(0)]);
    o.emit(Opcode::Cmp32, [imm(32768), r32(0)]);
    branch(o, Opcode::JumpNe, "os_open_kind");
    ptr(o, 752, 176);
    pointer(o, "proc", 184);
    call(o, "snprintf", &[slot(176), imm(64), slot(184), slot(160)]);
    call(o, "open", &[slot(176), imm(524288)]);
    o.emit(Opcode::ShiftLeft64, [imm(32), r64(0)]);
    o.emit(Opcode::ShiftArithmeticRight64, [imm(32), r64(0)]);
    mov(o, r64(0), slot(168));
    call(o, "close", &[slot(160)]);
    compare(o, slot(168), 0);
    branch(o, Opcode::JumpL, "os_open_failed");
    ptr(o, 608, 176);
    call(o, "fstat", &[slot(168), slot(176)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_restat");
    o.emit(Opcode::Mov32, [slot(632), r32(0)]);
    o.emit(Opcode::And32, [imm(61440), r32(0)]);
    o.emit(Opcode::Cmp32, [imm(32768), r32(0)]);
    branch(o, Opcode::JumpNe, "os_identity");
    for (a, b) in [(464, 608), (472, 616)] {
        mov(o, slot(a), r64(0));
        o.emit(Opcode::Cmp64, [slot(b), r64(0)]);
        branch(o, Opcode::JumpNe, "os_identity");
    }
    mov(o, slot(168), r64(0));
    mov(o, r64(0), slot(160));
    label(o, "os_open_ready");
    mov(o, slot(160), r64(0));
    mov(o, slot(152), r64(1));
    mov(o, r64(0), address(1, None, 0));
    result(o, 824, 0);
    label(o, "os_open_error");
    call(o, "cerune_files_errno", &[slot(144)]);
    finish(o, 824);
    label(o, "os_open_stat");
    mov(o, imm(4), slot(192));
    branch(o, Opcode::Jump, "os_open_close");
    label(o, "os_open_kind");
    mov(o, imm(3), slot(192));
    label(o, "os_open_close");
    call(o, "close", &[slot(160)]);
    mov(o, slot(192), r64(0));
    finish(o, 824);
    label(o, "os_identity");
    mov(o, imm(5), slot(192));
    branch(o, Opcode::Jump, "os_reopen_close");
    label(o, "os_restat");
    mov(o, imm(4), slot(192));
    label(o, "os_reopen_close");
    call(o, "close", &[slot(168)]);
    mov(o, slot(192), r64(0));
    finish(o, 824);
    label(o, "os_open_failed");
    result(o, 824, 4);
    for name in ["read", "write"] {
        start(o, &format!("cerune_file_host_{name}"), 152, 3);
        call(o, name, &[slot(128), slot(136), slot(144)]);
        compare(o, r64(0), 0);
        branch(o, Opcode::JumpGe, &format!("os_{name}_return"));
        call(o, "__errno_location", &[]);
        o.emit(Opcode::Cmp32, [imm(4), address(0, None, 0)]);
        branch(o, Opcode::JumpE, &format!("os_{name}_eintr"));
        mov(o, imm(-1), r64(0));
        branch(o, Opcode::Jump, &format!("os_{name}_return"));
        label(o, &format!("os_{name}_eintr"));
        mov(o, imm(-2), r64(0));
        label(o, &format!("os_{name}_return"));
        finish(o, 152);
    }
    start(o, "cerune_file_host_close", 136, 1);
    call(o, "close", &[slot(128)]);
    finish(o, 136);
}
