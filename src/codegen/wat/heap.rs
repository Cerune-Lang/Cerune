//! 非公開memoryの再利用可能な領域。所有は共通IR、空き領域管理は生成先が担当します。
use super::ir::Origin;
use crate::runtime::{FailureCode, RuntimeFailure};
use std::fmt::Write;
pub(super) fn support(limit: u64, start: usize) -> String {
    SOURCE
        .replace("@LIMIT@", &limit.to_string())
        .replace("@START@", &start.to_string())
        .replace("@EMPTY@", &(start - 8).to_string())
}
pub(super) fn concat(origin: Origin, name: &str, output: &mut super::trace::WatText) {
    writeln!(output, "  (func \u{24}{name} (param \u{24}left i32) (param \u{24}right i32) (result i32)\n    (local \u{24}length i64) (local \u{24}value i32) (local \u{24}error i32)").unwrap();
    output.push_str("    local.get $left\n    i64.load\n    local.get $right\n    i64.load\n    i64.add\n    local.tee $length\n    call $cerune_string_allocate\n    local.set $error\n    local.set $value\n");
    for (number, code) in [
        (1, FailureCode::AllocationSizeOverflow),
        (2, FailureCode::AllocationLimitExceeded),
        (3, FailureCode::AllocationFailed),
    ] {
        writeln!(
            output,
            "    local.get \u{24}error\n    i32.const {number}\n    i32.eq\n    if"
        )
        .unwrap();
        super::failure::emit(
            RuntimeFailure {
                code,
                node_id: origin.node_id,
                span: origin.span,
            },
            "      ",
            output,
        );
        output.push_str("    end\n");
    }
    output.push_str("    local.get $value\n    i32.const 8\n    i32.add\n    local.get $left\n    i32.const 8\n    i32.add\n    local.get $left\n    i64.load\n    i32.wrap_i64\n    memory.copy\n    local.get $value\n    i32.const 8\n    i32.add\n    local.get $left\n    i64.load\n    i32.wrap_i64\n    i32.add\n    local.get $right\n    i32.const 8\n    i32.add\n    local.get $right\n    i64.load\n    i32.wrap_i64\n    memory.copy\n    local.get $value\n  )\n");
}
const SOURCE: &str = r#"
  (global $cerune_heap_head (mut i32) (i32.const 0))
  (global $cerune_heap_end (mut i64) (i64.const @START@))
  (global $cerune_heap_live (mut i64) (i64.const 0))
  (func $cerune_string_retain (param $value i32)
    (local $p i32)
    global.get $cerune_heap_head
    local.set $p
    block $done
      loop $search
        local.get $p
        i32.eqz
        br_if $done
        local.get $p
        i32.const 16
        i32.add
        local.get $value
        i32.eq
        if
          local.get $p
          local.get $p
          i32.load offset=4
          i32.const 1
          i32.add
          i32.store offset=4
          return
        end
        local.get $p
        i32.load
        local.set $p
        br $search
      end
    end
  )
  (func $cerune_string_release (param $value i32)
    (local $p i32) (local $count i32)
    global.get $cerune_heap_head
    local.set $p
    block $done
      loop $search
        local.get $p
        i32.eqz
        br_if $done
        local.get $p
        i32.const 16
        i32.add
        local.get $value
        i32.eq
        if
          local.get $p
          i32.load offset=4
          i32.const 1
          i32.sub
          local.set $count
          local.get $p
          local.get $count
          i32.store offset=4
          local.get $count
          i32.eqz
          if
            global.get $cerune_heap_live
            local.get $p
            i64.load offset=16
            i64.sub
            global.set $cerune_heap_live
          end
          return
        end
        local.get $p
        i32.load
        local.set $p
        br $search
      end
    end
  )
  ;; 戻り値: 長さヘッダーへのaddress、失敗番号（0=成功）。
  (func $cerune_string_allocate (param $length i64) (result i32 i32)
    (local $p i32) (local $size i64) (local $end i64) (local $pages i64)
    local.get $length
    i64.eqz
    if
      i32.const @EMPTY@
      i32.const 0
      return
    end
    local.get $length
    i64.const 4294967271
    i64.gt_u
    if
      i32.const 0
      i32.const 1
      return
    end
    local.get $length
    i64.const @LIMIT@
    global.get $cerune_heap_live
    i64.sub
    i64.gt_u
    if
      i32.const 0
      i32.const 2
      return
    end
    global.get $cerune_heap_head
    local.set $p
    block $found
      block $new
        loop $search
          local.get $p
          i32.eqz
          br_if $new
          local.get $p
          i32.load offset=4
          i32.eqz
          local.get $p
          i64.load offset=8
          local.get $length
          i64.ge_u
          i32.and
          br_if $found
          local.get $p
          i32.load
          local.set $p
          br $search
        end
      end
      local.get $length
      i64.const 31
      i64.add
      i64.const -8
      i64.and
      local.set $size
      global.get $cerune_heap_end
      local.get $size
      i64.add
      local.tee $end
      i64.const 4294967296
      i64.gt_u
      if
        i32.const 0
        i32.const 3
        return
      end
      local.get $end
      i64.const 65535
      i64.add
      i64.const 65536
      i64.div_u
      local.set $pages
      local.get $pages
      memory.size
      i64.extend_i32_u
      i64.gt_u
      if
        local.get $pages
        memory.size
        i64.extend_i32_u
        i64.sub
        i32.wrap_i64
        memory.grow
        i32.const -1
        i32.eq
        if
          i32.const 0
          i32.const 3
          return
        end
      end
      global.get $cerune_heap_end
      i32.wrap_i64
      local.set $p
      local.get $end
      global.set $cerune_heap_end
      local.get $p
      global.get $cerune_heap_head
      i32.store
      local.get $p
      global.set $cerune_heap_head
      local.get $p
      local.get $size
      i64.const 24
      i64.sub
      i64.store offset=8
    end
    local.get $p
    i32.const 1
    i32.store offset=4
    local.get $p
    local.get $length
    i64.store offset=16
    global.get $cerune_heap_live
    local.get $length
    i64.add
    global.set $cerune_heap_live
    local.get $p
    i32.const 16
    i32.add
    i32.const 0
  )
"#;
