use std::fmt::Write;

use super::ir::{BinaryOp, Instruction, Module, Origin};
use crate::{
    codegen::IntegerBinaryOp as Op,
    runtime::{FailureCode as Code, RuntimeFailure},
    source::Span,
    types::NumericType as N,
};

// 表の位置は生成物内だけの契約です。公開レコードには理由の文字列を使います。
const CODES: [Code; 17] = [
    Code::IntegerOverflow,
    Code::DivisionByZero,
    Code::DivisionOverflow,
    Code::RemainderByZero,
    Code::InvalidShiftCount,
    Code::IntegerConversionOutOfRange,
    Code::ConversionOutOfRange,
    Code::ConversionInexact,
    Code::ConversionNotFinite,
    Code::ConversionNaN,
    Code::ConversionNegativeZero,
    Code::ArrayIndexOutOfBounds,
    Code::AllocationSizeOverflow,
    Code::AllocationLimitExceeded,
    Code::AllocationFailed,
    Code::ArrayRangeOutOfBounds,
    Code::ArrayLengthOutOfRange,
];

pub(super) fn index(code: Code) -> usize {
    CODES
        .iter()
        .position(|candidate| *candidate == code)
        .unwrap()
}

fn codes(instruction: &Instruction) -> Vec<Code> {
    match instruction {
        Instruction::ArrayRangeCheck { .. } => vec![Code::ArrayRangeOutOfBounds],
        Instruction::ArrayAllocate { .. } => vec![
            Code::ArrayLengthOutOfRange,
            Code::AllocationSizeOverflow,
            Code::AllocationLimitExceeded,
            Code::AllocationFailed,
        ],
        Instruction::StringConcat { .. } => vec![
            Code::AllocationSizeOverflow,
            Code::AllocationLimitExceeded,
            Code::AllocationFailed,
        ],
        Instruction::Binary { op, .. } => match op {
            BinaryOp::CheckedI64Div => vec![Code::DivisionByZero, Code::DivisionOverflow],
            BinaryOp::CheckedI64Add | BinaryOp::CheckedI64Sub | BinaryOp::CheckedI64Mul => {
                vec![Code::IntegerOverflow]
            }
            _ => vec![],
        },
        Instruction::IntegerBinary { op, .. } => match op {
            Op::Add | Op::Subtract | Op::Multiply => vec![Code::IntegerOverflow],
            Op::Divide => vec![Code::DivisionByZero],
            Op::Remainder => vec![Code::RemainderByZero],
            Op::ShiftLeft => vec![Code::InvalidShiftCount, Code::IntegerOverflow],
            Op::ShiftRight => vec![Code::InvalidShiftCount],
            _ => vec![],
        },
        Instruction::CheckIntegerRange { failure, .. } => vec![*failure],
        Instruction::ArrayGet { .. } | Instruction::ArraySet { .. } => {
            vec![Code::ArrayIndexOutOfBounds]
        }
        Instruction::ConvertNumeric { conversion, .. } if conversion.mode.saturates() => vec![],
        Instruction::ConvertNumeric { conversion, .. } if conversion.mode.rounding().is_some() => {
            vec![Code::ConversionNotFinite, Code::ConversionOutOfRange]
        }
        Instruction::ConvertNumeric { conversion, .. } => match (conversion.from, conversion.to) {
            (N::Integer(_), N::Integer(_)) => vec![Code::IntegerConversionOutOfRange],
            (N::Integer(_), _) => vec![Code::ConversionInexact],
            (_, N::Integer(_)) => vec![
                Code::ConversionNotFinite,
                Code::ConversionNegativeZero,
                Code::ConversionOutOfRange,
                Code::ConversionInexact,
            ],
            (N::F32, N::F64) => vec![Code::ConversionNaN],
            (N::F64, N::F32) => vec![
                Code::ConversionNaN,
                Code::ConversionOutOfRange,
                Code::ConversionInexact,
            ],
            _ => unreachable!("identity conversions need no helper"),
        },
        _ => vec![],
    }
}

pub(super) fn first_failure_span(module: &Module) -> Option<Span> {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .find_map(|item| match item.origin {
            Origin::Source { span, .. } if !codes(&item.instruction).is_empty() => Some(span),
            _ => None,
        })
}

