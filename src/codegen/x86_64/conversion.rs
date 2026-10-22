use super::failure::Reporter;
use super::machine::{Linkage, Opcode, Operand, r64};
use super::program::{NativeProgram, xmm};
use crate::runtime::FailureCode as Failure;
use crate::{codegen::NumericConversion, types::NumericType};

fn emit_truncation(
    conversion: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    let NumericType::Integer(ty) = conversion.to else {
        unreachable!()
    };
    let base = format!(".Lcerune_{prefix}_trunc_{label}");
    if conversion.from == NumericType::F32 {
        output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
    } else {
        output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
    }

    output.emit(Opcode::Ucomisd, [xmm(2), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_nonfinite"), Linkage::Local);
    output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
        load_bound(infinity, output);

        output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

        let symbol = output.symbol(&format!("{base}_nonfinite"), Linkage::Local);
        output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);
    }
    let (lower, exclusive) = conversion.integer_lower_bound();
    let jump = if exclusive {
        Opcode::JumpBe
    } else {
        Opcode::JumpB
    };
    load_bound(lower, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_range"), Linkage::Local);
    output.emit(jump, [Operand::Symbol(symbol)]);

    load_bound((ty.maximum() + 1) as f64, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_range"), Linkage::Local);
    output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

    if ty == crate::types::IntegerType::U64 {
        // SSE2の符号付き変換を使い、u64の上半分は2^63を引いてから最上位ビットを戻します。
        load_bound(9223372036854775808.0, output);

        output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

        let symbol = output.symbol(&format!("{base}_small"), Linkage::Local);
        output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

        output.emit(Opcode::Movapd, [xmm(2), xmm(3)]);
        output.emit(Opcode::Subsd, [xmm(1), xmm(3)]);
        output.emit(Opcode::Cvttsd2siq, [xmm(3), r64(0)]);
        output.emit(Opcode::BitComplement64, [Operand::Immediate(63), r64(0)]);

        let symbol = output.symbol(&format!("{base}_done"), Linkage::Local);
        output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

        output.label(&format!("{base}_small"));
    }

    output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);

    let symbol = output.symbol(&format!("{base}_done"), Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&format!("{base}_nonfinite"));

    reporter.emit(Failure::ConversionNotFinite, output);

    output.label(&format!("{base}_range"));

    reporter.emit(Failure::ConversionOutOfRange, output);

    output.label(&format!("{base}_done"));
}

pub(super) fn emit(
    conversion: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    if conversion.mode.rounding().is_some() && !conversion.truncates() {
        return super::rounding::emit(conversion, label, prefix, reporter, output);
    }
    if conversion.truncates() {
        return emit_truncation(conversion, label, prefix, reporter, output);
    }
    if conversion.uses_u64() {
        return super::unsigned::emit_conversion(conversion, label, prefix, reporter, output);
    }
    let bad = format!(".Lcerune_{prefix}_convert_bad_{label}");
    let done = format!(".Lcerune_{prefix}_convert_done_{label}");
    match (conversion.from, conversion.to) {
        (NumericType::Integer(_), NumericType::F32 | NumericType::F64) => {
            output.emit(Opcode::Mov64, [r64(0), r64(10)]);

            if conversion.to == NumericType::F32 {
                output.emit(Opcode::Cvtsi2ssq, [r64(0), xmm(0)]);
                output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
            } else {
                output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(0)]);
                output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
            }
            // i64の最大値が2^63へ丸められても、逆変換の前に検出します。
            load_bound(9223372036854775808.0, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);
            output.emit(Opcode::Cmp64, [r64(10), r64(0)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);
        }
        (NumericType::F32 | NumericType::F64, NumericType::Integer(ty)) => {
            if conversion.from == NumericType::F32 {
                output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
            } else {
                output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
            }
            check_finite_integer_input(&bad, reporter, output);
            load_bound(ty.minimum() as f64, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

            load_bound((ty.maximum() + 1) as f64, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);
            output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(1)]);
            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);
        }
        (NumericType::F32, NumericType::F64) => {
            output.emit(Opcode::Ucomiss, [xmm(0), xmm(0)]);

            let symbol = output.symbol(&format!("{bad}_nan"), Linkage::Local);
            output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(0)]);
        }
        (NumericType::F64, NumericType::F32) => {
            output.emit(Opcode::Ucomisd, [xmm(0), xmm(0)]);

            let symbol = output.symbol(&format!("{bad}_nan"), Linkage::Local);
            output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

            output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);

            // 無限大は保存できますが、有限値がf32最大値を越える場合は範囲外です。
            load_bound(f64::INFINITY, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_convert"), Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            load_bound(f64::NEG_INFINITY, output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_convert"), Linkage::Local);
            output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

            load_bound(f64::from(f32::MAX), output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

            load_bound(-f64::from(f32::MAX), output);

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{bad}_range"), Linkage::Local);
            output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

            output.label(&format!("{bad}_convert"));
            output.emit(Opcode::Cvtsd2ss, [xmm(0), xmm(0)]);
            output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(1)]);
            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&bad, Linkage::Local);
            output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);
        }
        _ => unreachable!("identity and integer conversions are lowered separately"),
    }

    let symbol = output.symbol(&done, Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&bad);

    reporter.emit(Failure::ConversionInexact, output);
    if matches!(conversion.from, NumericType::F32 | NumericType::F64) {
        output.label(&format!("{bad}_range"));

        reporter.emit(Failure::ConversionOutOfRange, output);

        output.label(&format!("{bad}_nan"));

        reporter.emit(Failure::ConversionNaN, output);
    }

    output.label(&done);
}

/// xmm2のfloatを整数化する前に、NaN/無限大と負のゼロを区別します。
pub(super) fn check_finite_integer_input(
    bad: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    output.emit(Opcode::Ucomisd, [xmm(2), xmm(2)]);

    let symbol = output.symbol(&format!("{bad}_nonfinite"), Linkage::Local);
    output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

    load_bound(f64::INFINITY, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{bad}_nonfinite"), Linkage::Local);
    output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    load_bound(f64::NEG_INFINITY, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{bad}_nonfinite"), Linkage::Local);
    output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

    output.emit(Opcode::Mov64, [xmm(2), r64(11)]);
    output.emit(
        Opcode::MovAbs64,
        [Operand::Immediate(-9223372036854775808), r64(10)],
    );
    output.emit(Opcode::Cmp64, [r64(10), r64(11)]);

    let symbol = output.symbol(&format!("{bad}_finite"), Linkage::Local);
    output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

    reporter.emit(Failure::ConversionNegativeZero, output);

    output.label(&format!("{bad}_nonfinite"));

    reporter.emit(Failure::ConversionNotFinite, output);

    output.label(&format!("{bad}_finite"));
}

fn load_bound(value: f64, output: &mut NativeProgram) {
    output.emit(
        Opcode::MovAbs64,
        [
            Operand::Immediate((value.to_bits() as i64).to_owned() as i128),
            r64(11),
        ],
    );
    output.emit(Opcode::Mov64, [r64(11), xmm(1)]);
}
