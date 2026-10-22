use super::failure::Reporter;
use crate::{
    codegen::NumericConversion,
    runtime::FailureCode,
    types::{IntegerType, NumericType, RoundingMode},
};

fn bound(value: f64, output: &mut String) {
    output.push_str(&format!(
        "  movabsq ${}, %r11\n  movq %r11, %xmm1\n",
        value.to_bits() as i64
    ));
}

pub(super) fn emit(
    c: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut String,
) {
    let NumericType::Integer(ty) = c.to else {
        unreachable!()
    };
    let base = format!(".Lcerune_{prefix}_{}_{label}", c.mode.name());
    output.push_str(&format!("  # policy: {}\n", c.mode.name()));
    if c.from == NumericType::F32 {
        output.push_str("  cvtss2sd %xmm0, %xmm2\n");
    } else {
        output.push_str("  movapd %xmm0, %xmm2\n");
    }
    output.push_str(&format!("  ucomisd %xmm2, %xmm2\n  jp {base}_nonfinite\n"));
    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
        bound(infinity, output);
        output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {base}_nonfinite\n"));
    }
    output.push_str("  # round finite input; large values are already integral\n");
    bound(4503599627370496.0, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {base}_bounds\n"));
    bound(-4503599627370496.0, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jbe {base}_bounds\n  cvttsd2siq %xmm2, %rax\n  cvtsi2sdq %rax, %xmm3\n  subsd %xmm3, %xmm2\n"));
    match c.mode.rounding().unwrap() {
        RoundingMode::Truncate => {}
        RoundingMode::Floor | RoundingMode::Ceil => {
            bound(0.0, output);
            let (jump, direction) = if c.mode.rounding() == Some(RoundingMode::Floor) {
                ("jb", "down")
            } else {
                ("ja", "up")
            };
            output.push_str(&format!(
                "  ucomisd %xmm1, %xmm2\n  {jump} {base}_{direction}\n"
            ));
        }
        RoundingMode::Round | RoundingMode::TiesEven => {
            let even = c.mode.rounding() == Some(RoundingMode::TiesEven);
            bound(0.5, output);
            let up = if even { "ja" } else { "jae" };
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  {up} {base}_up\n"));
            if even {
                output.push_str(&format!("  je {base}_tie\n"));
            }
            bound(-0.5, output);
            let down = if even { "jb" } else { "jbe" };
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  {down} {base}_down\n"));
            if even {
                output.push_str(&format!(
                    "  jne {base}_rounded\n{base}_tie:\n  testq $1, %rax\n  je {base}_rounded\n"
                ));
                bound(0.0, output);
                output.push_str(&format!(
                    "  ucomisd %xmm1, %xmm2\n  ja {base}_up\n  jmp {base}_down\n"
                ));
            }
        }
    }
    output.push_str(&format!("  jmp {base}_rounded\n{base}_up:\n  addq $1, %rax\n  jmp {base}_rounded\n{base}_down:\n  subq $1, %rax\n{base}_rounded:\n  cvtsi2sdq %rax, %xmm2\n{base}_bounds:\n  # range policy after rounding, then convert\n"));
    bound(ty.minimum() as f64, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {base}_minimum\n"));
    bound((ty.maximum() + 1) as f64, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {base}_maximum\n"));
    if ty == IntegerType::U64 {
        bound(9223372036854775808.0, output);
        output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {base}_small\n  movapd %xmm2, %xmm3\n  subsd %xmm1, %xmm3\n  cvttsd2siq %xmm3, %rax\n  btcq $63, %rax\n  jmp {base}_done\n{base}_small:\n"));
    }
    output.push_str(&format!(
        "  cvttsd2siq %xmm2, %rax\n  jmp {base}_done\n{base}_nonfinite:\n"
    ));
    if c.mode.saturates() {
        output.push_str(&format!("  ucomisd %xmm2, %xmm2\n  jp {base}_zero\n"));
        bound(0.0, output);
        output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  ja {base}_maximum\n  jmp {base}_minimum\n{base}_zero:\n  xorq %rax, %rax\n  jmp {base}_done\n{base}_minimum:\n  movabsq ${}, %rax\n  jmp {base}_done\n{base}_maximum:\n  movabsq ${}, %rax\n", ty.minimum(), ty.maximum() as i64));
    } else {
        reporter.emit(FailureCode::ConversionNotFinite, output);
        output.push_str(&format!("{base}_minimum:\n{base}_maximum:\n"));
        reporter.emit(FailureCode::ConversionOutOfRange, output);
    }
    output.push_str(&format!("{base}_done:\n"));
}
