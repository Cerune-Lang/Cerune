//! 上限分を起動前に確保し、短いバイト結果を追加確保せず配列へ確定します。
//! 共通IRの子プロセス命令への接続は後続です。
use super::{ArrayError, ArrayHeap, ArrayValue, Result};

/// 未公開の2配列を所有します。確定せずに戻ると、部分的な初期化も取り消します。
pub(crate) struct ReservedByteOutputs<'a, T: Clone> {
    heap: &'a mut ArrayHeap<T>,
    arrays: Option<[ArrayValue<T>; 2]>,
}

impl<T: Clone> ArrayHeap<T> {
    pub(crate) fn reserve_byte_outputs(
        &mut self,
        stdout: u64,
        stderr: u64,
    ) -> Result<ReservedByteOutputs<'_, T>> {
        // u8も共通の配列格納表現では64ビットです。転送bufferの使用量とは別です。
        let first = self.allocate(stdout, 8)?;
        let second = match self.allocate(stderr, 8) {
            Ok(value) => value,
            Err(error) => {
                self.discard_unpublished(&first);
                return Err(error);
            }
        };
        Ok(ReservedByteOutputs {
            heap: self,
            arrays: Some([first, second]),
        })
    }

    fn discard_unpublished(&mut self, array: &ArrayValue<T>) {
        // この所有物はguardの外へ出ておらず、他の論理的な所有者はいません。
        array.storage.borrow_mut().owners = 0;
        self.free(array).expect("unpublished array reservation");
    }
}

impl<T: Clone> ReservedByteOutputs<'_, T> {
    /// 失敗返却用の空配列を保持したまま、検査後に上限分へ置き換えます。
    /// 2個目の失敗でも元の予約を残し、追加分だけを取り消します。
    pub(crate) fn reserve_limits(&mut self, stdout: u64, stderr: u64) -> Result<()> {
        let mut replacement = self.heap.reserve_byte_outputs(stdout, stderr)?;
        let arrays = replacement.arrays.take().unwrap();
        drop(replacement);
        for old in self.arrays.replace(arrays).unwrap() {
            self.heap.discard_unpublished(&old);
        }
        Ok(())
    }

    /// from_byteは既存実行器の即値構築に使い、追加の領域確保を行わない契約です。
    /// 配列の有効長だけを確定し、予約capacity分の使用量は解放まで保持します。
    pub(crate) fn finish(
        mut self,
        bytes: [&[u8]; 2],
        mut from_byte: impl FnMut(u8) -> T,
    ) -> Result<[ArrayValue<T>; 2]> {
        let arrays = self.arrays.as_ref().unwrap();
        if arrays.iter().zip(bytes).any(|(a, b)| b.len() > a.len()) {
            return Err(ArrayError::IndexOutOfBounds);
        }
        for (array, bytes) in arrays.iter().zip(bytes) {
            for &byte in bytes {
                self.heap.initialize(array, from_byte(byte))?;
            }
            let mut storage = array.storage.borrow_mut();
            storage.length = storage.values.len();
        }
        Ok(self.arrays.take().unwrap())
    }
}

