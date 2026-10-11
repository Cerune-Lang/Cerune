//! 共通IRの配列所有命令を非公開Wasm memoryへ具体化します。
//! header: next/free/i64 refs/initialized/length/logical bytes/stride/capacity (56 bytes)。
//! refs=0でも要素の逆順解放中は生存し、ArrayFreeで初めて再利用可能にします。
use super::ir::{Instruction, Origin};
use crate::runtime::FailureCode;
use std::fmt::Write;

pub(super) fn support(limit: u64, start: usize, owns_cursor: bool) -> String {
    let cursor = if owns_cursor {
        format!("  (global $cerune_heap_end (mut i64) (i64.const {start}))\n")
    } else {
        String::new()
    };
    cursor + &SOURCE.replace("@LIMIT@", &limit.to_string())
}

pub(super) fn checked(instruction: &Instruction, origin: Origin, name: &str, out: &mut String) {
    match instruction {
        Instruction::ArrayAllocate { width, stride } => {
            writeln!(out, "  (func \u{24}{name} (param $length i64) (result i32)\n    (local $value i32) (local $error i32)").unwrap();
            super::failure::emit_if(
                "    local.get $length\n    i64.const 0\n    i64.lt_s\n",
                FailureCode::ArrayLengthOutOfRange,
                origin,
                out,
            );
            writeln!(out, "    local.get $length\n    i64.const {width}\n    i64.const {stride}\n    call $cerune_array_allocate\n    local.set $error\n    local.set $value").unwrap();
            for (number, code) in [
                (1, FailureCode::AllocationSizeOverflow),
                (2, FailureCode::AllocationLimitExceeded),
                (3, FailureCode::AllocationFailed),
            ] {
                super::failure::emit_if(
                    &format!("    local.get $error\n    i32.const {number}\n    i32.eq\n"),
                    code,
                    origin,
                    out,
                );
            }
            out.push_str("    local.get $value\n  )\n");
        }
        Instruction::ArrayAddress => {
            writeln!(
                out,
                "  (func \u{24}{name} (param $owner i32) (param $index i64) (result i32)"
            )
            .unwrap();
            super::failure::emit_if(
                "    (i32.or (i64.lt_s (local.get $index) (i64.const 0))\n      (i64.ge_s (local.get $index) (call $cerune_array_length (local.get $owner))))\n",
                FailureCode::ArrayIndexOutOfBounds,
                origin,
                out,
            );
            out.push_str("    (if (i64.ge_u (local.get $index) (i64.load offset=16 (local.get $owner))) (then unreachable))\n    (call $cerune_array_element_address (local.get $owner) (local.get $index))\n  )\n");
        }
        Instruction::ArrayRangeCheck => {
            writeln!(
                out,
                "  (func \u{24}{name} (param $length i64) (param $start i64) (param $end i64)"
            )
            .unwrap();
            super::failure::emit_if(
                "    (i32.or (i64.lt_s (local.get $start) (i64.const 0))\n      (i32.or (i64.lt_s (local.get $end) (local.get $start)) (i64.gt_s (local.get $end) (local.get $length))))\n",
                FailureCode::ArrayRangeOutOfBounds,
                origin,
                out,
            );
            out.push_str("  )\n");
        }
        _ => unreachable!(),
    }
}

