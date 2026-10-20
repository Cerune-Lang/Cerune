pub mod arguments;
pub(crate) mod array_heap;
pub(crate) mod float_output;
pub mod input;
pub(crate) mod numeric;
pub mod output;
pub(crate) mod string_heap;
pub(crate) mod value;

// 出力経路をまたいで照合する、ソースに由来する実行時停止の契約です。

use crate::{ir::NodeId, source::Span, vm::VmErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    AllocationSizeOverflow,
    AllocationLimitExceeded,
    AllocationFailed,
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
    ArrayRangeOutOfBounds,
    ArrayLengthOutOfRange,
}

impl FailureCode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AllocationSizeOverflow => "allocation-size-overflow",
            Self::AllocationLimitExceeded => "allocation-limit-exceeded",
            Self::AllocationFailed => "allocation-failed",
            Self::IntegerOverflow => "integer-overflow",
            Self::DivisionByZero => "division-by-zero",
            Self::DivisionOverflow => "division-overflow",
            Self::RemainderByZero => "remainder-by-zero",
            Self::InvalidShiftCount => "invalid-shift-count",
            Self::IntegerConversionOutOfRange => "integer-conversion-out-of-range",
            Self::ConversionOutOfRange => "conversion-out-of-range",
            Self::ConversionInexact => "conversion-inexact",
            Self::ConversionNotFinite => "conversion-not-finite",
            Self::ConversionNaN => "conversion-nan",
            Self::ConversionNegativeZero => "conversion-negative-zero",
            Self::ArrayIndexOutOfBounds => "array-index-out-of-bounds",
            Self::ArrayRangeOutOfBounds => "array-range-out-of-bounds",
            Self::ArrayLengthOutOfRange => "array-length-out-of-range",
        }
    }

    /// 壊れたbytecode等のVM内部エラーを、言語の停止理由へ偽装しません。
    pub fn from_vm(kind: VmErrorKind) -> Option<Self> {
        use crate::vm::NumericConversionFailure as C;
        Some(match kind {
            VmErrorKind::AllocationSizeOverflow => Self::AllocationSizeOverflow,
            VmErrorKind::AllocationLimitExceeded => Self::AllocationLimitExceeded,
            VmErrorKind::AllocationFailed => Self::AllocationFailed,
            VmErrorKind::IntegerOverflow { .. } => Self::IntegerOverflow,
            VmErrorKind::DivisionByZero => Self::DivisionByZero,
            VmErrorKind::DivisionOverflow => Self::DivisionOverflow,
            VmErrorKind::RemainderByZero => Self::RemainderByZero,
            VmErrorKind::InvalidShiftCount { .. } => Self::InvalidShiftCount,
            VmErrorKind::IntegerConversionOutOfRange { .. } => Self::IntegerConversionOutOfRange,
            VmErrorKind::NumericConversionFailed { reason, .. } => match reason {
                C::OutOfRange => Self::ConversionOutOfRange,
                C::Inexact => Self::ConversionInexact,
                C::NotFinite => Self::ConversionNotFinite,
                C::NaN => Self::ConversionNaN,
                C::NegativeZero => Self::ConversionNegativeZero,
            },
            VmErrorKind::ArrayIndexOutOfBounds { .. } => Self::ArrayIndexOutOfBounds,
            VmErrorKind::ArrayRangeOutOfBounds => Self::ArrayRangeOutOfBounds,
            VmErrorKind::ArrayLengthOutOfRange => Self::ArrayLengthOutOfRange,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFailure {
    pub code: FailureCode,
    pub node_id: NodeId,
    pub span: Span,
}

impl RuntimeFailure {
    /// ASCIIのみ。パス・ソース本文・実行時の値を含めません。
    pub fn record(self) -> String {
        format!(
            "runtime-v1 code={} node={}{} bytes={}..{}",
            self.code.name(),
            self.node_id.0,
            self.span.source_id().record_field(),
            self.span.start(),
            self.span.end()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_record_does_not_include_vm_internals_or_source_text() {
        assert_eq!(
            RuntimeFailure {
                code: FailureCode::DivisionByZero,
                node_id: NodeId(2),
                span: Span::new(6, 11)
            }
            .record(),
            "runtime-v1 code=division-by-zero node=2 bytes=6..11"
        );
        assert_eq!(FailureCode::from_vm(VmErrorKind::StackUnderflow), None);
        assert_eq!(
            FailureCode::from_vm(VmErrorKind::InstructionOutOfBounds),
            None
        );
    }
}
