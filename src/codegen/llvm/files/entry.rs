use super::super::Target;
use std::fmt::Write;

pub(in crate::codegen::llvm) fn adapt(entry: &str) -> String {
    entry.replace(
        "  %has.name = icmp sgt i32 %argc, 0\n  %less = sub i32 %argc, 1\n  %n = select i1 %has.name, i32 %less, i32 0\n  %count = zext i32 %n to i64",
        "  %configuration = call { i64, ptr } @cerune.files.parse(i32 %argc, ptr %argv)\n  %start = extractvalue { i64, ptr } %configuration, 0\n  %root = extractvalue { i64, ptr } %configuration, 1\n  %argc.wide = sext i32 %argc to i64\n  %has.values = icmp sgt i64 %argc.wide, %start\n  %remaining = sub i64 %argc.wide, %start\n  %count = select i1 %has.values, i64 %remaining, i64 0")
        .replace("  %source.slot = getelementptr ptr, ptr %argv, i64 %next",
            "  %source.index = add i64 %i, %start\n  %source.slot = getelementptr ptr, ptr %argv, i64 %source.index")
        .replace("  %result = call i32 @cerune.program()",
            "  call void @cerune.files.connect_optional(ptr %root)\n  %result = call i32 @cerune.program()\n  call void @cerune.files.disconnect()")
}
pub(super) fn support(target: Target, out: &mut String) {
    let windows = target == Target::X86_64PcWindowsMsvc;
    for word in [
        "configure",
        "open-root",
        "invalid-root",
        "unknown-setting",
        "duplicate-setting",
        "missing-delimiter",
    ] {
        super::data(out, &format!("@cerune.files.{word}"), word);
    }
    super::data(
        out,
        "@cerune.files.startup.format",
        "cerune: file-v1 operation=connect phase=%s code=%s origin=host\n",
    );
    for (name, value) in [
        ("marker", "--cerune-runtime-v1"),
        ("root.option", "--file-root"),
        ("delimiter", "--"),
    ] {
        if windows {
            write!(
                out,
                "@cerune.files.{name} = private constant [{} x i16] [",
                value.len() + 1
            )
            .unwrap();
            for (i, b) in value.bytes().chain([0]).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write!(out, "i16 {b}").unwrap();
            }
            out.push_str("]\n");
        } else {
            super::data(out, &format!("@cerune.files.{name}"), value);
        }
    }
    let declaration = if windows {
        "declare i32 @wcscmp(ptr, ptr)\n"
    } else {
        "declare i32 @strcmp(ptr, ptr)\n"
    };
    if !out.contains(declaration) {
        out.push_str(declaration);
    }
    out.push_str(FAILURE);
    out.push_str(if windows {
        "  call i32 @_write(i32 2, ptr %text, i32 %count)\n"
    } else {
        "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %text, i64 %wide)\n"
    });
    out.push_str("  br label %exit\nexit:\n  call void @_Exit(i32 1)\n  unreachable\n}\n");
    out.push_str(
        &PARSE
            .replace("@CHAR@", if windows { "i16" } else { "i8" })
            .replace("@CMP@", if windows { "wcscmp" } else { "strcmp" }),
    );
}
const FAILURE: &str = r#"
define internal void @cerune.files.startup_failed(ptr %phase, ptr %code) noreturn {
entry:
  %text = alloca [256 x i8]
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 256, ptr @cerune.files.startup.format, ptr %phase, ptr %code)
  %positive = icmp sgt i32 %length, 0
  br i1 %positive, label %write, label %exit
write:
  %fits = icmp ult i32 %length, 256
  %count = select i1 %fits, i32 %length, i32 255
"#;
const PARSE: &str = r#"
; runtime-startup-v1: only the leading marker introduces configuration.
define internal { i64, ptr } @cerune.files.parse(i32 %argc, ptr %argv) {
entry:
  %index.slot = alloca i64
  %root.slot = alloca ptr
  store ptr null, ptr %root.slot
  store i64 2, ptr %index.slot
  %argc.wide = sext i32 %argc to i64
  %has.argument = icmp sgt i32 %argc, 1
  br i1 %has.argument, label %marker, label %unchanged
marker:
  %first.slot = getelementptr ptr, ptr %argv, i64 1
  %first = load ptr, ptr %first.slot
  %comparison = call i32 @@CMP@(ptr %first, ptr @cerune.files.marker)
  %is.marker = icmp eq i32 %comparison, 0
  br i1 %is.marker, label %loop, label %unchanged
unchanged:
  ret { i64, ptr } { i64 1, ptr null }
loop:
  %index = load i64, ptr %index.slot
  %end = icmp sge i64 %index, %argc.wide
  br i1 %end, label %missing.delimiter, label %argument
argument:
  %slot = getelementptr ptr, ptr %argv, i64 %index
  %arg = load ptr, ptr %slot
  %delim.cmp = call i32 @@CMP@(ptr %arg, ptr @cerune.files.delimiter)
  %is.delimiter = icmp eq i32 %delim.cmp, 0
  br i1 %is.delimiter, label %done, label %option
option:
  %option.cmp = call i32 @@CMP@(ptr %arg, ptr @cerune.files.root.option)
  %recognized = icmp eq i32 %option.cmp, 0
  br i1 %recognized, label %duplicate.check, label %unknown
duplicate.check:
  %previous = load ptr, ptr %root.slot
  %duplicate = icmp ne ptr %previous, null
  br i1 %duplicate, label %duplicate.setting, label %value.check
value.check:
  %value.index = add i64 %index, 1
  %missing = icmp sge i64 %value.index, %argc.wide
  br i1 %missing, label %invalid.root, label %value
value:
  %value.slot = getelementptr ptr, ptr %argv, i64 %value.index
  %root = load ptr, ptr %value.slot
  %first.char = load @CHAR@, ptr %root
  %empty = icmp eq @CHAR@ %first.char, 0
  br i1 %empty, label %invalid.root, label %prefix
prefix:
  %second.ptr = getelementptr @CHAR@, ptr %root, i64 1
  %second.char = load @CHAR@, ptr %second.ptr
  %first.dash = icmp eq @CHAR@ %first.char, 45
  %second.dash = icmp eq @CHAR@ %second.char, 45
  %looks.option = and i1 %first.dash, %second.dash
  br i1 %looks.option, label %invalid.root, label %accept
accept:
  store ptr %root, ptr %root.slot
  %next = add i64 %value.index, 1
  store i64 %next, ptr %index.slot
  br label %loop
done:
  %start = add i64 %index, 1
  %selected.root = load ptr, ptr %root.slot
  %with.start = insertvalue { i64, ptr } poison, i64 %start, 0
  %result = insertvalue { i64, ptr } %with.start, ptr %selected.root, 1
  ret { i64, ptr } %result
missing.delimiter:
  call void @cerune.files.startup_failed(ptr @cerune.files.configure, ptr @cerune.files.missing-delimiter)
  unreachable
unknown:
  call void @cerune.files.startup_failed(ptr @cerune.files.configure, ptr @cerune.files.unknown-setting)
  unreachable
duplicate.setting:
  call void @cerune.files.startup_failed(ptr @cerune.files.configure, ptr @cerune.files.duplicate-setting)
  unreachable
invalid.root:
  call void @cerune.files.startup_failed(ptr @cerune.files.configure, ptr @cerune.files.invalid-root)
  unreachable
}
"#;
