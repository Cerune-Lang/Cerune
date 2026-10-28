use std::fmt::Write;
pub(super) const DECLARATIONS: &str = r#"
declare ptr @GetModuleHandleW(ptr)
declare ptr @GetProcAddress(ptr, ptr)
declare i32 @GetFullPathNameW(ptr, i32, ptr, ptr)
declare i32 @GetFileType(ptr)
declare i32 @GetFileInformationByHandleEx(ptr, i32, ptr, i32)
declare i32 @CloseHandle(ptr)
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @ReadFile(ptr, ptr, i32, ptr, ptr)
declare i32 @WriteFile(ptr, ptr, i32, ptr, ptr)
declare i32 @GetLastError()
declare i32 @wcsncmp(ptr, ptr, i64)
"#;
pub(super) fn emit(out: &mut String) {
    for (name, text) in [
        ("ntdll", "ntdll.dll"),
        ("nt.prefix", "\\??\\"),
        ("unc.prefix", "UNC\\"),
        ("extended", "\\\\?\\"),
        ("device", "\\\\.\\"),
    ] {
        write!(
            out,
            "@cerune.files.{name} = private constant [{} x i16] [",
            text.len() + 1
        )
        .unwrap();
        for (i, b) in text.bytes().chain([0]).enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "i16 {b}").unwrap();
        }
        out.push_str("]\n");
    }
    super::super::data(out, "@cerune.files.nt_create", "NtCreateFile");
    let mut source = SOURCE.to_owned();
    for (name, code) in [
        ("NAME_NOT_FOUND", 0xc0000034u32),
        ("PATH_NOT_FOUND", 0xc000003a),
        ("NAME_COLLISION", 0xc0000035),
        ("IS_DIRECTORY", 0xc00000ba),
        ("NOT_DIRECTORY", 0xc0000103),
        ("REPARSE_NOT_RESOLVED", 0xc0000279),
        ("REPARSE_ENCOUNTERED", 0xc000050b),
    ] {
        source = source.replace(&format!("@{name}@"), &code.to_string());
    }
    out.push_str(&source);
}
// Win64: UNICODE_STRING=16、OBJECT_ATTRIBUTES=48、IO_STATUS_BLOCK=16バイト。
// NT関数はntdllから明示取得し、追加のimport libraryを要求しません。
const SOURCE: &str = r#"
%cerune.files.nt_string = type { i16, i16, ptr }
%cerune.files.nt_attributes = type { i32, ptr, ptr, i32, ptr, ptr }
define internal i32 @cerune.files.nt_open(ptr %root, ptr %name, i64 %length, i1 %directory, i1 %create, ptr %handle) {
entry:
  %text = alloca %cerune.files.nt_string
  %attributes = alloca %cerune.files.nt_attributes
  %status = alloca [2 x i64]
  %long = icmp ugt i64 %length, 32767
  br i1 %long, label %failed, label %prepare
prepare:
  store %cerune.files.nt_string zeroinitializer, ptr %text
  store %cerune.files.nt_attributes zeroinitializer, ptr %attributes
  store [2 x i64] zeroinitializer, ptr %status
  %bytes = mul i64 %length, 2
  %short = trunc i64 %bytes to i16
  store i16 %short, ptr %text
  %maximum = getelementptr %cerune.files.nt_string, ptr %text, i32 0, i32 1
  store i16 %short, ptr %maximum
  %buffer = getelementptr %cerune.files.nt_string, ptr %text, i32 0, i32 2
  store ptr %name, ptr %buffer
  store i32 48, ptr %attributes
  %root.p = getelementptr %cerune.files.nt_attributes, ptr %attributes, i32 0, i32 1
  store ptr %root, ptr %root.p
  %name.p = getelementptr %cerune.files.nt_attributes, ptr %attributes, i32 0, i32 2
  store ptr %text, ptr %name.p
  %flags.p = getelementptr %cerune.files.nt_attributes, ptr %attributes, i32 0, i32 3
  store i32 64, ptr %flags.p
  %module = call ptr @GetModuleHandleW(ptr @cerune.files.ntdll)
  %open_file = call ptr @GetProcAddress(ptr %module, ptr @cerune.files.nt_create)
  %absent = icmp eq ptr %open_file, null
  br i1 %absent, label %failed, label %open
open:
  %data.access = select i1 %create, i32 1048706, i32 1048705
  %access = select i1 %directory, i32 1048736, i32 %data.access
  %options = select i1 %directory, i32 2097185, i32 2097248
  %disposition = select i1 %create, i32 2, i32 1
  %result = call i32 %open_file(ptr %handle, i32 %access, ptr %attributes, ptr %status, ptr null, i32 0, i32 7, i32 %disposition, i32 %options, ptr null, i32 0)
  %negative = icmp slt i32 %result, 0
  br i1 %negative, label %error, label %check
error:
  switch i32 %result, label %failed [
    i32 @NAME_NOT_FOUND@, label %missing
    i32 @PATH_NOT_FOUND@, label %missing
    i32 @NAME_COLLISION@, label %exists
    i32 @IS_DIRECTORY@, label %is.directory
    i32 @NOT_DIRECTORY@, label %unsupported
    i32 @REPARSE_NOT_RESOLVED@, label %unsupported
    i32 @REPARSE_ENCOUNTERED@, label %unsupported
  ]
is.directory:
  %kind.code = select i1 %create, i32 2, i32 3
  ret i32 %kind.code
check:
  %h = load ptr, ptr %handle
  %null = icmp eq ptr %h, null
  %invalid = icmp eq ptr %h, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  %code = select i1 %bad, i32 5, i32 0
  ret i32 %code
missing:
  ret i32 1
exists:
  ret i32 2
unsupported:
  ret i32 3
failed:
  ret i32 4
}
define internal i32 @cerune.files.kind(ptr %handle, i1 %directory) {
entry:
  %info = alloca [2 x i32]
  %kind = call i32 @GetFileType(ptr %handle)
  %disk = icmp eq i32 %kind, 1
  br i1 %disk, label %query, label %unsupported
query:
  %result = call i32 @GetFileInformationByHandleEx(ptr %handle, i32 9, ptr %info, i32 8)
  %ok = icmp ne i32 %result, 0
  br i1 %ok, label %check, label %failed
check:
  %attributes = load i32, ptr %info
  %reparse.bits = and i32 %attributes, 1024
  %reparse = icmp ne i32 %reparse.bits, 0
  %directory.bits = and i32 %attributes, 16
  %is.directory = icmp ne i32 %directory.bits, 0
  %wrong.kind = icmp ne i1 %directory, %is.directory
  %unsupported.kind = or i1 %reparse, %wrong.kind
  %code = select i1 %unsupported.kind, i32 3, i32 0
  ret i32 %code
unsupported:
  ret i32 3
failed:
  ret i32 4
}
define internal void @cerune.files.connect(ptr %root) {
entry:
  %offset.slot = alloca i64
  %prefix.slot = alloca i64
  %handle.slot = alloca ptr
  store ptr null, ptr %handle.slot
  store i64 0, ptr %offset.slot
  store i64 4, ptr %prefix.slot
  %length = call i32 @GetFullPathNameW(ptr %root, i32 0, ptr null, ptr null)
  %zero = icmp eq i32 %length, 0
  %long = icmp ugt i32 %length, 32760
  %bad = or i1 %zero, %long
  br i1 %bad, label %invalid.root, label %allocate
allocate:
  %capacity = add i32 %length, 1
  %wide = zext i32 %length to i64
  %absolute.units = add i64 %wide, 1
  %name.units = add i64 %wide, 9
  %absolute.size = mul i64 %absolute.units, 2
  %name.size = mul i64 %name.units, 2
  %absolute = call ptr @malloc(i64 %absolute.size)
  %name = call ptr @malloc(i64 %name.size)
  %no.absolute = icmp eq ptr %absolute, null
  %no.name = icmp eq ptr %name, null
  %allocation.failed = or i1 %no.absolute, %no.name
  br i1 %allocation.failed, label %allocation.failure, label %resolve
resolve:
  %actual = call i32 @GetFullPathNameW(ptr %root, i32 %capacity, ptr %absolute, ptr null)
  %empty = icmp eq i32 %actual, 0
  %too.long = icmp ugt i32 %actual, %length
  %invalid = or i1 %empty, %too.long
  br i1 %invalid, label %invalid.root, label %prefix
prefix:
  call ptr @memcpy(ptr %name, ptr @cerune.files.nt.prefix, i64 8)
  %actual.wide = zext i32 %actual to i64
  %four = icmp uge i32 %actual, 4
  br i1 %four, label %extended, label %unc.check
extended:
  %extended.cmp = call i32 @wcsncmp(ptr %absolute, ptr @cerune.files.extended, i64 4)
  %is.extended = icmp eq i32 %extended.cmp, 0
  br i1 %is.extended, label %strip.extended, label %device
strip.extended:
  store i64 4, ptr %offset.slot
  br label %validate
device:
  %device.cmp = call i32 @wcsncmp(ptr %absolute, ptr @cerune.files.device, i64 4)
  %is.device = icmp eq i32 %device.cmp, 0
  br i1 %is.device, label %invalid.root, label %unc.check
unc.check:
  %two = icmp uge i32 %actual, 2
  br i1 %two, label %unc.prefix, label %validate
unc.prefix:
  %first = load i16, ptr %absolute
  %second.p = getelementptr i16, ptr %absolute, i64 1
  %second = load i16, ptr %second.p
  %first.slash = icmp eq i16 %first, 92
  %second.slash = icmp eq i16 %second, 92
  %unc = and i1 %first.slash, %second.slash
  br i1 %unc, label %add.unc, label %validate
add.unc:
  %after.prefix = getelementptr i16, ptr %name, i64 4
  call ptr @memcpy(ptr %after.prefix, ptr @cerune.files.unc.prefix, i64 8)
  store i64 8, ptr %prefix.slot
  store i64 2, ptr %offset.slot
  br label %validate
validate:
  %offset = load i64, ptr %offset.slot
  %prefix.length = load i64, ptr %prefix.slot
  %remaining = sub i64 %actual.wide, %offset
  %suffix = getelementptr i16, ptr %absolute, i64 %offset
  %prefix.unc = icmp eq i64 %prefix.length, 8
  br i1 %prefix.unc, label %copy, label %drive.check
drive.check:
  %three = icmp uge i64 %remaining, 3
  br i1 %three, label %drive, label %invalid.root
drive:
  %colon.p = getelementptr i16, ptr %suffix, i64 1
  %slash.p = getelementptr i16, ptr %suffix, i64 2
  %colon = load i16, ptr %colon.p
  %slash = load i16, ptr %slash.p
  %is.colon = icmp eq i16 %colon, 58
  %is.slash = icmp eq i16 %slash, 92
  %is.drive = and i1 %is.colon, %is.slash
  br i1 %is.drive, label %copy, label %extended.unc.check
extended.unc.check:
  %unc.long = icmp uge i64 %remaining, 4
  br i1 %unc.long, label %extended.unc, label %invalid.root
extended.unc:
  %unc.cmp = call i32 @wcsncmp(ptr %suffix, ptr @cerune.files.unc.prefix, i64 4)
  %unc.ok = icmp eq i32 %unc.cmp, 0
  br i1 %unc.ok, label %copy, label %invalid.root
copy:
  %destination = getelementptr i16, ptr %name, i64 %prefix.length
  %copy.bytes = mul i64 %remaining, 2
  call ptr @memcpy(ptr %destination, ptr %suffix, i64 %copy.bytes)
  %name.length = add i64 %prefix.length, %remaining
  br label %trim
trim:
  %n = phi i64 [ %name.length, %copy ], [ %previous, %trim.more ]
  %minimum = icmp ule i64 %n, 7
  br i1 %minimum, label %open, label %last
last:
  %previous = sub i64 %n, 1
  %last.p = getelementptr i16, ptr %name, i64 %previous
  %last.char = load i16, ptr %last.p
  %trailing = icmp eq i16 %last.char, 92
  br i1 %trailing, label %trim.more, label %open
trim.more:
  br label %trim
open:
  %code = call i32 @cerune.files.nt_open(ptr null, ptr %name, i64 %n, i1 true, i1 false, ptr %handle.slot)
  call void @free(ptr %name)
  call void @free(ptr %absolute)
  %opened = icmp eq i32 %code, 0
  br i1 %opened, label %kind, label %open.failed
kind:
  %handle = load ptr, ptr %handle.slot
  %kind.code = call i32 @cerune.files.kind(ptr %handle, i1 true)
  %valid.kind = icmp eq i32 %kind.code, 0
  br i1 %valid.kind, label %connected, label %kind.failed
kind.failed:
  call i32 @CloseHandle(ptr %handle)
  br label %failure
open.failed:
  br label %failure
failure:
  %error = phi i32 [ %code, %open.failed ], [ %kind.code, %kind.failed ]
  %error.text = call ptr @cerune.files.open_code(i32 %error)
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr %error.text)
  unreachable
