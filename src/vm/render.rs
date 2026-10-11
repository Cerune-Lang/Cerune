use crate::bytecode::Type;
use crate::source::{SourceId, SourceLocation, SourceMap, Span};

use super::{IntegerOperation, VmError, VmErrorKind};

/// VM実行エラーをソース本文やファイルパスを含まない簡潔な形式で描画します。
pub fn render_compact(error: &VmError) -> String {
    let position = match error.function_id() {
        Some(function_id) => format!(
            "function {function_id} bytecode instruction {:04}",
            error.instruction_index()
        ),
        None => format!("bytecode instruction {:04}", error.instruction_index()),
    };
    format!("{} at {position}", render_message(error))
}

/// VM実行エラーをソース位置とbytecode命令番号を含む簡潔な形式で描画します。
///
/// ソース位置を解決できない場合は、bytecode命令番号だけを含む形式へ戻します。
pub fn render_compact_with_source(error: &VmError, source: &str, span: Span) -> String {
    if span.source_id() != SourceId::ANONYMOUS {
        return render_compact_with_sources(error, &SourceMap::new(), span);
    }
    let Some(location) = SourceLocation::from_offset(source, span.start()) else {
        return render_compact(error);
    };

    let position = match error.function_id() {
        Some(function_id) => format!(
            "function {function_id} bytecode instruction {:04}",
            error.instruction_index()
        ),
        None => format!("bytecode instruction {:04}", error.instruction_index()),
    };
    format!(
        "{} at {}:{} ({position})",
        render_message(error),
        location.line(),
        location.column()
    )
}

/// 実行命令の出自に対応するファイルを明示して診断します。
pub fn render_compact_with_sources(error: &VmError, sources: &SourceMap, span: Span) -> String {
    format!(
        "{}; source {}",
        render_compact(error),
        crate::diagnostic::render::render_source_position(sources, span)
    )
}

fn render_message(error: &VmError) -> String {
    match error.kind() {
        VmErrorKind::ArrayRangeOutOfBounds => "array range is out of bounds".into(),
        VmErrorKind::ArrayLengthOutOfRange => "array length is out of range".into(),
        VmErrorKind::InvalidArrayOwnership => "invalid dynamic array ownership".into(),
        VmErrorKind::AllocationSizeOverflow => {
            "allocation-size-overflow: allocation size overflow".into()
        }
        VmErrorKind::AllocationLimitExceeded => {
            "allocation-limit-exceeded: live storage exceeds the configured heap limit".into()
        }
        VmErrorKind::InvalidStringOwnership => {
            "invalid dynamic string ownership in bytecode".into()
        }
        VmErrorKind::AllocationFailed => "allocation-failed: allocation failed".into(),
        VmErrorKind::InvalidNumericConversion { from, to } => format!(
            "invalid rounded conversion from {} to {}",
            from.name(),
            to.name()
        ),
        VmErrorKind::NumericConversionFailed { from, to, reason } => {
            let detail = match reason {
                super::NumericConversionFailure::OutOfRange => {
                    "the value is outside the supported range"
                }
                super::NumericConversionFailure::Inexact => "the conversion would change the value",
                super::NumericConversionFailure::NotFinite => {
                    "this conversion cannot preserve NaN or infinity"
                }
                super::NumericConversionFailure::NaN => {
                    "NaN cannot be preserved when changing types"
                }
                super::NumericConversionFailure::NegativeZero => {
                    "integers cannot preserve the sign of negative zero"
                }
            };
            format!("cannot convert {} to {}: {detail}", from.name(), to.name())
        }
        VmErrorKind::InvalidShiftCount { ty, count } => format!(
            "shift count {count} must be between 0 and {} for {}",
            ty.bit_width() - 1,
            ty.name()
        ),
        VmErrorKind::RemainderByZero => {
            "cannot calculate an integer remainder with a zero divisor".to_owned()
        }
        VmErrorKind::IntegerConversionOutOfRange { from, to } => format!(
            "cannot convert {} to {}: the value is outside the supported range",
            from.name(),
            to.name()
        ),
        VmErrorKind::InvalidIntegerValue { ty } => {
            format!("Cerune VM found a value outside the {} range", ty.name())
        }
        VmErrorKind::InvalidNegationType { ty } => format!("cannot apply `-` to {}", ty.name()),
        VmErrorKind::InstructionOutOfBounds => {
            "Cerune VM reached an instruction outside the bytecode program".to_owned()
        }
        VmErrorKind::InvalidSlot { slot } => {
            format!("Cerune VM tried to access slot {slot}, but that slot does not exist")
        }
        VmErrorKind::InvalidType { type_id } => {
            format!("Cerune VM tried to use product type {type_id}, but that type does not exist")
        }
        VmErrorKind::InvalidField { type_id, field_id } => format!(
            "Cerune VM tried to use field {field_id} of product type {type_id}, but that field does not exist"
        ),
        VmErrorKind::InvalidFunction { function_id } => format!(
            "Cerune VM tried to call function {function_id}, but that function does not exist"
        ),
        VmErrorKind::InvalidArgumentCount { expected, actual } => {
            format!("Cerune VM expected {expected} function arguments, but found {actual}")
        }
        VmErrorKind::InvalidReturn => {
            "Cerune VM found a return instruction that does not match the function".to_owned()
        }
        VmErrorKind::UninitializedSlot { slot } => {
            format!("Cerune VM tried to read slot {slot} before it was initialized")
        }
        VmErrorKind::SlotAlreadyInitialized { slot } => {
            format!("Cerune VM tried to initialize slot {slot} more than once")
        }
        VmErrorKind::AssignmentToUninitializedSlot { slot } => {
            format!("Cerune VM tried to assign slot {slot} before it was initialized")
        }
        VmErrorKind::AssignmentToImmutableSlot { slot } => {
            format!("Cerune VM tried to assign immutable slot {slot}")
        }
        VmErrorKind::StackUnderflow => {
            "Cerune VM needed another value, but the stack was empty".to_owned()
        }
        VmErrorKind::TypeMismatch { expected, actual } => format!(
            "Cerune VM expected an {} value, but found an {} value",
            type_name(expected),
            type_name(actual)
        ),
        VmErrorKind::InvalidComparisonType { ty } => format!(
            "Cerune VM cannot use this comparison with {} values",
            type_name(ty)
        ),
        VmErrorKind::DivisionByZero => "cannot divide an integer by zero".to_owned(),
        VmErrorKind::DivisionOverflow => {
            "integer division produced a value outside the supported range".to_owned()
        }
        VmErrorKind::IntegerOverflow { operation, ty } => format!(
            "{} with {} values produced a result outside the supported range",
            integer_operation_name(operation),
            type_name(ty)
        ),
        VmErrorKind::ArrayIndexOutOfBounds { index, length } => {
            format!("array index {index} is outside an array of length {length}")
        }
        VmErrorKind::UnusedStackValues { count: 1 } => {
            "Cerune VM stopped with 1 unused value on the stack".to_owned()
        }
        VmErrorKind::UnusedStackValues { count } => {
            format!("Cerune VM stopped with {count} unused values on the stack")
        }
    }
}

