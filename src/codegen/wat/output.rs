//! private memoryを保持したまま、値で渡すversioned stdout接続を生成します。
use super::ir::{Instruction, Module, Origin};
use crate::ir::{Program, Statement, StatementKind};
use std::fmt::Write;

pub(super) fn uses_output(p: &Program) -> bool {
    fn body(ss: &[Statement]) -> bool {
        ss.iter().any(|s| match &s.kind {
            StatementKind::Print { .. } | StatementKind::Write { .. } => true,
            StatementKind::If {
                then_body,
                else_body,
                ..
            } => body(then_body) || body(else_body),
            StatementKind::For { body: b, .. } | StatementKind::While { body: b, .. } => body(b),
            _ => false,
        })
    }
    body(&p.statements) || p.function_definitions.iter().any(|f| body(&f.body))
}
pub(super) const IMPORT: &str = "  (import \"cerune\" \"write_stdout_v1\" (func $write_stdout_v1 (param i64 i32) (result i32)))\n";
pub(super) fn float_import(kind: &str) -> String {
    format!(
        "  (import \"cerune\" \"format_{kind}_v1\" (func $format_{kind}_v1 (param {kind} i32) (result i64 i32)))\n"
    )
}
fn ty(kind: &str) -> &str {
    match kind {
        "string" | "quoted" | "bool" => "i32",
        "f32" => "f32",
        "f64" => "f64",
        _ => "i64",
    }
}
fn diagnostic(text: &str, out: &mut String) {
    for b in text.bytes() {
        writeln!(out, "    i32.const {b}\n    call $write_error_byte").unwrap();
    }
}
pub(super) fn located(kind: &str, newline: bool, origin: Origin, name: &str, out: &mut String) {
    let suffix = format!(
        " node={}{} bytes={}..{}\n",
        origin.node_id.0,
        origin.span.source_id().record_field(),
        origin.span.start(),
        origin.span.end()
    );
    writeln!(
        out,
        "  ;; output {kind}, newline={newline};{}",
        suffix.trim_end()
    )
    .unwrap();
    writeln!(out, "  (func ${name} (param $value {})\n    (local $requested i64) (local $accepted i64) (local $status i32)\n    local.get $value\n    i32.const {}\n    call $cerune_output_{kind}\n    local.set $status\n    local.set $accepted\n    local.set $requested\n    local.get $status\n    if\n      local.get $status\n      i32.const 4\n      i32.eq\n      if", ty(kind), u8::from(newline)).unwrap();
    diagnostic("cerune: output formatting failed", out);
    out.push_str("      else\n");
    diagnostic("cerune: io-v1 operation=write requested=", out);
    out.push_str("    local.get $requested\n    call $cerune_output_error_u64\n");
    diagnostic(" accepted=", out);
    out.push_str("    local.get $accepted\n    call $cerune_output_error_u64\n      end\n");
    diagnostic(&suffix, out);
    out.push_str("      unreachable\n    end\n  )\n");
}
pub(super) fn float_kinds(module: &Module) -> std::collections::BTreeSet<&'static str> {
    fn walk(i: &Instruction, kinds: &mut std::collections::BTreeSet<&'static str>) {
        match i {
            Instruction::Output {
                kind: kind @ ("f32" | "f64"),
                ..
            } => {
                kinds.insert(*kind);
            }
            Instruction::If {
                then_instructions: a,
                else_instructions: b,
            }
            | Instruction::IfBool {
                then_instructions: a,
                else_instructions: b,
            } => {
                for i in a.iter().chain(b) {
                    walk(i, kinds);
                }
            }
            Instruction::Loop {
                condition_instructions: a,
                body_instructions: b,
                update_instructions: c,
                ..
            } => {
                for i in a.iter().chain(b).chain(c) {
                    walk(i, kinds);
                }
            }
            _ => {}
        }
    }
    let mut kinds = std::collections::BTreeSet::new();
    for i in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        walk(i, &mut kinds);
    }
    kinds
}
pub(super) fn support(module: &Module, out: &mut String) {
    out.push_str(COMMON);
    for kind in float_kinds(module) {
        out.push_str(&FLOAT.replace("@KIND@", kind));
    }
    if module.uses_strings {
        out.push_str(STRINGS);
        out.push_str("  (func $cerune_output_escape (param $byte i32) (result i64 i32)\n");
        for (byte, text) in crate::codegen::display::escapes() {
            let mut bits = [0u8; 8];
            bits[..text.len()].copy_from_slice(text.as_bytes());
            writeln!(out, "    (if (i32.eq (local.get $byte) (i32.const {byte}))\n      (then (return (i64.const {}) (i32.const {}))))", i64::from_le_bytes(bits), text.len()).unwrap();
        }
        out.push_str("    (i64.extend_i32_u (local.get $byte)) (i32.const 1)\n  )\n");
    }
}

