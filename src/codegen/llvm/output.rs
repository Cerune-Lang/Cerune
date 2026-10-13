//! 共通IRの出力操作を、要求数・受理数・不変の出自を持つLLVM処理にします。
use super::{
    Target,
    ir::{Instruction, Module, Origin},
};
use crate::source::Span;
use std::fmt::Write;

pub(super) fn first_span(module: &Module) -> Option<Span> {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .find_map(|item| match (&item.instruction, item.origin) {
            (Instruction::Output { .. }, Origin::Source { span, .. }) => Some(span),
            _ => None,
        })
}
pub(super) fn used(module: &Module) -> bool {
    first_span(module).is_some()
}
pub(super) fn origin_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        panic!("output requires source origin")
    };
    format!(
        "@cerune.output.origin.{}.{}.{}",
        node_id.0,
        span.start(),
        span.end()
    )
}
fn data(out: &mut String, name: &str, bytes: &[u8]) {
    write!(out, "{name} = private constant [{} x i8] c\"", bytes.len()).unwrap();
    for b in bytes {
        if (32..=126).contains(b) && *b != b'"' && *b != b'\\' {
            out.push(char::from(*b));
        } else {
            write!(out, "\\{b:02X}").unwrap();
        }
    }
    out.push_str("\"\n");
}
pub(super) fn emit_support(module: &Module, out: &mut String) {
    if !used(module) {
        return;
    }
    let windows = module.target.expect("output target") == Target::X86_64PcWindowsMsvc;
    let mut seen = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if !matches!(item.instruction, Instruction::Output { .. }) || seen.contains(&item.origin) {
            continue;
        }
        seen.push(item.origin);
        let Origin::Source { node_id, span } = item.origin else {
            unreachable!()
        };
        let origin = format!(
            " node={}{} bytes={}..{}\n\0",
            node_id.0,
            span.source_id().record_field(),
            span.start(),
            span.end()
        );
        data(out, &origin_name(item.origin), origin.as_bytes());
    }
    data(
        out,
        "@cerune.output.record",
        b"cerune: io-v1 operation=write requested=%llu accepted=%llu%s\0",
    );
    data(
        out,
        "@cerune.output.format.error",
        b"cerune: output formatting failed%s\0",
    );
    data(out, "@cerune.output.true", b"true\n");
    data(out, "@cerune.output.false", b"false\n");
    data(out, "@cerune.output.lf", b"\n");
    data(out, "@cerune.output.quote", b"\"");
    for declaration in [
        "declare i32 @snprintf(ptr, i64, ptr, ...)\n",
        "declare void @_Exit(i32) noreturn\n",
        if windows {
            "declare i32 @_write(i32, ptr, i32)\n"
        } else {
            "declare i64 @write(i32, ptr, i64)\n"
        },
        if windows {
            "declare ptr @_errno()\n"
        } else {
            "declare ptr @__errno_location()\n"
        },
    ] {
        if !out.contains(declaration) {
            out.push_str(declaration);
        }
    }
    if !windows && !out.contains("declare ptr @signal(i32, ptr)\n") {
        out.push_str("declare ptr @signal(i32, ptr)\n");
    }
    out.push_str("%cerune.output = type { i64, i64, ptr, ptr }\n");
    out.push_str("define internal i64 @cerune.output.write(ptr %data, i64 %length) {\nentry:\n");
    out.push_str(if windows {
        "  %count = trunc i64 %length to i32\n  %n = call i32 @_write(i32 1, ptr %data, i32 %count)\n  %result = sext i32 %n to i64\n"
    } else {
        "  %result = call i64 @write(i32 1, ptr %data, i64 %length)\n"
    });
    out.push_str("  ret i64 %result\n}\n");
    out.push_str("define internal void @cerune.output.diagnostic(ptr %data, i32 %length) noreturn {\nentry:\n  %positive = icmp sgt i32 %length, 0\n  br i1 %positive, label %write, label %exit\nwrite:\n  %fits = icmp ult i32 %length, 512\n  %count = select i1 %fits, i32 %length, i32 511\n");
    out.push_str(if windows {
        "  call i32 @_write(i32 2, ptr %data, i32 %count)\n"
    } else {
        "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %data, i64 %wide)\n"
    });
    out.push_str("  br label %exit\nexit:\n  call void @_Exit(i32 1)\n  unreachable\n}\n");
    out.push_str(COMMON);
    out.push_str("define internal void @cerune.output.begin(ptr %op, i64 %length, ptr %origin) {\nentry:\n  %initial = insertvalue %cerune.output zeroinitializer, i64 %length, 0\n  %state = insertvalue %cerune.output %initial, ptr %origin, 2\n  store %cerune.output %state, ptr %op\n");
    if !windows {
        out.push_str("  %prior = call ptr @signal(i32 13, ptr inttoptr (i64 1 to ptr))\n  %saved = getelementptr %cerune.output, ptr %op, i32 0, i32 3\n  store ptr %prior, ptr %saved\n  %bad = icmp eq ptr %prior, inttoptr (i64 -1 to ptr)\n  br i1 %bad, label %failed, label %done\nfailed:\n  call void @cerune.output.failed(ptr %op)\n  unreachable\ndone:\n");
    }
    out.push_str("  ret void\n}\n");
    out.push_str("define internal void @cerune.output.end(ptr %op) {\nentry:\n");
    if !windows {
        out.push_str("  %saved = getelementptr %cerune.output, ptr %op, i32 0, i32 3\n  %prior = load ptr, ptr %saved\n  %restored = call ptr @signal(i32 13, ptr %prior)\n  %bad = icmp eq ptr %restored, inttoptr (i64 -1 to ptr)\n  br i1 %bad, label %failed, label %done\nfailed:\n  call void @cerune.output.failed(ptr %op)\n  unreachable\ndone:\n");
    }
    out.push_str("  ret void\n}\n");
    out.push_str(&PART.replace(
        "ERRNO",
        if windows {
            "_errno"
        } else {
            "__errno_location"
        },
    ));
    for (kind, ty, fmt) in [
        ("i64", "i64", "%lld"),
        ("u64", "i64", "%llu"),
        ("f32", "float", "%.9g"),
        ("f64", "double", "%.17g"),
    ] {
        data(
            out,
            &format!("@cerune.output.format.{kind}"),
            format!("{fmt}\0").as_bytes(),
        );
        writeln!(out, "define internal void @cerune.output.{kind}({ty} %value, i1 %newline, ptr %origin) {{\nentry:\n  %text = alloca [128 x i8]").unwrap();
        let (argument_type, value) = if kind == "f32" {
            out.push_str("  %wide = fpext float %value to double\n");
            ("double", "%wide")
        } else {
            (ty, "%value")
        };
        writeln!(out, "  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 128, ptr @cerune.output.format.{kind}, {argument_type} {value})").unwrap();
        out.push_str("  %valid = icmp ult i32 %n, 127\n  br i1 %valid, label %formatted, label %failed\nfailed:\n  %error = alloca [512 x i8]\n  %error.length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %error, i64 512, ptr @cerune.output.format.error, ptr %origin)\n  call void @cerune.output.diagnostic(ptr %error, i32 %error.length)\n  unreachable\nformatted:\n  %length = zext i32 %n to i64\n  %tail = getelementptr i8, ptr %text, i64 %length\n  store i8 10, ptr %tail\n  %extra = zext i1 %newline to i64\n  %requested = add i64 %length, %extra\n  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)\n  ret void\n}\n");
    }
    out.push_str("define internal void @cerune.output.bool(i1 %value, i1 %newline, ptr %origin) {\nentry:\n  %text = select i1 %value, ptr @cerune.output.true, ptr @cerune.output.false\n  %length = select i1 %value, i64 4, i64 5\n  %extra = zext i1 %newline to i64\n  %requested = add i64 %length, %extra\n  call void @cerune.output.text(ptr %text, i64 %requested, ptr %origin)\n  ret void\n}\n");
    if module.uses_strings {
        emit_escape(out);
        out.push_str(STRINGS);
    }
}
fn emit_escape(out: &mut String) {
    out.push_str("define internal i64 @cerune.output.escape(i8 %byte, ptr %out) {\nentry:\n  switch i8 %byte, label %plain [\n");
    let escapes: Vec<_> = (0..=127u8)
        .filter_map(|b| {
            let text = match b {
                0 => "\\0".into(),
                9 => "\\t".into(),
                10 => "\\n".into(),
                13 => "\\r".into(),
                34 => "\\\"".into(),
                92 => "\\\\".into(),
                1..=31 | 127 => format!("\\u{{{b:02x}}}"),
                _ => return None,
            };
            Some((b, text))
        })
        .collect();
    for (b, _) in &escapes {
        writeln!(out, "    i8 {b}, label %escape{b}").unwrap();
    }
    out.push_str("  ]\nplain:\n  store i8 %byte, ptr %out\n  ret i64 1\n");
    for (b, text) in escapes {
        writeln!(out, "escape{b}:").unwrap();
        for (i, c) in text.bytes().enumerate() {
            writeln!(
                out,
                "  %p{b}.{i} = getelementptr i8, ptr %out, i64 {i}\n  store i8 {c}, ptr %p{b}.{i}"
            )
            .unwrap();
        }
        writeln!(out, "  ret i64 {}\n", text.len()).unwrap();
    }
    out.push_str("}\n");
}
const COMMON: &str = r#"
define internal void @cerune.output.failed(ptr %op) noreturn {
entry:
  %state = load %cerune.output, ptr %op
  %requested = extractvalue %cerune.output %state, 0
  %accepted = extractvalue %cerune.output %state, 1
  %origin = extractvalue %cerune.output %state, 2
  %text = alloca [512 x i8]
  %n = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 512, ptr @cerune.output.record, i64 %requested, i64 %accepted, ptr %origin)
  call void @cerune.output.diagnostic(ptr %text, i32 %n)
  unreachable
}
define internal void @cerune.output.text(ptr %data, i64 %length, ptr %origin) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %done, label %write
write:
  %op = alloca %cerune.output
  call void @cerune.output.begin(ptr %op, i64 %length, ptr %origin)
  call void @cerune.output.part(ptr %op, ptr %data, i64 %length)
  call void @cerune.output.end(ptr %op)
  br label %done
