//! 行入力を明示的なOS接続・UTF-8検査・文字列所有へ展開します。
use super::{
    Target,
    ir::{FailureOrigin, Instruction, Module},
};
use std::fmt::Write;

pub(super) fn origin_name(origin: FailureOrigin) -> String {
    format!(
        "cerune_input_origin_{}_{}_{}",
        origin.node.0,
        origin.span.start(),
        origin.span.end()
    )
}
fn data(out: &mut String, section: &str, name: &str, text: &str) {
    write!(out, "section \"{section}\" data ${name} = {{ ").unwrap();
    for (i, b) in text.bytes().chain([0]).enumerate() {
        if i != 0 {
            out.push_str(", ");
        }
        write!(out, "b {b}").unwrap();
    }
    out.push_str(" }\n");
}
pub(super) fn emit(module: &Module, out: &mut String) {
    let mut origins = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::ReadLine { origin, .. } = item
            && !origins.contains(origin)
        {
            origins.push(*origin);
        }
    }
    if origins.is_empty() {
        return;
    }
    let target = module.target.expect("input target");
    let section = target.read_only_section();
    for origin in origins {
        let span = origin.span;
        data(
            out,
            section,
            &origin_name(origin),
            &format!(
                " node={}{} bytes={}..{}\n",
                origin.node.0,
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
        data(
            out,
            section,
            &format!("cerune_input_{}", code.replace('-', "_")),
            code,
        );
    }
    data(
        out,
        section,
        "cerune_input_format",
        "cerune: io-v1 operation=read-line code=%s limit=%lld consumed=%llu consumption=%s%s",
    );
    out.push_str(FAILURE);
    out.push_str(if target == Target::X86_64PcWindowsMsvc {
        "  call $_write(w 2, l %text, w %count)\n"
    } else {
        "  %wide =l extuw %count\n  call $write(w 2, l %text, l %wide)\n"
    });
    out.push_str("  jmp @exit\n@exit\n  call $_Exit(w 1)\n  hlt\n}\n");
    out.push_str(if target == Target::X86_64PcWindowsMsvc {
        WINDOWS
    } else {
        LINUX
    });
    out.push_str(UTF8);
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
function $cerune_input_failed(l %code, l %limit, l %consumed, w %known, l %origin) {
@start
  %text =l alloc8 512
  jnz %known, @known, @unknown
@known
  jmp @format
@unknown
  jmp @format
@format
  %state =l phi @known $cerune_input_known, @unknown $cerune_input_unknown
  %length =w call $snprintf(l %text, l 512, l $cerune_input_format, ..., l %code, l %limit, l %consumed, l %state, l %origin)
  %positive =w csgtw %length, 0
  jnz %positive, @bound, @exit
@bound
  %fits =w cultw %length, 512
  jnz %fits, @fit, @limit
@fit
  jmp @write
@limit
  jmp @write
@write
  %count =w phi @fit %length, @limit 511
"#;
// read_byte: 0=EOF、1=byte、-2=EINTR（未消費）、-1=失敗。
// 1より大きい応答は共通処理で契約違反として停止します。
const LINUX: &str = r#"
function l $cerune_input_connection() {
@start
  ret 0
}
function l $cerune_input_read_byte(l %byte) {
@start
  %count =l call $read(w 0, l %byte, l 1)
  %failed =w csltl %count, 0
  jnz %failed, @error, @done
@error
  %ep =l call $__errno_location()
  %errno =w loadw %ep
  %interrupted =w ceqw %errno, 4
  jnz %interrupted, @retry, @failed
@retry
  ret -2
@failed
  ret -1
@done
  ret %count
}
"#;
const WINDOWS: &str = r#"
data $cerune_input_initialized = align 4 { w 0 }
data $cerune_input_handle = align 8 { l 0 }
data $cerune_input_pipe = align 4 { w 0 }
data $cerune_input_connection_error = align 8 { l 0 }
function l $cerune_input_connection() {
@start
  %mode =l alloc4 4
  %initialized =w loadw $cerune_input_initialized
  jnz %initialized, @done, @initialize
@initialize
  storew 1, $cerune_input_initialized
  %handle =l call $GetStdHandle(w -10)
  storel %handle, $cerune_input_handle
  %null =w ceql %handle, 0
  %invalid =w ceql %handle, -1
  %bad =w or %null, %invalid
  jnz %bad, @failed, @classify
@failed
  storel $cerune_input_read_failed, $cerune_input_connection_error
  jmp @done
@classify
  %console =w call $GetConsoleMode(l %handle, l %mode)
  jnz %console, @reject, @kind
@reject
  storel $cerune_input_unsupported_connection, $cerune_input_connection_error
  jmp @done
@kind
  %type =w call $GetFileType(l %handle)
  %pipe =w ceqw %type, 3
  storew %pipe, $cerune_input_pipe
@done
  %error =l loadl $cerune_input_connection_error
  ret %error
}
function l $cerune_input_read_byte(l %byte) {
@start
  %count_ptr =l alloc4 4
  %handle =l loadl $cerune_input_handle
  %pipe =w loadw $cerune_input_pipe
  jmp @read
@read
  storew 0, %count_ptr
  %ok =w call $ReadFile(l %handle, l %byte, w 1, l %count_ptr, l 0)
  %count =w loadw %count_ptr
  %wide =l extuw %count
  jnz %ok, @received, @error
@received
  %zero =w ceqw %count, 0
  %empty_message =w and %zero, %pipe
  jnz %empty_message, @read, @done
@error
  %code =w call $GetLastError()
  %more =w ceqw %code, 234
  %positive =w cugtw %count, 0
  %partial =w and %more, %positive
  jnz %partial, @done, @check_eof
@check_eof
  %broken =w ceqw %code, 109
  %eof =w ceqw %code, 38
  %end =w or %broken, %eof
  jnz %end, @eof, @failed
@eof
  ret 0
@failed
  ret -1
@done
  ret %wide
}
"#;
const UTF8: &str = r#"
function w $cerune_input_utf8(l %data, l %length) {
@start
  jmp @loop
@loop
  %i =l phi @start 0, @advance %next
  %remaining =w phi @start 0, @advance %rest
  %low =w phi @start 128, @advance %next_low
  %high =w phi @start 191, @advance %next_high
  %end =w ceql %i, %length
  jnz %end, @done, @load
@load
  %p =l add %data, %i
  %b =w loadub %p
  jnz %remaining, @check, @lead
@check
  %above =w cugew %b, %low
  %below =w culew %b, %high
  %valid =w and %above, %below
  %less =w sub %remaining, 1
  jnz %valid, @advance, @invalid
@lead
  %ascii =w cultw %b, 128
  jnz %ascii, @advance, @classify2
@classify2
  %offset2 =w sub %b, 194
  %two =w cultw %offset2, 30
  jnz %two, @advance, @classify3
@classify3
  %offset3 =w sub %b, 224
  %three =w cultw %offset3, 16
  jnz %three, @three, @classify4
@three
  %e0 =w ceqw %b, 224
  %ed =w ceqw %b, 237
  %add3 =w mul %e0, 32
  %sub3 =w mul %ed, 32
  %low3 =w add 128, %add3
  %high3 =w sub 191, %sub3
  jmp @advance
@classify4
  %offset4 =w sub %b, 240
  %four =w cultw %offset4, 5
  jnz %four, @four, @invalid
@four
  %f0 =w ceqw %b, 240
  %f4 =w ceqw %b, 244
  %add4 =w mul %f0, 16
  %sub4 =w mul %f4, 48
  %low4 =w add 128, %add4
  %high4 =w sub 191, %sub4
  jmp @advance
@advance
  %rest =w phi @check %less, @lead 0, @classify2 1, @three 2, @four 3
  %next_low =w phi @check 128, @lead 128, @classify2 128, @three %low3, @four %low4
  %next_high =w phi @check 191, @lead 191, @classify2 191, @three %high3, @four %high4
  %next =l add %i, 1
  jmp @loop
@done
  %complete =w ceqw %remaining, 0
  ret %complete
@invalid
  ret 0
}
"#;
const READ_LINE: &str = r#"
data $cerune_input_eof = align 4 { w 0 }
function l $cerune_input_read_line(l %limit, l %origin) {
@start
  %byte =l alloc4 4
  %offset =l sub %limit, 1
  %valid =w cultl %offset, 2147483647
  jnz %valid, @connection, @invalid_limit
@invalid_limit
  call $cerune_input_failed(l $cerune_input_invalid_limit, l %limit, l 0, w 1, l %origin)
  hlt
@connection
  %error =l call $cerune_input_connection()
  %connection_bad =w cnel %error, 0
  jnz %connection_bad, @connection_failed, @eof
@connection_failed
  call $cerune_input_failed(l %error, l %limit, l 0, w 1, l %origin)
  hlt
@eof
  %known_eof =w loadw $cerune_input_eof
  jnz %known_eof, @empty, @budget
@budget
  %live =l loadl $cerune_string_live
  %available =l sub @LIMIT@, %live
  %over =w cugtl %limit, %available
  jnz %over, @budget_failed, @allocate
@budget_failed
  call $cerune_input_failed(l $cerune_input_budget_exceeded, l %limit, l 0, w 1, l %origin)
  hlt
@allocate
  %size =l add %limit, 24
  %owner =l call $malloc(l %size)
  %null =w ceql %owner, 0
  jnz %null, @allocation_failed, @reserve
@allocation_failed
  call $cerune_input_failed(l $cerune_input_allocation_failed, l %limit, l 0, w 1, l %origin)
  hlt
@reserve
  %reserved =l add %live, %limit
  storel %reserved, $cerune_string_live
  %data =l add %owner, 24
  jmp @read
@read
  %length =l phi @reserve 0, @retry %length, @store %more
  %count =l call $cerune_input_read_byte(l %byte)
  %interrupted =w ceql %count, -2
  jnz %interrupted, @retry, @check
@retry
  jmp @read
@check
  %failed =w csltl %count, 0
  jnz %failed, @read_failed, @response
@response
  %invalid =w cugtl %count, 1
  jnz %invalid, @invalid_response, @check_eof
@check_eof
  %end =w ceql %count, 0
  jnz %end, @mark_eof, @received
@mark_eof
  storew 1, $cerune_input_eof
  jmp @validate
@received
  %more =l add %length, 1
  %full =w ceql %length, %limit
  jnz %full, @limit_failed, @store
@store
  %b =w loadub %byte
  %p =l add %data, %length
  storeb %b, %p
  %lf =w ceqw %b, 10
  jnz %lf, @validate, @read
@validate
  %total =l phi @mark_eof %length, @store %more
  %utf8 =w call $cerune_input_utf8(l %data, l %total)
  jnz %utf8, @finish, @invalid_utf8
@finish
  %zero =w ceql %total, 0
  jnz %zero, @release_empty, @adopt
@release_empty
  storel %live, $cerune_string_live
  call $free(l %owner)
  jmp @empty
@empty
  ret $cerune_string_empty
@adopt
  %head =l loadl $cerune_string_owners
  storel %head, %owner
  %ref =l add %owner, 8
  storel 1, %ref
  %string =l add %owner, 16
  storel %total, %string
  storel %owner, $cerune_string_owners
  %charged =l add %live, %total
  storel %charged, $cerune_string_live
  ret %string
@read_failed
  jmp @failure
@invalid_response
  jmp @failure
@limit_failed
  jmp @failure
@invalid_utf8
  jmp @failure
@failure
  %code =l phi @read_failed $cerune_input_read_failed, @invalid_response $cerune_input_invalid_response, @limit_failed $cerune_input_limit_exceeded, @invalid_utf8 $cerune_input_invalid_utf8
  %consumed =l phi @read_failed %length, @invalid_response %length, @limit_failed %more, @invalid_utf8 %total
  %known =w phi @read_failed 1, @invalid_response 0, @limit_failed 1, @invalid_utf8 1
  storel %live, $cerune_string_live
  call $free(l %owner)
  call $cerune_input_failed(l %code, l %limit, l %consumed, w %known, l %origin)
  hlt
}
"#;
