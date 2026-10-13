//! 演算の意味を変えずに、局所値参照の表現だけを写します。
use super::{InstructionKind as I, Operation as O};

impl<R: Copy> O<R> {
    pub fn map_operands<S>(&self, mut f: impl FnMut(R) -> S) -> O<S> {
        match self {
            O::Literal(v) => O::Literal(v.clone()),
            O::Copy(v) => O::Copy(f(*v)),
            O::Unary { op, value } => O::Unary {
                op: *op,
                value: f(*value),
            },
            O::Binary { op, left, right } => O::Binary {
                op: *op,
                left: f(*left),
                right: f(*right),
            },
            O::ConvertInteger {
                value,
                from,
                to,
                syntax,
            } => O::ConvertInteger {
                value: f(*value),
                from: *from,
                to: *to,
                syntax: *syntax,
            },
            O::ConvertNumeric {
                value,
                from,
                to,
                mode,
                syntax,
            } => O::ConvertNumeric {
                value: f(*value),
                from: *from,
                to: *to,
                mode: *mode,
                syntax: *syntax,
            },
            O::Array(v) => O::Array(v.iter().map(|v| f(*v)).collect()),
            O::Construct { ty, base, fields } => O::Construct {
                ty: *ty,
                base: base.map(&mut f),
                fields: fields.iter().map(|(id, v)| (*id, f(*v))).collect(),
            },
            O::Field { base, ty, field } => O::Field {
                base: f(*base),
                ty: *ty,
                field: *field,
            },
            O::Index { base, index } => O::Index {
                base: f(*base),
                index: f(*index),
            },
            O::Call {
                function,
                arguments,
                ownership,
            } => O::Call {
                function: *function,
                arguments: arguments.iter().map(|v| f(*v)).collect(),
                ownership: *ownership,
            },
            O::ArrayLength(v) => O::ArrayLength(f(*v)),
            O::StringByteLength(v) => O::StringByteLength(f(*v)),
            O::StringConcat { left, right } => O::StringConcat {
                left: f(*left),
                right: f(*right),
            },
            O::ArrayAllocate {
                length,
                element_width,
            } => O::ArrayAllocate {
                length: f(*length),
                element_width: *element_width,
            },
            O::ArrayReleaseOwner(v) => O::ArrayReleaseOwner(f(*v)),
        }
    }
}
impl<R: Copy> I<R> {
    /// 書込み先も含みます。読取り集合とは区別します。
    pub fn map_operands<S>(&self, mut f: impl FnMut(R) -> S) -> I<S> {
        match self {
            I::Assign { destination, value } => I::Assign {
                destination: f(*destination),
                value: value.map_operands(f),
            },
            I::Call {
                function,
                arguments,
                ownership,
            } => I::Call {
                function: *function,
                arguments: arguments.iter().map(|v| f(*v)).collect(),
                ownership: *ownership,
            },
            I::CheckIndex { root, path } => I::CheckIndex {
                root: f(*root),
                path: path.iter().map(|v| f(*v)).collect(),
            },
            I::Store { root, path, value } => I::Store {
                root: f(*root),
                path: path.iter().map(|v| f(*v)).collect(),
                value: f(*value),
            },
            I::Flush => I::Flush,
            I::Output {
                value,
                newline,
                quoted,
            } => I::Output {
                value: f(*value),
                newline: *newline,
                quoted: *quoted,
            },
            I::ArrayInitialize { array, value } => I::ArrayInitialize {
                array: f(*array),
                value: f(*value),
            },
            I::ArrayRetain { value } => I::ArrayRetain { value: f(*value) },
            I::ArrayFree { value } => I::ArrayFree { value: f(*value) },
            I::ArrayRangeCheck { length, start, end } => I::ArrayRangeCheck {
                length: f(*length),
                start: f(*start),
                end: f(*end),
            },
            I::StringManage { value, retain } => I::StringManage {
                value: f(*value),
                retain: *retain,
            },
        }
    }
}
