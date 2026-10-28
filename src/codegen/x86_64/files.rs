//! ファイルの意味処理を型付き機械命令で構築します。接続は現在、試験時だけ供給します。
use super::ir::{Instruction, Module};
use super::machine::{EntryKind, Linkage, Opcode, Operand, Section, r32, r64};
use super::program::{NativeProgram, address};
mod names;
#[cfg(test)]
mod tests;

pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| {
            matches!(
                i,
                Instruction::FileReadText { .. } | Instruction::FileWriteNew { .. }
            )
        })
}
// 入口: rax=上限、r10=名前、r9=本文、r11=出自。戻り値: rax=文字列またはunit。
// 呼出し前RSPを16byte整列し、先頭128byteを両ABIの外部引数に使います。
const FRAME: i128 = 840;
const NAME: i128 = 128;
const TEXT: i128 = 136;
const LIMIT: i128 = 144;
const ORIGIN: i128 = 152;
const MODE: i128 = 160;
const OWNER: i128 = 168;
const HANDLE: i128 = 176;
const TRANSFER: i128 = 184;
const REQUEST: i128 = 192;
const LENGTH: i128 = 200;
const PHASE: i128 = 208;
const CODE: i128 = 216;
const KNOWN: i128 = 224;
const CREATED: i128 = 232;
const CLEANUP: i128 = 240;
const REQTEXT: i128 = 248;
const DECIMAL: i128 = 256;
const PROBE: i128 = 288;
const POINTER: i128 = 296;
const OPERATION: i128 = 304;
const BUFFER: i128 = 320;
fn slot(n: i128) -> Operand {
    address(4, None, n)
}
fn imm(n: i128) -> Operand {
    Operand::Immediate(n)
}
fn mov(o: &mut NativeProgram, from: Operand, to: Operand) {
    o.emit(Opcode::Mov64, [from, to]);
}
fn branch(o: &mut NativeProgram, op: Opcode, label: &str) {
    let s = o.symbol(
        &format!(".Lfile_{}", label.replace('-', "_").as_str()),
        Linkage::Local,
    );
    o.emit(op, [Operand::Symbol(s)]);
}
fn label(o: &mut NativeProgram, s: &str) {
    o.label(&format!(".Lfile_{}", s.replace('-', "_").as_str()));
}
fn literal(o: &mut NativeProgram, s: &str) -> Operand {
    let s = o.symbol(
        &format!(".Lfile_text_{}", s.replace('-', "_")),
        Linkage::Local,
    );
    Operand::RipRelative(s)
}
fn pointer(o: &mut NativeProgram, s: &str, to: i128) {
    let p = literal(o, s);
    o.emit(Opcode::Lea64, [p, r64(0)]);
    mov(o, r64(0), slot(to));
}
fn local(o: &mut NativeProgram, s: &str) -> Operand {
    Operand::RipRelative(o.symbol(s, Linkage::Local))
}
fn compare(o: &mut NativeProgram, value: Operand, with: i128) {
    o.emit(Opcode::Cmp64, [imm(with), value]);
}
fn call(o: &mut NativeProgram, name: &str, args: &[Operand]) {
    let linux = o.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu;
    let regs: &[u8] = if linux {
        &[7, 6, 2, 1, 8, 9]
    } else {
        &[1, 2, 8, 9]
    };
    // 引数はframe上または即値。レジスタの並べ替えによる上書きを避けます。
    for (i, arg) in args.iter().enumerate() {
        if i < regs.len() {
            mov(o, arg.clone(), r64(regs[i]));
        } else {
            mov(o, arg.clone(), r64(0));
            mov(
                o,
                r64(0),
                slot((if linux { 0 } else { 32 }) + 8 * (i - regs.len()) as i128),
            );
        }
    }
    if linux {
        o.emit(Opcode::Xor32, [r32(0), r32(0)]);
    }
    let symbol = o.symbol(name, Linkage::External);
    o.emit(Opcode::Call, [Operand::Symbol(symbol)]);
}
fn fail(o: &mut NativeProgram, code: &str, to: &str) {
    pointer(o, code, CODE);
    branch(o, Opcode::Jump, to);
}
pub(super) fn append(o: &mut NativeProgram) {
    o.push(EntryKind::Section(Section::ReadOnlyData));
    for s in [
        "invalid-limit",
        "invalid-name",
        "unconfigured",
        "budget-exceeded",
        "allocation-failed",
        "not-found",
        "already-exists",
        "unsupported-target",
        "open-failed",
        "read-failed",
        "write-failed",
        "close-failed",
        "zero-progress",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
        "known",
        "unknown",
        "no",
        "yes",
        "not-attempted",
        "closed",
        "failed",
        "validate",
        "reserve",
        "open",
        "read",
        "write",
        "close",
        "decode",
        "read-text",
        "write-new",
    ] {
        o.label(&format!(".Lfile_text_{}", s.replace('-', "_")));
        o.push(EntryKind::CString(s.into()));
    }
    o.label(".Lfile_text_format");
    o.push(EntryKind::CString("cerune: file-v1 operation=%s phase=%s code=%s limit=%lld requested=%s transferred=%llu progress=%s created=%s cleanup=%s%s".into()));
    o.label(".Lfile_text_decimal");
    o.push(EntryKind::CString("%llu".into()));
    o.push(EntryKind::Section(Section::Text));
    names::append(o);
    for write in [false, true] {
        o.label(if write {
            "cerune_file_write_new"
        } else {
            "cerune_file_read_text"
        });
        o.emit(Opcode::Sub64, [imm(FRAME), r64(4)]);
        mov(o, r64(0), slot(LIMIT));
        mov(o, r64(10), slot(NAME));
        mov(o, r64(11), slot(ORIGIN));
        mov(o, imm(i128::from(write)), slot(MODE));
        if write {
            mov(o, r64(9), slot(TEXT));
            mov(o, address(9, None, 0), r64(0));
            mov(o, r64(0), slot(LENGTH));
        } else {
            mov(o, imm(-1), slot(LENGTH));
        }
        pointer(o, if write { "write-new" } else { "read-text" }, OPERATION);
        branch(o, Opcode::Jump, "begin");
    }
    label(o, "begin");
    for n in [OWNER, CODE, TRANSFER] {
        mov(o, imm(0), slot(n));
    }
    pointer(o, "validate", PHASE);
    pointer(o, "known", KNOWN);
    pointer(o, "no", CREATED);
    pointer(o, "not-attempted", CLEANUP);
    compare(o, slot(LIMIT), 2147483647);
    branch(o, Opcode::JumpA, "bad_limit");
    mov(o, slot(NAME), r64(10));
    let name = o.symbol("cerune_file_valid_name", Linkage::Local);
    o.emit(Opcode::Call, [Operand::Symbol(name)]);
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpE, "bad_name");
    call(o, "cerune_file_host_configured", &[]);
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpE, "unconfigured");
    compare(o, slot(MODE), 0);
    branch(o, Opcode::JumpNe, "write_validate");
    pointer(o, "reserve", PHASE);
    let heap = local(o, ".Lcerune_heap_limit");
    mov(o, heap, r64(0));
    let live = local(o, ".Lcerune_heap_live");
    o.emit(Opcode::Sub64, [live, r64(0)]);
    o.emit(Opcode::Cmp64, [slot(LIMIT), r64(0)]);
    branch(o, Opcode::JumpB, "budget");
    mov(o, slot(LIMIT), r64(0));
    o.emit(Opcode::Add64, [imm(24), r64(0)]);
    mov(o, r64(0), slot(REQUEST));
    call(o, "malloc", &[slot(REQUEST)]);
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpE, "allocation");
    mov(o, r64(0), slot(OWNER));
    mov(o, slot(LIMIT), r64(0));
    let live = local(o, ".Lcerune_heap_live");
    o.emit(Opcode::Add64, [r64(0), live]);
    branch(o, Opcode::Jump, "open");
    label(o, "write_validate");
    mov(o, slot(LENGTH), r64(0));
    o.emit(Opcode::Cmp64, [slot(LIMIT), r64(0)]);
    branch(o, Opcode::JumpA, "limit_before_open");
    label(o, "open");
    pointer(o, "open", PHASE);
    mov(o, slot(NAME), r64(0));
    mov(o, address(0, None, 0), r64(1));
    mov(o, r64(1), slot(REQUEST));
    o.emit(Opcode::Add64, [imm(8), r64(0)]);
    mov(o, r64(0), slot(POINTER));
    o.emit(Opcode::Lea64, [slot(HANDLE), r64(0)]);
    mov(o, r64(0), slot(REQTEXT));
    call(
        o,
        "cerune_file_host_open",
        &[slot(POINTER), slot(REQUEST), slot(MODE), slot(REQTEXT)],
    );
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpNe, "open_error");
    compare(o, slot(MODE), 0);
    branch(o, Opcode::JumpE, "read_begin");
    pointer(o, "yes", CREATED);
    pointer(o, "write", PHASE);
    branch(o, Opcode::Jump, "write_loop");
    label(o, "read_begin");
    pointer(o, "read", PHASE);
    label(o, "read_loop");
    mov(o, slot(LIMIT), r64(0));
    o.emit(Opcode::Sub64, [slot(TRANSFER), r64(0)]);
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpE, "probe");
    compare(o, r64(0), 4096);
    branch(o, Opcode::JumpBe, "read_size");
    mov(o, imm(4096), r64(0));
    label(o, "read_size");
    mov(o, r64(0), slot(REQUEST));
    mov(o, slot(OWNER), r64(0));
    o.emit(Opcode::Add64, [imm(24), r64(0)]);
    o.emit(Opcode::Add64, [slot(TRANSFER), r64(0)]);
    mov(o, r64(0), slot(POINTER));
    branch(o, Opcode::Jump, "read_call");
    label(o, "probe");
    mov(o, imm(1), slot(REQUEST));
    o.emit(Opcode::Lea64, [slot(PROBE), r64(0)]);
    mov(o, r64(0), slot(POINTER));
    label(o, "read_call");
    call(
        o,
        "cerune_file_host_read",
        &[slot(HANDLE), slot(POINTER), slot(REQUEST)],
    );
    compare(o, r64(0), -2);
    branch(o, Opcode::JumpE, "read_loop");
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpS, "read_error");
    o.emit(Opcode::Cmp64, [slot(REQUEST), r64(0)]);
    branch(o, Opcode::JumpA, "invalid_transfer");
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpE, "eof");
    o.emit(Opcode::Add64, [r64(0), slot(TRANSFER)]);
    mov(o, slot(TRANSFER), r64(0));
    o.emit(Opcode::Cmp64, [slot(LIMIT), r64(0)]);
    branch(o, Opcode::JumpA, "read_limit");
    branch(o, Opcode::Jump, "read_loop");
    label(o, "eof");
    mov(o, slot(TRANSFER), r64(0));
    mov(o, r64(0), slot(LENGTH));
    branch(o, Opcode::Jump, "close");
    label(o, "write_loop");
    mov(o, slot(LENGTH), r64(0));
    o.emit(Opcode::Sub64, [slot(TRANSFER), r64(0)]);
    branch(o, Opcode::JumpE, "close");
    mov(o, r64(0), slot(REQUEST));
    mov(o, slot(TEXT), r64(0));
    o.emit(Opcode::Add64, [imm(8), r64(0)]);
    o.emit(Opcode::Add64, [slot(TRANSFER), r64(0)]);
    mov(o, r64(0), slot(POINTER));
    call(
        o,
        "cerune_file_host_write",
        &[slot(HANDLE), slot(POINTER), slot(REQUEST)],
    );
    compare(o, r64(0), -2);
    branch(o, Opcode::JumpE, "write_loop");
    o.emit(Opcode::Test64, [r64(0), r64(0)]);
    branch(o, Opcode::JumpS, "write_error");
    branch(o, Opcode::JumpE, "zero");
    o.emit(Opcode::Cmp64, [slot(REQUEST), r64(0)]);
    branch(o, Opcode::JumpA, "invalid_transfer");
    o.emit(Opcode::Add64, [r64(0), slot(TRANSFER)]);
    branch(o, Opcode::Jump, "write_loop");
    label(o, "close");
    call(o, "cerune_file_host_close", &[slot(HANDLE)]);
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpNe, "close_error");
    pointer(o, "closed", CLEANUP);
    branch(o, Opcode::Jump, "closed");
    label(o, "close_error");
    pointer(o, "failed", CLEANUP);
    compare(o, slot(CODE), 0);
    branch(o, Opcode::JumpNe, "failure");
    pointer(o, "close", PHASE);
    fail(o, "close-failed", "failure");
    label(o, "closed");
    compare(o, slot(CODE), 0);
    branch(o, Opcode::JumpNe, "failure");
    compare(o, slot(MODE), 0);
    branch(o, Opcode::JumpNe, "write_done");
    pointer(o, "decode", PHASE);
    mov(o, slot(OWNER), r64(2));
    o.emit(Opcode::Add64, [imm(24), r64(2)]);
    mov(o, slot(TRANSFER), r64(1));
    let utf = o.symbol("cerune_utf8", Linkage::Local);
    o.emit(Opcode::Call, [Operand::Symbol(utf)]);
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpE, "utf8");
    mov(o, slot(LIMIT), r64(0));
    let live = local(o, ".Lcerune_heap_live");
    o.emit(Opcode::Sub64, [r64(0), live]);
    compare(o, slot(TRANSFER), 0);
    branch(o, Opcode::JumpE, "empty");
    mov(o, slot(OWNER), r64(0));
    let head = local(o, ".Lcerune_heap_head");
    mov(o, head, r64(1));
    mov(o, r64(1), address(0, None, 0));
    mov(o, imm(1), address(0, None, 8));
    mov(o, slot(TRANSFER), r64(1));
    mov(o, r64(1), address(0, None, 16));
    let live = local(o, ".Lcerune_heap_live");
    o.emit(Opcode::Add64, [r64(1), live]);
    let head = local(o, ".Lcerune_heap_head");
    mov(o, r64(0), head);
    o.emit(Opcode::Add64, [imm(16), r64(0)]);
    branch(o, Opcode::Jump, "return");
    label(o, "empty");
    call(o, "free", &[slot(OWNER)]);
    let empty = local(o, ".Lcerune_heap_empty");
    o.emit(Opcode::Lea64, [empty, r64(0)]);
    branch(o, Opcode::Jump, "return");
    label(o, "write_done");
    mov(o, imm(0), r64(0));
    label(o, "return");
    o.emit(Opcode::Add64, [imm(FRAME), r64(4)]);
    o.emit(Opcode::Ret, []);
    label(o, "open_error");
    for (n, code) in [
        (1, "not-found"),
        (2, "already-exists"),
        (3, "unsupported-target"),
        (4, "open-failed"),
    ] {
        o.emit(Opcode::Cmp32, [imm(n), r32(0)]);
        branch(o, Opcode::JumpE, code);
    }
    compare(o, slot(MODE), 0);
    branch(o, Opcode::JumpE, "open_invalid");
    pointer(o, "unknown", CREATED);
    label(o, "open_invalid");
    fail(o, "invalid-response", "failure");
    label(o, "invalid_transfer");
    pointer(o, "unknown", KNOWN);
    fail(o, "invalid-response", "close");
    for (label_name, code, dest) in [
        ("bad_limit", "invalid-limit", "failure"),
        ("bad_name", "invalid-name", "failure"),
        ("unconfigured", "unconfigured", "failure"),
        ("budget", "budget-exceeded", "failure"),
        ("allocation", "allocation-failed", "failure"),
        ("limit_before_open", "limit-exceeded", "failure"),
        ("read_limit", "limit-exceeded", "close"),
        ("read_error", "read-failed", "close"),
        ("write_error", "write-failed", "close"),
        ("zero", "zero-progress", "close"),
        ("utf8", "invalid-utf8", "failure"),
        ("not-found", "not-found", "failure"),
        ("already-exists", "already-exists", "failure"),
        ("unsupported-target", "unsupported-target", "failure"),
        ("open-failed", "open-failed", "failure"),
    ] {
        label(o, label_name);
        fail(o, code, dest);
    }
    diagnostic(o);
}
fn diagnostic(o: &mut NativeProgram) {
    label(o, "failure");
    compare(o, slot(OWNER), 0);
    branch(o, Opcode::JumpE, "format");
    mov(o, slot(LIMIT), r64(0));
    let live = local(o, ".Lcerune_heap_live");
    o.emit(Opcode::Sub64, [r64(0), live]);
    call(o, "free", &[slot(OWNER)]);
    label(o, "format");
    call(o, "fflush", &[imm(0)]);
    pointer(o, "unknown", REQTEXT);
    compare(o, slot(LENGTH), -1);
    branch(o, Opcode::JumpE, "format_record");
    o.emit(Opcode::Lea64, [slot(DECIMAL), r64(0)]);
    mov(o, r64(0), slot(REQTEXT));
    pointer(o, "decimal", POINTER);
    call(
        o,
        "snprintf",
        &[slot(REQTEXT), imm(32), slot(POINTER), slot(LENGTH)],
    );
    label(o, "format_record");
    o.emit(Opcode::Lea64, [slot(BUFFER), r64(0)]);
    mov(o, r64(0), slot(POINTER));
    pointer(o, "format", REQUEST);
    call(
        o,
        "snprintf",
        &[
            slot(POINTER),
            imm(512),
            slot(REQUEST),
            slot(OPERATION),
            slot(PHASE),
            slot(CODE),
            slot(LIMIT),
            slot(REQTEXT),
            slot(TRANSFER),
            slot(KNOWN),
            slot(CREATED),
            slot(CLEANUP),
            slot(ORIGIN),
        ],
    );
    o.emit(Opcode::Test32, [r32(0), r32(0)]);
    branch(o, Opcode::JumpLe, "exit");
    o.emit(Opcode::Cmp32, [imm(512), r32(0)]);
    branch(o, Opcode::JumpB, "diagnostic");
    mov(o, imm(511), r64(0));
    label(o, "diagnostic");
    mov(o, r64(0), slot(REQUEST));
    let name = if o.program.target == sashimi_assembler::Target::X86_64UnknownLinuxGnu {
        "write"
    } else {
        "_write"
    };
    call(o, name, &[imm(2), slot(POINTER), slot(REQUEST)]);
    label(o, "exit");
    call(o, "_Exit", &[imm(1)]);
    o.emit(Opcode::Undefined, []);
}
