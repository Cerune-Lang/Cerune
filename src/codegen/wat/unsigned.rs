use super::{failure::emit_if, ir::Origin};
use crate::{
    codegen::{IntegerBinaryOp as Op, NumericConversion},
    runtime::FailureCode as Failure,
    types::{IntegerType, NumericType as N},
};
use std::fmt::Write;

pub(super) fn emit_binary(op: Op, origin: Origin, name: &str, output: &mut super::trace::WatText) {
    writeln!(
        output,
        "  (func ${name} (param $left i64) (param $right i64) (result i64)"
    )
    .unwrap();
    let instruction = match op {
        Op::Add => {
            emit_if(
                "    local.get $left\n    i64.const -1\n    local.get $right\n    i64.sub\n    i64.gt_u\n",
                Failure::IntegerOverflow,
                origin,
                output,
            );
            "add"
        }
        Op::Subtract => {
            emit_if(
                "    local.get $left\n    local.get $right\n    i64.lt_u\n",
                Failure::IntegerOverflow,
                origin,
                output,
            );
            "sub"
        }
        Op::Multiply => {
            output.push_str("    local.get $right\n    i64.eqz\n    if\n    else\n");
            emit_if(
                "    local.get $left\n    i64.const -1\n    local.get $right\n    i64.div_u\n    i64.gt_u\n",
                Failure::IntegerOverflow,
                origin,
                output,
            );
            output.push_str("    end\n");
            "mul"
        }
        Op::Divide | Op::Remainder => {
            emit_if(
                "    local.get $right\n    i64.eqz\n",
                if op == Op::Divide {
                    Failure::DivisionByZero
                } else {
                    Failure::RemainderByZero
                },
                origin,
                output,
            );
            if op == Op::Divide { "div_u" } else { "rem_u" }
        }
        Op::BitAnd => "and",
        Op::BitOr => "or",
        Op::BitXor => "xor",
        Op::ShiftLeft | Op::ShiftRight => {
            emit_if(
                "    local.get $right\n    i64.const 64\n    i64.ge_u\n",
                Failure::InvalidShiftCount,
                origin,
                output,
            );
            if op == Op::ShiftLeft {
                emit_if(
                    "    local.get $left\n    i64.const -1\n    local.get $right\n    i64.shr_u\n    i64.gt_u\n",
                    Failure::IntegerOverflow,
                    origin,
                    output,
                );
                "shl"
            } else {
                "shr_u"
            }
        }
    };
    writeln!(
        output,
        "    local.get $left\n    local.get $right\n    i64.{instruction}\n  )\n"
    )
    .unwrap();
}

pub(super) fn emit_conversion(
    c: NumericConversion,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    let from = super::conversion::type_name(c.from);
    let to = super::conversion::type_name(c.to);
    writeln!(output, "  (func ${name} (param $value {from}) (result {to})\n    (local $result {to}) (local $number f64)").unwrap();
    match (c.from, c.to) {
        (N::Integer(from), N::Integer(to)) => {
            let condition = if from == IntegerType::U64 {
                format!(
                    "    local.get $value\n    i64.const {}\n    i64.gt_u\n",
                    to.maximum()
                )
            } else {
                "    local.get $value\n    i64.const 0\n    i64.lt_s\n".into()
            };
            emit_if(
                &condition,
                Failure::IntegerConversionOutOfRange,
                origin,
                output,
            );
            output.push_str("    local.get $value\n    local.set $result\n");
        }
        (N::Integer(_), N::F32 | N::F64) => {
            writeln!(
                output,
                "    local.get $value\n    {to}.convert_i64_u\n    local.tee $result"
            )
            .unwrap();
            if c.to == N::F32 {
                output.push_str("    f64.promote_f32\n");
            }
            output.push_str("    local.set $number\n");
            emit_if(
                "    local.get $number\n    f64.const 18446744073709551616\n    f64.ge\n",
                Failure::ConversionInexact,
                origin,
                output,
            );
            emit_if(
                "    local.get $number\n    i64.trunc_f64_u\n    local.get $value\n    i64.ne\n",
                Failure::ConversionInexact,
                origin,
                output,
            );
        }
        (N::F32 | N::F64, N::Integer(_)) => {
            super::conversion::emit_float_integer(c.from, IntegerType::U64, origin, output);
        }
        _ => unreachable!("u64 conversion"),
    }
    output.push_str("    local.get $result\n  )\n");
}
