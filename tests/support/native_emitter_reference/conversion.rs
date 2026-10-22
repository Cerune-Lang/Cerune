use super::failure::Reporter;
use crate::runtime::FailureCode as Failure;
use crate::{codegen::NumericConversion, types::NumericType};

fn emit_truncation(
    conversion: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut String,
) {
    let NumericType::Integer(ty) = conversion.to else {
        unreachable!()
    };
    let base = format!(".Lcerune_{prefix}_trunc_{label}");
    if conversion.from == NumericType::F32 {
        output.push_str("  cvtss2sd %xmm0, %xmm2\n");
    } else {
        output.push_str("  movapd %xmm0, %xmm2\n");
    }
    output.push_str(&format!("  ucomisd %xmm2, %xmm2\n  jp {base}_nonfinite\n"));
    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
        load_bound(infinity, output);
        output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {base}_nonfinite\n"));
    }
    let (lower, exclusive) = conversion.integer_lower_bound();
    let jump = if exclusive { "jbe" } else { "jb" };
    load_bound(lower, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  {jump} {base}_range\n"));
    load_bound((ty.maximum() + 1) as f64, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {base}_range\n"));
    if ty == crate::types::IntegerType::U64 {
        // SSE2の符号付き変換を使い、u64の上半分は2^63を引いてから最上位ビットを戻します。
        load_bound(9223372036854775808.0, output);
        output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {base}_small\n  movapd %xmm2, %xmm3\n  subsd %xmm1, %xmm3\n  cvttsd2siq %xmm3, %rax\n  btcq $63, %rax\n  jmp {base}_done\n{base}_small:\n"));
    }
    output.push_str(&format!(
        "  cvttsd2siq %xmm2, %rax\n  jmp {base}_done\n{base}_nonfinite:\n"
    ));
    reporter.emit(Failure::ConversionNotFinite, output);
    output.push_str(&format!("{base}_range:\n"));
    reporter.emit(Failure::ConversionOutOfRange, output);
    output.push_str(&format!("{base}_done:\n"));
}

pub(super) fn emit(
    conversion: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut String,
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
            output.push_str("  movq %rax, %r10\n");
            if conversion.to == NumericType::F32 {
                output.push_str("  cvtsi2ssq %rax, %xmm0\n  cvtss2sd %xmm0, %xmm2\n");
            } else {
                output.push_str("  cvtsi2sdq %rax, %xmm0\n  movapd %xmm0, %xmm2\n");
            }
            // i64の最大値が2^63へ丸められても、逆変換の前に検出します。
            load_bound(9223372036854775808.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {bad}\n  cvttsd2siq %xmm2, %rax\n  cmpq %r10, %rax\n  jne {bad}\n"));
        }
        (NumericType::F32 | NumericType::F64, NumericType::Integer(ty)) => {
            if conversion.from == NumericType::F32 {
                output.push_str("  cvtss2sd %xmm0, %xmm2\n");
            } else {
                output.push_str("  movapd %xmm0, %xmm2\n");
            }
            check_finite_integer_input(&bad, reporter, output);
            load_bound(ty.minimum() as f64, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {bad}_range\n"));
            load_bound((ty.maximum() + 1) as f64, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {bad}_range\n  cvttsd2siq %xmm2, %rax\n  cvtsi2sdq %rax, %xmm1\n  ucomisd %xmm1, %xmm2\n  jne {bad}\n"));
        }
        (NumericType::F32, NumericType::F64) => {
            output.push_str(&format!(
                "  ucomiss %xmm0, %xmm0\n  jp {bad}_nan\n  cvtss2sd %xmm0, %xmm0\n"
            ));
        }
        (NumericType::F64, NumericType::F32) => {
            output.push_str(&format!(
                "  ucomisd %xmm0, %xmm0\n  jp {bad}_nan\n  movapd %xmm0, %xmm2\n"
            ));
            // 無限大は保存できますが、有限値がf32最大値を越える場合は範囲外です。
            load_bound(f64::INFINITY, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {bad}_convert\n"));
            load_bound(f64::NEG_INFINITY, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {bad}_convert\n"));
            load_bound(f64::from(f32::MAX), output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  ja {bad}_range\n"));
            load_bound(-f64::from(f32::MAX), output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {bad}_range\n{bad}_convert:\n  cvtsd2ss %xmm0, %xmm0\n  cvtss2sd %xmm0, %xmm1\n  ucomisd %xmm1, %xmm2\n  jne {bad}\n"));
        }
        _ => unreachable!("identity and integer conversions are lowered separately"),
    }
    output.push_str(&format!("  jmp {done}\n{bad}:\n"));
    reporter.emit(Failure::ConversionInexact, output);
    if matches!(conversion.from, NumericType::F32 | NumericType::F64) {
        output.push_str(&format!("{bad}_range:\n"));
        reporter.emit(Failure::ConversionOutOfRange, output);
        output.push_str(&format!("{bad}_nan:\n"));
        reporter.emit(Failure::ConversionNaN, output);
    }
    output.push_str(&format!("{done}:\n"));
}

/// xmm2のfloatを整数化する前に、NaN/無限大と負のゼロを区別します。
pub(super) fn check_finite_integer_input(bad: &str, reporter: &mut Reporter, output: &mut String) {
    output.push_str(&format!("  ucomisd %xmm2, %xmm2\n  jp {bad}_nonfinite\n"));
    load_bound(f64::INFINITY, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {bad}_nonfinite\n"));
    load_bound(f64::NEG_INFINITY, output);
    output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  je {bad}_nonfinite\n  movq %xmm2, %r11\n  movabsq $-9223372036854775808, %r10\n  cmpq %r10, %r11\n  jne {bad}_finite\n"));
    reporter.emit(Failure::ConversionNegativeZero, output);
    output.push_str(&format!("{bad}_nonfinite:\n"));
    reporter.emit(Failure::ConversionNotFinite, output);
    output.push_str(&format!("{bad}_finite:\n"));
}

fn load_bound(value: f64, output: &mut String) {
    output.push_str(&format!(
        "  movabsq ${}, %r11\n  movq %r11, %xmm1\n",
        value.to_bits() as i64
    ));
}