connected:
  %raw = ptrtoint ptr %handle to i64
  store i64 %raw, ptr @cerune.files.root
  store i1 true, ptr @cerune.files.configured
  ret void
invalid.root:
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr @cerune.files.invalid-root)
  unreachable
allocation.failure:
  call void @cerune.files.startup_failed(ptr @cerune.files.open-root, ptr @cerune.files.allocation-failed)
  unreachable
}
define internal i32 @cerune_file_host_open(ptr %name, i64 %length, i32 %writing, ptr %result) {
entry:
  %wide = alloca [256 x i16]
  %handle.slot = alloca ptr
  store ptr null, ptr %handle.slot
  %narrow = trunc i64 %length to i32
  %converted = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %name, i32 %narrow, ptr %wide, i32 256)
  %failed = icmp eq i32 %converted, 0
  br i1 %failed, label %conversion.failed, label %open
conversion.failed:
  ret i32 4
open:
  %root.raw = load i64, ptr @cerune.files.root
  %root = inttoptr i64 %root.raw to ptr
  %count = zext i32 %converted to i64
  %create = icmp ne i32 %writing, 0
  %code = call i32 @cerune.files.nt_open(ptr %root, ptr %wide, i64 %count, i1 false, i1 %create, ptr %handle.slot)
  %opened = icmp eq i32 %code, 0
  br i1 %opened, label %opened.handle, label %open.failed
