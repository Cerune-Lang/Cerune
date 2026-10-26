use super::*;
use crate::{ir::NodeId, source::Span};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

#[derive(Clone, Copy)]
enum Step {
    Count(usize),
    Interrupted,
    Fail,
    Invalid,
    Zero,
}
#[derive(Default)]
struct State {
    files: BTreeMap<String, Vec<u8>>,
    calls: Vec<String>,
    reads: VecDeque<Step>,
    writes: VecDeque<Step>,
    open_error: Option<OpenError>,
    close_error: bool,
}
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<State>>);
struct MemoryHandle {
    memory: Memory,
    name: String,
    position: usize,
}
impl Backend for Memory {
    type Handle = MemoryHandle;
    fn open(&mut self, name: &str, operation: Operation) -> Result<MemoryHandle, OpenError> {
        let mut s = self.0.borrow_mut();
        s.calls.push(format!("open:{operation:?}:{name}"));
        if let Some(error) = s.open_error {
            return Err(error);
        }
        match operation {
            Operation::ReadText if !s.files.contains_key(name) => return Err(OpenError::NotFound),
            Operation::WriteNew => {
                if s.files.contains_key(name) {
                    return Err(OpenError::AlreadyExists);
                }
                s.files.insert(name.to_owned(), Vec::new());
            }
            _ => {}
        }
        Ok(MemoryHandle {
            memory: self.clone(),
            name: name.to_owned(),
            position: 0,
        })
    }
}
fn count(step: Option<Step>, requested: usize) -> io::Result<usize> {
    match step {
        Some(Step::Count(n)) => Ok(n.min(requested)),
        Some(Step::Interrupted) => Err(io::ErrorKind::Interrupted.into()),
        Some(Step::Fail) => Err(io::Error::other("private OS error")),
        Some(Step::Invalid) => Ok(requested + 1),
        Some(Step::Zero) => Ok(0),
        None => Ok(requested),
    }
}
impl Read for MemoryHandle {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push(format!("read:{}", into.len()));
        let n = count(s.reads.pop_front(), into.len())?;
        if n > into.len() {
            return Ok(n);
        }
        let data = &s.files[&self.name];
        let n = n.min(data.len() - self.position);
        into[..n].copy_from_slice(&data[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}
impl Write for MemoryHandle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push(format!("write:{}", bytes.len()));
        let n = count(s.writes.pop_front(), bytes.len())?;
        if n <= bytes.len() {
            s.files
                .get_mut(&self.name)
                .unwrap()
                .extend_from_slice(&bytes[..n]);
        }
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("file operations must not implicitly flush stdout")
    }
}
impl Handle for MemoryHandle {
    fn close(self) -> io::Result<()> {
        let mut s = self.memory.0.borrow_mut();
        s.calls.push("close".to_owned());
        if s.close_error {
            Err(io::ErrorKind::Interrupted.into())
        } else {
            Ok(())
        }
    }
}
fn origin() -> Origin {
    Origin {
        node_id: NodeId(7),
        span: Span::new(10, 22),
    }
}
fn input(bytes: &[u8]) -> Memory {
    let m = Memory::default();
    m.0.borrow_mut()
        .files
        .insert("日本語.txt".to_owned(), bytes.to_vec());
    m
}
fn read_file(f: &mut Files<Memory>, limit: i64) -> Result<String, Failure> {
    f.read_text("日本語.txt", limit, 4096, origin())
}