// output helpers return requested/accepted/status. status: 0 success, 1 zero,
// 2 invalid response, 3 write failure, 4 formatting failure. Interrupted (-2)
// is retried only inside send; it is not reported as a completed operation.
const COMMON: &str = r#"
  (func $cerune_output_send (param $bits i64) (param $count i32) (result i32 i32)
    (local $accepted i32) (local $n i32)
    (loop $retry
      (local.set $n (call $write_stdout_v1 (local.get $bits) (local.get $count)))
      (br_if $retry (i32.eq (local.get $n) (i32.const -2)))
      (if (i32.eqz (local.get $n)) (then (return (local.get $accepted) (i32.const 1))))
      (if (i32.eq (local.get $n) (i32.const -1)) (then (return (local.get $accepted) (i32.const 3))))
      (if (i32.gt_u (local.get $n) (local.get $count)) (then (return (local.get $accepted) (i32.const 2))))
      (local.set $accepted (i32.add (local.get $accepted) (local.get $n)))
      (local.set $count (i32.sub (local.get $count) (local.get $n)))
      (if (i32.eqz (local.get $count)) (then (return (local.get $accepted) (i32.const 0))))
      (local.set $bits (i64.shr_u (local.get $bits) (i64.extend_i32_u (i32.mul (local.get $n) (i32.const 8)))))
      (br $retry))
    unreachable)
  (func $cerune_output_pack (param $ptr i32) (param $count i32) (result i64)
    (local $bits i64) (local $i i32)
    (loop $pack
      (local.set $bits (i64.or (local.get $bits) (i64.shl
        (i64.load8_u (i32.add (local.get $ptr) (local.get $i)))
        (i64.extend_i32_u (i32.mul (local.get $i) (i32.const 8))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $pack (i32.lt_u (local.get $i) (local.get $count))))
    local.get $bits)
  (func $cerune_output_bytes (param $ptr i32) (param $length i32) (param $newline i32) (result i64 i64 i32)
    (local $requested i64) (local $accepted i64) (local $count i32) (local $n i32) (local $status i32)
    (local.set $requested (i64.add (i64.extend_i32_u (local.get $length)) (i64.extend_i32_u (local.get $newline))))
    (block $end (loop $write
      (br_if $end (i32.eqz (local.get $length)))
      (local.set $count (select (i32.const 8) (local.get $length) (i32.gt_u (local.get $length) (i32.const 8))))
      (call $cerune_output_send (call $cerune_output_pack (local.get $ptr) (local.get $count)) (local.get $count))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
      (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
      (local.set $ptr (i32.add (local.get $ptr) (local.get $count)))
      (local.set $length (i32.sub (local.get $length) (local.get $count)))
      (br $write)))
    (if (local.get $newline) (then
      (call $cerune_output_send (i64.const 10) (i32.const 1))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))))
    local.get $requested local.get $accepted local.get $status)
  (func $cerune_output_integer (param $value i64) (param $negative i32) (param $newline i32) (result i64 i64 i32)
    (local $ptr i32)
    (local.set $ptr (i32.const 128))
    (loop $digits
      (local.set $ptr (i32.sub (local.get $ptr) (i32.const 1)))
      (i32.store8 (local.get $ptr) (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48)))
      (local.set $value (i64.div_u (local.get $value) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $value) (i64.const 0))))
    (if (local.get $negative) (then
      (local.set $ptr (i32.sub (local.get $ptr) (i32.const 1)))
      (i32.store8 (local.get $ptr) (i32.const 45))))
    (call $cerune_output_bytes (local.get $ptr) (i32.sub (i32.const 128) (local.get $ptr)) (local.get $newline)))
  (func $cerune_output_i64 (param $value i64) (param $newline i32) (result i64 i64 i32)
    (local $negative i32)
    (local.set $negative (i64.lt_s (local.get $value) (i64.const 0)))
    (if (local.get $negative) (then (local.set $value (i64.sub (i64.const 0) (local.get $value)))))
    (call $cerune_output_integer (local.get $value) (local.get $negative) (local.get $newline)))
  (func $cerune_output_u64 (param $value i64) (param $newline i32) (result i64 i64 i32)
    (call $cerune_output_integer (local.get $value) (i32.const 0) (local.get $newline)))
  (func $cerune_output_bool (param $value i32) (param $newline i32) (result i64 i64 i32)
    (i64.store (i32.const 0) (select (i64.const 1702195828) (i64.const 435728179558) (local.get $value)))
    (call $cerune_output_bytes (i32.const 0) (select (i32.const 4) (i32.const 5) (local.get $value)) (local.get $newline)))
  (func $cerune_output_error_u64 (param $value i64)
    (if (i64.ge_u (local.get $value) (i64.const 10))
      (then (call $cerune_output_error_u64 (i64.div_u (local.get $value) (i64.const 10)))))
    (call $write_error_byte (i32.add (i32.wrap_i64 (i64.rem_u (local.get $value) (i64.const 10))) (i32.const 48))))
