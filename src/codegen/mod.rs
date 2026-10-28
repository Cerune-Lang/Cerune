pub mod c;
mod call;
mod display;
pub mod llvm;
mod output;
pub mod qbe;
mod support;
#[cfg(test)]
#[path = "../../tests/support/process.rs"]
mod test_process;
pub mod wat;
pub mod x86_64;
pub mod x86_64_win;

pub use c::emit_c;
pub use llvm::emit_llvm;
pub use qbe::emit_qbe;
pub use wat::emit_wat;
pub use x86_64_win::emit_x86_64_win_asm;

/// 変換の意味を保持します。Emitterがソース構文を解釈し直す必要はありません。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NumericConversion {
    pub mode: crate::types::ConversionMode,
    pub from: crate::types::NumericType,
    pub to: crate::types::NumericType,
}

impl NumericConversion {
    pub fn truncates(self) -> bool {
        self.mode.rounding() == Some(crate::types::RoundingMode::Truncate) && !self.mode.saturates()
    }
    /// 下限と、その境界自体を拒否するかを返します。
    /// i64の最小値-1はf64で区別できないため、表せる最小値を含む境界にします。
    pub fn integer_lower_bound(self) -> (f64, bool) {
        let crate::types::NumericType::Integer(ty) = self.to else {
            unreachable!()
        };
        if self.truncates() && ty != crate::types::IntegerType::I64 {
            ((ty.minimum() - 1) as f64, true)
        } else {
            (ty.minimum() as f64, false)
        }
    }
    pub fn uses_u64(self) -> bool {
        matches!(
            self.from,
            crate::types::NumericType::Integer(crate::types::IntegerType::U64)
        ) || matches!(
            self.to,
            crate::types::NumericType::Integer(crate::types::IntegerType::U64)
        )
    }

    pub fn helper(self) -> String {
        let operation = if self.mode == crate::types::ConversionMode::Exact {
            "convert"
        } else {
            self.mode.name()
        };
        format!("cerune_{operation}_{}_{}", self.from.name(), self.to.name())
    }
}

/// 演算は64ビットで行い、値の範囲は元の整数型で検査します。
/// 格納幅と意味上の幅を区別し、狭い整数の桁あふれを隠しません。
fn integer_range_check(expr: &crate::ir::Expr) -> Option<crate::types::IntegerType> {
    use crate::{
        ir::{ExprKind, Type},
        types::IntegerType,
    };
    if !matches!(
        expr.kind,
        ExprKind::Binary { .. } | ExprKind::Unary { .. } | ExprKind::ConvertInteger { .. }
    ) {
        return None;
    }
    let Type::Integer(ty) = expr.ty else {
        return None;
    };
    match ty {
        IntegerType::I8
        | IntegerType::U8
        | IntegerType::I16
        | IntegerType::U16
        | IntegerType::I32
        | IntegerType::U32 => Some(ty),
        IntegerType::I64 | IntegerType::U64 => None,
    }
}

/// 整数専用の演算を、元の型の幅とともに各出力先へ渡します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegerBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

impl IntegerBinaryOp {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::BitAnd => "bit_and",
            Self::BitOr => "bit_or",
            Self::BitXor => "bit_xor",
            Self::ShiftLeft => "shl",
            Self::ShiftRight => "shr",
        }
    }
    pub fn helper(self, ty: crate::types::IntegerType) -> String {
        format!("cerune_{}_{}", ty.name(), self.name())
    }
}

fn integer_binary_op(op: crate::ir::BinaryOp, ty: &crate::ir::Type) -> Option<IntegerBinaryOp> {
    if is_u64(ty) {
        match op {
            crate::ir::BinaryOp::Add => return Some(IntegerBinaryOp::Add),
            crate::ir::BinaryOp::Subtract => return Some(IntegerBinaryOp::Subtract),
            crate::ir::BinaryOp::Multiply => return Some(IntegerBinaryOp::Multiply),
            crate::ir::BinaryOp::Divide => return Some(IntegerBinaryOp::Divide),
            _ => {}
        }
    }
    use crate::ir::BinaryOp;
    match op {
        BinaryOp::Remainder => Some(IntegerBinaryOp::Remainder),
        BinaryOp::BitAnd => Some(IntegerBinaryOp::BitAnd),
        BinaryOp::BitOr => Some(IntegerBinaryOp::BitOr),
        BinaryOp::BitXor => Some(IntegerBinaryOp::BitXor),
        BinaryOp::ShiftLeft => Some(IntegerBinaryOp::ShiftLeft),
        BinaryOp::ShiftRight => Some(IntegerBinaryOp::ShiftRight),
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => None,
    }
}

fn integer_type(ty: &crate::ir::Type) -> crate::types::IntegerType {
    let crate::ir::Type::Integer(ty) = ty else {
        unreachable!("integer operation must have integer operands")
    };
    *ty
}

fn complement_mask(ty: &crate::ir::Type) -> i64 {
    let ty = integer_type(ty);
    if ty.is_signed() {
        -1
    } else {
        ty.maximum() as i64
    }
}

fn is_u64(ty: &crate::ir::Type) -> bool {
    matches!(ty, crate::ir::Type::Integer(crate::types::IntegerType::U64))
}

fn u64_integer_conversion(expr: &crate::ir::Expr) -> Option<(&crate::ir::Expr, NumericConversion)> {
    let crate::ir::ExprKind::ConvertInteger {
        value, from, to, ..
    } = &expr.kind
    else {
        return None;
    };
    let conversion = NumericConversion {
        mode: crate::types::ConversionMode::Exact,
        from: crate::types::NumericType::Integer(*from),
        to: crate::types::NumericType::Integer(*to),
    };
    (from != to && conversion.uses_u64()).then_some((value, conversion))
}
