//! Probe transport only: subprocess semantics remain in the existing runtime ABI.
use cerune_lang::runtime::process::abi::{self, Bytes, Context, RequestV1};
use std::ptr;
use wasmi::{Caller, Error, Linker};
const SHAPE: i32 = 11;
const STATE: i32 = 12;
const RANGE: i32 = 13;
const ALLOCATION: i32 = 15;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Receiving,
    Pending,
    Ready,
}
struct Receiving {
    metadata: [u64; 8],
    fields: Vec<Vec<u8>>,
    arguments: usize,
    length: usize,
}
impl Receiving {
    fn field_complete(&self) -> bool {
        self.fields.last().is_none_or(|v| v.len() == self.length)
    }
    fn complete(&self) -> bool {
        self.fields.len() == self.arguments + 2 && self.field_complete()
    }
}
pub struct Process {
    context: *mut Context,
    phase: Phase,
    receiving: Option<Receiving>,
}
fn bytes(value: &[u8]) -> Bytes {
    Bytes {
        data: value.as_ptr(),
        len: value.len() as u64,
    }
}
fn length(value: i64) -> Result<usize, i32> {
    u32::try_from(value as u64)
        .map(|n| n as usize)
        .map_err(|_| RANGE)
}
impl Process {
    /// target=1 Windows MSVC / 2 Linux GNU; caller explicitly selects it.
    pub fn new(target: u64, config: &[u8]) -> Result<Self, i32> {
        let mut context = ptr::null_mut();
        // SAFETY: input bytes live for this call; output points to an exclusive local.
        let status =
            unsafe { abi::cerune_process_v1_create(1, target, bytes(config), &mut context) };
        if status != 0 {
            return Err(status as i32);
        }
        assert!(!context.is_null());
        Ok(Self {
            context,
            phase: Phase::Idle,
            receiving: None,
        })
    }
    pub fn begin(&mut self, metadata: [i64; 8]) -> i32 {
        if self.phase != Phase::Idle {
            return STATE;
        }
        let arguments = match length(metadata[0]) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let metadata = metadata.map(|n| n as u64);
        if metadata[6] > metadata[7] {
            return RANGE;
        }
        let Some(count) = arguments.checked_add(2) else {
            return RANGE;
        };
        let mut fields = Vec::new();
        if fields.try_reserve_exact(count).is_err() {
            return ALLOCATION;
        }
        self.receiving = Some(Receiving {
            metadata,
            fields,
            arguments,
            length: 0,
        });
        self.phase = Phase::Receiving;
        0
    }
    pub fn field(&mut self, kind: i32, index: i64, size: i64) -> i32 {
        let Some(r) = &mut self.receiving else {
            return STATE;
        };
        if !r.field_complete() {
            return STATE;
        }
        let next = r.fields.len();
        let expected = if next == 0 {
            (0, 0)
        } else if next <= r.arguments {
            (1, (next - 1) as u64)
        } else if next == r.arguments + 1 {
            (2, 0)
        } else {
            return STATE;
        };
        if (kind, index as u64) != expected {
            return STATE;
        }
        let n = match length(size) {
            Ok(n) => n,
            Err(e) => return e,
        };
        let mut field = Vec::new();
        if field.try_reserve_exact(n).is_err() {
            return ALLOCATION;
        }
        r.fields.push(field);
        r.length = n;
        0
    }
    pub fn chunk(&mut self, bits: i64, count: i32) -> i32 {
        if !(1..=8).contains(&count) {
            return RANGE;
        }
        let Some(r) = &mut self.receiving else {
            return STATE;
        };
        let Some(field) = r.fields.last_mut() else {
            return STATE;
        };
        let n = count as usize;
        if n > r.length - field.len() {
            return SHAPE;
        }
        field.extend_from_slice(&bits.to_le_bytes()[..n]);
        0
    }
    pub fn prepare(&mut self) -> i32 {
        let Some(r) = &self.receiving else {
            return STATE;
        };
        if !r.complete() {
            return STATE;
        }
        let mut arguments = Vec::new();
        if arguments.try_reserve_exact(r.arguments).is_err() {
            return ALLOCATION;
        }
        arguments.extend(r.fields[1..=r.arguments].iter().map(|v| bytes(v)));
        let m = r.metadata;
        let request = RequestV1 {
            version: 1,
            size: size_of::<RequestV1>() as u64,
            target: bytes(&r.fields[0]),
            arguments: arguments.as_ptr(),
            argument_count: r.arguments as u64,
            stdin: bytes(&r.fields[r.arguments + 1]),
            stdout_limit: m[1],
            stderr_limit: m[2],
            timeout_ms: m[3],
            node: m[4],
            source: m[5],
            start: m[6],
            end: m[7],
        };
        // SAFETY: exclusively owned context; descriptors and all input bytes live through
        // prepare, which copies them. No pointer to Wasm memory is involved.
        let status = unsafe { abi::cerune_process_v1_prepare(self.context, &request) } as i32;
        match status {
            1 => self.phase = Phase::Pending,
            2 => self.phase = Phase::Ready,
            _ => return status,
        }
        self.receiving = None;
        status
    }
    pub fn execute(&mut self, reservation: i32) -> i32 {
        if self.phase != Phase::Pending {
            return STATE;
        }
        if !(0..=2).contains(&reservation) {
            return RANGE;
        }
        // SAFETY: context is live and exclusively used; no ABI input/output pointers.
        let status =
            unsafe { abi::cerune_process_v1_execute(self.context, reservation as u64) } as i32;
        if status == 2 {
            self.phase = Phase::Ready;
        }
        status
    }
    pub fn report(&self, index: i32) -> (i64, i32) {
        if !matches!(self.phase, Phase::Pending | Phase::Ready) {
            return (0, STATE);
        }
        if !(0..32).contains(&index) {
            return (0, RANGE);
        }
        let mut report = [0u64; 32];
        // SAFETY: live, instance-local context, used serially. ABI report only reads
        // the outcome; its destination is a separate initialized 32-word array.
        let status =
            unsafe { abi::cerune_process_v1_report(self.context, report.as_mut_ptr(), 32) } as i32;
        if status == 0 {
            (report[index as usize] as i64, 0)
        } else {
            (0, status)
        }
    }
    pub fn copy(&self, channel: i32, offset: i64, requested: i32) -> (i64, i32, i32) {
        if self.phase != Phase::Ready {
            return (0, 0, STATE);
        }
        if !(1..=2).contains(&channel) || !(1..=8).contains(&requested) {
            return (0, 0, RANGE);
        }
        let mut data = [0u8; 8];
        let mut count = 0u64;
        // SAFETY: the copy operation only reads the ready outcome. The destination
        // has eight bytes, capacity is 1..=8, and count is a separate live local.
        let status = unsafe {
            abi::cerune_process_v1_copy(
                self.context,
                channel as u64,
                offset as u64,
                data.as_mut_ptr(),
                requested as u64,
                &mut count,
            )
        } as i32;
        if status != 0 {
            return (0, 0, status);
        }
        if count > requested as u64 {
            return (0, 0, SHAPE);
        }
        (i64::from_le_bytes(data), count as i32, 0)
    }
    pub fn release(&mut self) -> i32 {
        match self.phase {
            Phase::Idle => return STATE,
            Phase::Receiving => self.receiving = None,
            _ => {
                // SAFETY: live, exclusively used context; release retains Backend
                // ownership of unreclaimed resources and its disabled state.
                let status = unsafe { abi::cerune_process_v1_release(self.context) } as i32;
                if status != 0 {
                    return status;
                }
            }
        }
        self.phase = Phase::Idle;
        0
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: this owner alone destroys its live context once.
        let _ = unsafe { abi::cerune_process_v1_destroy(&mut self.context) };
    }
}
pub fn imports(linker: &mut Linker<Process>) -> Result<(), Error> {
    linker.func_wrap(
        "cerune",
        "process_begin_v1",
        |mut c: Caller<'_, Process>,
         arguments: i64,
         stdout: i64,
         stderr: i64,
         timeout: i64,
         node: i64,
         source: i64,
         start: i64,
         end: i64| {
            c.data_mut()
                .begin([arguments, stdout, stderr, timeout, node, source, start, end])
        },
    )?;
    linker.func_wrap(
        "cerune",
        "process_field_v1",
        |mut c: Caller<'_, Process>, k: i32, i: i64, n: i64| c.data_mut().field(k, i, n),
    )?;
    linker.func_wrap(
        "cerune",
        "process_chunk_v1",
        |mut c: Caller<'_, Process>, b: i64, n: i32| c.data_mut().chunk(b, n),
    )?;
    linker.func_wrap(
        "cerune",
        "process_prepare_v1",
        |mut c: Caller<'_, Process>| c.data_mut().prepare(),
    )?;
    linker.func_wrap(
        "cerune",
        "process_execute_v1",
        |mut c: Caller<'_, Process>, r: i32| c.data_mut().execute(r),
    )?;
    linker.func_wrap(
        "cerune",
        "process_report_v1",
        |c: Caller<'_, Process>, i: i32| c.data().report(i),
    )?;
    linker.func_wrap(
        "cerune",
        "process_copy_v1",
        |c: Caller<'_, Process>, ch: i32, o: i64, n: i32| c.data().copy(ch, o, n),
    )?;
    linker.func_wrap(
        "cerune",
        "process_release_v1",
        |mut c: Caller<'_, Process>| c.data_mut().release(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_text_and_all_byte_values_are_preserved_without_normalization() {
        let mut p = Process::new(if cfg!(windows) { 1 } else { 2 }, &[]).unwrap();
        assert_eq!(p.begin([1, 0, 0, 1, 0, 0, 0, 0]), 0);
        let text = "日本語\0\r\ne\u{0301}".as_bytes();
        assert_eq!(p.field(0, 0, text.len() as i64), 0);
        for b in text {
            assert_eq!(p.chunk(i64::from(*b) | -256i64, 1), 0);
        }
        assert_eq!(p.field(1, 0, 0), 0);
        let input: Vec<u8> = (0..=255).collect();
        assert_eq!(p.field(2, 0, 256), 0);
        for c in input.chunks(8) {
            assert_eq!(p.chunk(i64::from_le_bytes(c.try_into().unwrap()), 8), 0);
        }
        let r = p.receiving.as_ref().unwrap();
        assert_eq!(r.fields, [text.to_vec(), vec![], input]);
        assert_eq!(p.prepare(), 2); // Valid UTF-8 with NUL reaches common validation.
        assert_eq!(p.report(19), (1, 0));
    }
}
