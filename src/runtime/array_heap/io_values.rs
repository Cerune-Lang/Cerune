use super::ArrayValue;
use crate::{ir, types::IntegerType};

#[derive(Clone, Copy)]
pub(crate) enum Element {
    Byte,
    String,
}

pub(crate) trait ArrayIoValue: Clone {
    fn byte(n: u8) -> Self;
    fn unsigned(&self, ty: IntegerType) -> Option<u64>;
    fn array(&self, element: Element) -> Option<&ArrayValue<Self>>;
    fn byte_array(storage: ArrayValue<Self>) -> Self;
}
impl ArrayIoValue for crate::runtime::value::Value {
    fn byte(n: u8) -> Self {
        Self::Number(crate::runtime::numeric::Number::Integer(
            i128::from(n),
            IntegerType::U8,
        ))
    }
    fn unsigned(&self, ty: IntegerType) -> Option<u64> {
        match self {
            Self::Number(crate::runtime::numeric::Number::Integer(n, t))
                if *t == ty && ty.contains(*n) =>
            {
                u64::try_from(*n).ok()
            }
            _ => None,
        }
    }
    fn array(&self, expected: Element) -> Option<&ArrayValue<Self>> {
        let ty = match expected {
            Element::Byte => ir::Type::Integer(IntegerType::U8),
            Element::String => ir::Type::String,
        };
        match self {
            Self::DynamicArray { element, storage } if *element == ty => Some(storage),
            _ => None,
        }
    }
    fn byte_array(storage: ArrayValue<Self>) -> Self {
        Self::DynamicArray {
            element: ir::Type::Integer(IntegerType::U8),
            storage,
        }
    }
}
impl ArrayIoValue for crate::vm::Value {
    fn byte(n: u8) -> Self {
        Self::Integer(i128::from(n), IntegerType::U8)
    }
    fn unsigned(&self, ty: IntegerType) -> Option<u64> {
        match self {
            Self::Integer(n, t) if *t == ty && ty.contains(*n) => u64::try_from(*n).ok(),
            _ => None,
        }
    }
    fn array(&self, expected: Element) -> Option<&ArrayValue<Self>> {
        let ty = match expected {
            Element::Byte => crate::bytecode::Type::Integer(IntegerType::U8),
            Element::String => crate::bytecode::Type::String,
        };
        match self {
            Self::DynamicArray { element, storage } if *element == ty => Some(storage),
            _ => None,
        }
    }
    fn byte_array(storage: ArrayValue<Self>) -> Self {
        Self::DynamicArray {
            element: crate::bytecode::Type::Integer(IntegerType::U8),
            storage,
        }
    }
}
