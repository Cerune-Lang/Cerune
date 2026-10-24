use super::ir::Module;
use std::fmt::Write;

pub(super) fn emit(module: &Module, output: &mut String) {
    for (address, value) in &module.strings {
        write!(output, "  (data (i32.const {address}) \"").unwrap();
        for byte in (value.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(value.bytes())
        {
            write!(output, "\\{byte:02x}").unwrap();
        }
        output.push_str("\")\n");
    }
    output.push_str(SUPPORT);
}

// メモリはexportせず、出力先には読み出したバイト値だけを渡します。
const SUPPORT: &str = r#"
  (func $cerune_string_equal (param $left i32) (param $right i32) (result i32)
    (local $length i32) (local $index i32)
    local.get $left
    i32.load
    local.tee $length
    local.get $right
    i32.load
    i32.ne
    if
      i32.const 0
      return
    end
    block $equal
      loop $compare
        local.get $index
        local.get $length
        i32.eq
        br_if $equal
        local.get $left
        local.get $index
        i32.add
        i32.load8_u offset=8
        local.get $right
        local.get $index
        i32.add
        i32.load8_u offset=8
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $compare
      end
    end
    i32.const 1
  )

"#;

pub(super) fn byte_at(name: &str, output: &mut String) {
    output.push_str(&BYTE_AT.replace("BYTE_AT", name));
}
const BYTE_AT: &str = r#"
  (func $BYTE_AT (param $value i32) (param $index i64) (result i64)
    local.get $index
    i64.const 0
    i64.lt_s
    local.get $index
    local.get $value
    i64.load
    i64.ge_u
    i32.or
    if (result i64)
      i64.const -1
    else
      local.get $value
      i32.const 8
      i32.add
      local.get $index
      i32.wrap_i64
      i32.add
      i64.load8_u
    end)
"#;
