use super::{
    Target,
    ir::{Instruction, Module, Origin},
};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    super::files::used(module)
        || module
            .instructions
            .iter()
            .chain(module.functions.iter().flat_map(|f| &f.instructions))
            .any(|i| {
                matches!(
                    i.instruction,
                    Instruction::ArgumentCount { .. } | Instruction::ArgumentGet { .. }
                )
            })
}
pub(super) fn origin_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        panic!("arguments require source origin")
    };
    format!(
        "@cerune.arguments.origin.{}.{}.{}",
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
    if !used(module) {
        return;
    }
    let mut origins = Vec::new();
    for i in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if matches!(
            i.instruction,
            Instruction::ArgumentCount { .. } | Instruction::ArgumentGet { .. }
        ) && !origins.contains(&i.origin)
        {
            origins.push(i.origin);
        }
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
        "count-limit-exceeded",
        "byte-limit-exceeded",
        "invalid-utf8",
        "embedded-nul",
        "invalid-unicode",
        "allocation-failed",
        "unconfigured",
        "invalid-index",
        "index-out-of-bounds",
        "budget-exceeded",
    ] {
        data(out, &format!("@cerune.arguments.{code}"), code);
    }
    for (name, text) in [
        ("capture", "cerune: arguments-v1 phase=capture code=%s\n"),
        (
            "capture.index",
            "cerune: arguments-v1 phase=capture code=%s index=%lld\n",
        ),
        (
            "get",
            "cerune: arguments-v1 phase=execute operation=get code=%s index=%lld%s",
        ),
        (
            "count",
            "cerune: arguments-v1 phase=execute operation=count code=unconfigured%s",
        ),
    ] {
        data(out, &format!("@cerune.arguments.format.{name}"), text);
    }
    writeln!(out, "@cerune.arguments.max_count = internal constant i64 {}\n@cerune.arguments.max_bytes = internal constant i64 {}", module.argument_limits.max_count, module.argument_limits.max_bytes).unwrap();
    let windows = module.target.expect("argument target") == Target::X86_64PcWindowsMsvc;
    for declaration in [
        "declare i32 @snprintf(ptr, i64, ptr, ...)\n",
        "declare i32 @fflush(ptr)\n",
        "declare void @_Exit(i32) noreturn\n",
        "declare i64 @strlen(ptr)\n",
        "declare ptr @memchr(ptr, i32, i64)\n",
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
    if !out.contains("define internal i1 @cerune.utf8(") {
        out.push_str(super::utf8::SUPPORT);
    }
    out.push_str(FAILURE);
    out.push_str(if windows {
        "  call i32 @_write(i32 2, ptr %text, i32 %count)\n"
    } else {
        "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %text, i64 %wide)\n"
    });
    out.push_str("  br label %exit\nexit:\n  call void @_Exit(i32 1)\n  unreachable\n}\n");
    out.push_str(SNAPSHOT);
    out.push_str(
        &GET.replace(
            "@LIMIT@",
            &module
                .string_heap_limit
                .expect("arguments string heap")
                .to_string(),
        ),
    );
}

const FAILURE: &str = r#"
define internal void @cerune.arguments.failed(ptr %format.string, ptr %code, i64 %index, ptr %origin, i1 %flush) noreturn {
entry:
  %text = alloca [512 x i8]
  br i1 %flush, label %flush.output, label %format
flush.output:
  call i32 @fflush(ptr null)
  br label %format
format:
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr %format.string, ptr %code, i64 %index, ptr %origin)
  %positive = icmp sgt i32 %length, 0
  br i1 %positive, label %write, label %exit
write:
  %fits = icmp ult i32 %length, 512
  %count = select i1 %fits, i32 %length, i32 511
"#;
const SNAPSHOT: &str = r#"
@cerune.arguments.values = internal global ptr null
@cerune.arguments.length = internal global i64 0
@cerune.arguments.configured = internal global i1 false

define internal void @cerune.arguments.capture.failed(ptr %code, i64 %index) noreturn {
entry:
  %indexed = icmp sge i64 %index, 0
  %format = select i1 %indexed, ptr @cerune.arguments.format.capture.index, ptr @cerune.arguments.format.capture
  call void @cerune.arguments.failed(ptr %format, ptr %code, i64 %index, ptr null, i1 false)
  unreachable
}
define internal void @cerune.arguments.free_values(ptr %values, i64 %count) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %end = icmp eq i64 %i, %count
  br i1 %end, label %done, label %value
