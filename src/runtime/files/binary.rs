use super::{Access, Code, Failure, Origin};
use crate::runtime::{
    array_heap::io_values::{ArrayIoValue, Element},
    array_heap::{ArrayError, ArrayHeap, ArrayValue},
};
use crate::types::IntegerType;

pub(crate) fn read_array<V: ArrayIoValue>(
    files: &mut dyn Access,
    heap: &mut ArrayHeap<V>,
    name: &str,
    limit: i64,
    origin: Origin,
) -> Result<V, Failure> {
    let mut reserved = None;
    let result = files.read_binary(name, limit, origin, &mut |capacity| {
        reserved = Some(heap.allocate(capacity as u64, 8).map_err(|e| match e {
            ArrayError::AllocationLimitExceeded => Code::BudgetExceeded,
            _ => Code::AllocationFailed,
        })?);
        Ok(())
    });
    match result {
        Ok(bytes) => {
            let array = reserved.expect("read reserved the result before opening the file");
            heap.finish_byte_array(&array, &bytes, V::byte);
            Ok(V::byte_array(array))
        }
        Err(e) => {
            if let Some(array) = reserved {
                heap.discard_unpublished(&array);
            }
            Err(e)
        }
    }
}

pub(crate) fn byte_storage<V: ArrayIoValue>(value: &V) -> Option<&ArrayValue<V>> {
    value.array(Element::Byte)
}

pub(crate) fn write_array<V: ArrayIoValue>(
    files: &mut dyn Access,
    name: &str,
    array: &ArrayValue<V>,
    limit: i64,
    origin: Origin,
) -> Result<(), Failure> {
    files.write_binary(name, array.len(), limit, origin, &mut || {
        let values = array.borrow_values().map_err(|_| Code::InvalidResponse)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(values.len())
            .map_err(|_| Code::AllocationFailed)?;
        for v in values.iter() {
            bytes.push(v.unsigned(IntegerType::U8).ok_or(Code::InvalidResponse)? as u8);
        }
        Ok(bytes)
    })
}
