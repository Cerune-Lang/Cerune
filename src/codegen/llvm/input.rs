//! 行入力を明示的なOS接続・UTF-8検査・文字列所有へ展開します。
use super::{
    Target,
    ir::{Instruction, Module, Origin},
};
use std::fmt::Write;

pub(super) fn origin_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        panic!("input requires source origin")
    };
    format!(
        "@cerune.input.origin.{}.{}.{}",
        node_id.0,
        span.start(),
        span.end()
    )
}
fn data(out: &mut String, name: &str, text: &str) {
    write!(
        out,
        "{name} = private constant [{} x i8] c\"",
        text.len() + 1
    )
    .unwrap();
    for b in text.bytes().chain([0]) {
        write!(out, "\\{b:02X}").unwrap();
    }
    out.push_str("\"\n");
}
pub(super) fn emit_support(module: &Module, out: &mut String) {
    let mut origins = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if matches!(item.instruction, Instruction::ReadLine { .. })
            && !origins.contains(&item.origin)
        {
            origins.push(item.origin);
        }
    }
    if origins.is_empty() {
        return;
    }
    for origin in origins {
        let Origin::Source { node_id, span } = origin else {
            unreachable!()
        };
        data(
            out,
            &origin_name(origin),
            &format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
        );
    }
    for code in [
        "invalid-limit",
        "unsupported-connection",
        "budget-exceeded",
        "allocation-failed",
        "read-failed",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
        "known",
        "unknown",
    ] {
        data(out, &format!("@cerune.input.{code}"), code);
    }
    data(
        out,
        "@cerune.input.format",
        "cerune: io-v1 operation=read-line code=%s limit=%lld consumed=%llu consumption=%s%s",
    );
    let windows = module.target.expect("input target") == Target::X86_64PcWindowsMsvc;
    for declaration in [
        "declare i32 @snprintf(ptr, i64, ptr, ...)\n",
        "declare void @_Exit(i32) noreturn\n",
        if windows {
            "declare i32 @_write(i32, ptr, i32)\n"
        } else {
            "declare i64 @write(i32, ptr, i64)\n"
        },
    ] {
        if !out.contains(declaration) {
            out.push_str(declaration);
        }
    }
    out.push_str(FAILURE);
    out.push_str(if windows {
        "  call i32 @_write(i32 2, ptr %text, i32 %count)\n"
    } else {
        "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %text, i64 %wide)\n"
    });
    out.push_str("  br label %exit\nexit:\n  call void @_Exit(i32 1)\n  unreachable\n}\n");
    if windows {
        out.push_str(WINDOWS);
    } else {
        if !out.contains("declare ptr @__errno_location()\n") {
            out.push_str("declare ptr @__errno_location()\n");
        }
        out.push_str(LINUX);
    }
    if !out.contains("define internal i1 @cerune.utf8(") {
        out.push_str(super::utf8::SUPPORT);
    }
    out.push_str(
        &READ_LINE.replace(
            "@LIMIT@",
            &module
                .string_heap_limit
                .expect("input string heap")
                .to_string(),
        ),
    );
}

const FAILURE: &str = r#"
define internal void @cerune.input.failed(ptr %code, i64 %limit, i64 %consumed, i1 %known, ptr %origin) noreturn {
entry:
  %text = alloca [512 x i8]
  %state = select i1 %known, ptr @cerune.input.known, ptr @cerune.input.unknown
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr @cerune.input.format, ptr %code, i64 %limit, i64 %consumed, ptr %state, ptr %origin)
  %positive = icmp sgt i32 %length, 0
  br i1 %positive, label %write, label %exit
write:
  %fits = icmp ult i32 %length, 512
  %count = select i1 %fits, i32 %length, i32 511
"#;

