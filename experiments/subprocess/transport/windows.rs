use super::{case, require};
use serde_json::json;
use std::{
    ffi::c_void,
    fs::File,
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
type Handle = *mut c_void;
const INVALID: Handle = -1isize as Handle;
const PENDING: i32 = 997;
const INCOMPLETE: i32 = 996;
const ABORTED: i32 = 995;
static QUARANTINED: AtomicUsize = AtomicUsize::new(0);
#[repr(C)]
struct Security {
    size: u32,
    descriptor: Handle,
    inherit: i32,
}
#[repr(C)]
#[derive(Default)]
struct Overlapped {
    internal: usize,
    high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        mode: u32,
        pipe_mode: u32,
        instances: u32,
        output: u32,
        input: u32,
        timeout: u32,
        security: *const Security,
    ) -> Handle;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: Handle,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn ConnectNamedPipe(pipe: Handle, overlapped: *mut Overlapped) -> i32;
    fn CreateEventW(security: Handle, manual: i32, initial: i32, name: *const u16) -> Handle;
    fn ReadFile(
        handle: Handle,
        buffer: *mut c_void,
        size: u32,
        read: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn WriteFile(
        handle: Handle,
        buffer: *const c_void,
        size: u32,
        written: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn GetOverlappedResult(
        handle: Handle,
        overlapped: *mut Overlapped,
        transferred: *mut u32,
        wait: i32,
    ) -> i32;
    fn CancelIoEx(handle: Handle, overlapped: *mut Overlapped) -> i32;
    fn WaitForSingleObject(handle: Handle, timeout: u32) -> u32;
    fn GetHandleInformation(handle: Handle, flags: *mut u32) -> i32;
    fn LocalFree(memory: Handle) -> Handle;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut Handle,
        size: *mut u32,
    ) -> i32;
}
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: Handle, buffer: *mut u8, size: u32, flags: u32) -> i32;
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
fn owned(handle: Handle) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}
struct Descriptor(Handle);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}
struct Pair {
    server: Arc<OwnedHandle>,
    client: Arc<OwnedHandle>,
    name: Vec<u16>,
}
fn pair() -> io::Result<Pair> {
    let mut random = [0u8; 16];
    require(
        unsafe { BCryptGenRandom(ptr::null_mut(), random.as_mut_ptr(), 16, 2) } >= 0,
        "pipe randomness",
    )?;
    let suffix = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let name = wide(&format!(
        r"\\.\pipe\cerune-transport-{}-{suffix}",
        std::process::id()
    ));
    let mut raw = ptr::null_mut();
    require(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide("D:P(A;;GA;;;OW)(A;;GA;;;SY)").as_ptr(),
                1,
                &mut raw,
                ptr::null_mut(),
            )
        } != 0,
        "pipe security descriptor",
    )?;
    let descriptor = Descriptor(raw);
    let security = Security {
        size: size_of::<Security>() as u32,
        descriptor: descriptor.0,
        inherit: 0,
    };
    let server = Arc::new(owned(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            3 | 0x40000000 | 0x80000,
            8,
            1,
            4096,
            4096,
            1000,
            &security,
        )
    })?);
    let client = Arc::new(owned(unsafe {
        CreateFileW(
            name.as_ptr(),
            0xc0000000,
            0,
            ptr::null_mut(),
            3,
            0x40000000,
            ptr::null_mut(),
        )
    })?);
    // clientが先に接続するのでPIPE_CONNECTEDが期待値です。未完了なら期限付きで回収します。
    let mut connect = Operation::empty(server.clone())?;
    let r = connect.resources.as_mut().unwrap();
    let result = unsafe { ConnectNamedPipe(server.as_raw_handle(), &mut r.overlapped) };
    let error = if result == 0 {
        io::Error::last_os_error().raw_os_error()
    } else {
        None
    };
    if error == Some(535) || result != 0 {
        connect.delivered = true;
    } else if error == Some(PENDING) {
        connect.pending = true;
        let completion = connect.wait()?;
        require(completion.error.is_none(), "pipe connection failed")?;
    } else {
        return Err(io::Error::from_raw_os_error(error.unwrap_or(1)));
    }
    Ok(Pair {
        server,
        client,
        name,
    })
}
struct Resources {
    handle: Arc<OwnedHandle>,
    event: OwnedHandle,
    overlapped: Overlapped,
    buffer: [u8; 65536],
}
#[derive(Debug, Clone, Copy)]
struct Completion {
    accepted: Option<u32>,
    error: Option<i32>,
}
struct Operation {
    resources: Option<Box<Resources>>,
    pending: bool,
    delivered: bool,
    immediate: Option<Completion>,
}
impl Operation {
    fn empty(handle: Arc<OwnedHandle>) -> io::Result<Self> {
        let event = owned(unsafe { CreateEventW(ptr::null_mut(), 1, 0, ptr::null()) })?;
        let overlapped = Overlapped {
            event: event.as_raw_handle(),
            ..Overlapped::default()
        };
        Ok(Self {
            resources: Some(Box::new(Resources {
                handle,
                event,
                overlapped,
                buffer: [0; 65536],
            })),
            pending: false,
            delivered: false,
            immediate: None,
        })
    }
    fn start(handle: Arc<OwnedHandle>, payload: Option<&[u8]>, size: usize) -> io::Result<Self> {
        require(size > 0 && size <= 65536, "probe transfer capacity")?;
        let mut op = Self::empty(handle)?;
        let r = op.resources.as_mut().unwrap();
        let result = if let Some(bytes) = payload {
            require(bytes.len() == size, "write size")?;
            r.buffer[..size].copy_from_slice(bytes);
            unsafe {
                WriteFile(
                    r.handle.as_raw_handle(),
                    r.buffer.as_ptr().cast(),
                    size as u32,
                    ptr::null_mut(),
                    &mut r.overlapped,
                )
            }
        } else {
            unsafe {
                ReadFile(
                    r.handle.as_raw_handle(),
                    r.buffer.as_mut_ptr().cast(),
                    size as u32,
                    ptr::null_mut(),
                    &mut r.overlapped,
                )
            }
        };
        let error = if result == 0 {
            io::Error::last_os_error().raw_os_error()
        } else {
            None
        };
        if result != 0 || error == Some(PENDING) {
            op.pending = true;
        } else {
            op.immediate = Some(Completion {
                accepted: None,
                error,
            });
        }
        Ok(op)
    }
    fn cancel(&mut self) -> Option<i32> {
        let r = self.resources.as_mut().unwrap();
        if unsafe { CancelIoEx(r.handle.as_raw_handle(), &mut r.overlapped) } != 0 {
            None
        } else {
            io::Error::last_os_error().raw_os_error()
        }
    }
    fn poll(&mut self) -> Option<Completion> {
        if self.delivered {
            return None;
        }
        if let Some(completion) = self.immediate.take() {
            self.delivered = true;
            return Some(completion);
        }
        if !self.pending {
            return None;
        }
        let r = self.resources.as_mut().unwrap();
        let mut bytes = 0;
        let ok = unsafe {
            GetOverlappedResult(r.handle.as_raw_handle(), &mut r.overlapped, &mut bytes, 0)
        };
        let error = if ok == 0 {
            io::Error::last_os_error().raw_os_error()
        } else {
            None
        };
        if error == Some(INCOMPLETE) {
            return None;
        }
        self.pending = false;
        self.delivered = true;
        // エラー時の出力引数を確認済みバイト数に転用しません。
        Some(Completion {
            accepted: if ok != 0 { Some(bytes) } else { None },
            error,
        })
    }
    fn wait_until(&mut self, deadline: Instant) -> io::Result<Completion> {
        loop {
            if let Some(done) = self.poll() {
                return Ok(done);
            }
            require(!self.delivered, "completion already consumed")?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "I/O terminal notification missing",
                ));
            }
            let r = self.resources.as_ref().unwrap();
            let status = unsafe {
                WaitForSingleObject(
                    r.event.as_raw_handle(),
                    remaining.as_millis().clamp(1, 100) as u32,
                )
            };
            if status == u32::MAX {
                return Err(io::Error::last_os_error());
            }
        }
    }
    fn wait(&mut self) -> io::Result<Completion> {
        self.wait_until(Instant::now() + Duration::from_secs(5))
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if self.pending {
            let _ = self.cancel();
            let _ = self.wait_until(Instant::now() + Duration::from_millis(250));
            if self.pending {
                // OS参照中のメモリを解放しません。この試作は隔離保持を失敗として報告します。
                let _ = Box::leak(self.resources.take().unwrap());
                QUARANTINED.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}
pub fn helper(_: &[String]) -> Option<io::Result<()>> {
    None
}
pub fn run(log: &mut File) -> io::Result<()> {
    case(log, "restricted-pipe-pair", || {
        let p = pair()?;
        for handle in [&p.server, &p.client] {
            let mut flags = 0;
            require(
                unsafe { GetHandleInformation(handle.as_raw_handle(), &mut flags) } != 0
                    && flags & 1 == 0,
                "inheritable control handle",
            )?;
        }
        let duplicate = unsafe {
            CreateNamedPipeW(
                p.name.as_ptr(),
                3 | 0x40000000 | 0x80000,
                8,
                1,
                4096,
                4096,
                1000,
                ptr::null(),
            )
        };
        if duplicate != INVALID {
            drop(owned(duplicate)?);
            return Err(io::Error::other("duplicate pipe instance accepted"));
        }
        Ok(
            json!({"owner_system_acl_configured":true,"remote_client_rejection_configured":true,"first_instance_enforced":true,"handles_noninheritable":true}),
        )
    })?;
    case(log, "short-read-binary", || {
        let p = pair()?;
        let bytes = "日本語\0\r\n".as_bytes();
        let mut read = Operation::start(p.server.clone(), None, 64)?;
        let mut write = Operation::start(p.client.clone(), Some(bytes), bytes.len())?;
        let sent = write.wait()?;
        let got = read.wait()?;
        require(
            sent.accepted == Some(bytes.len() as u32) && got.accepted == sent.accepted,
            "short transfer counts",
        )?;
        require(
            &read.resources.as_ref().unwrap().buffer[..bytes.len()] == bytes,
            "binary payload",
        )?;
        require(read.poll().is_none(), "duplicate completion")?;
        Ok(
            json!({"requested":64,"accepted":got.accepted,"write_accepted":sent.accepted,"counted_once":true}),
        )
    })?;
    case(log, "pending-read-cancel", || {
        let p = pair()?;
        let mut read = Operation::start(p.server.clone(), None, 64)?;
        require(read.poll().is_none(), "read unexpectedly complete")?;
        let address = (&read.resources.as_ref().unwrap().overlapped) as *const Overlapped as usize;
        let cancel = read.cancel();
        require(
            cancel.is_none() || cancel == Some(1168),
            "cancel request error",
        )?;
        let done = read.wait()?;
        require(
            done.error == Some(ABORTED) && !read.pending,
            "cancel terminal outcome",
        )?;
        require(
            address == (&read.resources.as_ref().unwrap().overlapped) as *const Overlapped as usize,
            "overlapped moved",
        )?;
        require(read.poll().is_none(), "cancel counted twice")?;
        Ok(
            json!({"cancel_request_error":cancel,"terminal_error":done.error,"accepted":done.accepted,"fixed_address":true,"counted_once":true}),
        )
    })?;
    case(log, "completion-before-cancel", || {
        let p = pair()?;
        let bytes = b"prefix";
        let mut write = Operation::start(p.client.clone(), Some(bytes), bytes.len())?;
        require(write.wait()?.accepted == Some(6), "prefix write")?;
        let mut read = Operation::start(p.server.clone(), None, 64)?;
        // OS完了を待ちますが、終端結果は取消し要求後に初めて取得します。
        require(
            unsafe {
                WaitForSingleObject(read.resources.as_ref().unwrap().event.as_raw_handle(), 5000)
            } == 0,
            "read not completed before cancellation",
        )?;
        let cancel = read.cancel();
        let done = read.wait()?;
        // OSで既に完了した操作への取消しが間に合わなくても、受理数を保持します。
        require(
            done.error.is_none() && done.accepted == Some(6),
            "late cancellation lost success",
        )?;
        require(read.poll().is_none(), "late completion twice")?;
        Ok(json!({"cancel_request_error":cancel,"accepted":done.accepted,"counted_once":true}))
    })?;
    case(log, "read-write-cancel-race", || {
        let (mut succeeded, mut cancelled) = (0, 0);
        for _ in 0..32 {
            let p = pair()?;
            let mut read = Operation::start(p.server.clone(), None, 64)?;
            let mut write = Operation::start(p.client.clone(), Some(b"race"), 4)?;
            let cancel = read.cancel();
            require(
                cancel.is_none() || cancel == Some(1168),
                "race cancel request",
            )?;
            let result = read.wait()?;
            require(write.wait()?.accepted == Some(4), "race write")?;
            match result.error {
                None => {
                    require(result.accepted == Some(4), "race accepted bytes")?;
                    succeeded += 1;
                }
                Some(ABORTED) => {
                    require(result.accepted.is_none(), "cancel bytes invented")?;
                    cancelled += 1;
                }
                _ => return Err(io::Error::other("unexpected race outcome")),
            }
            require(read.poll().is_none(), "race double count")?;
        }
        Ok(
            json!({"attempts":32,"success":succeeded,"cancelled":cancelled,"unknown_counts_not_zeroed":true}),
        )
    })?;
    case(log, "pending-write-cancel", || {
        let p = pair()?;
        let bytes = vec![42; 65536];
        let mut write = Operation::start(p.server.clone(), Some(&bytes), bytes.len())?;
        require(write.poll().is_none(), "large write unexpectedly complete")?;
        let cancel = write.cancel();
        let done = write.wait()?;
        require(done.error == Some(ABORTED), "write cancellation terminal")?;
        Ok(
            json!({"requested":bytes.len(),"cancel_request_error":cancel,"terminal_error":done.error,"accepted":done.accepted}),
        )
    })?;
    case(log, "peer-closed", || {
        let p = pair()?;
        let mut read = Operation::start(p.server.clone(), None, 64)?;
        drop(p.client);
        let done = read.wait()?;
        require(done.error == Some(109), "broken pipe terminal")?;
        Ok(json!({"terminal_error":done.error,"accepted":done.accepted,"terminal_confirmed":true}))
    })?;
    case(log, "pending-drop", || {
        let p = pair()?;
        let mut read = Operation::start(p.server.clone(), None, 64)?;
        require(read.poll().is_none(), "drop fixture unexpectedly complete")?;
        drop(read);
        require(
            Arc::strong_count(&p.server) == 1,
            "drop retained pipe resources",
        )?;
        Ok(json!({"terminal_settled_before_release":true}))
    })?;
    case(log, "buffer-ownership-settled", || {
        require(
            QUARANTINED.load(Ordering::SeqCst) == 0,
            "unsettled I/O resources quarantined",
        )?;
        Ok(json!({"quarantined_operations":0}))
    })?;
    println!("subprocess transport: 9 Windows cases passed");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_x64_overlapped_layout() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(size_of::<Overlapped>(), 32);
            assert_eq!(size_of::<Security>(), 24);
        }
    }
}
