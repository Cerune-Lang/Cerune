use super::*;
/// 実行器への接続。文字列本文のコピーや観測callbackを挟みません。
pub(crate) trait Access {
    fn read_binary(
        &mut self,
        name: &str,
        limit: i64,
        origin: Origin,
        reserve: &mut dyn FnMut(usize) -> Result<(), Code>,
    ) -> Result<Vec<u8>, Failure>;
    fn write_binary(
        &mut self,
        name: &str,
        length: usize,
        limit: i64,
        origin: Origin,
        encode: &mut dyn FnMut() -> Result<Vec<u8>, Code>,
    ) -> Result<(), Failure>;
    fn read_value(
        &mut self,
        heap: &mut crate::runtime::string_heap::StringHeap,
        name: &str,
        limit: i64,
        origin: Origin,
    ) -> Result<crate::runtime::string_heap::StringValue, Failure>;
    fn write_value(
        &mut self,
        name: &str,
        text: &str,
        limit: i64,
        origin: Origin,
    ) -> Result<(), Failure>;
}
impl<B: Backend> Access for Files<B> {
    fn read_binary(
        &mut self,
        name: &str,
        limit: i64,
        origin: Origin,
        reserve: &mut dyn FnMut(usize) -> Result<(), Code>,
    ) -> Result<Vec<u8>, Failure> {
        if let Some(e) = self.failure {
            return Err(e);
        }
        let mut details = Details::new(Operation::ReadBytes, limit, None, origin);
        let result = self
            .read_contents(name, u64::MAX, &mut details, reserve)
            .map(|(bytes, ())| bytes);
        self.complete(details, result)
    }
    fn write_binary(
        &mut self,
        name: &str,
        length: usize,
        limit: i64,
        origin: Origin,
        encode: &mut dyn FnMut() -> Result<Vec<u8>, Code>,
    ) -> Result<(), Failure> {
        let details = Details::new(Operation::WriteBytesNew, limit, Some(length as u64), origin);
        self.write_prepared(name, details, length, encode)
    }
    fn read_value(
        &mut self,
        heap: &mut crate::runtime::string_heap::StringHeap,
        name: &str,
        limit: i64,
        origin: Origin,
    ) -> Result<crate::runtime::string_heap::StringValue, Failure> {
        let available = heap.available();
        self.read_text_prepared(name, limit, available, origin, |size| {
            heap.reserve(size).map_err(|e| match e {
                crate::runtime::string_heap::HeapError::AllocationLimitExceeded => {
                    Code::BudgetExceeded
                }
                _ => Code::AllocationFailed,
            })
        })
        .map(|(text, reservation)| reservation.finish(text))
    }
    fn write_value(
        &mut self,
        name: &str,
        text: &str,
        limit: i64,
        origin: Origin,
    ) -> Result<(), Failure> {
        self.write_new(name, text, limit, origin)
    }
}
pub(crate) struct Unconfigured;
impl Backend for Unconfigured {
    type Handle = Self;
    fn open(&mut self, _: &str, _: Operation) -> Result<Self::Handle, OpenError> {
        unreachable!("unconfigured files are rejected before open")
    }
}
impl Read for Unconfigured {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        unreachable!("unconfigured handle")
    }
}
impl Write for Unconfigured {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        unreachable!("unconfigured handle")
    }
    fn flush(&mut self) -> io::Result<()> {
        unreachable!("unconfigured handle")
    }
}
impl Handle for Unconfigured {
    fn close(self) -> io::Result<()> {
        unreachable!("unconfigured handle")
    }
}
pub(crate) fn unconfigured() -> Files<Unconfigured> {
    Files::unconfigured()
}