// read_byte: 0=EOF, 1=byte, -2=EINTR（未消費）, -1=失敗。
// 1より大きい応答は共通処理で契約違反として停止します。
const LINUX: &str = r#"
declare i64 @read(i32, ptr, i64)
define internal ptr @cerune.input.connection() {
entry:
  ret ptr null
}
define internal i64 @cerune.input.read_byte(ptr %byte) {
entry:
  %count = call i64 @read(i32 0, ptr %byte, i64 1)
  %failed = icmp slt i64 %count, 0
  br i1 %failed, label %error, label %done
error:
  %ep = call ptr @__errno_location()
  %errno = load i32, ptr %ep
  %interrupted = icmp eq i32 %errno, 4
  %failure = select i1 %interrupted, i64 -2, i64 -1
  ret i64 %failure
done:
  ret i64 %count
}
"#;
const WINDOWS: &str = r#"
declare ptr @GetStdHandle(i32)
declare i32 @GetConsoleMode(ptr, ptr)
declare i32 @GetFileType(ptr)
declare i32 @ReadFile(ptr, ptr, i32, ptr, ptr)
declare i32 @GetLastError()
@cerune.input.initialized = internal global i1 false
@cerune.input.handle = internal global ptr null
@cerune.input.pipe = internal global i1 false
@cerune.input.connection.error = internal global ptr null
define internal ptr @cerune.input.connection() {
entry:
  %initialized = load i1, ptr @cerune.input.initialized
  br i1 %initialized, label %done, label %initialize
initialize:
  store i1 true, ptr @cerune.input.initialized
  %handle = call ptr @GetStdHandle(i32 -10)
  store ptr %handle, ptr @cerune.input.handle
  %null = icmp eq ptr %handle, null
  %invalid = icmp eq ptr %handle, inttoptr (i64 -1 to ptr)
  %bad = or i1 %null, %invalid
  br i1 %bad, label %failed, label %classify
failed:
  store ptr @cerune.input.read-failed, ptr @cerune.input.connection.error
  br label %done
classify:
  %mode = alloca i32
  %console = call i32 @GetConsoleMode(ptr %handle, ptr %mode)
  %unsupported = icmp ne i32 %console, 0
  br i1 %unsupported, label %reject, label %kind
reject:
  store ptr @cerune.input.unsupported-connection, ptr @cerune.input.connection.error
  br label %done
kind:
  %type = call i32 @GetFileType(ptr %handle)
  %pipe = icmp eq i32 %type, 3
  store i1 %pipe, ptr @cerune.input.pipe
  br label %done
done:
  %error = load ptr, ptr @cerune.input.connection.error
  ret ptr %error
}
define internal i64 @cerune.input.read_byte(ptr %byte) {
entry:
  %count.ptr = alloca i32
  %handle = load ptr, ptr @cerune.input.handle
  %pipe = load i1, ptr @cerune.input.pipe
  br label %read
read:
  store i32 0, ptr %count.ptr
  %ok = call i32 @ReadFile(ptr %handle, ptr %byte, i32 1, ptr %count.ptr, ptr null)
  %count = load i32, ptr %count.ptr
  %wide = zext i32 %count to i64
  %success = icmp ne i32 %ok, 0
  br i1 %success, label %received, label %error
received:
  %zero = icmp eq i32 %count, 0
  %empty.message = and i1 %zero, %pipe
  br i1 %empty.message, label %read, label %done
error:
  %code = call i32 @GetLastError()
  %more = icmp eq i32 %code, 234
  %positive = icmp ugt i32 %count, 0
  %partial = and i1 %more, %positive
  br i1 %partial, label %done, label %check.eof
check.eof:
  %broken = icmp eq i32 %code, 109
  %eof = icmp eq i32 %code, 38
  %end = or i1 %broken, %eof
  %result = select i1 %end, i64 0, i64 -1
  ret i64 %result
done:
  ret i64 %wide
}
"#;

const READ_LINE: &str = r#"
@cerune.input.eof = internal global i1 false
declare ptr @realloc(ptr, i64)