impl<T: Clone> Drop for ReservedByteOutputs<'_, T> {
    fn drop(&mut self) {
        if let Some(arrays) = self.arrays.take() {
            for array in arrays {
                self.heap.discard_unpublished(&array);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free<T: Clone>(heap: &mut ArrayHeap<T>, array: &ArrayValue<T>) {
        assert!(heap.release_owner(array).unwrap());
        heap.free(array).unwrap();
    }

    #[test]
    fn shorter_results_keep_reserved_capacity_and_budget_without_growing() {
        let mut heap = ArrayHeap::<u64>::new(80);
        let reservation = heap.reserve_byte_outputs(8, 2).unwrap();
        let before = reservation.arrays.as_ref().unwrap().each_ref().map(|a| {
            let s = a.storage.borrow();
            (s.values.as_ptr(), s.values.capacity())
        });
        assert_eq!(reservation.heap.live, 80);
        reservation.heap.fail_next_allocation();
        let [stdout, stderr] = reservation
            .finish([&[255, 0, 13, 10], &[]], u64::from)
            .unwrap();
        for (array, before) in [&stdout, &stderr].into_iter().zip(before) {
            let s = array.storage.borrow();
            assert_eq!((s.values.as_ptr(), s.values.capacity()), before);
        }
        assert_eq!(stdout.len(), 4);
        assert_eq!(stderr.len(), 0);
        assert_eq!(stdout.get(0), Ok(255));
        assert_eq!(stdout.get(3), Ok(10));
        assert_eq!(stdout.get(4), Err(ArrayError::IndexOutOfBounds));
        assert_eq!(heap.live, 80);
        assert_eq!(
            heap.allocate(1, 8).unwrap_err(),
            ArrayError::AllocationLimitExceeded
        );
        free(&mut heap, &stdout);
        free(&mut heap, &stderr);
        heap.assert_empty();
        // 確定処理は新しいArrayHeap領域を確保していません。
        assert_eq!(
            heap.allocate(1, 8).unwrap_err(),
            ArrayError::AllocationFailed
        );
    }

    #[test]
    fn cancellation_and_second_reservation_failure_restore_the_original_heap() {
        let mut heap = ArrayHeap::<u64>::new(32);
        let existing = heap.allocate(1, 8).unwrap();
        heap.initialize(&existing, 42).unwrap();
        assert!(matches!(
            heap.reserve_byte_outputs(2, 2),
            Err(ArrayError::AllocationLimitExceeded)
        ));
        assert_eq!(heap.live, 8);
        assert_eq!(existing.get(0), Ok(42));
        drop(heap.reserve_byte_outputs(1, 2).unwrap());
        assert_eq!(heap.live, 8);
        free(&mut heap, &existing);
        heap.assert_empty();
        heap.fail_next_allocation();
        assert!(matches!(
            heap.reserve_byte_outputs(0, 1),
            Err(ArrayError::AllocationFailed)
        ));
        heap.assert_empty();
    }

    #[test]
    fn overflow_and_oversized_results_do_not_publish_or_leak_either_array() {
        let mut heap = ArrayHeap::<u64>::new(u64::MAX);
        assert!(matches!(
            heap.reserve_byte_outputs(1, u64::MAX),
            Err(ArrayError::AllocationSizeOverflow)
        ));
        heap.assert_empty();
        let reservation = heap.reserve_byte_outputs(2, 1).unwrap();
        assert!(matches!(
            reservation.finish([b"ok", b"no"], u64::from),
            Err(ArrayError::IndexOutOfBounds)
        ));
        heap.assert_empty();
    }

    #[test]
    fn zero_limits_and_exact_results_follow_existing_owner_lifecycle() {
        let mut heap = ArrayHeap::<u64>::new(16);
        let [out, err] = heap
            .reserve_byte_outputs(0, 2)
            .unwrap()
            .finish([b"", &[0, 255]], u64::from)
            .unwrap();
        assert_eq!(out.len(), 0);
        assert_eq!(err.len(), 2);
        heap.retain(&err).unwrap();
        assert!(!heap.release_owner(&err).unwrap());
        assert_eq!(err.get(1), Ok(255));
        free(&mut heap, &out);
        free(&mut heap, &err);
        heap.assert_empty();
    }

    #[test]
    fn both_existing_executor_value_representations_accept_reserved_bytes() {
        use crate::{
            runtime::{numeric::Number, value::Value},
            types::IntegerType,
        };
        let mut heap = ArrayHeap::<Value>::new(16);
        let [a, b] = heap
            .reserve_byte_outputs(1, 1)
            .unwrap()
            .finish([&[255], &[0]], |n| {
                Value::Number(Number::Integer(i128::from(n), IntegerType::U8))
            })
            .unwrap();
        assert!(matches!(
            a.get(0),
            Ok(Value::Number(Number::Integer(255, IntegerType::U8)))
        ));
        free(&mut heap, &a);
        free(&mut heap, &b);
        heap.assert_empty();
        let mut heap = ArrayHeap::<crate::vm::Value>::new(16);
        let [a, b] = heap
            .reserve_byte_outputs(1, 1)
            .unwrap()
            .finish([&[255], &[0]], |n| {
                crate::vm::Value::Integer(i128::from(n), IntegerType::U8)
            })
            .unwrap();
        assert!(matches!(
            a.get(0),
            Ok(crate::vm::Value::Integer(255, IntegerType::U8))
        ));
        free(&mut heap, &a);
        free(&mut heap, &b);
        heap.assert_empty();
    }

    #[test]
    fn unwind_during_conversion_releases_partially_initialized_storage() {
        let mut heap = ArrayHeap::<u64>::new(32);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            heap.reserve_byte_outputs(2, 2)
                .unwrap()
                .finish([b"ab", b"cd"], |b| {
                    assert_ne!(b, b'd', "test conversion panic");
                    u64::from(b)
                })
        }));
        assert!(result.is_err());
        heap.assert_empty();
    }
}

#[cfg(test)]
mod replacement_tests {
    use super::*;
    #[test]
    fn replacement_failure_keeps_previous_reservation_and_partial_storage_unreadable() {
        let mut heap = ArrayHeap::<u64>::new(32);
        let mut reservation = heap.reserve_byte_outputs(0, 0).unwrap();
        assert!(matches!(
            reservation.reserve_limits(2, 3),
            Err(ArrayError::AllocationLimitExceeded)
        ));
        let [a, b] = reservation.finish([&[], &[]], u64::from).unwrap();
        assert_eq!((a.len(), b.len()), (0, 0));
        heap.assert_empty();
        let array = heap.allocate(2, 8).unwrap();
        heap.initialize(&array, 255).unwrap();
        assert_eq!(
            array.borrow_values().unwrap_err(),
            ArrayError::InvalidOwnership
        );
        heap.initialize(&array, 0).unwrap();
        assert_eq!(&*array.borrow_values().unwrap(), &[255, 0]);
        assert!(heap.release_owner(&array).unwrap());
        assert_eq!(
            array.borrow_values().unwrap_err(),
            ArrayError::InvalidOwnership
        );
        heap.free(&array).unwrap();
        assert_eq!(
            array.borrow_values().unwrap_err(),
            ArrayError::InvalidOwnership
        );
        heap.assert_empty();
    }
}