open.failed:
  ret i32 %code
opened.handle:
  %handle = load ptr, ptr %handle.slot
  br i1 %create, label %ready, label %kind
kind:
  %kind.code = call i32 @cerune.files.kind(ptr %handle, i1 false)
  %valid = icmp eq i32 %kind.code, 0
  br i1 %valid, label %ready, label %kind.failed
kind.failed:
  call i32 @CloseHandle(ptr %handle)
  ret i32 %kind.code
ready:
  %raw = ptrtoint ptr %handle to i64
  store i64 %raw, ptr %result
  ret i32 0
}
define internal i64 @cerune_file_host_read(i64 %raw, ptr %data, i64 %length) {
entry:
  %count.slot = alloca i32
  store i32 0, ptr %count.slot
  %handle = inttoptr i64 %raw to ptr
  %request = trunc i64 %length to i32
  %status = call i32 @ReadFile(ptr %handle, ptr %data, i32 %request, ptr %count.slot, ptr null)
  %count = load i32, ptr %count.slot
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %done, label %check.progress
done:
  %wide = zext i32 %count to i64
  ret i64 %wide
check.progress:
  %some = icmp ne i32 %count, 0
  br i1 %some, label %invalid, label %error
invalid:
  %unknown = add i64 %length, 1
  ret i64 %unknown
error:
  %code = call i32 @GetLastError()
  %eof = icmp eq i32 %code, 38
  %result = select i1 %eof, i64 0, i64 -1
  ret i64 %result
}
define internal i64 @cerune_file_host_write(i64 %raw, ptr %data, i64 %length) {
entry:
  %count.slot = alloca i32
  store i32 0, ptr %count.slot
  %handle = inttoptr i64 %raw to ptr
  %request = trunc i64 %length to i32
  %status = call i32 @WriteFile(ptr %handle, ptr %data, i32 %request, ptr %count.slot, ptr null)
  %count = load i32, ptr %count.slot
  %ok = icmp ne i32 %status, 0
  br i1 %ok, label %done, label %error
done:
  %wide = zext i32 %count to i64
  ret i64 %wide
error:
  %some = icmp ne i32 %count, 0
  %unknown = add i64 %length, 1
  %result = select i1 %some, i64 %unknown, i64 -1
  ret i64 %result
}
define internal i32 @cerune_file_host_close(i64 %raw) {
entry:
  %handle = inttoptr i64 %raw to ptr
  %status = call i32 @CloseHandle(ptr %handle)
  %ok = icmp ne i32 %status, 0
  %result = select i1 %ok, i32 0, i32 -1
  ret i32 %result
}
"#;