value:
  %slot = getelementptr %cerune.string, ptr %values, i64 %i
  %v = load %cerune.string, ptr %slot
  %len = extractvalue %cerune.string %v, 1
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %advance, label %free
free:
  %data = extractvalue %cerune.string %v, 0
  call void @free(ptr %data)
  br label %advance
advance:
  %next = add i64 %i, 1
  br label %loop
done:
  call void @free(ptr %values)
  ret void
}
define internal void @cerune.arguments.dispose() {
entry:
  %values = load ptr, ptr @cerune.arguments.values
  %count = load i64, ptr @cerune.arguments.length
  call void @cerune.arguments.free_values(ptr %values, i64 %count)
  store ptr null, ptr @cerune.arguments.values
  store i64 0, ptr @cerune.arguments.length
  store i1 false, ptr @cerune.arguments.configured
  ret void
}
; Validate every entry, then copy and publish a complete independent snapshot.
define internal void @cerune.arguments.capture(i64 %count, ptr %values, i64 %max_count, i64 %max_bytes) {
entry:
  %invalid = icmp ugt i64 %max_count, 9223372036854775807
  br i1 %invalid, label %invalid.limit, label %check.count
invalid.limit:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.invalid-limit, i64 -1)
  unreachable
check.count:
  %over = icmp ugt i64 %count, %max_count
  br i1 %over, label %count.failed, label %validate
count.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.count-limit-exceeded, i64 -1)
  unreachable
validate:
  %i = phi i64 [ 0, %check.count ], [ %next, %advance ]
  %bytes = phi i64 [ 0, %check.count ], [ %total, %advance ]
  %end = icmp eq i64 %i, %count
  br i1 %end, label %table, label %check.bytes
check.bytes:
  %slot = getelementptr %cerune.string, ptr %values, i64 %i
  %v = load %cerune.string, ptr %slot
  %data = extractvalue %cerune.string %v, 0
  %len = extractvalue %cerune.string %v, 1
  %space = sub i64 %max_bytes, %bytes
  %excess = icmp ugt i64 %len, %space
  br i1 %excess, label %bytes.failed, label %check.utf8
bytes.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.byte-limit-exceeded, i64 %i)
  unreachable
check.utf8:
  %valid = call i1 @cerune.utf8(ptr %data, i64 %len)
  br i1 %valid, label %check.empty, label %utf8.failed
utf8.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.invalid-utf8, i64 %i)
  unreachable
check.empty:
  %empty = icmp eq i64 %len, 0
  br i1 %empty, label %advance, label %check.nul
check.nul:
  %nul = call ptr @memchr(ptr %data, i32 0, i64 %len)
  %has.nul = icmp ne ptr %nul, null
  br i1 %has.nul, label %nul.failed, label %advance
nul.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.embedded-nul, i64 %i)
  unreachable
advance:
  %next = add i64 %i, 1
  %total = add i64 %bytes, %len
  br label %validate
table:
  %no.entries = icmp eq i64 %count, 0
  br i1 %no.entries, label %copy, label %table.size
table.size:
  %too.large = icmp ugt i64 %count, 1152921504606846975
  br i1 %too.large, label %table.failed, label %table.allocate
table.allocate:
  %size = mul i64 %count, 16
  %allocated = call ptr @malloc(i64 %size)
  %null = icmp eq ptr %allocated, null
  br i1 %null, label %table.failed, label %copy
table.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.allocation-failed, i64 -1)
  unreachable
copy:
  %copied = phi ptr [ null, %table ], [ %allocated, %table.allocate ]
  br label %copy.loop
copy.loop:
  %j = phi i64 [ 0, %copy ], [ %j.next, %copy.advance ]
  %finished = icmp eq i64 %j, %count
  br i1 %finished, label %publish, label %copy.value
copy.value:
  %source.slot = getelementptr %cerune.string, ptr %values, i64 %j
  %source = load %cerune.string, ptr %source.slot
  %source.data = extractvalue %cerune.string %source, 0
  %source.len = extractvalue %cerune.string %source, 1
  %source.empty = icmp eq i64 %source.len, 0
  br i1 %source.empty, label %copy.advance, label %copy.allocate
copy.allocate:
  %owned = call ptr @malloc(i64 %source.len)
  %failed = icmp eq ptr %owned, null
  br i1 %failed, label %copy.failed, label %copy.bytes
copy.failed:
  call void @cerune.arguments.free_values(ptr %copied, i64 %j)
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.allocation-failed, i64 %j)
  unreachable