fn integer_operation_name(operation: IntegerOperation) -> &'static str {
    match operation {
        IntegerOperation::Add => "addition",
        IntegerOperation::Subtract => "subtraction",
        IntegerOperation::Multiply => "multiplication",
        IntegerOperation::Negate => "negation",
        IntegerOperation::ShiftLeft => "left shift",
    }
}

fn type_name(ty: Type) -> String {
    match ty {
        Type::DynamicArray { element } => format!("[{}]", type_name(*element)),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Integer(ty) => ty.name().into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Named(id) => format!("product type {id}"),
        Type::Array { element, length } => format!("[{}; {length}]", type_name(*element)),
    }
}

#[cfg(test)]
mod tests {
    use super::{render_compact, render_compact_with_source};
    use crate::{
        source::Span,
        vm::{VmError, VmErrorKind},
    };

    #[test]
    fn renders_division_by_zero_in_plain_language() {
        let error = VmError::new(VmErrorKind::DivisionByZero, 2);

        assert_eq!(
            render_compact(&error),
            "cannot divide an integer by zero at bytecode instruction 0002"
        );
    }

    #[test]
    fn renders_integer_overflow_in_plain_language() {
        let error = VmError::new(
            VmErrorKind::IntegerOverflow {
                operation: crate::vm::IntegerOperation::Add,
                ty: crate::bytecode::Type::Integer(crate::types::IntegerType::I64),
            },
            2,
        );

        assert_eq!(
            render_compact(&error),
            "addition with i64 values produced a result outside the supported range at bytecode instruction 0002"
        );
    }

    #[test]
    fn renders_error_specific_values() {
        let error = VmError::new(VmErrorKind::InvalidSlot { slot: 7 }, 12);

        assert_eq!(
            render_compact(&error),
            "Cerune VM tried to access slot 7, but that slot does not exist at bytecode instruction 0012"
        );
    }

    #[test]
    fn renders_source_position_and_instruction_index() {
        let error = VmError::new(VmErrorKind::DivisionByZero, 2);

        assert_eq!(
            render_compact_with_source(&error, "print(1 / 0);", Span::new(6, 11)),
            "cannot divide an integer by zero at 1:7 (bytecode instruction 0002)"
        );
    }

    #[test]
    fn falls_back_when_source_position_is_invalid() {
        let error = VmError::new(VmErrorKind::DivisionByZero, 2);

        assert_eq!(
            render_compact_with_source(&error, "x", Span::empty(2)),
            "cannot divide an integer by zero at bytecode instruction 0002"
        );
    }
}
