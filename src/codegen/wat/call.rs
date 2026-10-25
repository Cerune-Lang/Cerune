use super::ir::{Instruction, Module, Origin};
use crate::codegen::call::Site;
use std::fmt::Write;
fn site(function: usize, origin: Option<Origin>) -> Site {
    Site {
        function,
        origin: origin.map(|o| (o.node_id, o.span)),
    }
}
fn sites(m: &Module) -> Vec<Site> {
    fn walk(m: &Module, body: &[Instruction], sites: &mut Vec<Site>) {
        for i in body {
            match i {
                Instruction::Call {
                    function_id,
                    origin,
                } if m.source_functions[*function_id] => {
                    let s = site(*function_id, Some(*origin));
                    if !sites.contains(&s) {
                        sites.push(s);
                    }
                }
                Instruction::If {
                    then_instructions,
                    else_instructions,
                }
                | Instruction::IfBool {
                    then_instructions,
                    else_instructions,
                } => {
                    walk(m, then_instructions, sites);
                    walk(m, else_instructions, sites);
                }
                Instruction::Loop {
                    condition_instructions,
                    body_instructions,
                    update_instructions,
                    ..
                } => {
                    walk(m, condition_instructions, sites);
                    walk(m, body_instructions, sites);
                    walk(m, update_instructions, sites);
                }
                _ => {}
            }
        }
    }
    let mut result = Vec::new();
    walk(m, &m.instructions, &mut result);
    for f in &m.functions {
        walk(m, &f.instructions, &mut result);
    }
    if let Some(id) = m.explicit_main
        && m.source_functions[id]
    {
        result.push(site(id, None));
    }
    result
}
pub(super) fn enter(
    m: &Module,
    function: usize,
    origin: Option<Origin>,
    prefix: &str,
    out: &mut String,
) {
    if m.call_limit.is_some() && m.source_functions[function] {
        let id = sites(m)
            .iter()
            .position(|s| *s == site(function, origin))
            .unwrap();
        writeln!(
            out,
            "{prefix}i32.const {id}\n{prefix}call $cerune_call_enter"
        )
        .unwrap();
    }
}
pub(super) fn leave(m: &Module, function: usize, prefix: &str, out: &mut String) {
    if m.call_limit.is_some() && m.source_functions[function] {
        writeln!(out, "{prefix}call $cerune_call_leave").unwrap();
    }
}
fn bytes(value: &str, out: &mut String) {
    writeln!(out, "    ;; {}", value.trim_end()).unwrap();
    for b in value.bytes() {
        writeln!(out, "    i32.const {b}\n    call $write_error_byte").unwrap();
    }
}
pub(super) fn support(m: &Module, out: &mut String) {
    let Some(limit) = m.call_limit else { return };
    out.push_str("  (func $cerune_call_site_write (param $site i32) (param $allocation i32)\n");
    for (id, s) in sites(m).into_iter().enumerate() {
        writeln!(out,"    (if (i32.eq (local.get $site) (i32.const {id})) (then\n      (if (local.get $allocation) (then").unwrap();
        bytes(&s.allocation(), out);
        out.push_str("      ) (else\n");
        bytes(&s.suffix(), out);
        out.push_str("      ))\n      return\n    ))\n");
    }
    out.push_str("    unreachable\n  )\n  (func $cerune_call_header\n");
    bytes(&format!("call-v1 reason=depth-limit limit={limit}"), out);
    out.push_str("  )\n  (func $cerune_call_prefix\n");
    bytes("call-v1 frame=", out);
    out.push_str("  )\n");
    out.push_str(&SOURCE.replace("$LIMIT", &limit.to_string()));
}
const SOURCE: &str = r#"
  (global $cerune_call_depth (mut i64) (i64.const 0))
  (global $cerune_call_head (mut i32) (i32.const 0))
  (global $cerune_call_tail (mut i32) (i32.const 0))
  ;; Private linked history: next:u32, previous:u32, immutable-site-id:u32.
  (func $cerune_call_number (param $n i64) (local $divisor i64) (local $started i32) (local $digit i64)
    (local.set $divisor (i64.const 1000000000))
    (loop $digits
      (local.set $digit (i64.div_u (local.get $n) (local.get $divisor)))
      (if (i32.or (local.get $started) (i32.or (i64.ne (local.get $digit) (i64.const 0)) (i64.eq (local.get $divisor) (i64.const 1))))
        (then
          (call $write_error_byte (i32.add (i32.wrap_i64 (local.get $digit)) (i32.const 48)))
          (local.set $started (i32.const 1))))
      (local.set $n (i64.rem_u (local.get $n) (local.get $divisor)))
      (local.set $divisor (i64.div_u (local.get $divisor) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $divisor) (i64.const 0)))))
  (func $cerune_call_enter (param $site i32) (local $p i32) (local $i i64)
    (if (i64.ge_u (global.get $cerune_call_depth) (i64.const $LIMIT))
      (then
        (call $cerune_call_header)
        (call $cerune_call_site_write (local.get $site) (i32.const 0))
        (local.set $p (global.get $cerune_call_head))
        (loop $history
          (call $cerune_call_prefix)
          (call $cerune_call_number (local.get $i))
          (call $cerune_call_site_write (i32.load offset=8 (local.get $p)) (i32.const 0))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (local.set $p (i32.load (local.get $p)))
          (br_if $history (local.get $p)))
        unreachable))
    (local.set $p (call $cerune_frame_allocate (i64.const 16)))
    (if (i32.eqz (local.get $p))
      (then (call $cerune_call_site_write (local.get $site) (i32.const 1)) unreachable))
    (i32.store (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (global.get $cerune_call_tail))
    (i32.store offset=8 (local.get $p) (local.get $site))
    (if (global.get $cerune_call_tail)
      (then (i32.store (global.get $cerune_call_tail) (local.get $p)))
      (else (global.set $cerune_call_head (local.get $p))))
    (global.set $cerune_call_tail (local.get $p))
    (global.set $cerune_call_depth (i64.add (global.get $cerune_call_depth) (i64.const 1))))
  (func $cerune_call_leave (local $p i32)
    (local.set $p (global.get $cerune_call_tail))
    (global.set $cerune_call_tail (i32.load offset=4 (local.get $p)))
    (if (global.get $cerune_call_tail)
      (then (i32.store (global.get $cerune_call_tail) (i32.const 0)))
      (else (global.set $cerune_call_head (i32.const 0))))
    (call $cerune_frame_release (local.get $p))
    (global.set $cerune_call_depth (i64.sub (global.get $cerune_call_depth) (i64.const 1))))
"#;
