use std::fmt::Write;

use crate::runtime::FailureCode;

use super::ir::{BinaryOp, FailureOrigin, Instruction, Module};

fn origin(instruction: &Instruction) -> Option<FailureOrigin> {
    match instruction {
        Instruction::ArrayAllocate { origin, .. }
        | Instruction::ArrayRangeCheck { origin, .. }
        | Instruction::ArrayAddress { origin, .. }
        | Instruction::StringConcat { origin, .. }
        | Instruction::ConvertNumeric { origin, .. }
        | Instruction::IntegerBinary { origin, .. }
        | Instruction::CheckIntegerRange { origin, .. }
        | Instruction::CheckedI64Negate { origin, .. }
        | Instruction::Abort { origin }
        | Instruction::Binary {
            origin,
            op:
                BinaryOp::CheckedI64Add
                | BinaryOp::CheckedI64Subtract
                | BinaryOp::CheckedI64Multiply
                | BinaryOp::CheckedI64Divide,
            ..
        } => Some(*origin),
        _ => None,
    }
}

fn suffix(origin: FailureOrigin) -> String {
    format!(
        " node={}{} bytes={}..{}\n",
        origin.node.0,
        origin.span.source_id().record_field(),
        origin.span.start(),
        origin.span.end()
    )
}

fn symbol(origin: FailureOrigin) -> String {
    format!(
        "cerune_origin_{}_{}_{}",
        origin.node.0,
        origin.span.start(),
        origin.span.end()
    )
}

pub(super) fn arguments(origin: FailureOrigin) -> String {
    format!("l ${}, l {}", symbol(origin), suffix(origin).len())
}

pub(super) fn code_arguments(code: FailureCode) -> String {
    format!(
        "l $cerune_code_{}, l {}",
        code.name().replace('-', "_"),
        prefix(code).len()
    )
}

fn prefix(code: FailureCode) -> String {
    format!("cerune: runtime-v1 code={}", code.name())
}

pub(super) fn call(code: FailureCode, output: &mut String) {
    writeln!(
        output,
        "  call $cerune_runtime_failure({}, l %origin, l %origin_len)\n  hlt",
        code_arguments(code)
    )
    .unwrap();
}

pub(super) fn block(label: &str, code: FailureCode, output: &mut String) {
    writeln!(output, "@{label}").unwrap();
    call(code, output);
}

pub(super) fn emit(module: &Module, output: &mut String) {
    let mut origins = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .filter_map(origin)
    {
        if !origins.contains(&item) {
            origins.push(item);
        }
    }
    if origins.is_empty() && module.string_heap_limit.is_none() && module.array_heap_limit.is_none()
    {
        return;
    }
    for origin in origins {
        data(module, &symbol(origin), &suffix(origin), output);
    }
    use FailureCode::*;
    let mut codes = vec![
        IntegerOverflow,
        DivisionByZero,
        DivisionOverflow,
        RemainderByZero,
        InvalidShiftCount,
        IntegerConversionOutOfRange,
        ConversionOutOfRange,
        ConversionInexact,
        ConversionNotFinite,
        ConversionNaN,
        ConversionNegativeZero,
        ArrayIndexOutOfBounds,
    ];
    if module.string_heap_limit.is_some() || module.array_heap_limit.is_some() {
        codes.extend([
            AllocationSizeOverflow,
            AllocationLimitExceeded,
            AllocationFailed,
        ]);
    }
    if module.array_heap_limit.is_some() {
        codes.push(ArrayRangeOutOfBounds);
        codes.push(ArrayLengthOutOfRange);
    }
    for code in codes {
        data(
            module,
            &format!("cerune_code_{}", code.name().replace('-', "_")),
            &prefix(code),
            output,
        );
        writeln!(
            output,
            "function $cerune_fail_{}(l %origin, l %origin_len) {{\n@start",
            code.name().replace('-', "_")
        )
        .unwrap();
        call(code, output);
        output.push_str("}\n\n");
    }
    // 失敗時だけstdoutをflushし、明示ターゲットのCRTで診断を出力します。
    output.push_str("function $cerune_runtime_failure(l %code, l %code_len, l %origin, l %origin_len) {\n@start\n  call $fflush(l 0)\n");
    if module.target == Some(super::Target::X86_64PcWindowsMsvc) {
        // 診断片の長さはu32に収まります。_writeのcountはWindowsでは32bitです。
        output.push_str("  %code_count =w copy %code_len\n  %origin_count =w copy %origin_len\n  call $_write(w 2, l %code, w %code_count)\n  call $_write(w 2, l %origin, w %origin_count)\n");
        // 診断済みの言語エラーでCRTの追加メッセージ・クラッシュ収集を起動しません。
        output.push_str("  call $_set_abort_behavior(w 0, w 3)\n");
    } else {
        output.push_str("  call $write(w 2, l %code, l %code_len)\n  call $write(w 2, l %origin, l %origin_len)\n");
    }
    output.push_str("  call $abort()\n  hlt\n}\n\n");
}

fn data(module: &Module, name: &str, text: &str, output: &mut String) {
    let section = module
        .target
        .unwrap_or(super::Target::X86_64UnknownLinuxGnu)
        .read_only_section();
    write!(output, "section \"{section}\" data ${name} = {{ ").unwrap();
    for (index, byte) in text.bytes().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        write!(output, "b {byte}").unwrap();
    }
    output.push_str(" }\n");
}
