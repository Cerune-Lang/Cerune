//! private memoryを公開せず、値と状態のimportから行入力を実行します。
use super::ir::Origin;
use std::fmt::Write;
pub(super) const IMPORT: &str = "  (import \"cerune\" \"stdin_status_v1\" (func $stdin_status_v1 (result i32)))\n  (import \"cerune\" \"read_stdin_byte_v1\" (func $read_stdin_byte_v1 (result i32 i32)))\n";
fn diagnostic(text: &str, out: &mut String) {
    for b in text.bytes() {
        writeln!(out, "    i32.const {b}\n    call $write_error_byte").unwrap();
    }
}
pub(super) fn located(origin: Origin, name: &str, out: &mut String) {
    writeln!(out, "  (func ${name} (param $limit i64) (result i32)\n    (local $value i32) (local $code i32) (local $consumed i64) (local $known i32)\n    local.get $limit\n    call $cerune_input_read_line\n    local.set $known\n    local.set $consumed\n    local.set $code\n    local.set $value\n    local.get $code\n    if\n      i32.const 1\n      global.set $cerune_input_failed").unwrap();
    diagnostic("cerune: io-v1 operation=read-line code=", out);
    for (i, code) in [
        "invalid-limit",
        "unconfigured",
        "unsupported-connection",
        "budget-exceeded",
        "allocation-failed",
        "read-failed",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
    ]
    .iter()
    .enumerate()
    {
        writeln!(
            out,
            "    local.get $code\n    i32.const {}\n    i32.eq\n    if",
            i + 1
        )
        .unwrap();
        diagnostic(code, out);
        out.push_str("    end\n");
    }
    diagnostic(" limit=", out);
    out.push_str("    local.get $limit\n    i64.const 0\n    i64.lt_s\n    if\n      i32.const 45\n      call $write_error_byte\n      i64.const 0\n      local.get $limit\n      i64.sub\n      call $cerune_input_error_u64\n    else\n      local.get $limit\n      call $cerune_input_error_u64\n    end\n");
    diagnostic(" consumed=", out);
    out.push_str("    local.get $consumed\n    call $cerune_input_error_u64\n");
    diagnostic(" consumption=", out);
    out.push_str("    local.get $known\n    if\n");
    diagnostic("known", out);
    out.push_str("    else\n");
    diagnostic("unknown", out);
    out.push_str("    end\n");
    diagnostic(
        &format!(
            " node={}{} bytes={}..{}\n",
            origin.node_id.0,
            origin.span.source_id().record_field(),
            origin.span.start(),
            origin.span.end()
        ),
        out,
    );
    out.push_str("      unreachable\n    end\n    local.get $value\n  )\n");
}
pub(super) fn support(start: usize) -> String {
    SOURCE.replace("@EMPTY@", &(start - 8).to_string())
}
const SOURCE: &str = r#"
  (global $cerune_input_eof (mut i32) (i32.const 0))
  (global $cerune_input_failed (mut i32) (i32.const 0))
  (func $cerune_input_error_u64 (param $value i64)
    (if (i64.ge_u (local.get $value) (i64.const 10))
      (then (call $cerune_input_error_u64 (i64.div_u (local.get $value) (i64.const 10)))))
    (call $write_error_byte (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48))))
  ;; 戻り値は文字列address、失敗番号、確認済み消費数、消費数の完全性です。
  (func $cerune_input_read_line (param $limit i64) (result i32 i32 i64 i32)
    (local $value i32) (local $code i32) (local $status i32) (local $byte i32)
    (local $length i32) (local $consumed i64) (local $known i32)
    (if (global.get $cerune_input_failed) (then unreachable))
    (if (i64.ge_u (i64.sub (local.get $limit) (i64.const 1)) (i64.const 2147483647))
      (then (return (i32.const 0) (i32.const 1) (i64.const 0) (i32.const 1))))
    (local.set $status (call $stdin_status_v1))
    (if (i32.eq (local.get $status) (i32.const 1))
      (then (return (i32.const 0) (i32.const 2) (i64.const 0) (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 2))
      (then (return (i32.const 0) (i32.const 3) (i64.const 0) (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 3))
      (then (return (i32.const 0) (i32.const 6) (i64.const 0) (i32.const 1))))
    (if (local.get $status)
      (then (return (i32.const 0) (i32.const 7) (i64.const 0) (i32.const 0))))
    (if (global.get $cerune_input_eof)
      (then (return (i32.const @EMPTY@) (i32.const 0) (i64.const 0) (i32.const 1))))
    (call $cerune_string_allocate (local.get $limit))
    local.set $code
    local.set $value
    (if (i32.eq (local.get $code) (i32.const 2))
      (then (return (i32.const 0) (i32.const 4) (i64.const 0) (i32.const 1))))
    (if (local.get $code)
      (then (return (i32.const 0) (i32.const 5) (i64.const 0) (i32.const 1))))
    (local.set $known (i32.const 1))
    (block $done
      (loop $read
        call $read_stdin_byte_v1
        local.set $status
        local.set $byte
        (if (i32.or
              (i32.gt_u (local.get $status) (i32.const 3))
              (select (i32.ne (local.get $byte) (i32.const 0)) (i32.gt_u (local.get $byte) (i32.const 255)) (local.get $status)))
          (then
            (local.set $code (i32.const 7))
            (local.set $known (i32.const 0))
            (br $done)))
        (br_if $read (i32.eq (local.get $status) (i32.const 2)))
        (if (i32.eq (local.get $status) (i32.const 3))
          (then (local.set $code (i32.const 6)) (br $done)))
        (if (i32.eq (local.get $status) (i32.const 1))
          (then (global.set $cerune_input_eof (i32.const 1)) (br $done)))
        (local.set $consumed (i64.add (local.get $consumed) (i64.const 1)))
        (if (i64.eq (i64.extend_i32_u (local.get $length)) (local.get $limit))
          (then (local.set $code (i32.const 9)) (br $done)))
        (i32.store8 (i32.add (i32.add (local.get $value) (i32.const 8)) (local.get $length)) (local.get $byte))
        (local.set $length (i32.add (local.get $length) (i32.const 1)))
        (br_if $done (i32.eq (local.get $byte) (i32.const 10)))
        (br $read)))
    (if (i32.eqz (local.get $code))
      (then
        (if (i32.eqz (call $cerune_utf8 (i32.add (local.get $value) (i32.const 8)) (local.get $length)))
          (then (local.set $code (i32.const 8))))))
    (if (i32.or (local.get $code) (i32.eqz (local.get $length)))
      (then
        (call $cerune_string_release (local.get $value))
        (return (i32.const @EMPTY@) (local.get $code) (local.get $consumed) (local.get $known))))
    (global.set $cerune_heap_live (i64.add (i64.sub (global.get $cerune_heap_live) (local.get $limit)) (i64.extend_i32_u (local.get $length))))
    (i64.store (local.get $value) (i64.extend_i32_u (local.get $length)))
    (local.get $value) (i32.const 0) (local.get $consumed) (local.get $known))
"#;
