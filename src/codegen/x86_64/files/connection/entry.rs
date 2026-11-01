use super::*;
pub(super) fn append(o: &mut NativeProgram) {
    let linux = is_linux(o);
    o.push(EntryKind::Section(Section::ReadOnlyData));
    for s in [
        "configure",
        "open-root",
        "invalid-root",
        "unknown-setting",
        "duplicate-setting",
        "missing-delimiter",
    ] {
        o.label(&format!(".Lfile_text_{}", s.replace('-', "_")));
        o.push(EntryKind::CString(s.into()));
    }
    o.label(".Lfile_text_startup_format");
    o.push(EntryKind::CString(
        "cerune: file-v1 operation=connect phase=%s code=%s origin=host\n".into(),
    ));
    for (name, text) in [
        ("marker", "--cerune-runtime-v1"),
        ("root_option", "--file-root"),
        ("delimiter", "--"),
    ] {
        o.label(&format!(".Lfile_text_{name}"));
        if linux {
            o.push(EntryKind::CString(text.into()));
        } else {
            for b in text.encode_utf16().chain([0]) {
                o.push(EntryKind::Data(b.to_le_bytes().to_vec()));
            }
        }
    }
    start(o, "cerune_files_open_code", 136, 1);
    for (n, code) in [
        (1, "not-found"),
        (2, "already-exists"),
        (3, "unsupported-target"),
        (5, "invalid-response"),
    ] {
        compare(o, slot(128), n);
        branch(o, Opcode::JumpE, &format!("os_code_{n}"));
        // 両端の変換失敗やOSエラーは共通のopen-failedへまとめます。
        let _ = code;
    }
    let p = literal(o, "open-failed");
    o.emit(Opcode::Lea64, [p, r64(0)]);
    finish(o, 136);
    for (n, code) in [
        (1, "not-found"),
        (2, "already-exists"),
        (3, "unsupported-target"),
        (5, "invalid-response"),
    ] {
        label(o, &format!("os_code_{n}"));
        let p = literal(o, code);
        o.emit(Opcode::Lea64, [p, r64(0)]);
        finish(o, 136);
    }
    start(o, "cerune_files_startup_failed", 424, 2);
    pointer(o, "startup_format", 144);
    ptr(o, 160, 152);
    call(
        o,
        "snprintf",
        &[slot(152), imm(256), slot(144), slot(128), slot(136)],
    );
    test(o);
    branch(o, Opcode::JumpLe, "os_startup_exit");
    compare(o, r64(0), 256);
    branch(o, Opcode::JumpB, "os_startup_write");
    mov(o, imm(255), r64(0));
    label(o, "os_startup_write");
    mov(o, r64(0), slot(144));
    call(
        o,
        if linux { "write" } else { "_write" },
        &[imm(2), slot(152), slot(144)],
    );
    label(o, "os_startup_exit");
    call(o, "_Exit", &[imm(1)]);
    o.emit(Opcode::Undefined, []);
    // argc/argv → rax=最初のプログラム引数のindex、rdx=借用rootポインタ。
    start(o, "cerune_files_parse", 200, 2);
    mov(o, imm(0), slot(144));
    mov(o, imm(2), slot(152));
    compare(o, slot(128), 1);
    branch(o, Opcode::JumpLe, "os_parse_plain");
    mov(o, slot(136), r64(0));
    mov(o, address(0, None, 8), r64(0));
    mov(o, r64(0), slot(160));
    pointer(o, "marker", 168);
    let cmp = if linux { "strcmp" } else { "wcscmp" };
    call(o, cmp, &[slot(160), slot(168)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_parse_plain");
    label(o, "os_parse_loop");
    mov(o, slot(152), r64(0));
    o.emit(Opcode::Cmp64, [slot(128), r64(0)]);
    branch(o, Opcode::JumpGe, "os_parse_missing");
    mov(o, slot(136), r64(1));
    mov(o, address(1, Some((0, Scale::Eight)), 0), r64(0));
    mov(o, r64(0), slot(160));
    pointer(o, "delimiter", 168);
    call(o, cmp, &[slot(160), slot(168)]);
    test(o);
    branch(o, Opcode::JumpE, "os_parse_done");
    pointer(o, "root_option", 168);
    call(o, cmp, &[slot(160), slot(168)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_parse_unknown");
    compare(o, slot(144), 0);
    branch(o, Opcode::JumpNe, "os_parse_duplicate");
    o.emit(Opcode::Add64, [imm(1), slot(152)]);
    mov(o, slot(152), r64(0));
    o.emit(Opcode::Cmp64, [slot(128), r64(0)]);
    branch(o, Opcode::JumpGe, "os_parse_invalid");
    mov(o, slot(136), r64(1));
    mov(o, address(1, Some((0, Scale::Eight)), 0), r64(0));
    mov(o, r64(0), slot(144));
    let step = if linux { 1 } else { 2 };
    if linux {
        o.emit(Opcode::ZeroExtend8To32, [address(0, None, 0), r32(1)]);
    } else {
        word(o, 0, None, 0, 1);
    }
    o.emit(Opcode::Test32, [r32(1), r32(1)]);
    branch(o, Opcode::JumpE, "os_parse_invalid");
    o.emit(Opcode::Cmp32, [imm(45), r32(1)]);
    branch(o, Opcode::JumpNe, "os_parse_next");
    if linux {
        o.emit(Opcode::ZeroExtend8To32, [address(0, None, step), r32(1)]);
    } else {
        word(o, 0, None, step, 1);
    }
    o.emit(Opcode::Cmp32, [imm(45), r32(1)]);
    branch(o, Opcode::JumpE, "os_parse_invalid");
    label(o, "os_parse_next");
    o.emit(Opcode::Add64, [imm(1), slot(152)]);
    branch(o, Opcode::Jump, "os_parse_loop");
    label(o, "os_parse_plain");
    mov(o, imm(1), r64(0));
    mov(o, imm(0), r64(2));
    finish(o, 200);
    label(o, "os_parse_done");
    mov(o, slot(152), r64(0));
    o.emit(Opcode::Add64, [imm(1), r64(0)]);
    mov(o, slot(144), r64(2));
    finish(o, 200);
    for (label_name, code) in [
        ("missing", "missing-delimiter"),
        ("invalid", "invalid-root"),
        ("duplicate", "duplicate-setting"),
        ("unknown", "unknown-setting"),
    ] {
        label(o, &format!("os_parse_{label_name}"));
        startup(o, "configure", code);
    }
}
