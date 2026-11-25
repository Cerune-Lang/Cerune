use super::{conversion::type_name, failure::emit_if, ir::Origin};
use crate::{
    codegen::NumericConversion,
    runtime::FailureCode,
    types::{IntegerType, NumericType, RoundingMode},
};
use std::fmt::Write;

pub(super) fn emit(
    c: NumericConversion,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    let NumericType::Integer(ty) = c.to else {
        unreachable!()
    };
    writeln!(output, "  (func ${name} (param $value {}) (result i64)\n    (local $number f64)\n    (local $base f64)\n    (local $fraction f64)\n    ;; policy: {}\n    local.get $value", type_name(c.from), c.mode.name()).unwrap();
    if c.from == NumericType::F32 {
        output.push_str("    f64.promote_f32\n");
    }
    output.push_str("    local.set $number\n");
    if c.mode.saturates() {
        writeln!(output, "    local.get $number\n    local.get $number\n    f64.ne\n    if\n      i64.const 0\n      return\n    end\n    local.get $number\n    f64.abs\n    f64.const inf\n    f64.eq\n    if\n      local.get $number\n      f64.const 0\n      f64.gt\n      if (result i64)\n        i64.const {}\n      else\n        i64.const {}\n      end\n      return\n    end", ty.maximum() as i64, ty.minimum()).unwrap();
    } else {
        emit_if(
            "    local.get $number\n    local.get $number\n    f64.ne\n    local.get $number\n    f64.abs\n    f64.const inf\n    f64.eq\n    i32.or\n",
            FailureCode::ConversionNotFinite,
            origin,
            output,
        );
    }
    output.push_str("    ;; round finite input\n");
    match c.mode.rounding().unwrap() {
        RoundingMode::Round => {
            // x + 0.5では直前の表現可能値まで丸めてしまうため、整数部との差を比較します。
            output.push_str("    local.get $number\n    f64.abs\n    f64.const 4503599627370496\n    f64.lt\n    if\n      local.get $number\n      f64.trunc\n      local.set $base\n      local.get $number\n      local.get $base\n      f64.sub\n      local.set $fraction\n      local.get $fraction\n      f64.const 0.5\n      f64.ge\n      if\n        local.get $base\n        f64.const 1\n        f64.add\n        local.set $base\n      else\n        local.get $fraction\n        f64.const -0.5\n        f64.le\n        if\n          local.get $base\n          f64.const 1\n          f64.sub\n          local.set $base\n        end\n      end\n      local.get $base\n      local.set $number\n    end\n");
        }
        rounding => {
            let operation = match rounding {
                RoundingMode::Truncate => "trunc",
                RoundingMode::Floor => "floor",
                RoundingMode::Ceil => "ceil",
                RoundingMode::TiesEven => "nearest",
                RoundingMode::Round => unreachable!(),
            };
            writeln!(
                output,
                "    local.get $number\n    f64.{operation}\n    local.set $number"
            )
            .unwrap();
        }
    }
    output.push_str("    ;; range policy after rounding, then convert\n");
    if c.mode.saturates() {
        writeln!(output, "    local.get $number\n    f64.const {}\n    f64.lt\n    if\n      i64.const {}\n      return\n    end\n    local.get $number\n    f64.const {}\n    f64.ge\n    if\n      i64.const {}\n      return\n    end", ty.minimum(), ty.minimum(), ty.maximum()+1, ty.maximum() as i64).unwrap();
    } else {
        emit_if(
            &format!(
                "    local.get $number\n    f64.const {}\n    f64.lt\n    local.get $number\n    f64.const {}\n    f64.ge\n    i32.or\n",
                ty.minimum(),
                ty.maximum() + 1
            ),
            FailureCode::ConversionOutOfRange,
            origin,
            output,
        );
    }
    let signedness = if ty == IntegerType::U64 { "u" } else { "s" };
    writeln!(
        output,
        "    local.get $number\n    i64.trunc_f64_{signedness}\n  )"
    )
    .unwrap();
}
