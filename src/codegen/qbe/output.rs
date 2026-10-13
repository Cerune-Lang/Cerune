//! 共通IRの出力を、要求数・受理数・不変の出自を持つQBE処理へ写します。
use super::{
    Target,
    ir::{FailureOrigin, Instruction, Module},
};
use crate::source::Span;
use std::fmt::Write;

pub(super) fn first_span(module: &Module) -> Option<Span> {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .find_map(|item| match item {
            Instruction::Output { origin, .. } => Some(origin.span),
            _ => None,
        })
}
pub(super) fn origin_name(origin: FailureOrigin) -> String {
    format!(
        "cerune_output_origin_{}_{}_{}",
        origin.node.0,
        origin.span.start(),
        origin.span.end()
    )
}
fn data(out: &mut String, section: &str, name: &str, bytes: &[u8]) {
    write!(out, "section \"{section}\" data ${name} = {{ ").unwrap();
    let mut i = 0;
    while i < bytes.len() {
        if i != 0 {
            out.push_str(", ");
        }
        if (32..=126).contains(&bytes[i]) && !matches!(bytes[i], b'"' | b'\\') {
            out.push_str("b \"");
            while i < bytes.len()
                && (32..=126).contains(&bytes[i])
                && !matches!(bytes[i], b'"' | b'\\')
            {
                out.push(char::from(bytes[i]));
                i += 1;
            }
            out.push('"');
        } else {
            write!(out, "b {}", bytes[i]).unwrap();
            i += 1;
        }
    }
    out.push_str(" }\n");
}
pub(super) fn emit(module: &Module, out: &mut String) {
    if first_span(module).is_none() {
        return;
    }
    let target = module.target.expect("output target");
    let windows = target == Target::X86_64PcWindowsMsvc;
    let section = target.read_only_section();
    let mut seen = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        let Instruction::Output { origin, .. } = item else {
            continue;
        };
        if seen.contains(origin) {
            continue;
        }
        seen.push(*origin);
        let text = format!(
            " node={}{} bytes={}..{}\n\0",
            origin.node.0,
            origin.span.source_id().record_field(),
            origin.span.start(),
            origin.span.end()
        );
        data(out, section, &origin_name(*origin), text.as_bytes());
    }
    for (name, bytes) in [
        (
            "record",
            b"cerune: io-v1 operation=write requested=%llu accepted=%llu%s\0".as_slice(),
        ),
        ("format_error", b"cerune: output formatting failed%s\0"),
        ("true", b"true\n"),
        ("false", b"false\n"),
        ("lf", b"\n"),
        ("quote", b"\""),
    ] {
        data(out, section, &format!("cerune_output_{name}"), bytes);
    }
    out.push_str("function l $cerune_output_write(l %data, l %length) {\n@start\n");
    out.push_str(if windows {
        "  %n =w call $_write(w 1, l %data, w %length)\n  %result =l extsw %n\n"
    } else {
        "  %result =l call $write(w 1, l %data, l %length)\n"
    });
    out.push_str("  ret %result\n}\n");
    out.push_str("function $cerune_output_diagnostic(l %data, w %length) {\n@start\n  %positive =w csgtw %length, 0\n  jnz %positive, @bound, @exit\n@bound\n  %fits =w cultw %length, 512\n  jnz %fits, @fit, @limit\n@fit\n  jmp @write\n@limit\n  jmp @write\n@write\n  %count =w phi @fit %length, @limit 511\n");
    out.push_str(if windows {
        "  call $_write(w 2, l %data, w %count)\n"
    } else {
        "  %wide =l extuw %count\n  call $write(w 2, l %data, l %wide)\n"
    });
    out.push_str("  jmp @exit\n@exit\n  call $_Exit(w 1)\n  hlt\n}\n");
    out.push_str(COMMON);
    out.push_str("function $cerune_output_begin(l %op, l %length, l %origin) {\n@start\n  storel %length, %op\n  %accepted_ptr =l add %op, 8\n  storel 0, %accepted_ptr\n  %origin_ptr =l add %op, 16\n  storel %origin, %origin_ptr\n");
    if !windows {
        out.push_str("  %prior =l call $signal(w 13, l 1)\n  %saved =l add %op, 24\n  storel %prior, %saved\n  %bad =w ceql %prior, -1\n  jnz %bad, @failed, @done\n@failed\n  call $cerune_output_failed(l %op)\n  hlt\n@done\n");
    }
    out.push_str("  ret\n}\n");
    out.push_str("function $cerune_output_end(l %op) {\n@start\n");
    if !windows {
        out.push_str("  %saved =l add %op, 24\n  %prior =l loadl %saved\n  %restored =l call $signal(w 13, l %prior)\n  %bad =w ceql %restored, -1\n  jnz %bad, @failed, @done\n@failed\n  call $cerune_output_failed(l %op)\n  hlt\n@done\n");
    }
    out.push_str("  ret\n}\n");
    out.push_str(&PART.replace(
        "ERRNO",
        if windows {
            "_errno"
        } else {
            "__errno_location"
        },
    ));
    for (kind, ty, fmt) in [
        ("i64", "l", "%lld"),
        ("u64", "l", "%llu"),
        ("f32", "s", "%.9g"),
        ("f64", "d", "%.17g"),
    ] {
        data(
            out,
            section,
            &format!("cerune_output_format_{kind}"),
            format!("{fmt}\0").as_bytes(),
        );
        writeln!(out, "function $cerune_output_{kind}({ty} %value, w %newline, l %origin) {{\n@start\n  %text =l alloc8 128").unwrap();
        let (arg_ty, value) = if kind == "f32" {
            out.push_str("  %wide =d exts %value\n");
            ("d", "%wide")
        } else {
            (ty, "%value")
        };
        writeln!(out, "  %n =w call $snprintf(l %text, l 128, l $cerune_output_format_{kind}, ..., {arg_ty} {value})").unwrap();
        out.push_str("  %valid =w cultw %n, 127\n  jnz %valid, @formatted, @failed\n@failed\n  %error =l alloc8 512\n  %error_length =w call $snprintf(l %error, l 512, l $cerune_output_format_error, ..., l %origin)\n  call $cerune_output_diagnostic(l %error, w %error_length)\n  hlt\n@formatted\n  %length =l extuw %n\n  %tail =l add %text, %length\n  storeb 10, %tail\n  %extra =l extuw %newline\n  %requested =l add %length, %extra\n  call $cerune_output_text(l %text, l %requested, l %origin)\n  ret\n}\n");
    }
    out.push_str("function $cerune_output_bool(w %value, w %newline, l %origin) {\n@start\n  jnz %value, @true, @false\n@true\n  jmp @write\n@false\n  jmp @write\n@write\n  %text =l phi @true $cerune_output_true, @false $cerune_output_false\n  %length =l phi @true 4, @false 5\n  %extra =l extuw %newline\n  %requested =l add %length, %extra\n  call $cerune_output_text(l %text, l %requested, l %origin)\n  ret\n}\n");
    if module.uses_strings {
        emit_escape(out);
        out.push_str(STRINGS);
    }
}
fn emit_escape(out: &mut String) {
    out.push_str("function l $cerune_output_escape(w %byte, l %out) {\n@start\n");
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
        writeln!(
            out,
            "  %is{b} =w ceqw %byte, {b}\n  jnz %is{b}, @escape{b}, @next{b}\n@next{b}"
        )
        .unwrap();
    }
    out.push_str("  storeb %byte, %out\n  ret 1\n");
    for (b, text) in escapes {
        writeln!(out, "@escape{b}").unwrap();
        for (i, c) in text.bytes().enumerate() {
            writeln!(out, "  %p{b}_{i} =l add %out, {i}\n  storeb {c}, %p{b}_{i}").unwrap();
        }
        writeln!(out, "  ret {}", text.len()).unwrap();
    }
    out.push_str("}\n");
}
// 操作の32バイト領域は要求数、受理数、出自への参照、LinuxのSIGPIPE設定を保持します。
// 書込みを分割しても受理数と出自は同じ操作に属します。
const COMMON: &str = r#"
function $cerune_output_failed(l %op) {
@start
  %requested =l loadl %op
  %accepted_ptr =l add %op, 8
  %accepted =l loadl %accepted_ptr
  %origin_ptr =l add %op, 16
  %origin =l loadl %origin_ptr
  %text =l alloc8 512
  %n =w call $snprintf(l %text, l 512, l $cerune_output_record, ..., l %requested, l %accepted, l %origin)
  call $cerune_output_diagnostic(l %text, w %n)
  hlt
}
function $cerune_output_text(l %data, l %length, l %origin) {
@start
  %empty =w ceql %length, 0
  jnz %empty, @done, @write
@write
  %op =l alloc8 32
  call $cerune_output_begin(l %op, l %length, l %origin)
  call $cerune_output_part(l %op, l %data, l %length)
  call $cerune_output_end(l %op)
  jmp @done
@done
  ret
}
"#;
const PART: &str = r#"
function $cerune_output_part(l %op, l %data, l %length) {
@start
  %accepted_ptr =l add %op, 8
  jmp @condition
@condition
  %cursor =l phi @start %data, @advance %next
  %remaining =l phi @start %length, @advance %rest
  %done =w ceql %remaining, 0
  jnz %done, @end, @prepare
@prepare
  %small =w cultl %remaining, 65536
  jnz %small, @short, @long
@short
  jmp @request
@long
  jmp @request
@request
  %request =l phi @short %remaining, @long 65536
  jmp @write
@write
  %n =l call $cerune_output_write(l %cursor, l %request)
  %negative =w csltl %n, 0
  jnz %negative, @error, @check
@error
  %errno_ptr =l call $ERRNO()
  %errno =w loadw %errno_ptr
  %interrupted =w ceqw %errno, 4
  jnz %interrupted, @write, @failed
@check
  %positive =w cugtl %n, 0
  %bounded =w culel %n, %request
  %valid =w and %positive, %bounded
  jnz %valid, @advance, @failed
@advance
  %accepted =l loadl %accepted_ptr
  %total =l add %accepted, %n
  storel %total, %accepted_ptr
  %next =l add %cursor, %n
  %rest =l sub %remaining, %n
  jmp @condition
@failed
  call $cerune_output_failed(l %op)
  hlt
@end
  ret
}
"#;
const STRINGS: &str = r#"
function $cerune_output_string(l %value, w %newline, l %origin) {
@start
  %length =l loadl %value
  %data =l add %value, 8
  %extra =l extuw %newline
  %requested =l add %length, %extra
  %op =l alloc8 32
  call $cerune_output_begin(l %op, l %requested, l %origin)
  call $cerune_output_part(l %op, l %data, l %length)
  call $cerune_output_part(l %op, l $cerune_output_lf, l %extra)
  call $cerune_output_end(l %op)
  ret
}
function $cerune_output_quoted(l %value, w %newline, l %origin) {
@start
  %length =l loadl %value
  %data =l add %value, 8
  %extra =l extuw %newline
  %initial =l add 2, %extra
  %scratch =l alloc8 8
  %buffer =l alloc8 256
  %op =l alloc8 32
  jmp @count
@count
  %index =l phi @start 0, @count_byte %next_index
  %requested =l phi @start %initial, @count_byte %next_count
  %counted =w ceql %index, %length
  jnz %counted, @begin, @count_byte
@count_byte
  %source =l add %data, %index
  %byte =w loadub %source
  %n =l call $cerune_output_escape(w %byte, l %scratch)
  %next_count =l add %requested, %n
  %next_index =l add %index, 1
  jmp @count
@begin
  call $cerune_output_begin(l %op, l %requested, l %origin)
  storeb 34, %buffer
  jmp @condition
@condition
  %i =l phi @begin 0, @append %next
  %used =l phi @begin 1, @append %filled
  %done =w ceql %i, %length
  jnz %done, @tail, @space
@space
  %full =w cugtl %used, 250
  jnz %full, @drain, @append
@drain
  call $cerune_output_part(l %op, l %buffer, l %used)
  jmp @append
@append
  %offset =l phi @space %used, @drain 0
  %p =l add %data, %i
  %b =w loadub %p
  %destination =l add %buffer, %offset
  %size =l call $cerune_output_escape(w %b, l %destination)
  %filled =l add %offset, %size
  %next =l add %i, 1
  jmp @condition
@tail
  call $cerune_output_part(l %op, l %buffer, l %used)
  call $cerune_output_part(l %op, l $cerune_output_quote, l 1)
  call $cerune_output_part(l %op, l $cerune_output_lf, l %extra)
  call $cerune_output_end(l %op)
  ret
}
"#;
