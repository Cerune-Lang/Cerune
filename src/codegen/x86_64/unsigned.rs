use super::failure::Reporter;
use super::machine::{Linkage, Opcode, Operand, r8, r64};
use super::program::{NativeProgram, xmm};
use crate::runtime::FailureCode as Failure;
use crate::{
    codegen::{IntegerBinaryOp as Op, NumericConversion},
    types::{IntegerType, NumericType as N},
};

// 左辺はrax、右辺はrcxです。演算が失敗したときだけ明示的にtrapします。
pub(super) fn emit_binary(
    op: Op,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    let bad = format!(".Lcerune_{prefix}_u64_bad_{label}");
    let done = format!(".Lcerune_{prefix}_u64_done_{label}");
    match op {
        Op::Add => {
            output.emit(Opcode::Add64, [r64(1), r64(0)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);
        }
        Op::Subtract => {
            output.emit(Opcode::Sub64, [r64(1), r64(0)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);
        }
        Op::Multiply => {
            output.emit(Opcode::Mul64, [r64(1)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);
        }
        Op::Divide | Op::Remainder => {
            output.emit(Opcode::Test64, [r64(1), r64(1)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Xor64, [r64(2), r64(2)]);
            output.emit(Opcode::Div64, [r64(1)]);

            if op == Op::Remainder {
                output.emit(Opcode::Mov64, [r64(2), r64(0)]);
            }
        }
        Op::BitAnd => {
            return {
                output.emit(Opcode::And64, [r64(1), r64(0)]);
            };
        }
        Op::BitOr => {
            return {
                output.emit(Opcode::Or64, [r64(1), r64(0)]);
            };
        }
        Op::BitXor => {
            return {
                output.emit(Opcode::Xor64, [r64(1), r64(0)]);
            };
        }
        Op::ShiftLeft | Op::ShiftRight => {
            output.emit(Opcode::Cmp64, [Operand::Immediate(64), r64(1)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            if op == Op::ShiftLeft {
                output.emit(Opcode::Mov64, [Operand::Immediate(-1), r64(11)]);
                output.emit(Opcode::ShiftRight64, [r8(1), r64(11)]);
                output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

                let symbol = output.symbol(&format!("{bad}_overflow"), Linkage::Local);
                output.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

                output.emit(Opcode::ShiftLeft64, [r8(1), r64(0)]);

                let symbol = output.symbol(&done, Linkage::Local);
                output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

                output.label(&format!("{bad}_overflow"));

                reporter.emit(Failure::IntegerOverflow, output);
            } else {
                output.emit(Opcode::ShiftRight64, [r8(1), r64(0)]);
            }
        }
    }

    let symbol = output.symbol(&done, Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&bad);

    reporter.emit(
        match op {
            Op::Divide => Failure::DivisionByZero,
            Op::Remainder => Failure::RemainderByZero,
            Op::ShiftLeft | Op::ShiftRight => Failure::InvalidShiftCount,
            _ => Failure::IntegerOverflow,
        },
        output,
    );

    output.label(&done);
}

pub(super) fn emit_conversion(
    c: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    let base = format!(".Lcerune_{prefix}_u64_convert_{label}");
    let bad = format!("{base}_bad");
    let done = format!("{base}_done");
    match (c.from, c.to) {
        (N::Integer(from), N::Integer(to)) => {
            if from == IntegerType::U64 {
                output.emit(
                    Opcode::MovAbs64,
                    [Operand::Immediate((to.maximum()).to_owned()), r64(11)],
                );
                output.emit(Opcode::Cmp64, [r64(11), r64(0)]);

                let symbol = output.symbol(&bad, Linkage::Local);
                output.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);
            } else if from.is_signed() {
                output.emit(Opcode::Test64, [r64(0), r64(0)]);

                let symbol = output.symbol(&bad, Linkage::Local);
                output.emit(Opcode::JumpS, [Operand::Symbol(symbol)]);
            }
        }
        (N::Integer(_), N::F32 | N::F64) => {
            // SSE2の符号付き変換で上半分を扱うため、下位ビットを残して半分にしてから倍にします。

            output.emit(Opcode::Mov64, [r64(0), r64(10)]);
            output.emit(Opcode::Test64, [r64(0), r64(0)]);

            let symbol = output.symbol(&format!("{base}_small"), Linkage::Local);
            output.emit(Opcode::JumpNs, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Mov64, [r64(0), r64(11)]);
            output.emit(Opcode::And64, [Operand::Immediate(1), r64(11)]);
            output.emit(Opcode::ShiftRight64, [Operand::Immediate(1), r64(0)]);
            output.emit(Opcode::Or64, [r64(11), r64(0)]);
            output.emit(
                if c.to == N::F32 {
                    Opcode::Cvtsi2ssq
                } else {
                    Opcode::Cvtsi2sdq
                },
                [r64(0), xmm(0)],
            );
            output.emit(
                if c.to == N::F32 {
                    Opcode::Addss
                } else {
                    Opcode::Addsd
                },
                [xmm(0), xmm(0)],
            );

            let symbol = output.symbol(&format!("{base}_number"), Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&format!("{base}_small"));
            output.emit(
                if c.to == N::F32 {
                    Opcode::Cvtsi2ssq
                } else {
                    Opcode::Cvtsi2sdq
                },
                [r64(0), xmm(0)],
            );
            output.label(&format!("{base}_number"));

            if c.to == N::F32 {
                output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
            } else {
                output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
            }
            bound(18446744073709551616.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            bound(9223372036854775808.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{base}_back_small"), Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Movapd, [xmm(2), xmm(3)]);
            output.emit(Opcode::Subsd, [xmm(1), xmm(3)]);
            output.emit(Opcode::Cvttsd2siq, [xmm(3), r64(0)]);
            output.emit(Opcode::BitComplement64, [Operand::Immediate(63), r64(0)]);

            let symbol = output.symbol(&format!("{base}_compare"), Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&format!("{base}_back_small"));
            output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);
            output.label(&format!("{base}_compare"));
            output.emit(Opcode::Cmp64, [r64(10), r64(0)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);
        }
        (N::F32 | N::F64, N::Integer(_)) => {
            if c.from == N::F32 {
                output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
            } else {
                output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
            }
            super::conversion::check_finite_integer_input(&bad, reporter, output);
            bound(0.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

            bound(18446744073709551616.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            bound(9223372036854775808.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{base}_small"), Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Movapd, [xmm(2), xmm(3)]);
            output.emit(Opcode::Subsd, [xmm(1), xmm(3)]);
            output.emit(Opcode::Cvttsd2siq, [xmm(3), r64(0)]);
            output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(3)]);
            output.emit(Opcode::Addsd, [xmm(1), xmm(3)]);
            output.emit(Opcode::BitComplement64, [Operand::Immediate(63), r64(0)]);

            let symbol = output.symbol(&format!("{base}_compare"), Linkage::Local);
            output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

            output.label(&format!("{base}_small"));
            output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);
            output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(3)]);
            output.label(&format!("{base}_compare"));
            output.emit(Opcode::Ucomisd, [xmm(3), xmm(2)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);
        }
        _ => unreachable!("u64 conversion"),
    }

    let symbol = output.symbol(&done, Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&bad);

    reporter.emit(
        if matches!((c.from, c.to), (N::Integer(_), N::Integer(_))) {
            Failure::IntegerConversionOutOfRange
        } else {
            Failure::ConversionInexact
        },
        output,
    );
    if matches!(c.from, N::F32 | N::F64) {
        output.label(&format!("{bad}_range"));

        reporter.emit(Failure::ConversionOutOfRange, output);
    }

    output.label(&done);
}
fn bound(value: f64, output: &mut NativeProgram) {
    output.emit(
        Opcode::MovAbs64,
        [
            Operand::Immediate((value.to_bits() as i64).to_owned() as i128),
            r64(11),
        ],
    );
    output.emit(Opcode::Mov64, [r64(11), xmm(1)]);
}