copy.bytes:
  call ptr @memcpy(ptr %owned, ptr %source.data, i64 %source.len)
  br label %copy.advance
copy.advance:
  %owned.data = phi ptr [ @cerune.string.empty, %copy.value ], [ %owned, %copy.bytes ]
  %result.data = insertvalue %cerune.string poison, ptr %owned.data, 0
  %result = insertvalue %cerune.string %result.data, i64 %source.len, 1
  %destination = getelementptr %cerune.string, ptr %copied, i64 %j
  store %cerune.string %result, ptr %destination
  %j.next = add i64 %j, 1
  br label %copy.loop
publish:
  call void @cerune.arguments.dispose()
  store ptr %copied, ptr @cerune.arguments.values
  store i64 %count, ptr @cerune.arguments.length
  store i1 true, ptr @cerune.arguments.configured
  ret void
}
define internal i64 @cerune.arguments.count(ptr %origin) {
entry:
  %configured = load i1, ptr @cerune.arguments.configured
  br i1 %configured, label %ready, label %failed
failed:
  call void @cerune.arguments.failed(ptr @cerune.arguments.format.count, ptr %origin, i64 0, ptr null, i1 true)
  unreachable
ready:
  %count = load i64, ptr @cerune.arguments.length
  ret i64 %count
}
"#;
const GET: &str = r#"
define internal %cerune.string @cerune.arguments.get(i64 %index, ptr %origin) {
entry:
  %configured = load i1, ptr @cerune.arguments.configured
  br i1 %configured, label %check.index, label %unconfigured
unconfigured:
  br label %failed
check.index:
  %is.negative = icmp slt i64 %index, 0
  br i1 %is.negative, label %negative, label %check.bounds
negative:
  br label %failed
check.bounds:
  %count = load i64, ptr @cerune.arguments.length
  %out = icmp uge i64 %index, %count
  br i1 %out, label %bounds, label %value
bounds:
  br label %failed
value:
  %values = load ptr, ptr @cerune.arguments.values
  %slot = getelementptr %cerune.string, ptr %values, i64 %index
  %v = load %cerune.string, ptr %slot
  %length = extractvalue %cerune.string %v, 1
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %zero, label %budget
zero:
  ret %cerune.string { ptr @cerune.string.empty, i64 0 }
budget:
  %live = load i64, ptr @cerune.string.live
  %space = sub i64 @LIMIT@, %live
  %excess = icmp ugt i64 %length, %space
  br i1 %excess, label %budget.failed, label %size
budget.failed:
  br label %failed
size:
  %overflow = icmp ugt i64 %length, -25
  br i1 %overflow, label %allocation.failed, label %allocate
allocate:
  %bytes = add i64 %length, 24
  %owner = call ptr @malloc(i64 %bytes)
  %null = icmp eq ptr %owner, null
  br i1 %null, label %allocation.failed, label %copy
allocation.failed:
  br label %failed
copy:
  %head = load ptr, ptr @cerune.string.owners
  store ptr %head, ptr %owner
  %refs = getelementptr i8, ptr %owner, i64 8
  store i64 1, ptr %refs
  %lenptr = getelementptr i8, ptr %owner, i64 16
  store i64 %length, ptr %lenptr
  %data = getelementptr i8, ptr %owner, i64 24
  %source = extractvalue %cerune.string %v, 0
  call ptr @memcpy(ptr %data, ptr %source, i64 %length)
  store ptr %owner, ptr @cerune.string.owners
  %total = add i64 %live, %length
  store i64 %total, ptr @cerune.string.live
  %result.data = insertvalue %cerune.string poison, ptr %data, 0
  %result = insertvalue %cerune.string %result.data, i64 %length, 1
  ret %cerune.string %result
failed:
  %code = phi ptr [ @cerune.arguments.unconfigured, %unconfigured ], [ @cerune.arguments.invalid-index, %negative ], [ @cerune.arguments.index-out-of-bounds, %bounds ], [ @cerune.arguments.budget-exceeded, %budget.failed ], [ @cerune.arguments.allocation-failed, %allocation.failed ]
  call void @cerune.arguments.failed(ptr @cerune.arguments.format.get, ptr %code, i64 %index, ptr %origin, i1 true)
  unreachable
}
"#;