"#;

const FLOAT: &str = r#"
  (func $cerune_output_@KIND@ (param $value @KIND@) (param $newline i32) (result i64 i64 i32)
    (local $length i32) (local $reported i32) (local $offset i32) (local $bits i64) (local $byte i32)
    ;; Formatting has no output side effects. Snapshot all bytes before sending.
    (call $format_@KIND@_v1 (local.get $value) (i32.const 0))
    local.set $length local.set $bits
    (if (i32.or (i32.lt_s (local.get $length) (i32.const 1)) (i32.gt_u (local.get $length) (i32.const 128)))
      (then (return (i64.const 0) (i64.const 0) (i32.const 4))))
    (i64.store (i32.const 0) (local.get $bits))
    (local.set $offset (i32.const 8))
    (block $ready (loop $format
      (br_if $ready (i32.ge_u (local.get $offset) (local.get $length)))
      (call $format_@KIND@_v1 (local.get $value) (local.get $offset))
      local.set $reported local.set $bits
      (if (i32.ne (local.get $reported) (local.get $length))
        (then (return (i64.const 0) (i64.const 0) (i32.const 4))))
      (i64.store (local.get $offset) (local.get $bits))
      (local.set $offset (i32.add (local.get $offset) (i32.const 8)))
      (br $format)))
    (local.set $offset (i32.const 0))
    (loop $validate
      (local.set $byte (i32.load8_u (local.get $offset)))
      (if (i32.or (i32.lt_u (local.get $byte) (i32.const 33)) (i32.gt_u (local.get $byte) (i32.const 126)))
        (then (return (i64.const 0) (i64.const 0) (i32.const 4))))
      (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
      (br_if $validate (i32.lt_u (local.get $offset) (local.get $length))))
    (call $cerune_output_bytes (i32.const 0) (local.get $length) (local.get $newline)))
"#;

const STRINGS: &str = r#"
  (func $cerune_output_string (param $value i32) (param $newline i32) (result i64 i64 i32)
    (call $cerune_output_bytes (i32.add (local.get $value) (i32.const 8)) (i32.load (local.get $value)) (local.get $newline)))
  (func $cerune_output_quoted (param $value i32) (param $newline i32) (result i64 i64 i32)
    (local $length i32) (local $index i32) (local $bits i64) (local $count i32)
    (local $requested i64) (local $accepted i64) (local $status i32) (local $n i32)
    (local.set $length (i32.load (local.get $value)))
    (local.set $requested (i64.add (i64.const 2) (i64.extend_i32_u (local.get $newline))))
    (block $counted (loop $counting
      (br_if $counted (i32.eq (local.get $index) (local.get $length)))
      (call $cerune_output_escape (i32.load8_u offset=8 (i32.add (local.get $value) (local.get $index))))
      local.set $count drop
      (local.set $requested (i64.add (local.get $requested) (i64.extend_i32_u (local.get $count))))
      (local.set $index (i32.add (local.get $index) (i32.const 1)))
      (br $counting)))
    (call $cerune_output_send (i64.const 34) (i32.const 1))
    local.set $status local.set $n
    (local.set $accepted (i64.extend_i32_u (local.get $n)))
    (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
    (local.set $index (i32.const 0))
    (block $written (loop $writing
      (br_if $written (i32.eq (local.get $index) (local.get $length)))
      (call $cerune_output_escape (i32.load8_u offset=8 (i32.add (local.get $value) (local.get $index))))
      local.set $count local.set $bits
      (call $cerune_output_send (local.get $bits) (local.get $count))
      local.set $status local.set $n
      (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
      (if (local.get $status) (then (return (local.get $requested) (local.get $accepted) (local.get $status))))
      (local.set $index (i32.add (local.get $index) (i32.const 1)))
      (br $writing)))
    (call $cerune_output_send (select (i64.const 2594) (i64.const 34) (local.get $newline))
      (i32.add (i32.const 1) (local.get $newline)))
    local.set $status local.set $n
    (local.set $accepted (i64.add (local.get $accepted) (i64.extend_i32_u (local.get $n))))
    local.get $requested local.get $accepted local.get $status)
"#;
