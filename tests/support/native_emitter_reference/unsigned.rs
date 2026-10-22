use super::failure::Reporter;
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
    output: &mut String,
) {
    let bad = format!(".Lcerune_{prefix}_u64_bad_{label}");
    let done = format!(".Lcerune_{prefix}_u64_done_{label}");
    match op {
        Op::Add => output.push_str(&format!("  addq %rcx, %rax\n  jc {bad}\n")),
        Op::Subtract => output.push_str(&format!("  subq %rcx, %rax\n  jc {bad}\n")),
        Op::Multiply => output.push_str(&format!("  mulq %rcx\n  jc {bad}\n")),
        Op::Divide | Op::Remainder => {
            output.push_str(&format!(
                "  testq %rcx, %rcx\n  je {bad}\n  xorq %rdx, %rdx\n  divq %rcx\n"
            ));
            if op == Op::Remainder {
                output.push_str("  movq %rdx, %rax\n");
            }
        }
        Op::BitAnd => return output.push_str("  andq %rcx, %rax\n"),
        Op::BitOr => return output.push_str("  orq %rcx, %rax\n"),
        Op::BitXor => return output.push_str("  xorq %rcx, %rax\n"),
        Op::ShiftLeft | Op::ShiftRight => {
            output.push_str(&format!("  cmpq $64, %rcx\n  jae {bad}\n"));
            if op == Op::ShiftLeft {
                output.push_str(&format!("  movq $-1, %r11\n  shrq %cl, %r11\n  cmpq %r11, %rax\n  ja {bad}_overflow\n  shlq %cl, %rax\n  jmp {done}\n{bad}_overflow:\n"));
                reporter.emit(Failure::IntegerOverflow, output);
            } else {
                output.push_str("  shrq %cl, %rax\n");
            }
        }
    }
    output.push_str(&format!("  jmp {done}\n{bad}:\n"));
    reporter.emit(
        match op {
            Op::Divide => Failure::DivisionByZero,
            Op::Remainder => Failure::RemainderByZero,
            Op::ShiftLeft | Op::ShiftRight => Failure::InvalidShiftCount,
            _ => Failure::IntegerOverflow,
        },
        output,
    );
    output.push_str(&format!("{done}:\n"));
}

pub(super) fn emit_conversion(
    c: NumericConversion,
    label: usize,
    prefix: &str,
    reporter: &mut Reporter,
    output: &mut String,
) {
    let base = format!(".Lcerune_{prefix}_u64_convert_{label}");
    let bad = format!("{base}_bad");
    let done = format!("{base}_done");
    match (c.from, c.to) {
        (N::Integer(from), N::Integer(to)) => {
            if from == IntegerType::U64 {
                output.push_str(&format!(
                    "  movabsq ${}, %r11\n  cmpq %r11, %rax\n  ja {bad}\n",
                    to.maximum()
                ));
            } else if from.is_signed() {
                output.push_str(&format!("  testq %rax, %rax\n  js {bad}\n"));
            }
        }
        (N::Integer(_), N::F32 | N::F64) => {
            // SSE2の符号付き変換で上半分を扱うため、下位ビットを残して半分にしてから倍にします。
            let suffix = if c.to == N::F32 { "ss" } else { "sd" };
            output.push_str(&format!("  movq %rax, %r10\n  testq %rax, %rax\n  jns {base}_small\n  movq %rax, %r11\n  andq $1, %r11\n  shrq $1, %rax\n  orq %r11, %rax\n  cvtsi2{suffix}q %rax, %xmm0\n  add{suffix} %xmm0, %xmm0\n  jmp {base}_number\n{base}_small:\n  cvtsi2{suffix}q %rax, %xmm0\n{base}_number:\n"));
            if c.to == N::F32 {
                output.push_str("  cvtss2sd %xmm0, %xmm2\n");
            } else {
                output.push_str("  movapd %xmm0, %xmm2\n");
            }
            bound(18446744073709551616.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {bad}\n"));
            bound(9223372036854775808.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {base}_back_small\n  movapd %xmm2, %xmm3\n  subsd %xmm1, %xmm3\n  cvttsd2siq %xmm3, %rax\n  btcq $63, %rax\n  jmp {base}_compare\n{base}_back_small:\n  cvttsd2siq %xmm2, %rax\n{base}_compare:\n  cmpq %r10, %rax\n  jne {bad}\n"));
        }
        (N::F32 | N::F64, N::Integer(_)) => {
            if c.from == N::F32 {
                output.push_str("  cvtss2sd %xmm0, %xmm2\n");
            } else {
                output.push_str("  movapd %xmm0, %xmm2\n");
            }
            super::conversion::check_finite_integer_input(&bad, reporter, output);
            bound(0.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {bad}_range\n"));
            bound(18446744073709551616.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jae {bad}_range\n"));
            bound(9223372036854775808.0, output);
            output.push_str(&format!("  ucomisd %xmm1, %xmm2\n  jb {base}_small\n  movapd %xmm2, %xmm3\n  subsd %xmm1, %xmm3\n  cvttsd2siq %xmm3, %rax\n  cvtsi2sdq %rax, %xmm3\n  addsd %xmm1, %xmm3\n  btcq $63, %rax\n  jmp {base}_compare\n{base}_small:\n  cvttsd2siq %xmm2, %rax\n  cvtsi2sdq %rax, %xmm3\n{base}_compare:\n  ucomisd %xmm3, %xmm2\n  jne {bad}\n"));
        }
        _ => unreachable!("u64 conversion"),
    }
    output.push_str(&format!("  jmp {done}\n{bad}:\n"));
    reporter.emit(
        if matches!((c.from, c.to), (N::Integer(_), N::Integer(_))) {
            Failure::IntegerConversionOutOfRange
        } else {
            Failure::ConversionInexact
        },
        output,
    );
    if matches!(c.from, N::F32 | N::F64) {
        output.push_str(&format!("{bad}_range:\n"));
        reporter.emit(Failure::ConversionOutOfRange, output);
    }
    output.push_str(&format!("{done}:\n"));
}
fn bound(value: f64, output: &mut String) {
    output.push_str(&format!(
        "  movabsq ${}, %r11\n  movq %r11, %xmm1\n",
        value.to_bits() as i64
    ));
}