const SOURCE: &str = r#"
  (global $cerune_array_head (mut i32) (i32.const 0))
  (global $cerune_array_live (mut i64) (i64.const 0))
  (func $cerune_array_allocate (param $length i64) (param $width i64) (param $stride i64) (result i32 i32)
    (local $bytes i64) (local $physical i64) (local $size i64)
    (local $end i64) (local $pages i64) (local $p i32)
    ;; i32へ狭める前に論理サイズとWasm32の物理サイズを検査します。
    (if (i64.lt_s (local.get $length) (i64.const 0))
      (then (return (i32.const 0) (i32.const 1))))
    (if (i64.eqz (local.get $length))
      (then (return (i32.const 0) (i32.const 0))))
    (if (i32.or
          (i64.gt_u (local.get $width) (i64.div_u (i64.const 9223372036854775807) (local.get $length)))
          (i64.gt_u (local.get $stride) (i64.div_u (i64.const 4294967232) (local.get $length))))
      (then (return (i32.const 0) (i32.const 1))))
    (local.set $bytes (i64.mul (local.get $length) (local.get $width)))
    (local.set $physical (i64.mul (local.get $length) (local.get $stride)))
    (if (i64.gt_u (local.get $bytes) (i64.sub (i64.const @LIMIT@) (global.get $cerune_array_live)))
      (then (return (i32.const 0) (i32.const 2))))
    (local.set $p (global.get $cerune_array_head))
    (block $found
      (loop $search
        (br_if $found (i32.eqz (local.get $p)))
        (br_if $found (i32.and (i32.load offset=4 (local.get $p))
            (i64.ge_u (i64.load offset=48 (local.get $p)) (local.get $physical))))
        (local.set $p (i32.load (local.get $p)))
        (br $search)))
    (if (i32.eqz (local.get $p))
      (then
        (local.set $size (i64.and (i64.add (local.get $physical) (i64.const 63)) (i64.const -8)))
        (local.set $end (i64.add (global.get $cerune_heap_end) (local.get $size)))
        (if (i64.gt_u (local.get $end) (i64.const 4294967296))
          (then (return (i32.const 0) (i32.const 3))))
        (local.set $pages (i64.div_u (i64.add (local.get $end) (i64.const 65535)) (i64.const 65536)))
        (if (i64.gt_u (local.get $pages) (i64.extend_i32_u (memory.size)))
          (then
            (if (i32.eq (memory.grow (i32.wrap_i64 (i64.sub (local.get $pages) (i64.extend_i32_u (memory.size))))) (i32.const -1))
              (then (return (i32.const 0) (i32.const 3))))))
        (local.set $p (i32.wrap_i64 (global.get $cerune_heap_end)))
        (global.set $cerune_heap_end (local.get $end))
        (i32.store (local.get $p) (global.get $cerune_array_head))
        (global.set $cerune_array_head (local.get $p))
        (i64.store offset=48 (local.get $p) (i64.sub (local.get $size) (i64.const 56)))))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i64.store offset=8 (local.get $p) (i64.const 1))
    (i64.store offset=16 (local.get $p) (i64.const 0))
    (i64.store offset=24 (local.get $p) (local.get $length))
    (i64.store offset=32 (local.get $p) (local.get $bytes))
    (i64.store offset=40 (local.get $p) (local.get $stride))
    (global.set $cerune_array_live (i64.add (global.get $cerune_array_live) (local.get $bytes)))
    (local.get $p) (i32.const 0))
  (func $cerune_array_length (param $p i32) (result i64)
    (if (result i64) (i32.eqz (local.get $p))
      (then (i64.const 0)) (else (i64.load offset=24 (local.get $p)))))
  (func $cerune_array_element_address (param $p i32) (param $index i64) (result i32)
    (i32.add (i32.add (local.get $p) (i32.const 56))
      (i32.wrap_i64 (i64.mul (local.get $index) (i64.load offset=40 (local.get $p))))))
  (func $cerune_array_init_address (param $p i32) (result i32)
    (if (i32.eqz (local.get $p)) (then unreachable))
    (if (i64.ge_u (i64.load offset=16 (local.get $p)) (i64.load offset=24 (local.get $p))) (then unreachable))
    (call $cerune_array_element_address (local.get $p) (i64.load offset=16 (local.get $p))))
  (func $cerune_array_initialized (param $p i32)
    (i64.store offset=16 (local.get $p) (i64.add (i64.load offset=16 (local.get $p)) (i64.const 1))))
  (func $cerune_array_retain (param $p i32)
    (local $refs i64)
    (if (i32.eqz (local.get $p)) (then return))
    (local.set $refs (i64.load offset=8 (local.get $p)))
    (if (i32.or (i64.eqz (local.get $refs)) (i64.eq (local.get $refs) (i64.const -1))) (then unreachable))
    (i64.store offset=8 (local.get $p) (i64.add (local.get $refs) (i64.const 1))))
  (func $cerune_array_release_owner (param $p i32) (result i32)
    (local $refs i64)
    (if (i32.eqz (local.get $p)) (then (return (i32.const 1))))
    (local.set $refs (i64.load offset=8 (local.get $p)))
    (if (i64.eqz (local.get $refs)) (then unreachable))
    (local.set $refs (i64.sub (local.get $refs) (i64.const 1)))
    (i64.store offset=8 (local.get $p) (local.get $refs))
    (i64.eqz (local.get $refs)))
  (func $cerune_array_free (param $p i32)
    (if (i32.eqz (local.get $p)) (then return))
    (if (i32.or (i64.ne (i64.load offset=8 (local.get $p)) (i64.const 0))
          (i32.load offset=4 (local.get $p))) (then unreachable))
    (global.set $cerune_array_live (i64.sub (global.get $cerune_array_live) (i64.load offset=32 (local.get $p))))
    (i32.store offset=4 (local.get $p) (i32.const 1)))
"#;