define internal %cerune.string @cerune.input.read_line(i64 %limit, ptr %origin) {
entry:
  %byte = alloca i8
  %offset = sub i64 %limit, 1
  %valid = icmp ult i64 %offset, 2147483647
  br i1 %valid, label %connection, label %invalid.limit
invalid.limit:
  call void @cerune.input.failed(ptr @cerune.input.invalid-limit, i64 %limit, i64 0, i1 true, ptr %origin)
  unreachable
connection:
  %error = call ptr @cerune.input.connection()
  %connected = icmp eq ptr %error, null
  br i1 %connected, label %eof, label %connection.failed
connection.failed:
  call void @cerune.input.failed(ptr %error, i64 %limit, i64 0, i1 true, ptr %origin)
  unreachable
eof:
  %known.eof = load i1, ptr @cerune.input.eof
  br i1 %known.eof, label %empty, label %budget
budget:
  %live = load i64, ptr @cerune.string.live
  %available = sub i64 @LIMIT@, %live
  %over = icmp ugt i64 %limit, %available
  br i1 %over, label %budget.failed, label %allocate
budget.failed:
  call void @cerune.input.failed(ptr @cerune.input.budget-exceeded, i64 %limit, i64 0, i1 true, ptr %origin)
  unreachable
allocate:
  %size = add i64 %limit, 24
  %owner = call ptr @malloc(i64 %size)
  %null = icmp eq ptr %owner, null
  br i1 %null, label %allocation.failed, label %reserve
allocation.failed:
  call void @cerune.input.failed(ptr @cerune.input.allocation-failed, i64 %limit, i64 0, i1 true, ptr %origin)
  unreachable
reserve:
  %reserved = add i64 %live, %limit
  store i64 %reserved, ptr @cerune.string.live
  %data = getelementptr i8, ptr %owner, i64 24
  br label %read
read:
  %length = phi i64 [ 0, %reserve ], [ %length, %retry ], [ %more, %store ]
  %count = call i64 @cerune.input.read_byte(ptr %byte)
  %interrupted = icmp eq i64 %count, -2
  br i1 %interrupted, label %retry, label %check
retry:
  br label %read
check:
  %failed = icmp slt i64 %count, 0
  br i1 %failed, label %read.failed, label %response
response:
  %invalid = icmp ugt i64 %count, 1
  br i1 %invalid, label %invalid.response, label %check.eof
check.eof:
  %end = icmp eq i64 %count, 0
  br i1 %end, label %mark.eof, label %received
mark.eof:
  store i1 true, ptr @cerune.input.eof
  br label %validate
received:
  %more = add i64 %length, 1
  %full = icmp eq i64 %length, %limit
  br i1 %full, label %limit.failed, label %store
store:
  %b = load i8, ptr %byte
  %p = getelementptr i8, ptr %data, i64 %length
  store i8 %b, ptr %p
  %lf = icmp eq i8 %b, 10
  br i1 %lf, label %validate, label %read
validate:
  %total = phi i64 [ %length, %mark.eof ], [ %more, %store ]
  %utf8 = call i1 @cerune.utf8(ptr %data, i64 %total)
  br i1 %utf8, label %finish, label %invalid.utf8
finish:
  %zero = icmp eq i64 %total, 0
  br i1 %zero, label %release.empty, label %adopt
release.empty:
  store i64 %live, ptr @cerune.string.live
  call void @free(ptr %owner)
  br label %empty
empty:
  ret %cerune.string { ptr @cerune.string.empty, i64 0 }
adopt:
  %half = udiv i64 %limit, 2
  %short = icmp ule i64 %total, %half
  %saved = sub i64 %limit, %total
  %worth = icmp uge i64 %saved, 4096
  %compact = and i1 %short, %worth
  br i1 %compact, label %shrink, label %publish
shrink:
  %small.size = add i64 %total, 24
  %small = call ptr @realloc(ptr %owner, i64 %small.size)
  %moved = icmp ne ptr %small, null
  %kept = select i1 %moved, ptr %small, ptr %owner
  br label %publish
publish:
  %final.owner = phi ptr [ %owner, %adopt ], [ %kept, %shrink ]
  %final.data = getelementptr i8, ptr %final.owner, i64 24
  %head = load ptr, ptr @cerune.string.owners
  store ptr %head, ptr %final.owner
  %ref = getelementptr i8, ptr %final.owner, i64 8
  store i64 1, ptr %ref
  %lenptr = getelementptr i8, ptr %final.owner, i64 16
  store i64 %total, ptr %lenptr
  store ptr %final.owner, ptr @cerune.string.owners
  %charged = add i64 %live, %total
  store i64 %charged, ptr @cerune.string.live
  %result.data = insertvalue %cerune.string poison, ptr %final.data, 0
  %result = insertvalue %cerune.string %result.data, i64 %total, 1
  ret %cerune.string %result
read.failed:
  br label %failure
invalid.response:
  br label %failure
limit.failed:
  br label %failure
invalid.utf8:
  br label %failure
failure:
  %code = phi ptr [ @cerune.input.read-failed, %read.failed ], [ @cerune.input.invalid-response, %invalid.response ], [ @cerune.input.limit-exceeded, %limit.failed ], [ @cerune.input.invalid-utf8, %invalid.utf8 ]
  %consumed = phi i64 [ %length, %read.failed ], [ %length, %invalid.response ], [ %more, %limit.failed ], [ %total, %invalid.utf8 ]
  %known = phi i1 [ true, %read.failed ], [ false, %invalid.response ], [ true, %limit.failed ], [ true, %invalid.utf8 ]
  store i64 %live, ptr @cerune.string.live
  call void @free(ptr %owner)
  call void @cerune.input.failed(ptr %code, i64 %limit, i64 %consumed, i1 %known, ptr %origin)
  unreachable
}
"#;