pub(super) fn emit_entry(module: &Module, out: &mut String) {
    if !used(module) {
        return;
    }
    let windows = module.target.expect("arguments target") == Target::X86_64PcWindowsMsvc;
    out.push_str(if windows { WINDOWS } else { LINUX });
    let entry = if super::files::used(module) {
        super::files::entry::adapt(ENTRY)
    } else {
        ENTRY.to_owned()
    };
    out.push_str(
        &entry
            .replace("@ENTRY@", if windows { "wmain" } else { "main" })
            .replace(
                "@TEMP_FREE@",
                if windows {
                    "  call void @cerune.arguments.free_values(ptr %values, i64 %count)"
                } else {
                    "  call void @free(ptr %values)"
                },
            ),
    );
    out.push_str("; End explicit target startup\n");
}
const LINUX: &str = r#"
define internal %cerune.string @cerune.arguments.os_value(ptr %text, i64 %index) {
entry:
  %length = call i64 @strlen(ptr %text)
  %valid = call i1 @cerune.utf8(ptr %text, i64 %length)
  br i1 %valid, label %done, label %failed
failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.invalid-unicode, i64 %index)
  unreachable
done:
  %v = insertvalue %cerune.string poison, ptr %text, 0
  %result = insertvalue %cerune.string %v, i64 %length, 1
  ret %cerune.string %result
}
"#;
const WINDOWS: &str = r#"
declare i32 @WideCharToMultiByte(i32, i32, ptr, i32, ptr, i32, ptr, ptr)
define internal %cerune.string @cerune.arguments.os_value(ptr %text, i64 %index) {
entry:
  %size = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %text, i32 -1, ptr null, i32 0, ptr null, ptr null)
  %invalid = icmp eq i32 %size, 0
  br i1 %invalid, label %unicode.failed, label %check.empty
unicode.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.invalid-unicode, i64 %index)
  unreachable
check.empty:
  %empty = icmp eq i32 %size, 1
  br i1 %empty, label %zero, label %allocate
zero:
  ret %cerune.string { ptr @cerune.string.empty, i64 0 }
allocate:
  %wide = zext i32 %size to i64
  %data = call ptr @malloc(i64 %wide)
  %null = icmp eq ptr %data, null
  br i1 %null, label %allocation.failed, label %convert
allocation.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.allocation-failed, i64 %index)
  unreachable
convert:
  %written = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %text, i32 -1, ptr %data, i32 %size, ptr null, ptr null)
  %valid = icmp eq i32 %written, %size
  br i1 %valid, label %done, label %unicode.failed
done:
  %length = sub i64 %wide, 1
  %v = insertvalue %cerune.string poison, ptr %data, 0
  %result = insertvalue %cerune.string %v, i64 %length, 1
  ret %cerune.string %result
}
"#;
const ENTRY: &str = r#"
; Explicit target startup: validate OS Unicode, capture, then execute Cerune.
define i32 @@ENTRY@(i32 %argc, ptr %argv) {
entry:
  %has.name = icmp sgt i32 %argc, 0
  %less = sub i32 %argc, 1
  %n = select i1 %has.name, i32 %less, i32 0
  %count = zext i32 %n to i64
  %empty = icmp eq i64 %count, 0
  br i1 %empty, label %prepare, label %allocate
allocate:
  %size = mul i64 %count, 16
  %table = call ptr @malloc(i64 %size)
  %failed = icmp eq ptr %table, null
  br i1 %failed, label %allocation.failed, label %prepare
allocation.failed:
  call void @cerune.arguments.capture.failed(ptr @cerune.arguments.allocation-failed, i64 -1)
  unreachable
prepare:
  %values = phi ptr [ null, %entry ], [ %table, %allocate ]
  br label %loop
loop:
  %i = phi i64 [ 0, %prepare ], [ %next, %value ]
  %end = icmp eq i64 %i, %count
  br i1 %end, label %capture, label %value
value:
  %next = add i64 %i, 1
  %source.slot = getelementptr ptr, ptr %argv, i64 %next
  %text = load ptr, ptr %source.slot
  %v = call %cerune.string @cerune.arguments.os_value(ptr %text, i64 %i)
  %destination = getelementptr %cerune.string, ptr %values, i64 %i
  store %cerune.string %v, ptr %destination
  br label %loop
capture:
  %max_count = load i64, ptr @cerune.arguments.max_count
  %max_bytes = load i64, ptr @cerune.arguments.max_bytes
  call void @cerune.arguments.capture(i64 %count, ptr %values, i64 %max_count, i64 %max_bytes)
@TEMP_FREE@
  %result = call i32 @cerune.program()
  call void @cerune.arguments.dispose()
  ret i32 %result
}
"#;