#[test]
fn portable_names_preserve_unicode_and_reject_path_and_device_forms() {
    for name in [
        "a",
        ".hidden",
        "a..b",
        "日本語.txt",
        "é",
        "e\u{301}",
        "😀.txt",
        "com0",
        "com10.txt",
        "nulled",
        "a b",
    ] {
        assert_eq!(validate_name(name), Ok(()), "{name:?}");
    }
    for name in [
        "",
        ".",
        "..",
        "/x",
        "a/b",
        "a\\b",
        "C:x",
        "\\\\server\\share",
        "a:stream",
        "a\0b",
        "a\nb",
        "a\u{7f}",
        "a?b",
        "a*b",
        "a<b",
        "a>b",
        "a|b",
        "a\"b",
        "a.",
        "a ",
        "CON",
        "con.txt",
        "NUL.tar.gz",
        "AUX",
        "prn",
        "COM1",
        "lpt9.TXT",
        "COM¹.txt",
        "lpt²",
        "LPT³",
        "CONIN$",
        "conout$.txt",
        "NUL .txt",
    ] {
        assert_eq!(validate_name(name), Err(Code::InvalidName), "{name:?}");
    }
    assert!(validate_name(&"x".repeat(255)).is_ok());
    assert!(validate_name(&"x".repeat(256)).is_err());
    assert!(validate_name(&"日".repeat(85)).is_ok());
    assert!(validate_name(&"日".repeat(86)).is_err());
}

