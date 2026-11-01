//! Win64のNT構造体を固定配置し、保持したdirectory handleから相対openします。
use super::*;
pub(super) fn append(o: &mut NativeProgram) {
    o.push(EntryKind::Section(Section::ReadOnlyData));
    for (name, text) in [
        ("ntdll", "ntdll.dll"),
        ("nt_prefix", "\\??\\"),
        ("unc_prefix", "UNC\\"),
        ("extended", "\\\\?\\"),
        ("device", "\\\\.\\"),
    ] {
        o.label(&format!(".Lfile_text_{name}"));
        o.push(EntryKind::Data(
            text.encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect(),
        ));
    }
    o.label(".Lfile_text_nt_create");
    o.push(EntryKind::CString("NtCreateFile".into()));
    nt_open(o);
    kind(o);
    connect(o);
    open(o);
    transfer(o);
}
fn nt_open(o: &mut NativeProgram) {
    // 引数root/name/length/directory/create/result。UNICODE_STRING=16、OBJECT_ATTRIBUTES=48。
    start(o, "cerune_files_nt_open", 312, 6);
    compare(o, slot(144), 32767);
    branch(o, Opcode::JumpA, "os_nt_failed");
    zero(o, 208, 10);
    mov(o, slot(144), r64(0));
    o.emit(Opcode::ShiftLeft64, [imm(1), r64(0)]);
    mov(o, r64(0), r64(1));
    o.emit(Opcode::ShiftLeft64, [imm(16), r64(1)]);
    o.emit(Opcode::Or32, [r32(1), r32(0)]);
    o.emit(Opcode::Mov32, [r32(0), slot(208)]);
    mov(o, slot(136), r64(0));
    mov(o, r64(0), slot(216));
    o.emit(Opcode::Mov32, [imm(48), slot(224)]);
    mov(o, slot(128), r64(0));
    mov(o, r64(0), slot(232));
    ptr(o, 208, 240);
    o.emit(Opcode::Mov32, [imm(64), slot(248)]);
    pointer(o, "ntdll", 288);
    call(o, "GetModuleHandleW", &[slot(288)]);
    mov(o, r64(0), slot(288));
    pointer(o, "nt_create", 296);
    call(o, "GetProcAddress", &[slot(288), slot(296)]);
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "os_nt_failed");
    mov(o, r64(0), slot(184));
    mov(o, imm(1048705), slot(176));
    mov(o, imm(1), slot(200));
    mov(o, imm(2097248), slot(192));
    compare(o, slot(160), 0);
    branch(o, Opcode::JumpE, "os_nt_directory");
    mov(o, imm(1048706), slot(176));
    mov(o, imm(2), slot(200));
    label(o, "os_nt_directory");
    compare(o, slot(152), 0);
    branch(o, Opcode::JumpE, "os_nt_call");
    mov(o, imm(1048736), slot(176));
    mov(o, imm(2097185), slot(192));
    label(o, "os_nt_call");
    ptr(o, 224, 288);
    ptr(o, 272, 296);
    super::super::call_arguments(
        o,
        &[
            slot(168),
            slot(176),
            slot(288),
            slot(296),
            imm(0),
            imm(0),
            imm(7),
            slot(200),
            slot(192),
            imm(0),
            imm(0),
        ],
    );
    mov(o, slot(184), r64(11));
    o.emit(Opcode::Call, [r64(11)]);
    test(o);
    branch(o, Opcode::JumpS, "os_nt_error");
    mov(o, slot(168), r64(0));
    mov(o, address(0, None, 0), r64(0));
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "os_nt_invalid");
    compare(o, r64(0), -1);
    branch(o, Opcode::JumpE, "os_nt_invalid");
    result(o, 312, 0);
    label(o, "os_nt_error");
    for (code, dest) in [
        (0xc0000034u32, 1),
        (0xc000003a, 1),
        (0xc0000035, 2),
        (0xc0000103, 3),
        (0xc0000279, 3),
        (0xc000050b, 3),
    ] {
        o.emit(Opcode::Cmp32, [imm(code as i32 as i128), r32(0)]);
        branch(o, Opcode::JumpE, &format!("os_nt_{dest}"));
    }
    o.emit(Opcode::Cmp32, [imm(0xc00000bau32 as i32 as i128), r32(0)]);
    branch(o, Opcode::JumpNe, "os_nt_failed");
    compare(o, slot(160), 0);
    branch(o, Opcode::JumpNe, "os_nt_2");
    branch(o, Opcode::Jump, "os_nt_3");
    for n in 1..=3 {
        label(o, &format!("os_nt_{n}"));
        result(o, 312, n);
    }
    label(o, "os_nt_invalid");
    result(o, 312, 5);
    label(o, "os_nt_failed");
    result(o, 312, 4);
}
fn kind(o: &mut NativeProgram) {
    start(o, "cerune_files_kind", 168, 2);
    call(o, "GetFileType", &[slot(128)]);
    o.emit(Opcode::Cmp32, [imm(1), r32(0)]);
    branch(o, Opcode::JumpNe, "os_kind_bad");
    ptr(o, 144, 152);
    call(
        o,
        "GetFileInformationByHandleEx",
        &[slot(128), imm(9), slot(152), imm(8)],
    );
    test(o);
    branch(o, Opcode::JumpE, "os_kind_failed");
    o.emit(Opcode::Mov32, [slot(144), r32(0)]);
    o.emit(Opcode::Test32, [imm(1024), r32(0)]);
    branch(o, Opcode::JumpNe, "os_kind_bad");
    o.emit(Opcode::And32, [imm(16), r32(0)]);
    o.emit(Opcode::ShiftRight64, [imm(4), r64(0)]);
    o.emit(Opcode::Cmp64, [slot(136), r64(0)]);
    branch(o, Opcode::JumpNe, "os_kind_bad");
    result(o, 168, 0);
    label(o, "os_kind_bad");
    result(o, 168, 3);
    label(o, "os_kind_failed");
    result(o, 168, 4);
}
fn connect(o: &mut NativeProgram) {
    // root128 length136 absolute144 ntname152 offset160 prefix168 actual176 remaining184
    // suffix192 destination200 code208 handle216 scratch224/232/240 capacity248.
    start(o, "cerune_files_connect", 264, 1);
    zero(o, 144, 2);
    mov(o, imm(0), slot(216));
    mov(o, imm(0), slot(160));
    mov(o, imm(4), slot(168));
    compare(o, slot(128), 0);
    branch(o, Opcode::JumpE, "os_win_root_done");
    call(o, "GetFullPathNameW", &[slot(128), imm(0), imm(0), imm(0)]);
    test(o);
    branch(o, Opcode::JumpE, "os_win_root_invalid");
    compare(o, r64(0), 32760);
    branch(o, Opcode::JumpA, "os_win_root_invalid");
    mov(o, r64(0), slot(136));
    o.emit(Opcode::Add64, [imm(1), r64(0)]);
    mov(o, r64(0), slot(248));
    o.emit(Opcode::ShiftLeft64, [imm(1), r64(0)]);
    mov(o, r64(0), slot(224));
    call(o, "malloc", &[slot(224)]);
    mov(o, r64(0), slot(144));
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "os_win_root_alloc");
    o.emit(Opcode::Add64, [imm(16), slot(224)]);
    call(o, "malloc", &[slot(224)]);
    mov(o, r64(0), slot(152));
    compare(o, r64(0), 0);
    branch(o, Opcode::JumpE, "os_win_root_alloc");
    call(
        o,
        "GetFullPathNameW",
        &[slot(128), slot(248), slot(144), imm(0)],
    );
    test(o);
    branch(o, Opcode::JumpE, "os_win_root_invalid");
    o.emit(Opcode::Cmp64, [slot(136), r64(0)]);
    branch(o, Opcode::JumpA, "os_win_root_invalid");
    mov(o, r64(0), slot(176));
    pointer(o, "nt_prefix", 224);
    call(o, "memcpy", &[slot(152), slot(224), imm(8)]);
    compare(o, slot(176), 4);
    branch(o, Opcode::JumpB, "os_win_unc");
    pointer(o, "extended", 224);
    call(o, "wcsncmp", &[slot(144), slot(224), imm(4)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_win_device");
    mov(o, imm(4), slot(160));
    branch(o, Opcode::Jump, "os_win_validate");
    label(o, "os_win_device");
    pointer(o, "device", 224);
    call(o, "wcsncmp", &[slot(144), slot(224), imm(4)]);
    test(o);
    branch(o, Opcode::JumpE, "os_win_root_invalid");
    label(o, "os_win_unc");
    compare(o, slot(176), 2);
    branch(o, Opcode::JumpB, "os_win_validate");
    mov(o, slot(144), r64(0));
    o.emit(Opcode::Cmp32, [imm(0x005c005c), address(0, None, 0)]);
    branch(o, Opcode::JumpNe, "os_win_validate");
    mov(o, slot(152), r64(0));
    o.emit(Opcode::Add64, [imm(8), r64(0)]);
    mov(o, r64(0), slot(232));
    pointer(o, "unc_prefix", 224);
    call(o, "memcpy", &[slot(232), slot(224), imm(8)]);
    mov(o, imm(8), slot(168));
    mov(o, imm(2), slot(160));
    label(o, "os_win_validate");
    mov(o, slot(176), r64(0));
    o.emit(Opcode::Sub64, [slot(160), r64(0)]);
    mov(o, r64(0), slot(184));
    mov(o, slot(160), r64(0));
    o.emit(Opcode::ShiftLeft64, [imm(1), r64(0)]);
    o.emit(Opcode::Add64, [slot(144), r64(0)]);
    mov(o, r64(0), slot(192));
    compare(o, slot(168), 8);
    branch(o, Opcode::JumpE, "os_win_copy");
    compare(o, slot(184), 3);
    branch(o, Opcode::JumpB, "os_win_root_invalid");
    o.emit(Opcode::Cmp32, [imm(0x005c003a), address(0, None, 2)]);
    branch(o, Opcode::JumpE, "os_win_copy");
    compare(o, slot(184), 4);
    branch(o, Opcode::JumpB, "os_win_root_invalid");
    pointer(o, "unc_prefix", 224);
    call(o, "wcsncmp", &[slot(192), slot(224), imm(4)]);
    test(o);
    branch(o, Opcode::JumpNe, "os_win_root_invalid");
    label(o, "os_win_copy");
    mov(o, slot(168), r64(0));
    o.emit(Opcode::ShiftLeft64, [imm(1), r64(0)]);
    o.emit(Opcode::Add64, [slot(152), r64(0)]);
    mov(o, r64(0), slot(200));
    mov(o, slot(184), r64(0));
    o.emit(Opcode::ShiftLeft64, [imm(1), r64(0)]);
    mov(o, r64(0), slot(224));
    call(o, "memcpy", &[slot(200), slot(192), slot(224)]);
    mov(o, slot(184), r64(0));
    o.emit(Opcode::Add64, [slot(168), r64(0)]);
    mov(o, r64(0), slot(240));
    label(o, "os_win_trim");
    mov(o, slot(240), r64(1));
    compare(o, r64(1), 7);
    branch(o, Opcode::JumpBe, "os_win_root_open");
    mov(o, slot(152), r64(0));
    word(o, 0, Some((1, Scale::Two)), -2, 2);
    compare(o, r64(2), 92);
    branch(o, Opcode::JumpNe, "os_win_root_open");
    o.emit(Opcode::Sub64, [imm(1), slot(240)]);
    branch(o, Opcode::Jump, "os_win_trim");
    label(o, "os_win_root_open");
    ptr(o, 216, 224);
    call(
        o,
        "cerune_files_nt_open",
        &[imm(0), slot(152), slot(240), imm(1), imm(0), slot(224)],
    );
    mov(o, r64(0), slot(208));
    call(o, "free", &[slot(144)]);
    call(o, "free", &[slot(152)]);
    compare(o, slot(208), 0);
    branch(o, Opcode::JumpNe, "os_win_root_error");
    call(o, "cerune_files_kind", &[slot(216), imm(1)]);
    mov(o, r64(0), slot(208));
    test(o);
    branch(o, Opcode::JumpNe, "os_win_root_kind");
    mov(o, slot(216), r64(0));
    let p = global(o, "root");
    mov(o, r64(0), p);
    let p = global(o, "configured");
    mov(o, imm(1), p);
    label(o, "os_win_root_done");
    finish(o, 264);
    label(o, "os_win_root_kind");
    call(o, "CloseHandle", &[slot(216)]);
    label(o, "os_win_root_error");
    open_failure(o, 208);
    for (l, code) in [("invalid", "invalid-root"), ("alloc", "allocation-failed")] {
        label(o, &format!("os_win_root_{l}"));
        call(o, "free", &[slot(144)]);
        call(o, "free", &[slot(152)]);
        startup(o, "open-root", code);
    }
}
fn open(o: &mut NativeProgram) {
    start(o, "cerune_file_host_open", 728, 4);
    mov(o, imm(0), slot(160));
    ptr(o, 208, 168);
    call(
        o,
        "MultiByteToWideChar",
        &[
            imm(65001),
            imm(8),
            slot(128),
            slot(136),
            slot(168),
            imm(256),
        ],
    );
    test(o);
    branch(o, Opcode::JumpE, "os_win_open_failed");
    mov(o, r64(0), slot(176));
    let p = global(o, "root");
    mov(o, p, r64(0));
    mov(o, r64(0), slot(184));
    ptr(o, 160, 192);
    call(
        o,
        "cerune_files_nt_open",
        &[
            slot(184),
            slot(168),
            slot(176),
            imm(0),
            slot(144),
            slot(192),
        ],
    );
    test(o);
    branch(o, Opcode::JumpNe, "os_win_open_return");
    compare(o, slot(144), 0);
    branch(o, Opcode::JumpNe, "os_win_open_ready");
    call(o, "cerune_files_kind", &[slot(160), imm(0)]);
    test(o);
    branch(o, Opcode::JumpE, "os_win_open_ready");
    mov(o, r64(0), slot(176));
    call(o, "CloseHandle", &[slot(160)]);
    mov(o, slot(176), r64(0));
    branch(o, Opcode::Jump, "os_win_open_return");
    label(o, "os_win_open_ready");
    mov(o, slot(160), r64(0));
    mov(o, slot(152), r64(1));
    mov(o, r64(0), address(1, None, 0));
    mov(o, imm(0), r64(0));
    label(o, "os_win_open_return");
    finish(o, 728);
    label(o, "os_win_open_failed");
    result(o, 728, 4);
}
fn transfer(o: &mut NativeProgram) {
    for (name, os) in [("read", "ReadFile"), ("write", "WriteFile")] {
        start(o, &format!("cerune_file_host_{name}"), 168, 3);
        mov(o, imm(0), slot(152));
        ptr(o, 152, 160);
        call(o, os, &[slot(128), slot(136), slot(144), slot(160), imm(0)]);
        test(o);
        branch(o, Opcode::JumpNe, &format!("os_win_{name}_done"));
        compare(o, slot(152), 0);
        branch(o, Opcode::JumpNe, &format!("os_win_{name}_unknown"));
        if name == "read" {
            call(o, "GetLastError", &[]);
            o.emit(Opcode::Cmp32, [imm(38), r32(0)]);
            branch(o, Opcode::JumpE, "os_win_read_done");
        }
        result(o, 168, -1);
        label(o, &format!("os_win_{name}_unknown"));
        mov(o, slot(144), r64(0));
        o.emit(Opcode::Add64, [imm(1), r64(0)]);
        finish(o, 168);
        label(o, &format!("os_win_{name}_done"));
        mov(o, slot(152), r64(0));
        finish(o, 168);
    }
    start(o, "cerune_file_host_close", 136, 1);
    call(o, "CloseHandle", &[slot(128)]);
    test(o);
    branch(o, Opcode::JumpE, "os_win_close_failed");
    result(o, 136, 0);
    label(o, "os_win_close_failed");
    result(o, 136, -1);
}
