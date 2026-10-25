//! Private invocation storage. Frames share the allocation cursor with heaps,
//! but have their own free list and never change string/array live-byte budgets.
use super::ir::{Module, Origin};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    module.call_limit.is_some()
        || module.entry_frame_size > 0
        || module.functions.iter().any(|f| f.frame_size > 0)
}
pub(super) fn support(module: &Module, out: &mut String) {
    if !used(module) {
        return;
    }
    if module.string_heap_limit.is_none() && module.array_heap_limit.is_none() {
        writeln!(
            out,
            "  (global $cerune_heap_end (mut i64) (i64.const {}))",
            module.string_heap_start
        )
        .unwrap();
    }
    out.push_str(SOURCE);
}
pub(super) fn allocate(size: usize, origin: Option<Origin>, prefix: &str, out: &mut String) {
    if size == 0 {
        writeln!(out, "{prefix}i32.const 0").unwrap();
        return;
    }
    writeln!(out,"{prefix};; invocation frame: {size} bytes\n{prefix}i64.const {size}\n{prefix}call $cerune_frame_allocate\n{prefix}local.tee $cerune.frame.next\n{prefix}i32.eqz\n{prefix}if").unwrap();
    let indent = format!("{prefix}  ");
    if let Some(origin) = origin {
        super::failure::emit(
            crate::runtime::RuntimeFailure {
                code: crate::runtime::FailureCode::AllocationFailed,
                node_id: origin.node_id,
                span: origin.span,
            },
            &indent,
            out,
        );
    } else {
        let message = "cerune: runtime-v1 code=allocation-failed origin=host\n";
        for byte in message.bytes() {
            writeln!(
                out,
                "{indent}i32.const {byte}\n{indent}call $write_error_byte"
            )
            .unwrap();
        }
        writeln!(out, "{indent}unreachable").unwrap();
    }
    writeln!(out, "{prefix}end\n{prefix}local.get $cerune.frame.next").unwrap();
}
pub(super) fn release(prefix: &str, out: &mut String) {
    writeln!(
        out,
        "{prefix}local.get $cerune.frame.base\n{prefix}call $cerune_frame_release"
    )
    .unwrap();
}
const SOURCE: &str = r#"
  ;; frame block: next:u32, padding:u32, capacity:u64, payload (8-byte aligned).
  (global $cerune_frame_free (mut i32) (i32.const 0))
  (func $cerune_frame_allocate (param $size i64) (result i32)
    (local $p i32) (local $prev i32) (local $next i32)
    (local $bytes i64) (local $end i64) (local $pages i64)
    (if (i64.gt_u (local.get $size) (i64.const 4294967272))
      (then (return (i32.const 0))))
    (local.set $p (global.get $cerune_frame_free))
    (block $new
      (loop $search
        (br_if $new (i32.eqz (local.get $p)))
        (if (i64.ge_u (i64.load offset=8 (local.get $p)) (local.get $size))
          (then
            (local.set $next (i32.load (local.get $p)))
            (if (i32.eqz (local.get $prev))
              (then (global.set $cerune_frame_free (local.get $next)))
              (else (i32.store (local.get $prev) (local.get $next))))
            (return (i32.add (local.get $p) (i32.const 16)))))
        (local.set $prev (local.get $p))
        (local.set $p (i32.load (local.get $p)))
        (br $search)))
    (local.set $bytes (i64.and (i64.add (local.get $size) (i64.const 23)) (i64.const -8)))
    (local.set $end (i64.add (global.get $cerune_heap_end) (local.get $bytes)))
    (if (i64.gt_u (local.get $end) (i64.const 4294967296))
      (then (return (i32.const 0))))
    (local.set $pages (i64.div_u (i64.add (local.get $end) (i64.const 65535)) (i64.const 65536)))
    (if (i64.gt_u (local.get $pages) (i64.extend_i32_u (memory.size)))
      (then
        (if (i32.eq (memory.grow (i32.wrap_i64 (i64.sub (local.get $pages) (i64.extend_i32_u (memory.size))))) (i32.const -1))
          (then (return (i32.const 0))))))
    (local.set $p (i32.wrap_i64 (global.get $cerune_heap_end)))
    (global.set $cerune_heap_end (local.get $end))
    (i64.store offset=8 (local.get $p) (i64.sub (local.get $bytes) (i64.const 16)))
    (i32.add (local.get $p) (i32.const 16)))
  (func $cerune_frame_release (param $base i32) (local $p i32)
    (if (i32.eqz (local.get $base)) (then (return)))
    (local.set $p (i32.sub (local.get $base) (i32.const 16)))
    (i32.store (local.get $p) (global.get $cerune_frame_free))
    (global.set $cerune_frame_free (local.get $p)))
"#;
