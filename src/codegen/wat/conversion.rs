use super::{failure::emit_if, ir::Origin};
use crate::{
    codegen::NumericConversion,
    runtime::FailureCode as Failure,
    types::{IntegerType, NumericType},
};
use std::fmt::Write;

pub(super) fn type_name(ty: NumericType) -> &'static str {
    match ty {
        NumericType::Integer(_) => "i64",
        NumericType::F32 => "f32",
        NumericType::F64 => "f64",
    }
}

fn emit_truncation(
    conversion: NumericConversion,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    let NumericType::Integer(ty) = conversion.to else {
        unreachable!()
    };
    let (lower, exclusive) = conversion.integer_lower_bound();
    let compare = if exclusive { "le" } else { "lt" };
    let signedness = if ty == IntegerType::U64 { "u" } else { "s" };
    writeln!(output, "  (func ${name} (param $value {}) (result i64)\n    (local $number f64)\n    local.get $value", type_name(conversion.from)).unwrap();
    if conversion.from == NumericType::F32 {
        output.push_str("    f64.promote_f32\n");
    }
    output.push_str("    local.set $number\n");
    emit_if(
        "    local.get $number\n    local.get $number\n    f64.ne\n    local.get $number\n    f64.abs\n    f64.const inf\n    f64.eq\n    i32.or\n",
        Failure::ConversionNotFinite,
        origin,
        output,
    );
    emit_if(
        &format!(
            "    local.get $number\n    f64.const {lower}\n    f64.{compare}\n    local.get $number\n    f64.const {}\n    f64.ge\n    i32.or\n",
            ty.maximum() + 1
        ),
        Failure::ConversionOutOfRange,
        origin,
        output,
    );
    writeln!(
        output,
        "    local.get $number\n    i64.trunc_f64_{signedness}\n  )"
    )
    .unwrap();
}

pub(super) fn emit_support(
    conversion: NumericConversion,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    if conversion.mode.rounding().is_some() && !conversion.truncates() {
        return super::rounding::emit(conversion, origin, name, output);
    }
    if conversion.truncates() {
        return emit_truncation(conversion, origin, name, output);
    }
    if conversion.uses_u64() {
        return super::unsigned::emit_conversion(conversion, origin, name, output);
    }
    let NumericConversion { from, to, .. } = conversion;
    writeln!(output, "  (func ${name} (param $value {}) (result {})\n    (local $result {})\n    (local $number f64)", type_name(from), type_name(to), type_name(to)).unwrap();
    match (from, to) {
        (NumericType::Integer(_), NumericType::F32 | NumericType::F64) => {
            writeln!(output, "    local.get $value\n    {}.convert_i64_s\n    local.set $result\n    local.get $result", type_name(to)).unwrap();
            if to == NumericType::F32 {
                output.push_str("    f64.promote_f32\n");
            }
            output.push_str("    local.set $number\n");
            // 整数へ戻す命令がtrapする丸めも、元の整数を保存できないinexactです。
            emit_if(
                "    local.get $number\n    f64.const -9223372036854775808\n    f64.lt\n    local.get $number\n    f64.const 9223372036854775808\n    f64.ge\n    i32.or\n",
                Failure::ConversionInexact,
                origin,
                output,
            );
            emit_if(
                "    local.get $number\n    i64.trunc_f64_s\n    local.get $value\n    i64.ne\n",
                Failure::ConversionInexact,
                origin,
                output,
            );
        }
        (NumericType::F32 | NumericType::F64, NumericType::Integer(ty)) => {
            emit_float_integer(from, ty, origin, output)
        }
        (NumericType::F32, NumericType::F64) => {
            emit_if(
                "    local.get $value\n    local.get $value\n    f32.ne\n",
                Failure::ConversionNaN,
                origin,
                output,
            );
            output.push_str("    local.get $value\n    f64.promote_f32\n    local.set $result\n");
        }
        (NumericType::F64, NumericType::F32) => {
            emit_if(
                "    local.get $value\n    local.get $value\n    f64.ne\n",
                Failure::ConversionNaN,
                origin,
                output,
            );
            // 無限大は保持できます。有限値だけをf32の最大有限値と比べます。
            emit_if(
                "    local.get $value\n    f64.abs\n    f64.const inf\n    f64.ne\n    local.get $value\n    f64.abs\n    f64.const 340282346638528859811704183484516925440\n    f64.gt\n    i32.and\n",
                Failure::ConversionOutOfRange,
                origin,
                output,
            );
            output.push_str("    local.get $value\n    f32.demote_f64\n    local.set $result\n");
            emit_if(
                "    local.get $result\n    f64.promote_f32\n    local.get $value\n    f64.ne\n",
                Failure::ConversionInexact,
                origin,
                output,
            );
        }
        _ => unreachable!("identity and integer conversions are lowered separately"),
    }
    output.push_str("    local.get $result\n  )\n");
}

pub(super) fn emit_float_integer(
    from: NumericType,
    ty: IntegerType,
    origin: Origin,
    output: &mut super::trace::WatText,
) {
    output.push_str("    local.get $value\n");
    if from == NumericType::F32 {
        output.push_str("    f64.promote_f32\n");
    }
    output.push_str("    local.set $number\n");
    // VMと同じ順序で非有限値、負のゼロ、範囲、精度を検査します。
    emit_if(
        "    local.get $number\n    local.get $number\n    f64.ne\n    local.get $number\n    f64.abs\n    f64.const inf\n    f64.eq\n    i32.or\n",
        Failure::ConversionNotFinite,
        origin,
        output,
    );
    emit_if(
        "    local.get $number\n    i64.reinterpret_f64\n    i64.const -9223372036854775808\n    i64.eq\n",
        Failure::ConversionNegativeZero,
        origin,
        output,
    );
    emit_if(
        &format!(
            "    local.get $number\n    f64.const {}\n    f64.lt\n    local.get $number\n    f64.const {}\n    f64.ge\n    i32.or\n",
            ty.minimum(),
            ty.maximum() + 1
        ),
        Failure::ConversionOutOfRange,
        origin,
        output,
    );
    let signedness = if ty == IntegerType::U64 { "u" } else { "s" };
    writeln!(
        output,
        "    local.get $number\n    i64.trunc_f64_{signedness}\n    local.set $result"
    )
    .unwrap();
    emit_if(
        &format!(
            "    local.get $result\n    f64.convert_i64_{signedness}\n    local.get $number\n    f64.ne\n"
        ),
        Failure::ConversionInexact,
        origin,
        output,
    );
}
