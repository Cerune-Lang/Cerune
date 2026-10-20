//! VMのスタック・返却値と共通の文字列領域をつなぎます。
use super::{Value, VmErrorKind, VmResult};
pub(super) use crate::runtime::string_heap::{StringHeap, StringValue};
pub(super) fn error(e: crate::runtime::string_heap::HeapError) -> VmErrorKind {
    use crate::runtime::string_heap::HeapError as E;
    match e {
        E::AllocationSizeOverflow => VmErrorKind::AllocationSizeOverflow,
        E::AllocationLimitExceeded => VmErrorKind::AllocationLimitExceeded,
        E::AllocationFailed => VmErrorKind::AllocationFailed,
        E::InvalidStringOwnership => VmErrorKind::InvalidStringOwnership,
    }
}
pub(super) fn pop(stack: &mut Vec<Value>) -> VmResult<StringValue> {
    match super::pop_value(stack)? {
        Value::String(v) => Ok(v),
        v => Err(VmErrorKind::TypeMismatch {
            expected: crate::bytecode::Type::String,
            actual: v.ty(),
        }),
    }
}
pub(super) fn freeze(value: Value) -> Value {
    match value {
        Value::String(v) => Value::String(v.text().into()),
        Value::Array { element, values } => Value::Array {
            element,
            values: values.into_iter().map(freeze).collect(),
        },
        Value::Aggregate { type_id, fields } => Value::Aggregate {
            type_id,
            fields: fields.into_iter().map(freeze).collect(),
        },
        v => v,
    }
}

#[cfg(test)]
mod lifetime_tests {
    use super::*;
    #[test]
    fn compiled_scope_cleanup_releases_every_live_entry_allocation() {
        let sources = [
            include_str!("../../examples/string_concat.ceru"),
            include_str!("../../examples/array_copy_lifetimes.ceru"),
            r#"enum E{A{text:string},B} fn make()->E {v:E=E::A{text:concat("a","b")};return v;} mut e:E=make(); saved:E=e; e=E::B{}; print(saved);"#,
            r#"for (item:string in [concat("a","b"),concat("c","d")]) {print(item);break;}"#,
        ];
        for source in sources {
            let code = crate::compile_to_bytecode(source).unwrap();
            let mut heap = StringHeap::new(code.string_heap_limit);
            crate::vm::execute_frame(
                &code,
                crate::vm::Frame::Entry,
                vec![],
                &mut crate::runtime::output::Capture::default(),
                &mut heap,
                &mut crate::runtime::array_heap::ArrayHeap::new(code.array_heap_limit),
                &mut crate::vm::ExecutionInput {
                    stdin: &mut crate::runtime::input::unconfigured(),
                    arguments: &crate::runtime::arguments::Arguments::unconfigured(),
                },
            )
            .unwrap();
            heap.assert_empty();
        }
    }
    #[test]
    fn duplicate_release_is_an_internal_error_not_a_language_failure() {
        let mut heap = StringHeap::new(2);
        let static_value = "a".to_owned().into();
        let value = heap.concat(&static_value, &static_value).unwrap();
        heap.manage(&value, false).unwrap();
        assert_eq!(
            heap.manage(&value, false).map_err(error),
            Err(VmErrorKind::InvalidStringOwnership)
        );
        assert_eq!(
            crate::runtime::FailureCode::from_vm(VmErrorKind::InvalidStringOwnership),
            None
        );
    }
}
