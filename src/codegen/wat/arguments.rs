use super::ir::{Instruction, Module, Origin};
use std::fmt::Write;
pub(super) const IMPORT: &str = "  (import \"cerune\" \"arguments_count_v1\" (func $arguments_count_v1 (result i64 i32)))\n  (import \"cerune\" \"argument_length_v1\" (func $argument_length_v1 (param i64) (result i64 i32)))\n  (import \"cerune\" \"argument_byte_v1\" (func $argument_byte_v1 (param i64 i64) (result i32 i32)))\n";
const CODES: &[&str] = &[
    "invalid-limit",
    "count-limit-exceeded",
    "byte-limit-exceeded",
    "invalid-utf8",
    "embedded-nul",
    "allocation-failed",
    "unconfigured",
    "invalid-index",
    "index-out-of-bounds",
    "budget-exceeded",
    "invalid-response",
    "connection-failed",
];
fn text(value: &str, out: &mut String) {
    for b in value.bytes() {
        writeln!(out, "    i32.const {b}\n    call $write_error_byte").unwrap();
    }
}
pub(super) fn located(instruction: &Instruction, origin: Origin, name: &str, out: &mut String) {
    let get = matches!(instruction, Instruction::ArgumentGet);
    writeln!(
        out,
        "  (func ${name} {} (result {})\n    (local $code i32) (local $value i32)",
        if get { "(param $index i64)" } else { "" },
        if get { "i32" } else { "i64" }
    )
    .unwrap();
    if get {
        out.push_str("    local.get $index\n    call $cerune_arguments_get\n    local.set $code\n    local.set $value\n");
    } else {
        out.push_str("    (if (i32.eqz (global.get $cerune_arguments_configured)) (then (local.set $code (i32.const 7))))\n");
    }
    out.push_str("    local.get $code\n    if\n      call $flush_stdout_v1\n      drop\n");
    text(
        if get {
            "cerune: arguments-v1 phase=execute operation=get code="
        } else {
            "cerune: arguments-v1 phase=execute operation=count code="
        },
        out,
    );
    out.push_str("      local.get $code\n      call $cerune_arguments_code\n");
    if get {
        text(" index=", out);
        out.push_str("      local.get $index\n      call $cerune_arguments_error_i64\n");
    }
    text(
        &format!(
            " node={}{} bytes={}..{}\n",
            origin.node_id.0,
            origin.span.source_id().record_field(),
            origin.span.start(),
            origin.span.end()
        ),
        out,
    );
    out.push_str("      unreachable\n    end\n");
    out.push_str(if get {
        "    local.get $value\n  )\n"
    } else {
        "    global.get $cerune_arguments_count\n  )\n"
    });
}
pub(super) fn support(module: &Module, out: &mut String) {
    writeln!(out,"  (global $cerune_arguments_max_count i64 (i64.const {}))\n  (global $cerune_arguments_max_bytes i64 (i64.const {}))",module.argument_limits.max_count,module.argument_limits.max_bytes).unwrap();
    out.push_str("  (func $cerune_arguments_code (param $code i32)\n");
    for (i, code) in CODES.iter().enumerate() {
        writeln!(
            out,
            "    (if (i32.eq (local.get $code) (i32.const {})) (then",
            i + 1
        )
        .unwrap();
        text(code, out);
        out.push_str("    ))\n");
    }
    out.push_str(
        "  )\n  (func $cerune_arguments_capture_failed (param $code i32) (param $index i64)\n",
    );
    text("cerune: arguments-v1 phase=capture code=", out);
    out.push_str("    local.get $code\n    call $cerune_arguments_code\n    (if (i64.ge_s (local.get $index) (i64.const 0)) (then\n");
    text(" index=", out);
    out.push_str("    local.get $index\n    call $cerune_arguments_error_i64\n    ))\n");
    text("\n", out);
    out.push_str("    unreachable\n  )\n");
    // Use the same UTF-8 transition rules as stdin, reading a frozen host snapshot.
    out.push_str(
        &super::utf8::SUPPORT
            .replace(
                "$cerune_utf8 (param $data i32) (param $length i32)",
                "$cerune_arguments_utf8 (param $index i64) (param $length i64)",
            )
            .replace("(local $i i32)", "(local $i i64)")
            .replace(
                "(i32.eq (local.get $i) (local.get $length))",
                "(i64.eq (local.get $i) (local.get $length))",
            )
            .replace(
                "(i32.load8_u (i32.add (local.get $data) (local.get $i)))",
                "(call $cerune_arguments_byte (local.get $index) (local.get $i))",
            )
            .replace(
                "(i32.add (local.get $i) (i32.const 1))",
                "(i64.add (local.get $i) (i64.const 1))",
            ),
    );
    out.push_str(SOURCE);
}
const SOURCE: &str = r#"
  (global $cerune_arguments_started (mut i32) (i32.const 0))
  (global $cerune_arguments_configured (mut i32) (i32.const 0))
  (global $cerune_arguments_count (mut i64) (i64.const 0))
  (global $cerune_arguments_table (mut i32) (i32.const 0))
  (func $cerune_arguments_error_u64 (param $n i64)
    (if (i64.ge_u (local.get $n) (i64.const 10))
      (then (call $cerune_arguments_error_u64 (i64.div_u (local.get $n) (i64.const 10)))))
    (call $write_error_byte (i32.add (i32.wrap_i64 (i64.rem_u (local.get $n) (i64.const 10))) (i32.const 48))))
  (func $cerune_arguments_error_i64 (param $n i64)
    (if (i64.lt_s (local.get $n) (i64.const 0))
      (then (call $write_error_byte (i32.const 45)) (local.set $n (i64.sub (i64.const 0) (local.get $n)))))
    (call $cerune_arguments_error_u64 (local.get $n)))
  (func $cerune_arguments_status (param $status i32) (param $index i64)
    (if (i32.eq (local.get $status) (i32.const 2))
      (then (call $cerune_arguments_capture_failed (i32.const 12) (local.get $index))))
    (if (local.get $status)
      (then (call $cerune_arguments_capture_failed (i32.const 11) (local.get $index)))))
  (func $cerune_arguments_length (param $index i64) (result i64)
    (local $length i64) (local $status i32)
    (call $argument_length_v1 (local.get $index))
    local.set $status
    local.set $length
    (call $cerune_arguments_status (local.get $status) (local.get $index))
    (if (i64.lt_s (local.get $length) (i64.const 0))
      (then (call $cerune_arguments_capture_failed (i32.const 11) (local.get $index))))
    (local.get $length))
  (func $cerune_arguments_byte (param $index i64) (param $offset i64) (result i32)
    (local $byte i32) (local $status i32)
    (call $argument_byte_v1 (local.get $index) (local.get $offset))
    local.set $status
    local.set $byte
    (call $cerune_arguments_status (local.get $status) (local.get $index))
    (if (i32.gt_u (local.get $byte) (i32.const 255))
      (then (call $cerune_arguments_capture_failed (i32.const 11) (local.get $index))))
    (local.get $byte))
  ;; Snapshot storage shares physical memory, not the returned-string budget or owner list.
  (func $cerune_arguments_reserve (param $size i64) (result i32)
    (local $end i64) (local $pages i64)
    (if (i64.gt_u (local.get $size) (i64.sub (i64.const 4294967296) (global.get $cerune_heap_end))) (then (return (i32.const 0))))
    (local.set $end (i64.and (i64.add (i64.add (global.get $cerune_heap_end) (local.get $size)) (i64.const 7)) (i64.const -8)))
    (if (i64.gt_u (local.get $end) (i64.const 4294967296)) (then (return (i32.const 0))))
    (local.set $pages (i64.div_u (i64.add (local.get $end) (i64.const 65535)) (i64.const 65536)))
    (if (i64.gt_u (local.get $pages) (i64.extend_i32_u (memory.size)))
      (then (if (i32.eq (memory.grow (i32.wrap_i64 (i64.sub (local.get $pages) (i64.extend_i32_u (memory.size))))) (i32.const -1)) (then (return (i32.const 0))))))
    (global.set $cerune_heap_end (local.get $end))
    (i32.const 1))
  (func $cerune_arguments_start
    (local $count i64) (local $status i32) (local $i i64) (local $j i64)
    (local $length i64) (local $bytes i64) (local $table_size i64)
    (local $base i64) (local $data i64) (local $end i64) (local $slot i32)
    (if (global.get $cerune_arguments_started) (then unreachable))
    (global.set $cerune_arguments_started (i32.const 1))
    (call $arguments_count_v1)
    local.set $status
    local.set $count
    (if (i32.eq (local.get $status) (i32.const 1))
      (then
        (if (i64.ne (local.get $count) (i64.const 0)) (then (call $cerune_arguments_capture_failed (i32.const 11) (i64.const -1))))
        return))
    (call $cerune_arguments_status (local.get $status) (i64.const -1))
    (if (i64.lt_s (local.get $count) (i64.const 0)) (then (call $cerune_arguments_capture_failed (i32.const 11) (i64.const -1))))
    (if (i64.gt_u (global.get $cerune_arguments_max_count) (i64.const 9223372036854775807)) (then (call $cerune_arguments_capture_failed (i32.const 1) (i64.const -1))))
    (if (i64.gt_u (local.get $count) (global.get $cerune_arguments_max_count)) (then (call $cerune_arguments_capture_failed (i32.const 2) (i64.const -1))))
    (block $validated (loop $validate
      (br_if $validated (i64.eq (local.get $i) (local.get $count)))
      (local.set $length (call $cerune_arguments_length (local.get $i)))
      (if (i64.gt_u (local.get $length) (i64.sub (global.get $cerune_arguments_max_bytes) (local.get $bytes)))
        (then (call $cerune_arguments_capture_failed (i32.const 3) (local.get $i))))
      (if (i32.eqz (call $cerune_arguments_utf8 (local.get $i) (local.get $length)))
        (then (call $cerune_arguments_capture_failed (i32.const 4) (local.get $i))))
      (local.set $j (i64.const 0))
      (block $nul_done (loop $nul
        (br_if $nul_done (i64.eq (local.get $j) (local.get $length)))
        (if (i32.eqz (call $cerune_arguments_byte (local.get $i) (local.get $j))) (then (call $cerune_arguments_capture_failed (i32.const 5) (local.get $i))))
        (local.set $j (i64.add (local.get $j) (i64.const 1))) (br $nul)))
      (local.set $bytes (i64.add (local.get $bytes) (local.get $length)))
      (local.set $i (i64.add (local.get $i) (i64.const 1))) (br $validate)))
    (if (i64.gt_u (local.get $count) (i64.const 268435455)) (then (call $cerune_arguments_capture_failed (i32.const 6) (i64.const -1))))
    (local.set $table_size (i64.mul (local.get $count) (i64.const 16)))
    (if (i64.gt_u (local.get $bytes) (i64.sub (i64.const 4294967296) (local.get $table_size))) (then (call $cerune_arguments_capture_failed (i32.const 6) (i64.const -1))))
    (local.set $base (global.get $cerune_heap_end))
    (if (i32.eqz (call $cerune_arguments_reserve (i64.add (local.get $table_size) (local.get $bytes)))) (then (call $cerune_arguments_capture_failed (i32.const 6) (i64.const -1))))
    (local.set $data (i64.add (local.get $base) (local.get $table_size)))
    (local.set $end (i64.add (local.get $data) (local.get $bytes)))
    (local.set $i (i64.const 0))
    (block $copied (loop $copy
      (br_if $copied (i64.eq (local.get $i) (local.get $count)))
      (local.set $length (call $cerune_arguments_length (local.get $i)))
      (if (i64.gt_u (local.get $length) (i64.sub (local.get $end) (local.get $data))) (then (call $cerune_arguments_capture_failed (i32.const 11) (local.get $i))))
      (local.set $slot (i32.wrap_i64 (i64.add (local.get $base) (i64.mul (local.get $i) (i64.const 16)))))
      (i32.store (local.get $slot) (i32.wrap_i64 (local.get $data)))
      (i64.store offset=8 (local.get $slot) (local.get $length))
      (local.set $j (i64.const 0))
      (block $bytes_done (loop $copy_bytes
        (br_if $bytes_done (i64.eq (local.get $j) (local.get $length)))
        (i32.store8 (i32.wrap_i64 (i64.add (local.get $data) (local.get $j))) (call $cerune_arguments_byte (local.get $i) (local.get $j)))
        (local.set $j (i64.add (local.get $j) (i64.const 1))) (br $copy_bytes)))
      (local.set $data (i64.add (local.get $data) (local.get $length)))
      (local.set $i (i64.add (local.get $i) (i64.const 1))) (br $copy)))
    (if (i64.ne (local.get $data) (local.get $end)) (then (call $cerune_arguments_capture_failed (i32.const 11) (i64.const -1))))
    (global.set $cerune_arguments_table (i32.wrap_i64 (local.get $base)))
    (global.set $cerune_arguments_count (local.get $count))
    (global.set $cerune_arguments_configured (i32.const 1)))
  (func $cerune_arguments_get (param $index i64) (result i32 i32)
    (local $slot i32) (local $length i64) (local $value i32) (local $code i32)
    (if (i32.eqz (global.get $cerune_arguments_configured)) (then (return (i32.const 0) (i32.const 7))))
    (if (i64.lt_s (local.get $index) (i64.const 0)) (then (return (i32.const 0) (i32.const 8))))
    (if (i64.ge_u (local.get $index) (global.get $cerune_arguments_count)) (then (return (i32.const 0) (i32.const 9))))
    (local.set $slot (i32.add (global.get $cerune_arguments_table) (i32.wrap_i64 (i64.mul (local.get $index) (i64.const 16)))))
    (local.set $length (i64.load offset=8 (local.get $slot)))
    (call $cerune_string_allocate (local.get $length)) local.set $code local.set $value
    (if (i32.eq (local.get $code) (i32.const 2)) (then (return (i32.const 0) (i32.const 10))))
    (if (local.get $code) (then (return (i32.const 0) (i32.const 6))))
    (if (i64.ne (local.get $length) (i64.const 0)) (then (memory.copy (i32.add (local.get $value) (i32.const 8)) (i32.load (local.get $slot)) (i32.wrap_i64 (local.get $length)))))
    (local.get $value) (i32.const 0))
"#;
