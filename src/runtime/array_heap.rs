//! 配列の領域と論理的な所有を管理します。要素のコピー・解放順は共通IRが決めます。
use std::{cell::RefCell, collections::HashMap, rc::Rc};
// 共通IR命令への接続に先立ち、実行時の予約契約を検証しています。
#[allow(dead_code)]
pub(crate) mod byte_outputs;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrayError {
    AllocationSizeOverflow,
    AllocationLimitExceeded,
    AllocationFailed,
    InvalidOwnership,
    IndexOutOfBounds,
}
type Result<T> = std::result::Result<T, ArrayError>;
#[derive(Debug)]
struct Storage<T> {
    values: Vec<T>,
    length: usize,
    owners: u64,
    bytes: u64,
    alive: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct ArrayValue<T> {
    id: Option<u64>,
    storage: Rc<RefCell<Storage<T>>>,
}
impl<T: Clone> ArrayValue<T> {
    pub(crate) fn len(&self) -> usize {
        self.storage.borrow().length
    }
    /// 値の変換用に、初期化済み領域だけを追加コピーせず読み取ります。
    pub(crate) fn borrow_values(&self) -> Result<std::cell::Ref<'_, [T]>> {
        let storage = self.storage.borrow();
        if !storage.alive || storage.owners == 0 || storage.values.len() != storage.length {
            return Err(ArrayError::InvalidOwnership);
        }
        Ok(std::cell::Ref::map(storage, |s| s.values.as_slice()))
    }
    pub(crate) fn get(&self, index: usize) -> Result<T> {
        let s = self.storage.borrow();
        if !s.alive {
            return Err(ArrayError::InvalidOwnership);
        }
        if index >= s.length {
            return Err(ArrayError::IndexOutOfBounds);
        }
        s.values
            .get(index)
            .cloned()
            .ok_or(ArrayError::InvalidOwnership)
    }
    pub(crate) fn set(&self, index: usize, value: T) -> Result<()> {
        let mut s = self.storage.borrow_mut();
        if !s.alive {
            return Err(ArrayError::InvalidOwnership);
        }
        if index >= s.length {
            return Err(ArrayError::IndexOutOfBounds);
        }
        let slot = s
            .values
            .get_mut(index)
            .ok_or(ArrayError::InvalidOwnership)?;
        *slot = value;
        Ok(())
    }
}
pub(crate) struct ArrayHeap<T> {
    limit: u64,
    live: u64,
    next: u64,
    allocations: HashMap<u64, ArrayValue<T>>,
    #[cfg(test)]
    fail_next: bool,
}
impl<T: Clone> ArrayHeap<T> {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            limit,
            live: 0,
            next: 0,
            allocations: HashMap::new(),
            #[cfg(test)]
            fail_next: false,
        }
    }
    pub(crate) fn allocate(&mut self, length: u64, width: u64) -> Result<ArrayValue<T>> {
        let bytes = length
            .checked_mul(width)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(ArrayError::AllocationSizeOverflow)?;
        let length = usize::try_from(length)
            .ok()
            .filter(|n| {
                *n <= i64::MAX as usize
                    && *n <= (isize::MAX as usize) / std::mem::size_of::<T>().max(1)
            })
            .ok_or(ArrayError::AllocationSizeOverflow)?;
        let live = self
            .live
            .checked_add(bytes)
            .filter(|n| *n <= self.limit)
            .ok_or(ArrayError::AllocationLimitExceeded)?;
        if length == 0 {
            return Ok(ArrayValue {
                id: None,
                storage: Rc::new(RefCell::new(Storage {
                    values: Vec::new(),
                    length: 0,
                    owners: 1,
                    bytes: 0,
                    alive: true,
                })),
            });
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next) {
            return Err(ArrayError::AllocationFailed);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(length)
            .map_err(|_| ArrayError::AllocationFailed)?;
        self.allocations
            .try_reserve(1)
            .map_err(|_| ArrayError::AllocationFailed)?;
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(ArrayError::AllocationFailed)?;
        let result = ArrayValue {
            id: Some(id),
            storage: Rc::new(RefCell::new(Storage {
                values,
                length,
                owners: 1,
                bytes,
                alive: true,
            })),
        };
        self.allocations.insert(id, result.clone());
        self.live = live;
        Ok(result)
    }
    fn validate(&self, value: &ArrayValue<T>) -> Result<()> {
        if let Some(id) = value.id
            && !self
                .allocations
                .get(&id)
                .is_some_and(|v| Rc::ptr_eq(&v.storage, &value.storage))
        {
            return Err(ArrayError::InvalidOwnership);
        }
        Ok(())
    }
    pub(crate) fn initialize(&mut self, array: &ArrayValue<T>, value: T) -> Result<()> {
        self.validate(array)?;
        let mut s = array.storage.borrow_mut();
        if !s.alive || s.owners == 0 || s.values.len() >= s.length {
            return Err(ArrayError::InvalidOwnership);
        }
        s.values.push(value);
        Ok(())
    }
    pub(crate) fn retain(&mut self, array: &ArrayValue<T>) -> Result<()> {
        self.validate(array)?;
        if array.id.is_none() {
            return Ok(());
        }
        let mut s = array.storage.borrow_mut();
        if s.owners == 0 {
            return Err(ArrayError::InvalidOwnership);
        }
        s.owners = s
            .owners
            .checked_add(1)
            .ok_or(ArrayError::InvalidOwnership)?;
        Ok(())
    }
    pub(crate) fn release_owner(&mut self, array: &ArrayValue<T>) -> Result<bool> {
        self.validate(array)?;
        if array.id.is_none() {
            return Ok(true);
        }
        let mut s = array.storage.borrow_mut();
        if s.owners == 0 || s.values.len() != s.length {
            return Err(ArrayError::InvalidOwnership);
        }
        s.owners -= 1;
        Ok(s.owners == 0)
    }
    pub(crate) fn free(&mut self, array: &ArrayValue<T>) -> Result<()> {
        self.validate(array)?;
        let Some(id) = array.id else {
            return Ok(());
        };
        let mut s = array.storage.borrow_mut();
        if s.owners != 0 {
            return Err(ArrayError::InvalidOwnership);
        }
        self.live -= s.bytes;
        s.alive = false;
        s.values.clear();
        self.allocations.remove(&id);
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn fail_next_allocation(&mut self) {
        self.fail_next = true;
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.live == 0 && self.allocations.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn assert_empty(&self) {
        assert_eq!(self.live, 0);
        assert!(self.allocations.is_empty());
    }
}
impl<T> Drop for ArrayHeap<T> {
    fn drop(&mut self) {
        // 失敗途中の未初期化領域も、ホストへ戻る前に回収します。
        for a in self.allocations.values() {
            let mut s = a.storage.borrow_mut();
            s.alive = false;
            s.values.clear();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_tracks_owners_and_reuses_freed_storage() {
        let mut heap = ArrayHeap::new(16);
        let a = heap.allocate(2, 8).unwrap();
        heap.initialize(&a, 10u64).unwrap();
        heap.initialize(&a, 20).unwrap();
        assert_eq!(
            heap.allocate(1, 8).unwrap_err(),
            ArrayError::AllocationLimitExceeded
        );
        heap.retain(&a).unwrap();
        assert!(!heap.release_owner(&a).unwrap());
        assert_eq!(a.get(1).unwrap(), 20);
        assert!(heap.release_owner(&a).unwrap());
        heap.free(&a).unwrap();
        assert_eq!(a.get(0), Err(ArrayError::InvalidOwnership));
        heap.assert_empty();
        let b = heap.allocate(1, 8).unwrap();
        heap.initialize(&b, 30).unwrap();
        assert_eq!(b.get(0).unwrap(), 30);
    }
    #[test]
    fn empty_overflow_allocation_failure_and_partial_cleanup_are_distinct() {
        let mut heap = ArrayHeap::<u64>::new(16);
        let empty = heap.allocate(0, 16).unwrap();
        assert_eq!(empty.get(0), Err(ArrayError::IndexOutOfBounds));
        assert_eq!(
            heap.allocate(i64::MAX as u64, 16).unwrap_err(),
            ArrayError::AllocationSizeOverflow
        );
        heap.fail_next = true;
        assert_eq!(
            heap.allocate(1, 8).unwrap_err(),
            ArrayError::AllocationFailed
        );
        let partial = heap.allocate(2, 8).unwrap();
        heap.initialize(&partial, 1).unwrap();
        drop(heap);
        assert_eq!(partial.get(0), Err(ArrayError::InvalidOwnership));
    }

    #[test]
    fn zero_width_and_partially_initialized_payloads_preserve_length_and_cleanup() {
        let mut heap = ArrayHeap::new(0);
        let a = heap.allocate(3, 0).unwrap();
        for _ in 0..3 {
            heap.initialize(&a, ()).unwrap();
        }
        assert_eq!(a.len(), 3);
        assert!(heap.release_owner(&a).unwrap());
        heap.free(&a).unwrap();
        heap.assert_empty();
        let payload = Rc::new(1);
        let weak = Rc::downgrade(&payload);
        let mut heap = ArrayHeap::new(16);
        let partial = heap.allocate(2, 8).unwrap();
        heap.initialize(&partial, payload).unwrap();
        drop(heap);
        assert!(weak.upgrade().is_none());
        assert_eq!(partial.get(0), Err(ArrayError::InvalidOwnership));
    }
}