pub(super) fn argument(instruction: &Instruction, origin: Origin) -> String {
    if codes(instruction).is_empty() {
        "null".into()
    } else {
        table_name(origin)
    }
}

fn table_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        panic!("language failures require a source origin")
    };
    format!(
        "@cerune.failure.{}.{}.{}",
        node_id.0,
        span.start(),
        span.end()
    )
}

pub(super) fn emit_data(module: &Module, output: &mut String) {
    let mut sites: Vec<(Origin, Vec<Code>)> = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        let required = codes(&item.instruction);
        if required.is_empty() {
            continue;
        }
        let slot = match sites.iter().position(|(origin, _)| *origin == item.origin) {
            Some(slot) => slot,
            None => {
                sites.push((item.origin, vec![]));
                sites.len() - 1
            }
        };
        for code in required {
            if !sites[slot].1.contains(&code) {
                sites[slot].1.push(code);
            }
        }
    }
    // 正常経路は不変な表を引数で渡すだけで、現在位置を書き換えません。
    for (origin, required) in sites {
        let Origin::Source { node_id, span } = origin else {
            unreachable!()
        };
        let name = table_name(origin);
        let count = if module.array_heap_limit.is_some() {
            CODES.len()
        } else if module.string_heap_limit.is_some() {
            15
        } else {
            12
        };
        let mut lengths = vec![0; count];
        for code in required {
            let id = index(code);
            let record = format!(
                "cerune: {}\n",
                RuntimeFailure {
                    code,
                    node_id,
                    span
                }
                .record()
            );
            lengths[id] = record.len();
            write!(
                output,
                "{name}.{id} = private unnamed_addr constant [{} x i8] c\"",
                record.len()
            )
            .unwrap();
            for byte in record.bytes() {
                write!(output, "\\{byte:02X}").unwrap();
            }
            output.push_str("\"\n");
        }
        write!(
            output,
            "{name} = private constant [{count} x {{ ptr, i64 }}] ["
        )
        .unwrap();
        for (id, length) in lengths.into_iter().enumerate() {
            if id > 0 {
                output.push_str(", ");
            }
            if length == 0 {
                output.push_str("{ ptr, i64 } zeroinitializer");
            } else {
                write!(output, "{{ ptr, i64 }} {{ ptr {name}.{id}, i64 {length} }}").unwrap();
            }
        }
        output.push_str("]\n");
    }
}

pub(super) fn emit_support(module: &Module, output: &mut String) {
    output.push_str("declare i32 @fflush(ptr)\n");
    match module
        .target
        .expect("runtime diagnostics require an explicit target")
    {
        super::Target::X86_64UnknownLinuxGnu => {
            output.push_str("declare i64 @write(i32, ptr, i64)\n")
        }
        super::Target::X86_64PcWindowsMsvc => {
            output.push_str("declare i32 @_write(i32, ptr, i32)\n")
        }
    }
    output.push_str("\ndefine internal void @cerune.runtime.fail(ptr %failure, i64 %code) {\nentry:\n  call i32 @fflush(ptr null)\n  %slot = getelementptr inbounds { ptr, i64 }, ptr %failure, i64 %code\n  %record = load { ptr, i64 }, ptr %slot\n  %data = extractvalue { ptr, i64 } %record, 0\n  %length = extractvalue { ptr, i64 } %record, 1\n");
    match module.target.unwrap() {
        super::Target::X86_64UnknownLinuxGnu => output.push_str("  call i64 @write(i32 2, ptr %data, i64 %length)\n"),
        super::Target::X86_64PcWindowsMsvc => output.push_str("  %count = trunc i64 %length to i32\n  call i32 @_write(i32 2, ptr %data, i32 %count)\n"),
    }
    output.push_str("  call void @llvm.trap()\n  unreachable\n}\n\n");
}

pub(super) fn emit_trap(code: Code, output: &mut String) {
    writeln!(
        output,
        "  call void @cerune.runtime.fail(ptr %failure, i64 {})\n  unreachable",
        index(code)
    )
    .unwrap();
}
