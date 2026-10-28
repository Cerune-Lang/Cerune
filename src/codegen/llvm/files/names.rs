//! UTF-8の名前を変換せず、共通の可搬な単一成分規則を検査します。
use std::fmt::Write;
pub(super) fn emit(out: &mut String) {
    out.push_str(SOURCE);
    for (i, word) in ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .enumerate()
    {
        writeln!(out, "reserved.{i}:\n  %matches.{i} = call i1 @cerune.files.ascii_equal(ptr %data, i64 %stem, ptr @cerune.files.{word}, i64 {})\n  br i1 %matches.{i}, label %invalid, label %reserved.{}", word.len(), i+1).unwrap();
    }
    out.push_str(TAIL);
}
const SOURCE: &str = r#"
define internal i1 @cerune.files.ascii_equal(ptr %data, i64 %length, ptr %word, i64 %size) {
entry:
  %same.length = icmp eq i64 %length, %size
  br i1 %same.length, label %loop, label %no
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %compare ]
  %end = icmp eq i64 %i, %length
  br i1 %end, label %yes, label %byte
byte:
  %p = getelementptr i8, ptr %data, i64 %i
  %b = load i8, ptr %p
  %from.a = sub i8 %b, 97
  %lower = icmp ult i8 %from.a, 26
  %upper = sub i8 %b, 32
  %c = select i1 %lower, i8 %upper, i8 %b
  %q = getelementptr i8, ptr %word, i64 %i
  %expected = load i8, ptr %q
  %match = icmp eq i8 %c, %expected
  br i1 %match, label %compare, label %no
compare:
  %next = add i64 %i, 1
  br label %loop
yes:
  ret i1 true
no:
  ret i1 false
}
define internal i1 @cerune.files.name_valid(%cerune.string %name) {
entry:
  %data = extractvalue %cerune.string %name, 0
  %length = extractvalue %cerune.string %name, 1
  %zero = icmp eq i64 %length, 0
  %long = icmp ugt i64 %length, 255
  %bad.length = or i1 %zero, %long
  br i1 %bad.length, label %invalid, label %last
last:
  %last.index = sub i64 %length, 1
  %last.p = getelementptr i8, ptr %data, i64 %last.index
  %last.b = load i8, ptr %last.p
  switch i8 %last.b, label %scan [ i8 32, label %invalid i8 46, label %invalid ]
scan:
  %i = phi i64 [ 0, %last ], [ %next, %advance ]
  %end = icmp eq i64 %i, %length
  br i1 %end, label %stem.loop, label %byte
byte:
  %p = getelementptr i8, ptr %data, i64 %i
  %b = load i8, ptr %p
  %control = icmp ult i8 %b, 32
  br i1 %control, label %invalid, label %punctuation
punctuation:
  switch i8 %b, label %advance [
    i8 127, label %invalid
    i8 60, label %invalid
    i8 62, label %invalid
    i8 58, label %invalid
    i8 34, label %invalid
    i8 47, label %invalid
    i8 92, label %invalid
    i8 124, label %invalid
    i8 63, label %invalid
    i8 42, label %invalid
  ]
advance:
  %next = add i64 %i, 1
  br label %scan
stem.loop:
  %s = phi i64 [ 0, %scan ], [ %s.next, %stem.advance ]
  %s.end = icmp eq i64 %s, %length
  br i1 %s.end, label %trim, label %stem.byte
stem.byte:
  %s.p = getelementptr i8, ptr %data, i64 %s
  %s.b = load i8, ptr %s.p
  %dot = icmp eq i8 %s.b, 46
  br i1 %dot, label %trim, label %stem.advance
stem.advance:
  %s.next = add i64 %s, 1
  br label %stem.loop
trim:
  %stem = phi i64 [ %s, %stem.loop ], [ %s, %stem.byte ], [ %previous, %trim.byte ]
  %empty = icmp eq i64 %stem, 0
  br i1 %empty, label %reserved.0, label %trim.byte
trim.byte:
  %previous = sub i64 %stem, 1
  %t.p = getelementptr i8, ptr %data, i64 %previous
  %t.b = load i8, ptr %t.p
  %space = icmp eq i8 %t.b, 32
  br i1 %space, label %trim, label %reserved.0
"#;
const TAIL: &str = r#"
reserved.6:
  %enough = icmp uge i64 %stem, 4
  br i1 %enough, label %device, label %valid
device:
  %com = call i1 @cerune.files.ascii_equal(ptr %data, i64 3, ptr @cerune.files.COM, i64 3)
  %lpt = call i1 @cerune.files.ascii_equal(ptr %data, i64 3, ptr @cerune.files.LPT, i64 3)
  %prefix = or i1 %com, %lpt
  br i1 %prefix, label %number, label %valid
number:
  %digit.p = getelementptr i8, ptr %data, i64 3
  %digit = load i8, ptr %digit.p
  switch i64 %stem, label %valid [ i64 4, label %ascii i64 5, label %superscript ]
ascii:
  %offset = sub i8 %digit, 49
  %reserved = icmp ult i8 %offset, 9
  br i1 %reserved, label %invalid, label %valid
superscript:
  %lead = icmp eq i8 %digit, 194
  br i1 %lead, label %superscript.tail, label %valid
superscript.tail:
  %tail.p = getelementptr i8, ptr %data, i64 4
  %tail = load i8, ptr %tail.p
  switch i8 %tail, label %valid [ i8 185, label %invalid i8 178, label %invalid i8 179, label %invalid ]
valid:
  ret i1 true
invalid:
  ret i1 false
}
define internal ptr @cerune.files.check(%cerune.string %name, i64 %limit) {
entry:
  %bad.limit = icmp ugt i64 %limit, 2147483647
  br i1 %bad.limit, label %invalid.limit, label %check.name
check.name:
  %valid = call i1 @cerune.files.name_valid(%cerune.string %name)
  br i1 %valid, label %connection, label %invalid.name
connection:
  %configured = call i32 @cerune_file_host_configured()
  %yes = icmp ne i32 %configured, 0
  %result = select i1 %yes, ptr null, ptr @cerune.files.unconfigured
  ret ptr %result
invalid.limit:
  ret ptr @cerune.files.invalid-limit
invalid.name:
  ret ptr @cerune.files.invalid-name
}
define internal ptr @cerune.files.open_file(%cerune.string %name, i32 %write, ptr %handle) {
entry:
  %data = extractvalue %cerune.string %name, 0
  %length = extractvalue %cerune.string %name, 1
  %status = call i32 @cerune_file_host_open(ptr %data, i64 %length, i32 %write, ptr %handle)
  switch i32 %status, label %invalid [
    i32 0, label %ok
    i32 1, label %missing
    i32 2, label %exists
    i32 3, label %unsupported
    i32 4, label %failed
  ]
ok:
  ret ptr null
missing:
  ret ptr @cerune.files.not-found
exists:
  ret ptr @cerune.files.already-exists
unsupported:
  ret ptr @cerune.files.unsupported-target
failed:
  ret ptr @cerune.files.open-failed
invalid:
  ret ptr @cerune.files.invalid-response
}
"#;
