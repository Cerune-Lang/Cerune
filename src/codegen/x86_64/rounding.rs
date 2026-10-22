use super::failure::Reporter;
use super::machine::{Linkage, Opcode, Operand, r64};
use super::program::{NativeProgram, xmm};
use crate::{
    codegen::NumericConversion,
    runtime::FailureCode,
    types::{IntegerType, NumericType, RoundingMode},
};

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

pub(super) fn emit(
    c: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut NativeProgram,
) {
    let NumericType::Integer(ty) = c.to else {
        unreachable!()
    };
    let base = format!(".Lcerune_{prefix}_{}_{label}", c.mode.name());

    output.comment(&format!("policy: {}", c.mode.name()));

    if c.from == NumericType::F32 {
        output.emit(Opcode::Cvtss2sd, [xmm(0), xmm(2)]);
    } else {
        output.emit(Opcode::Movapd, [xmm(0), xmm(2)]);
    }

    output.emit(Opcode::Ucomisd, [xmm(2), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_nonfinite"), Linkage::Local);
    output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
        bound(infinity, output);

        output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

        let symbol = output.symbol(&format!("{base}_nonfinite"), Linkage::Local);
        output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);
    }

    output.comment("round finite input; large values are already integral");

    bound(4503599627370496.0, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_bounds"), Linkage::Local);
    output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

    bound(-4503599627370496.0, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_bounds"), Linkage::Local);
    output.emit(Opcode::JumpBe, [Operand::Symbol(symbol)]);

    output.emit(Opcode::Cvttsd2siq, [xmm(2), r64(0)]);
    output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(3)]);
    output.emit(Opcode::Subsd, [xmm(3), xmm(2)]);

    match c.mode.rounding().unwrap() {
        RoundingMode::Truncate => {}
        RoundingMode::Floor | RoundingMode::Ceil => {
            bound(0.0, output);
            let (jump, direction) = if c.mode.rounding() == Some(RoundingMode::Floor) {
                (Opcode::JumpB, "down")
            } else {
                (Opcode::JumpA, "up")
            };

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{base}_{direction}"), Linkage::Local);
            output.emit(jump, [Operand::Symbol(symbol)]);
        }
        RoundingMode::Round | RoundingMode::TiesEven => {
            let even = c.mode.rounding() == Some(RoundingMode::TiesEven);
            bound(0.5, output);
            let up = if even { Opcode::JumpA } else { Opcode::JumpAe };

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{base}_up"), Linkage::Local);
            output.emit(up, [Operand::Symbol(symbol)]);

            if even {
                let symbol = output.symbol(&format!("{base}_tie"), Linkage::Local);
                output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);
            }
            bound(-0.5, output);
            let down = if even { Opcode::JumpB } else { Opcode::JumpBe };

            output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

            let symbol = output.symbol(&format!("{base}_down"), Linkage::Local);
            output.emit(down, [Operand::Symbol(symbol)]);

            if even {
                let symbol = output.symbol(&format!("{base}_rounded"), Linkage::Local);
                output.emit(Opcode::JumpNe, [Operand::Symbol(symbol)]);

                output.label(&format!("{base}_tie"));
                output.emit(Opcode::Test64, [Operand::Immediate(1), r64(0)]);

                let symbol = output.symbol(&format!("{base}_rounded"), Linkage::Local);
                output.emit(Opcode::JumpE, [Operand::Symbol(symbol)]);

                bound(0.0, output);

                output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

                let symbol = output.symbol(&format!("{base}_up"), Linkage::Local);
                output.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

                let symbol = output.symbol(&format!("{base}_down"), Linkage::Local);
                output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);
            }
        }
    }

    let symbol = output.symbol(&format!("{base}_rounded"), Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&format!("{base}_up"));
    output.emit(Opcode::Add64, [Operand::Immediate(1), r64(0)]);

    let symbol = output.symbol(&format!("{base}_rounded"), Linkage::Local);
    output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

    output.label(&format!("{base}_down"));
    output.emit(Opcode::Sub64, [Operand::Immediate(1), r64(0)]);
    output.label(&format!("{base}_rounded"));
    output.emit(Opcode::Cvtsi2sdq, [r64(0), xmm(2)]);
    output.label(&format!("{base}_bounds"));
    output.comment("range policy after rounding, then convert");

    bound(ty.minimum() as f64, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_minimum"), Linkage::Local);
    output.emit(Opcode::JumpB, [Operand::Symbol(symbol)]);

    bound((ty.maximum() + 1) as f64, output);

    output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

    let symbol = output.symbol(&format!("{base}_maximum"), Linkage::Local);
    output.emit(Opcode::JumpAe, [Operand::Symbol(symbol)]);

    if ty == IntegerType::U64 {
        bound(9223372036854775808.0, output);

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

    if c.mode.saturates() {
        output.emit(Opcode::Ucomisd, [xmm(2), xmm(2)]);

        let symbol = output.symbol(&format!("{base}_zero"), Linkage::Local);
        output.emit(Opcode::JumpP, [Operand::Symbol(symbol)]);

        bound(0.0, output);

        output.emit(Opcode::Ucomisd, [xmm(1), xmm(2)]);

        let symbol = output.symbol(&format!("{base}_maximum"), Linkage::Local);
        output.emit(Opcode::JumpA, [Operand::Symbol(symbol)]);

        let symbol = output.symbol(&format!("{base}_minimum"), Linkage::Local);
        output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

        output.label(&format!("{base}_zero"));
        output.emit(Opcode::Xor64, [r64(0), r64(0)]);

        let symbol = output.symbol(&format!("{base}_done"), Linkage::Local);
        output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

        output.label(&format!("{base}_minimum"));
        output.emit(
            Opcode::MovAbs64,
            [Operand::Immediate((ty.minimum()).to_owned()), r64(0)],
        );

        let symbol = output.symbol(&format!("{base}_done"), Linkage::Local);
        output.emit(Opcode::Jump, [Operand::Symbol(symbol)]);

        output.label(&format!("{base}_maximum"));
        output.emit(
            Opcode::MovAbs64,
            [
                Operand::Immediate((ty.maximum() as i64).to_owned() as i128),
                r64(0),
            ],
        );
    } else {
        reporter.emit(FailureCode::ConversionNotFinite, output);

        output.label(&format!("{base}_minimum"));
        output.label(&format!("{base}_maximum"));

        reporter.emit(FailureCode::ConversionOutOfRange, output);
    }

    output.label(&format!("{base}_done"));
}
