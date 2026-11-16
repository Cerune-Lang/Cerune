use super::*;
fn startup(o: &mut NativeProgram, code: &str) {
    literal(o, "configure", 128);
    literal(o, code, 136);
    call(o, "cerune_process_startup_failed", &[slot(128), slot(136)]);
    o.emit(Opcode::Undefined, []);
}
pub(super) fn append(o: &mut NativeProgram, files: bool) {
    o.push(EntryKind::Section(Section::ReadOnlyData));
    for (n, t) in [
        ("configure", "configure"),
        ("invalid-root", "invalid-root"),
        ("unknown-setting", "unknown-setting"),
        ("duplicate-setting", "duplicate-setting"),
        ("missing-delimiter", "missing-delimiter"),
        ("config-open", "config-open"),
        ("config-read", "config-read"),
        ("config-too-large", "config-too-large"),
        ("config-empty", "config-empty"),
        ("allocation", "allocation"),
        (
            "startup",
            "cerune-process-startup-v1 phase=%s code=%s origin=host\n",
        ),
        ("abi", "cerune-process-abi-v1 status=%u%s"),
        (
            "allocation_format",
            "cerune: runtime-v1 code=allocation-failed%s",
        ),
    ] {
        o.label(&format!(".Lprocess_text_{}", n.replace('-', "_")));
        o.push(EntryKind::CString(t.into()));
    }
    o.label(".Lprocess_host");
    for _ in 0..4 {
        o.push(EntryKind::Data64(0));
    }
    o.push(EntryKind::CString(" origin=host\n".into()));
    for (n, t) in [
        ("marker", "--cerune-runtime-v1"),
        ("root", "--file-root"),
        ("config", "--process-config"),
        ("delimiter", "--"),
        ("mode", "rb"),
        ("empty", ""),
    ] {
        o.label(&format!(".Lprocess_text_{}", n.replace('-', "_")));
        if linux(o) {
            o.push(EntryKind::CString(t.into()));
        } else {
            for b in t.encode_utf16().chain([0]) {
                o.push(EntryKind::Data(b.to_le_bytes().to_vec()));
            }
        }
    }
    o.push(EntryKind::Section(Section::Text));
    // Bounded diagnostics are written once, then terminate regardless of diagnostic-write success.
    start(o, "cerune_process_startup_failed", 456, 2);
    literal(o, "startup", 144);
    ptr(o, 192, 152);
    call(
        o,
        "snprintf",
        &[slot(152), imm(256), slot(144), slot(128), slot(136)],
    );
    branch(o, Opcode::Jump, "diagnostic_write");
    start(o, "cerune_process_check", 456, 3);
    mov(o, slot(128), r64(0));
    o.emit(Opcode::Cmp64, [slot(136), r64(0)]);
    branch(o, Opcode::JumpNe, "abi_failure");
    finish(o, 456);
    label(o, "abi_failure");
    mov(o, slot(144), r64(0));
    o.emit(Opcode::Add64, [imm(32), r64(0)]);
    mov(o, r64(0), slot(160));
    literal(o, "abi", 144);
    ptr(o, 192, 152);
    call(o, "fflush", &[imm(0)]);
    call(
        o,
        "snprintf",
        &[slot(152), imm(256), slot(144), slot(128), slot(160)],
    );
    branch(o, Opcode::Jump, "diagnostic_write");
    start(o, "cerune_process_pack_failed", 456, 1);
    mov(o, slot(128), r64(0));
    o.emit(Opcode::Add64, [imm(32), r64(0)]);
    mov(o, r64(0), slot(160));
    literal(o, "allocation_format", 144);
    ptr(o, 192, 152);
    call(o, "fflush", &[imm(0)]);
    call(o, "snprintf", &[slot(152), imm(256), slot(144), slot(160)]);
    label(o, "diagnostic_write");
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpLe, "diagnostic_exit");
    compare(o, r64(0), 256);
    branch(o, Opcode::JumpB, "diagnostic_count");
    mov(o, imm(255), r64(0));
    label(o, "diagnostic_count");
    mov(o, r64(0), slot(160));
    call(
        o,
        if linux(o) { "write" } else { "_write" },
        &[imm(2), slot(152), slot(160)],
    );
    label(o, "diagnostic_exit");
    call(o, "_Exit", &[imm(1)]);
    o.emit(Opcode::Undefined, []);
    parse(o, files);
    connect(o);
    start(o, "cerune_process_disconnect", 152, 0);
    let ctx = local(o, "cerune_process_context_value");
    o.emit(Opcode::Lea64, [ctx, r64(0)]);
    mov(o, r64(0), slot(128));
    call(o, "cerune_process_v1_destroy", &[slot(128)]);
    mov(o, r64(0), slot(136));
    let host = local(o, ".Lprocess_host");
    o.emit(Opcode::Lea64, [host, r64(0)]);
    mov(o, r64(0), slot(128));
    call(o, "cerune_process_check", &[slot(136), imm(0), slot(128)]);
    finish(o, 152);
}
fn parse(o: &mut NativeProgram, files: bool) {
    let cmp = if linux(o) { "strcmp" } else { "wcscmp" };
    start(o, "cerune_process_parse", 232, 2);
    for n in [144, 152] {
        mov(o, imm(0), slot(n));
    }
    mov(o, imm(2), slot(160));
    compare(o, slot(128), 1);
    branch(o, Opcode::JumpLe, "parse_plain");
    mov(o, slot(136), r64(0));
    copy(o, address(0, None, 8), slot(168));
    literal(o, "marker", 176);
    call(o, cmp, &[slot(168), slot(176)]);
    test(o);
    branch(o, Opcode::JumpNe, "parse_plain");
    label(o, "parse_loop");
    mov(o, slot(160), r64(0));
    o.emit(Opcode::Cmp64, [slot(128), r64(0)]);
    branch(o, Opcode::JumpGe, "parse_missing");
    mov(o, slot(136), r64(10));
    copy(o, address(10, Some((0, Scale::Eight)), 0), slot(168));
    literal(o, "delimiter", 176);
    call(o, cmp, &[slot(168), slot(176)]);
    test(o);
    branch(o, Opcode::JumpE, "parse_done");
    for (n, d) in [("config", 152), ("root", 144)] {
        if n == "root" && !files {
            continue;
        }
        literal(o, n, 176);
        call(o, cmp, &[slot(168), slot(176)]);
        test(o);
        branch(o, Opcode::JumpE, &format!("parse_{n}"));
        let _ = d;
    }
    branch(o, Opcode::Jump, "parse_unknown");
    for (n, d) in [("config", 152), ("root", 144)] {
        label(o, &format!("parse_{n}"));
        compare(o, slot(d), 0);
        branch(o, Opcode::JumpNe, "parse_duplicate");
        ptr(o, d, 184);
        branch(o, Opcode::Jump, "parse_value");
    }
    label(o, "parse_value");
    o.emit(Opcode::Add64, [imm(1), slot(160)]);
    mov(o, slot(160), r64(0));
    o.emit(Opcode::Cmp64, [slot(128), r64(0)]);
    branch(o, Opcode::JumpGe, "parse_invalid");
    mov(o, slot(136), r64(10));
    copy(o, address(10, Some((0, Scale::Eight)), 0), slot(168));
    literal(o, "empty", 176);
    call(o, cmp, &[slot(168), slot(176)]);
    test(o);
    branch(o, Opcode::JumpE, "parse_invalid");
    mov(o, slot(168), r64(10));
    let step = if linux(o) { 1 } else { 2 };
    // Compare a full Windows UTF-16 code unit; do not mistake U+012D for ASCII '-'.
    for k in 0..2 {
        o.emit(
            Opcode::ZeroExtend8To32,
            [address(10, None, k * step), r32(0)],
        );
        if !linux(o) {
            o.emit(
                Opcode::ZeroExtend8To32,
                [address(10, None, k * step + 1), r32(11)],
            );
            o.emit(Opcode::ShiftLeft64, [imm(8), r64(11)]);
            o.emit(Opcode::Or64, [r64(11), r64(0)]);
        }
        compare(o, r64(0), 45);
        branch(o, Opcode::JumpNe, "parse_store");
    }
    branch(o, Opcode::Jump, "parse_invalid");
    label(o, "parse_store");
    mov(o, slot(184), r64(10));
    copy(o, slot(168), address(10, None, 0));
    o.emit(Opcode::Add64, [imm(1), slot(160)]);
    branch(o, Opcode::Jump, "parse_loop");
    label(o, "parse_plain");
    mov(o, imm(1), r64(0));
    mov(o, imm(0), r64(2));
    finish(o, 232);
    label(o, "parse_done");
    let config = local(o, "cerune_process_selected_config");
    copy(o, slot(152), config);
    mov(o, slot(160), r64(0));
    o.emit(Opcode::Add64, [imm(1), r64(0)]);
    mov(o, slot(144), r64(2));
    finish(o, 232);
    for (n, c) in [
        ("missing", "missing-delimiter"),
        ("invalid", "invalid-root"),
        ("duplicate", "duplicate-setting"),
        ("unknown", "unknown-setting"),
    ] {
        label(o, &format!("parse_{n}"));
        startup(o, c);
    }
}
fn connect(o: &mut NativeProgram) {
    start(o, "cerune_process_connect", 232, 0);
    for n in [128, 136] {
        mov(o, imm(0), slot(n));
    }
    let config = local(o, "cerune_process_selected_config");
    copy(o, config, slot(144));
    compare(o, slot(144), 0);
    branch(o, Opcode::JumpE, "connect_create");
    literal(o, "mode", 152);
    call(
        o,
        if linux(o) { "fopen" } else { "_wfopen" },
        &[slot(144), slot(152)],
    );
    test(o);
    branch(o, Opcode::JumpE, "connect_open");
    mov(o, r64(0), slot(160));
    call(o, "malloc", &[imm(1048577)]);
    test(o);
    branch(o, Opcode::JumpE, "connect_allocation");
    mov(o, r64(0), slot(128));
    call(o, "fread", &[slot(128), imm(1), imm(1048577), slot(160)]);
    mov(o, r64(0), slot(136));
    call(o, "ferror", &[slot(160)]);
    mov(o, r64(0), slot(168));
    call(o, "fclose", &[slot(160)]);
    test(o);
    branch(o, Opcode::JumpNe, "connect_read");
    compare(o, slot(168), 0);
    branch(o, Opcode::JumpNe, "connect_read");
    compare(o, slot(136), 1048576);
    branch(o, Opcode::JumpA, "connect_large");
    compare(o, slot(136), 0);
    branch(o, Opcode::JumpE, "connect_empty");
    label(o, "connect_create");
    let ctx = local(o, "cerune_process_context_value");
    o.emit(Opcode::Lea64, [ctx, r64(0)]);
    mov(o, r64(0), slot(176));
    if linux(o) {
        call(
            o,
            "cerune_process_v1_create",
            &[imm(1), imm(2), slot(128), slot(136), slot(176)],
        );
    } else {
        ptr(o, 128, 184);
        call(
            o,
            "cerune_process_v1_create",
            &[imm(1), imm(1), slot(184), slot(176)],
        );
    }
    mov(o, r64(0), slot(192));
    call(o, "free", &[slot(128)]);
    let host = local(o, ".Lprocess_host");
    o.emit(Opcode::Lea64, [host, r64(0)]);
    mov(o, r64(0), slot(200));
    call(o, "cerune_process_check", &[slot(192), imm(0), slot(200)]);
    finish(o, 232);
    for (n, c) in [
        ("open", "config-open"),
        ("read", "config-read"),
        ("large", "config-too-large"),
        ("empty", "config-empty"),
        ("allocation", "allocation"),
    ] {
        label(o, &format!("connect_{n}"));
        startup(o, c);
    }
}