done:
  ret void
}
"#;
const PART: &str = r#"
define internal void @cerune.output.part(ptr %op, ptr %data, i64 %length) {
entry:
  %accepted.ptr = getelementptr %cerune.output, ptr %op, i32 0, i32 1
  br label %condition
condition:
  %cursor = phi ptr [ %data, %entry ], [ %next, %advance ]
  %remaining = phi i64 [ %length, %entry ], [ %rest, %advance ]
  %done = icmp eq i64 %remaining, 0
  br i1 %done, label %end, label %prepare
prepare:
  %small = icmp ult i64 %remaining, 65536
  %request = select i1 %small, i64 %remaining, i64 65536
  br label %write
write:
  %n = call i64 @cerune.output.write(ptr %cursor, i64 %request)
  %negative = icmp slt i64 %n, 0
  br i1 %negative, label %error, label %check
error:
  %errno.ptr = call ptr @ERRNO()
  %errno = load i32, ptr %errno.ptr
  %interrupted = icmp eq i32 %errno, 4
  br i1 %interrupted, label %write, label %failed
check:
  %positive = icmp ugt i64 %n, 0
  %bounded = icmp ule i64 %n, %request
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %advance, label %failed
advance:
  %accepted = load i64, ptr %accepted.ptr
  %total = add i64 %accepted, %n
  store i64 %total, ptr %accepted.ptr
  %next = getelementptr i8, ptr %cursor, i64 %n
  %rest = sub i64 %remaining, %n
  br label %condition
failed:
  call void @cerune.output.failed(ptr %op)
  unreachable
end:
  ret void
}
"#;
const STRINGS: &str = r#"
define internal void @cerune.output.string(%cerune.string %value, i1 %newline, ptr %origin) {
entry:
  %data = extractvalue %cerune.string %value, 0
  %length = extractvalue %cerune.string %value, 1
  %extra = zext i1 %newline to i64
  %requested = add i64 %length, %extra
  %op = alloca %cerune.output
  call void @cerune.output.begin(ptr %op, i64 %requested, ptr %origin)
  call void @cerune.output.part(ptr %op, ptr %data, i64 %length)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.lf, i64 %extra)
  call void @cerune.output.end(ptr %op)
  ret void
}
define internal void @cerune.output.quoted(%cerune.string %value, i1 %newline, ptr %origin) {
entry:
  %data = extractvalue %cerune.string %value, 0
  %length = extractvalue %cerune.string %value, 1
  %extra = zext i1 %newline to i64
  %initial = add i64 2, %extra
  %scratch = alloca [6 x i8]
  %buffer = alloca [256 x i8]
  %op = alloca %cerune.output
  br label %count
count:
  %index = phi i64 [ 0, %entry ], [ %next.index, %count.byte ]
  %requested = phi i64 [ %initial, %entry ], [ %next.count, %count.byte ]
  %counted = icmp eq i64 %index, %length
  br i1 %counted, label %begin, label %count.byte
count.byte:
  %source = getelementptr i8, ptr %data, i64 %index
  %byte = load i8, ptr %source
  %n = call i64 @cerune.output.escape(i8 %byte, ptr %scratch)
  %next.count = add i64 %requested, %n
  %next.index = add i64 %index, 1
  br label %count
begin:
  call void @cerune.output.begin(ptr %op, i64 %requested, ptr %origin)
  store i8 34, ptr %buffer
  br label %condition
condition:
  %i = phi i64 [ 0, %begin ], [ %next, %append ]
  %used = phi i64 [ 1, %begin ], [ %filled, %append ]
  %done = icmp eq i64 %i, %length
  br i1 %done, label %tail, label %space
space:
  %full = icmp ugt i64 %used, 250
  br i1 %full, label %drain, label %append
drain:
  call void @cerune.output.part(ptr %op, ptr %buffer, i64 %used)
  br label %append
append:
  %offset = phi i64 [ %used, %space ], [ 0, %drain ]
  %p = getelementptr i8, ptr %data, i64 %i
  %b = load i8, ptr %p
  %destination = getelementptr i8, ptr %buffer, i64 %offset
  %size = call i64 @cerune.output.escape(i8 %b, ptr %destination)
  %filled = add i64 %offset, %size
  %next = add i64 %i, 1
  br label %condition
tail:
  call void @cerune.output.part(ptr %op, ptr %buffer, i64 %used)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.quote, i64 1)
  call void @cerune.output.part(ptr %op, ptr @cerune.output.lf, i64 %extra)
  call void @cerune.output.end(ptr %op)
  ret void
}
"#;
