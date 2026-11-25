use super::{
    failure::emit_if,
    ir::{Instruction, Origin},
};
use crate::{codegen::IntegerBinaryOp, runtime::FailureCode as Failure, types::IntegerType};
use std::fmt::Write;

pub(super) fn emit_support(
    op: IntegerBinaryOp,
    ty: IntegerType,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    if ty == IntegerType::U64 {
        return super::unsigned::emit_binary(op, origin, name, output);
    }
    writeln!(
        output,
        "  (func ${name} (param $left i64) (param $right i64) (result i64)"
    )
    .unwrap();
    let instruction = match op {
        IntegerBinaryOp::Add
        | IntegerBinaryOp::Subtract
        | IntegerBinaryOp::Multiply
        | IntegerBinaryOp::Divide => unreachable!("u64 operations are emitted above"),
        IntegerBinaryOp::BitAnd => "and",
        IntegerBinaryOp::BitOr => "or",
        IntegerBinaryOp::BitXor => "xor",
        IntegerBinaryOp::Remainder => {
            emit_if(
                "    local.get $right\n    i64.eqz\n",
                Failure::RemainderByZero,
                origin,
                output,
            );
            // 最小値 % -1は0。除数がゼロの場合だけ停止します。
            "rem_s"
        }
        IntegerBinaryOp::ShiftLeft | IntegerBinaryOp::ShiftRight => {
            emit_if(
                &format!(
                    "    local.get $right\n    i64.const 0\n    i64.lt_s\n    local.get $right\n    i64.const {}\n    i64.ge_s\n    i32.or\n",
                    ty.bit_width()
                ),
                Failure::InvalidShiftCount,
                origin,
                output,
            );
            if op == IntegerBinaryOp::ShiftLeft {
                emit_if(
                    &format!(
                        "    local.get $left\n    i64.const {}\n    local.get $right\n    i64.shr_s\n    i64.lt_s\n    local.get $left\n    i64.const {}\n    local.get $right\n    i64.shr_u\n    i64.gt_s\n    i32.or\n",
                        ty.minimum(),
                        ty.maximum()
                    ),
                    Failure::IntegerOverflow,
                    origin,
                    output,
                );
                "shl"
            } else {
                "shr_s"
            }
        }
    };
    writeln!(
        output,
        "    local.get $left\n    local.get $right\n    i64.{instruction}\n  )\n"
    )
    .unwrap();
}

pub(super) fn emit_signed(
    instruction: &Instruction,
    origin: Origin,
    name: &str,
    output: &mut super::trace::WatText,
) {
    writeln!(
        output,
        "  (func ${name} (param $left i64) (param $right i64) (result i64)\n    (local $result i64)"
    )
    .unwrap();
    match instruction {
        Instruction::CheckedI64Add | Instruction::CheckedI64Sub => {
            let op = if matches!(instruction, Instruction::CheckedI64Add) {
                "add"
            } else {
                "sub"
            };
            writeln!(
                output,
                "    local.get $left\n    local.get $right\n    i64.{op}\n    local.set $result"
            )
            .unwrap();
            let condition = if matches!(instruction, Instruction::CheckedI64Add) {
                "    local.get $result\n    local.get $left\n    i64.xor\n    local.get $result\n    local.get $right\n    i64.xor\n"
            } else {
                "    local.get $left\n    local.get $right\n    i64.xor\n    local.get $left\n    local.get $result\n    i64.xor\n"
            };
            emit_if(
                &format!("{condition}    i64.and\n    i64.const 0\n    i64.lt_s\n"),
                Failure::IntegerOverflow,
                origin,
                output,
            );
        }
        Instruction::CheckedI64Mul => {
            output.push_str("    local.get $left\n    i64.eqz\n    if\n      i64.const 0\n      return\n    end\n");
            emit_if(
                "    local.get $left\n    i64.const -1\n    i64.eq\n    local.get $right\n    i64.const -9223372036854775808\n    i64.eq\n    i32.and\n    local.get $right\n    i64.const -1\n    i64.eq\n    local.get $left\n    i64.const -9223372036854775808\n    i64.eq\n    i32.and\n    i32.or\n",
                Failure::IntegerOverflow,
                origin,
                output,
            );
            output.push_str(
                "    local.get $left\n    local.get $right\n    i64.mul\n    local.set $result\n",
            );
            emit_if(
                "    local.get $result\n    local.get $left\n    i64.div_s\n    local.get $right\n    i64.ne\n",
                Failure::IntegerOverflow,
                origin,
                output,
            );
        }
        Instruction::CheckedI64DivS => {
            emit_if(
                "    local.get $right\n    i64.eqz\n",
                Failure::DivisionByZero,
                origin,
                output,
            );
            emit_if(
                "    local.get $left\n    i64.const -9223372036854775808\n    i64.eq\n    local.get $right\n    i64.const -1\n    i64.eq\n    i32.and\n",
                Failure::DivisionOverflow,
                origin,
                output,
            );
            output.push_str(
                "    local.get $left\n    local.get $right\n    i64.div_s\n    local.set $result\n",
            );
        }
        _ => unreachable!("signed checked arithmetic"),
    }
    output.push_str("    local.get $result\n  )\n\n");
}
