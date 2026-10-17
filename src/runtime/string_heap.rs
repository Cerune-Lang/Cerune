//! 論理的な所有数はRustの一時コピーと分離します。最後のreleaseで内容を回収します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeapError {
    AllocationSizeOverflow,
    AllocationLimitExceeded,
    AllocationFailed,
    InvalidStringOwnership,
}
type HeapResult<T> = Result<T, HeapError>;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
#[derive(Debug, Clone)]
pub(crate) struct StringValue {
    bytes: Rc<RefCell<String>>,
    allocation: Option<u64>,
}
impl From<String> for StringValue {
    fn from(value: String) -> Self {
        Self {
            bytes: Rc::new(RefCell::new(value)),
            allocation: None,
        }
    }
}
impl PartialEq for StringValue {
    fn eq(&self, other: &Self) -> bool {
        *self.bytes.borrow() == *other.bytes.borrow()
    }
}
impl Eq for StringValue {}
impl StringValue {
    pub(crate) fn len(&self) -> usize {
        self.bytes.borrow().len()
    }
    pub(crate) fn text(&self) -> String {
        self.bytes.borrow().clone()
    }
}
pub(crate) struct StringHeap {
    limit: u64,
    live: u64,
    next: u64,
    #[cfg(test)]
    fail_next_allocation: bool,
    allocations: HashMap<u64, (Rc<RefCell<String>>, u64)>,
}
impl StringHeap {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            limit,
            live: 0,
            next: 0,
            allocations: HashMap::new(),
            #[cfg(test)]
            fail_next_allocation: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn fail_next_allocation(&mut self) {
        self.fail_next_allocation = true;
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.live == 0 && self.allocations.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn assert_empty(&self) {
        assert_eq!(self.live, 0);
        assert!(self.allocations.is_empty());
    }
    pub(crate) fn concat(
        &mut self,
        left: &StringValue,
        right: &StringValue,
    ) -> HeapResult<StringValue> {
        let left = left.bytes.borrow();
        let right = right.bytes.borrow();
        let size = allocation_size(left.len(), right.len())?;
        if size == 0 {
            return Ok(String::new().into());
        }
        let reservation = self.reserve(size)?;
        let mut bytes = String::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| HeapError::AllocationFailed)?;
        bytes.push_str(&left);
        bytes.push_str(&right);
        Ok(reservation.finish(bytes))
    }
    pub(crate) fn available(&self) -> u64 {
        self.limit - self.live
    }
    /// 本文を読む前に、予算・所有者・登録先をすべて準備します。
    pub(crate) fn reserve(&mut self, size: usize) -> HeapResult<Reservation<'_>> {
        let size = allocation_size(size, 0)? as u64;
        if size > self.available() {
            return Err(HeapError::AllocationLimitExceeded);
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_allocation) {
            return Err(HeapError::AllocationFailed);
        }
        let next = self
            .next
            .checked_add(1)
            .ok_or(HeapError::AllocationFailed)?;
        self.allocations
            .try_reserve(1)
            .map_err(|_| HeapError::AllocationFailed)?;
        let bytes = Rc::new(RefCell::new(String::new()));
        let id = self.next;
        self.next = next;
        self.allocations.insert(id, (bytes.clone(), 1));
        self.live += size;
        Ok(Reservation {
            heap: self,
            id,
            size,
            bytes,
            committed: false,
        })
    }
    pub(crate) fn manage(&mut self, value: &StringValue, retain: bool) -> HeapResult<()> {
        let Some(id) = value.allocation else {
            return Ok(());
        };
        let (bytes, count) = self
            .allocations
            .get_mut(&id)
            .ok_or(HeapError::InvalidStringOwnership)?;
        // IDはヒープごとに採番するため、所属する実体も検証します。
        // 不正な所有数は、retainでも復活させず変更前に拒否します。
        if !Rc::ptr_eq(bytes, &value.bytes) || *count == 0 {
            return Err(HeapError::InvalidStringOwnership);
        }
        if retain {
            *count = count
                .checked_add(1)
                .ok_or(HeapError::InvalidStringOwnership)?;
            return Ok(());
        }
        *count -= 1;
        if *count == 0 {
            let (bytes, _) = self.allocations.remove(&id).unwrap();
            self.live -= bytes.borrow().len() as u64;
            *bytes.borrow_mut() = String::new();
        }
        Ok(())
    }
}
/// 可変借用で予約から登録までを排他的にし、途中失敗では必ず予約を戻します。
pub(crate) struct Reservation<'h> {
    heap: &'h mut StringHeap,
    id: u64,
    size: u64,
    bytes: Rc<RefCell<String>>,
    committed: bool,
}
impl Reservation<'_> {
    /// 予約済みの所有者へ領域を移します。追加の本文確保や登録用確保を行いません。
    pub(crate) fn finish(mut self, text: String) -> StringValue {
        assert!(
            text.len() as u64 <= self.size,
            "completed string exceeds its reservation"
        );
        self.heap.live -= self.size - text.len() as u64;
        *self.bytes.borrow_mut() = text;
        self.committed = true;
        StringValue {
            bytes: self.bytes.clone(),
            allocation: Some(self.id),
        }
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.heap.allocations.remove(&self.id);
            self.heap.live -= self.size;
        }
    }
}
impl Drop for StringHeap {
    fn drop(&mut self) {
        // 失敗した実行も、外部へ戻る前に内容を回収します。
        for (bytes, _) in self.allocations.values() {
            *bytes.borrow_mut() = String::new();
        }
    }
}
fn allocation_size(left: usize, right: usize) -> HeapResult<usize> {
    left.checked_add(right)
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(HeapError::AllocationSizeOverflow)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_sizes_do_not_wrap() {
        assert_eq!(
            allocation_size(usize::MAX, 1),
            Err(HeapError::AllocationSizeOverflow)
        );
        assert_eq!(
            allocation_size(isize::MAX as usize, 1),
            Err(HeapError::AllocationSizeOverflow)
        );
        assert_eq!(allocation_size(0, 0), Ok(0));
    }
    #[test]
    fn sharing_release_and_failed_allocation_keep_accounting() {
        let mut heap = StringHeap::new(4);
        let a = "a".to_owned().into();
        let b = "b".to_owned().into();
        let dynamic = heap.concat(&a, &b).unwrap();
        heap.manage(&dynamic, true).unwrap();
        assert_eq!(heap.live, 2);
        heap.manage(&dynamic, false).unwrap();
        assert_eq!(dynamic.text(), "ab");
        heap.fail_next_allocation = true;
        assert_eq!(
            heap.concat(&a, &b).unwrap_err(),
            HeapError::AllocationFailed
        );
        assert_eq!(heap.live, 2);
        heap.manage(&dynamic, false).unwrap();
        heap.assert_empty();
        // スロットにRustの一時コピーが残っていても内容は回収済みです。
        assert_eq!(dynamic.text(), "");
    }
    #[test]
    fn failing_execution_boundary_reclaims_surviving_handles() {
        let dynamic = {
            let mut heap = StringHeap::new(2);
            let a = "a".to_owned().into();
            let dynamic = heap.concat(&a, &a).unwrap();
            assert_eq!(
                heap.concat(&a, &a).unwrap_err(),
                HeapError::AllocationLimitExceeded
            );
            dynamic
        };
        assert_eq!(dynamic.text(), "");
    }

    #[test]
    fn foreign_handles_with_colliding_ids_cannot_change_either_heap() {
        for retain in [true, false] {
            let mut left = StringHeap::new(16);
            let mut right = StringHeap::new(16);
            let a = left
                .concat(&"a".to_owned().into(), &"b".to_owned().into())
                .unwrap();
            let b = right
                .concat(&"日本".to_owned().into(), &"".to_owned().into())
                .unwrap();
            assert_eq!(a.allocation, b.allocation);
            assert_eq!(
                left.manage(&b, retain),
                Err(HeapError::InvalidStringOwnership)
            );
            assert_eq!(
                right.manage(&a, retain),
                Err(HeapError::InvalidStringOwnership)
            );
            assert_eq!((left.live, right.live), (2, 6));
            assert_eq!((a.text(), b.text()), ("ab".to_owned(), "日本".to_owned()));
            // 失敗した操作では、どちらの領域も保持・解放しません。
            left.manage(&a, false).unwrap();
            right.manage(&b, false).unwrap();
            left.assert_empty();
            right.assert_empty();
        }
    }

    #[test]
    fn invalid_owner_counts_fail_without_changing_allocation() {
        for (count, retain) in [(0, false), (0, true), (u64::MAX, true)] {
            let mut heap = StringHeap::new(8);
            let value = heap
                .concat(&"a".to_owned().into(), &"b".to_owned().into())
                .unwrap();
            let id = value.allocation.unwrap();
            // 内部状態の破損を模擬します。ソースから行う操作ではありません。
            heap.allocations.get_mut(&id).unwrap().1 = count;
            assert_eq!(
                heap.manage(&value, retain),
                Err(HeapError::InvalidStringOwnership)
            );
            assert_eq!(heap.allocations[&id].1, count);
            assert_eq!(heap.live, 2);
            assert_eq!(value.text(), "ab");
            heap.allocations.get_mut(&id).unwrap().1 = 1;
            heap.manage(&value, false).unwrap();
            heap.assert_empty();
        }
    }

    #[test]
    fn released_and_missing_handles_do_not_touch_live_values() {
        let mut heap = StringHeap::new(8);
        let literal: StringValue = "a".to_owned().into();
        let old = heap.concat(&literal, &literal).unwrap();
        heap.manage(&old, false).unwrap();
        let live = heap.concat(&literal, &literal).unwrap();
        let missing = StringValue {
            bytes: live.bytes.clone(),
            allocation: Some(u64::MAX),
        };
        for invalid in [&old, &missing] {
            for retain in [true, false] {
                assert_eq!(
                    heap.manage(invalid, retain),
                    Err(HeapError::InvalidStringOwnership)
                );
                assert_eq!(heap.live, 2);
                assert_eq!(live.text(), "aa");
            }
        }
        // 静的・空文字列は動的な所有数を持ちません。
        let empty = heap
            .concat(&"".to_owned().into(), &"".to_owned().into())
            .unwrap();
        for value in [&literal, &empty] {
            heap.manage(value, true).unwrap();
            heap.manage(value, false).unwrap();
        }
        heap.manage(&live, false).unwrap();
        heap.assert_empty();
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::{
        ir::NodeId,
        runtime::input::{Code, Input, Origin},
        source::Span,
    };
    use std::io::{self, Read};
    fn origin() -> Origin {
        Origin {
            node_id: NodeId(4),
            span: Span::new(8, 20),
        }
    }
    fn code(error: HeapError) -> Code {
        match error {
            HeapError::AllocationLimitExceeded => Code::BudgetExceeded,
            _ => Code::AllocationFailed,
        }
    }
    fn read<R: Read>(
        input: &mut Input<R>,
        heap: &mut StringHeap,
        limit: i64,
    ) -> Result<Option<StringValue>, crate::runtime::input::Failure> {
        let available = heap.available();
        input
            .read_line_prepared(limit, available, origin(), |size| {
                heap.reserve(size).map_err(code)
            })
            .map(|line| line.map(|(text, reservation)| reservation.finish(text)))
    }
    #[test]
    fn input_adopts_storage_and_charges_only_live_bytes_until_last_release() {
        let mut heap = StringHeap::new(20);
        let mut input = Input::new(&b"abc\nlast"[..]);
        let available = heap.available();
        let (text, reservation) = input
            .read_line_prepared(10, available, origin(), |size| {
                heap.reserve(size).map_err(code)
            })
            .unwrap()
            .unwrap();
        let pointer = text.as_ptr();
        let first = reservation.finish(text);
        assert_eq!(first.bytes.borrow().as_ptr(), pointer);
        assert_eq!(heap.live, 4);
        let second = read(&mut input, &mut heap, 10).unwrap().unwrap();
        assert_eq!(second.text(), "last");
        assert_eq!(heap.live, 8);
        assert!(read(&mut input, &mut heap, 10).unwrap().is_none());
        heap.manage(&first, true).unwrap();
        heap.manage(&first, false).unwrap();
        assert_eq!(first.text(), "abc\n");
        heap.manage(&first, false).unwrap();
        assert_eq!(heap.live, 4);
        heap.manage(&second, false).unwrap();
        heap.assert_empty();
    }
    #[test]
    fn failed_or_empty_input_rolls_back_reservation_without_releasing_existing_values() {
        for bytes in [&b""[..], &b"\xff\n"[..], &b"longer than limit"[..]] {
            let mut heap = StringHeap::new(10);
            let a = "a".to_owned().into();
            let prior = heap.concat(&a, &a).unwrap();
            let mut input = Input::new(bytes);
            let result = read(&mut input, &mut heap, 4);
            assert!(result.is_err() || result.unwrap().is_none());
            assert_eq!(heap.live, 2);
            assert_eq!(heap.allocations.len(), 1);
            assert_eq!(prior.text(), "aa");
            heap.manage(&prior, false).unwrap();
            heap.assert_empty();
        }
    }
    struct NeverRead;
    impl Read for NeverRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("must not read")
        }
    }
    #[test]
    fn budget_and_metadata_failure_stop_before_consumption() {
        let mut heap = StringHeap::new(4);
        let a = "a".to_owned().into();
        let prior = heap.concat(&a, &a).unwrap();
        for allocation in [false, true] {
            let mut input = Input::with_trace_limit(NeverRead, 2);
            heap.fail_next_allocation = allocation;
            let failure = read(&mut input, &mut heap, if allocation { 2 } else { 3 }).unwrap_err();
            assert_eq!(
                failure.code,
                if allocation {
                    Code::AllocationFailed
                } else {
                    Code::BudgetExceeded
                }
            );
            assert_eq!(failure.consumed, 0);
            assert_eq!(input.events()[0].read_calls, 0);
            assert_eq!(heap.live, 2);
            assert_eq!(input.failure(), Some(failure));
        }
        heap.manage(&prior, false).unwrap();
        heap.assert_empty();
    }
    #[test]
    fn reservation_drop_restores_budget_and_identity_is_not_reused() {
        let mut heap = StringHeap::new(4);
        let first_id;
        {
            let reservation = heap.reserve(4).unwrap();
            first_id = reservation.id;
            assert_eq!(reservation.heap.live, 4);
        }
        heap.assert_empty();
        let value = heap.reserve(4).unwrap().finish("x".into());
        assert_ne!(value.allocation, Some(first_id));
        assert_eq!(heap.live, 1);
        heap.manage(&value, false).unwrap();
        heap.assert_empty();
    }

    #[test]
    fn retained_input_capacity_is_distinct_from_memory_limit() {
        const COUNT: usize = 8;
        for limit in [2, 1_048_576] {
            let mut heap = StringHeap::new(limit as u64 + (COUNT * 2) as u64);
            let data = b"x\n".repeat(COUNT * 3);
            let mut input = Input::with_trace_limit(data.as_slice(), COUNT * 3);
            for cycle in 0..3 {
                let mut values = Vec::new();
                for _ in 0..COUNT {
                    values.push(read(&mut input, &mut heap, limit).unwrap().unwrap());
                }
                let capacity: usize = values.iter().map(|v| v.bytes.borrow().capacity()).sum();
                assert_eq!(heap.live, (COUNT * 2) as u64);
                assert_eq!(heap.allocations.len(), COUNT);
                assert_eq!(capacity, COUNT * 2);
                println!(
                    "Rust limit={limit} cycle={cycle} logical={} owners={} capacity={capacity}",
                    heap.live,
                    heap.allocations.len()
                );
                // Rust側の一時ハンドルが残っていても最後の言語releaseで領域を返します。
                for value in &values {
                    assert_eq!(value.text(), "x\n");
                    heap.manage(value, false).unwrap();
                    assert_eq!(value.bytes.borrow().capacity(), 0);
                }
                heap.assert_empty();
            }
            assert!(
                input
                    .events()
                    .iter()
                    .all(|event| event.consumed == 2 && event.read_calls == 2)
            );
        }
    }
}
