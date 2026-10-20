use std::fmt::Write;

use crate::runtime::{FailureCode, RuntimeFailure};

use super::ir::{Instruction, Origin};

pub(super) fn helper_name(instruction: &Instruction, origin: Origin) -> String {
    let base = match instruction {
        Instruction::Output { kind, newline, .. } => {
            format!("cerune_output_{kind}_lf{}", u8::from(*newline))
        }
        Instruction::ArrayAllocate { width, stride } => {
            format!("cerune_array_allocate_w{width}_s{stride}")
        }
        Instruction::ArrayAddress => "cerune_array_index".into(),
        Instruction::ArrayRangeCheck => "cerune_array_range".into(),
        Instruction::ArgumentCount => "cerune_arguments_count".into(),
        Instruction::ArgumentGet => "cerune_arguments_get".into(),
        Instruction::ReadLine => "cerune_input_read_line".into(),
        Instruction::StringConcat => "cerune_string_concat".into(),
        Instruction::ConvertNumeric { conversion } => conversion.helper(),
        Instruction::IntegerBinary { op, ty } => op.helper(*ty),
        Instruction::CheckIntegerRange { ty, failure } => {
            format!("cerune_check_{}_{}", ty.name(), failure.name())
        }
        Instruction::CheckedI64Add => "cerune_i64_add".into(),
        Instruction::CheckedI64Sub => "cerune_i64_sub".into(),
        Instruction::CheckedI64Mul => "cerune_i64_mul".into(),
        Instruction::CheckedI64DivS => "cerune_i64_div".into(),
        _ => unreachable!("only checked operations have specialized helpers"),
    };
    format!(
        "{base}_n{}_b{}_{}",
        origin.node_id.0,
        origin.span.start(),
        origin.span.end()
    )
}

/// レコードを命令内の定数として持ち、ホストには出力バイトだけを渡します。
/// 線形メモリ、現在位置の可変変数、診断用の公開状態は追加しません。
pub(super) fn emit(record: RuntimeFailure, prefix: &str, output: &mut String) {
    // バイト定数を手で復号せず、生成物から理由とソース位置を読めるようにします。
    writeln!(output, "{prefix};; cerune: {}", record.record()).unwrap();
    for byte in format!("cerune: {}\n", record.record()).bytes() {
        writeln!(
            output,
            "{prefix}i32.const {byte}\n{prefix}call $write_error_byte"
        )
        .unwrap();
    }
    writeln!(output, "{prefix}unreachable").unwrap();
}

pub(super) fn emit_if(condition: &str, code: FailureCode, origin: Origin, output: &mut String) {
    output.push_str(condition);
    output.push_str("    if\n");
    emit(
        RuntimeFailure {
            code,
            node_id: origin.node_id,
            span: origin.span,
        },
        "      ",
        output,
    );
    output.push_str("    end\n");
}