#[test]
fn round_trip_preserves_bytes_and_exclusive_creation() {
    let m = Memory::default();
    let mut f = Files::with_trace_limit(m.clone(), 16);
    let text = "日本語\0\r\ne\u{301}\n\u{feff}";
    f.write_new("日本語.txt", text, 128, origin()).unwrap();
    assert_eq!(read_file(&mut f, 128).unwrap(), text);
    assert_eq!(read_file(&mut f, 128).unwrap(), text);
    assert_eq!(m.0.borrow().files["日本語.txt"], text.as_bytes());
    assert_eq!(
        f.events().iter().map(|e| e.sequence).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    let e = f
        .write_new("日本語.txt", "replace", 128, origin())
        .unwrap_err();
    assert_eq!(e.code, Code::AlreadyExists);
    assert_eq!(e.details.created, Created::No);
    assert_eq!(e.details.cleanup, Cleanup::NotAttempted);
    assert_eq!(m.0.borrow().files["日本語.txt"], text.as_bytes());
}

#[test]
fn zero_exact_and_over_limit_reads_distinguish_eof_from_overflow() {
    for (bytes, limit, code, consumed) in [
        (&b""[..], 0, None, 0),
        (b"x", 0, Some(Code::LimitExceeded), 1),
        (b"abc", 3, None, 3),
        (b"abcd", 3, Some(Code::LimitExceeded), 4),
        (&b"\xffabcd"[..], 3, Some(Code::LimitExceeded), 4),
        (&b"\xff"[..], 1, Some(Code::InvalidUtf8), 1),
    ] {
        let m = input(bytes);
        let mut f = Files::with_trace_limit(m.clone(), 8);
        let result = read_file(&mut f, limit);
        assert_eq!(result.as_ref().err().map(|e| e.code), code);
        let d = f.events()[0].details;
        assert_eq!(d.transferred, consumed);
        assert_eq!(d.cleanup, Cleanup::Closed);
        assert_eq!(m.0.borrow().calls.last().unwrap(), "close");
        if code == Some(Code::LimitExceeded) {
            assert_eq!(d.requested, None);
        } else {
            assert_eq!(d.requested, Some(consumed));
        }
        if bytes == b"abc" {
            assert_eq!(
                m.0.borrow().calls,
                ["open:ReadText:日本語.txt", "read:3", "read:1", "close"]
            );
        }
    }
}

#[test]
fn empty_write_creates_a_file_without_a_write_call() {
    let m = Memory::default();
    let mut f = Files::new(m.clone());
    f.write_new("empty", "", 0, origin()).unwrap();
    assert_eq!(m.0.borrow().files["empty"], b"");
    assert_eq!(m.0.borrow().calls, ["open:WriteNew:empty", "close"]);
}

#[test]
fn short_transfers_and_interruptions_preserve_progress() {
    let text = "日本語\0\r\ne\u{301}";
    let m = Memory::default();
    m.0.borrow_mut().writes = [Step::Count(1), Step::Interrupted, Step::Count(2)].into();
    m.0.borrow_mut().reads = [Step::Count(1), Step::Interrupted, Step::Count(2)].into();
    let mut f = Files::with_trace_limit(m.clone(), 4);
    f.write_new("日本語.txt", text, 128, origin()).unwrap();
    assert_eq!(read_file(&mut f, 128).unwrap(), text);
    for e in f.events() {
        assert_eq!(e.details.transferred, text.len() as u64);
        assert!(e.details.progress_known);
    }
    assert_eq!(
        m.0.borrow()
            .calls
            .iter()
            .filter(|s| s.as_str() == "close")
            .count(),
        2
    );
}

#[test]
fn read_failures_close_once_and_poison_the_connection() {
    for step in [Step::Fail, Step::Invalid] {
        let m = input(b"abcdef");
        m.0.borrow_mut().reads = [Step::Count(2), step].into();
        let mut f = Files::with_trace_limit(m.clone(), 4);
        let e = read_file(&mut f, 6).unwrap_err();
        assert_eq!(e.details.transferred, 2);
        assert_eq!(e.details.phase, Phase::Read);
        assert_eq!(e.details.progress_known, !matches!(step, Step::Invalid));
        let before = m.0.borrow().calls.clone();
        assert_eq!(f.write_new("later", "x", 1, origin()), Err(e));
        assert_eq!(f.read_text("bad/path", -1, 0, origin()), Err(e));
        assert_eq!(m.0.borrow().calls, before);
        assert_eq!(f.events().len(), 1);
        assert_eq!(before.iter().filter(|s| s.as_str() == "close").count(), 1);
    }
}

#[test]
fn write_failures_keep_partial_files_and_known_prefixes() {
    for (step, code, known) in [
        (Step::Fail, Code::WriteFailed, true),
        (Step::Zero, Code::ZeroProgress, true),
        (Step::Invalid, Code::InvalidResponse, false),
    ] {
        let m = Memory::default();
        m.0.borrow_mut().writes = [Step::Count(1), step].into();
        let mut f = Files::with_trace_limit(m.clone(), 8);
        let e = f
            .write_new("private.txt", "日本語", 32, origin())
            .unwrap_err();
        assert_eq!(e.code, code);
        assert_eq!(e.details.transferred, 1);
        assert_eq!(e.details.created, Created::Yes);
        assert_eq!(e.details.progress_known, known);
        assert_eq!(m.0.borrow().files["private.txt"], &"日本語".as_bytes()[..1]);
        assert_eq!(e.details.cleanup, Cleanup::Closed);
        let calls = m.0.borrow().calls.clone();
        assert_eq!(f.write_new("other", "x", 1, origin()), Err(e));
        assert_eq!(m.0.borrow().calls, calls);
        assert!(!e.record().contains("private"));
        assert!(!e.record().contains("日本語"));
    }
}

#[test]
fn close_failure_is_primary_only_if_transfer_succeeded() {
    for read in [false, true] {
        for failed_transfer in [false, true] {
            let m = input(b"\xff");
            m.0.borrow_mut().close_error = true;
            if failed_transfer {
                m.0.borrow_mut().reads = [Step::Fail].into();
                m.0.borrow_mut().writes = [Step::Fail].into();
            }
            let mut f = Files::new(m.clone());
            let e = if read {
                read_file(&mut f, 1).unwrap_err()
            } else {
                f.write_new("new", "abc", 3, origin()).unwrap_err()
            };
            assert_eq!(
                e.code,
                if failed_transfer {
                    if read {
                        Code::ReadFailed
                    } else {
                        Code::WriteFailed
                    }
                } else {
                    Code::CloseFailed
                }
            );
            assert_eq!(e.details.cleanup, Cleanup::Failed);
            assert_eq!(
                e.details.phase,
                if failed_transfer {
                    if read { Phase::Read } else { Phase::Write }
                } else {
                    Phase::Close
                }
            );
            assert_eq!(
                m.0.borrow()
                    .calls
                    .iter()
                    .filter(|s| s.as_str() == "close")
                    .count(),
                1
            );
        }
    }
}

#[test]
fn validation_order_and_reservations_precede_external_operations() {
    for (name, limit, available, expected) in [
        ("bad/path", -1, 0, Code::InvalidLimit),
        ("bad/path", 1, 0, Code::InvalidName),
        ("ok", 4, 3, Code::BudgetExceeded),
        ("ok", MAX_FILE_BYTES + 1, u64::MAX, Code::InvalidLimit),
    ] {
        let m = Memory::default();
        let mut f = Files::new(m.clone());
        assert_eq!(
            f.read_text(name, limit, available, origin())
                .unwrap_err()
                .code,
            expected
        );
        assert!(m.0.borrow().calls.is_empty());
    }
    let mut f = Files::<Memory>::unconfigured();
    assert_eq!(
        f.read_text("ok", 1, 0, origin()).unwrap_err().code,
        Code::Unconfigured
    );
    let m = Memory::default();
    let mut f = Files::new(m.clone());
    assert_eq!(
        f.write_new("ok", "long", 0, origin()).unwrap_err().code,
        Code::LimitExceeded
    );
    assert!(m.0.borrow().calls.is_empty());
    for fail_prepare in [false, true] {
        let m = Memory::default();
        let mut f = Files::new(m.clone());
        f.fail_allocation = !fail_prepare;
        let error = f
            .read_text_prepared("ok", 1, 1, origin(), |_| {
                if fail_prepare {
                    Err(Code::AllocationFailed)
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(error.code, Code::AllocationFailed);
        assert!(m.0.borrow().calls.is_empty());
    }
}

#[test]
fn open_errors_do_not_close_or_guess_unknown_creation() {
    for (error, code) in [
        (OpenError::NotFound, Code::NotFound),
        (OpenError::AlreadyExists, Code::AlreadyExists),
        (OpenError::UnsupportedTarget, Code::UnsupportedTarget),
        (OpenError::Failed, Code::OpenFailed),
        (OpenError::InvalidResponse, Code::InvalidResponse),
    ] {
        let m = Memory::default();
        m.0.borrow_mut().open_error = Some(error);
        let mut f = Files::new(m.clone());
        let e = f.write_new("new", "data", 4, origin()).unwrap_err();
        assert_eq!(e.code, code);
        assert_eq!(e.details.cleanup, Cleanup::NotAttempted);
        assert_eq!(
            e.details.created,
            if error == OpenError::InvalidResponse {
                Created::Unknown
            } else {
                Created::No
            }
        );
        assert_eq!(m.0.borrow().calls.len(), 1);
    }
}

#[test]
fn trace_capacity_and_record_allocation_do_not_change_execution() {
    let mut expected = None;
    for (capacity, fail) in [(0, false), (1, false), (8, false), (8, true)] {
        let m = Memory::default();
        let mut f = Files::with_trace_limit(m.clone(), capacity);
        f.fail_record_allocation = fail;
        f.write_new("日本語.txt", "abc", 3, origin()).unwrap();
        assert_eq!(read_file(&mut f, 3).unwrap(), "abc");
        let e = f.write_new("日本語.txt", "no", 2, origin()).unwrap_err();
        let result = (m.0.borrow().calls.clone(), m.0.borrow().files.clone(), e);
        if let Some(expected) = &expected {
            assert_eq!(&result, expected);
        } else {
            expected = Some(result);
        }
        assert_eq!(f.trace_truncated(), capacity == 1 || fail);
        if capacity == 0 {
            assert!(f.events().is_empty());
        }
    }
}

#[test]
fn failure_record_has_exact_origin_and_no_name_or_content() {
    let m = Memory::default();
    m.0.borrow_mut().writes = [Step::Count(1), Step::Fail].into();
    let e = Files::new(m)
        .write_new("secret", "abc", 3, origin())
        .unwrap_err();
    assert_eq!(
        e.record(),
        "file-v1 operation=write-new phase=write code=write-failed limit=3 requested=3 transferred=1 progress=known created=yes cleanup=closed node=7 bytes=10..22"
    );
    assert_eq!(e.to_string(), e.record());
}

#[test]
fn read_eof_and_progress_are_per_operation_not_cached() {
    let m = input(b"");
    let mut f = Files::new(m.clone());
    assert_eq!(read_file(&mut f, 8).unwrap(), "");
    m.0.borrow_mut()
        .files
        .insert("日本語.txt".to_owned(), b"changed".to_vec());
    let first = read_file(&mut f, 8).unwrap();
    assert_eq!(first, "changed");
    m.0.borrow_mut()
        .files
        .insert("日本語.txt".to_owned(), b"new".to_vec());
    assert_eq!(first, "changed");
    assert_eq!(read_file(&mut f, 8).unwrap(), "new");
}

#[test]
fn chunk_boundaries_and_eof_probe_failures_keep_exact_consumption() {
    for size in [4095, 4096, 4097, 8192] {
        for extra in [0, 1] {
            let bytes = vec![b'a'; size + extra];
            let m = input(&bytes);
            let mut f = Files::with_trace_limit(m, 2);
            let r = f.read_text("日本語.txt", size as i64, size as u64, origin());
            if extra == 0 {
                assert_eq!(r.unwrap().as_bytes(), bytes);
            } else {
                assert_eq!(r.unwrap_err().code, Code::LimitExceeded);
            }
            assert_eq!(f.events()[0].details.transferred, (size + extra) as u64);
        }
    }
    let m = input(b"abc");
    m.0.borrow_mut().reads = [Step::Count(3), Step::Fail].into();
    let e = read_file(&mut Files::new(m.clone()), 3).unwrap_err();
    assert_eq!(e.code, Code::ReadFailed);
    assert_eq!(e.details.transferred, 3);
    assert_eq!(e.details.requested, None);
    assert_eq!(e.details.cleanup, Cleanup::Closed);
    assert_eq!(
        m.0.borrow().calls,
        ["open:ReadText:日本語.txt", "read:3", "read:1", "close"]
    );
}

#[test]
fn every_utf8_transfer_split_keeps_the_same_bytes() {
    let text = "日本語\0\r\ne\u{301}😀";
    for split in 1..=text.len() {
        let m = Memory::default();
        m.0.borrow_mut().writes = [Step::Count(split), Step::Interrupted].into();
        m.0.borrow_mut().reads = [Step::Count(split), Step::Interrupted].into();
        let mut f = Files::new(m.clone());
        f.write_new("日本語.txt", text, text.len() as i64, origin())
            .unwrap();
        assert_eq!(read_file(&mut f, text.len() as i64).unwrap(), text);
        assert_eq!(m.0.borrow().files["日本語.txt"], text.as_bytes());
    }
}

#[test]
fn prepared_ownership_is_released_on_failure_and_transferred_on_success() {
    use std::cell::Cell;
    struct Reservation(Rc<Cell<usize>>);
    impl Drop for Reservation {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    for case in 0..4 {
        let m = input(b"abc");
        let mut f = Files::new(m.clone());
        if case == 1 {
            f.fail_allocation = true;
        }
        if case == 2 {
            m.0.borrow_mut().close_error = true;
        }
        if case == 3 {
            m.0.borrow_mut()
                .files
                .insert("日本語.txt".to_owned(), vec![255]);
        }
        let dropped = Rc::new(Cell::new(0));
        let result = f.read_text_prepared("日本語.txt", 3, 3, origin(), |_| {
            assert!(m.0.borrow().calls.is_empty());
            Ok(Reservation(dropped.clone()))
        });
        if case == 0 {
            let (text, reservation) = result.unwrap();
            assert_eq!(text, "abc");
            assert_eq!(dropped.get(), 0);
            drop(reservation);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(dropped.get(), 1);
    }
}

#[test]
fn source_id_survives_in_failure_records() {
    let mut sources = crate::source::SourceMap::new();
    let id = sources.add_standard("std:fs", 1, "internal operation");
    let site = Origin {
        node_id: NodeId(9),
        span: Span::in_source(id, 1, 4),
    };
    let e = Files::<Memory>::unconfigured()
        .write_new("secret", "secret", 6, site)
        .unwrap_err();
    assert_eq!(e.details.origin, site);
    assert!(
        e.record()
            .contains(&format!("node=9 file={} bytes=1..4", id.index()))
    );
    assert!(!e.record().contains("secret"));
}
