//! 親側は非同期、子側は同期のbyte pipeです。内部bufferを公開しません。
use super::*;
use std::{ffi::c_void, mem::size_of, ptr};
pub(super) type Handle = *mut c_void;
#[repr(C)]
struct Security {
    size: u32,
    descriptor: Handle,
    inherit: i32,
}
#[repr(C)]
#[derive(Default)]
pub(super) struct Overlapped {
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
    fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
    fn LocalFree(memory: Handle) -> Handle;
    fn CreateEventW(security: Handle, manual: i32, initial: i32, name: *const u16) -> Handle;
    fn ReadFile(
        handle: Handle,
        buffer: *mut c_void,
        size: u32,
        read: *mut u32,
        ov: *mut c_void,
    ) -> i32;
    fn WriteFile(
        handle: Handle,
        buffer: *const c_void,
        size: u32,
        written: *mut u32,
        ov: *mut c_void,
    ) -> i32;
    fn GetOverlappedResult(
        handle: Handle,
        ov: *mut c_void,
        transferred: *mut u32,
        wait: i32,
    ) -> i32;
    fn CancelIoEx(handle: Handle, ov: *mut c_void) -> i32;
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
fn owned(raw: Handle) -> io::Result<OwnedHandle> {
    if raw.is_null() || raw == -1isize as Handle {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
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
pub(super) fn pair() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let mut random = [0u8; 16];
    if unsafe { BCryptGenRandom(ptr::null_mut(), random.as_mut_ptr(), 16, 2) } < 0 {
        return Err(io::Error::other("pipe name allocation"));
    }
    let suffix = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let name = wide(&format!(
        r"\\.\pipe\cerune-process-{}-{suffix}",
        std::process::id()
    ));
    let mut raw = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide("D:P(A;;GA;;;OW)(A;;GA;;;SY)").as_ptr(),
            1,
            &mut raw,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let descriptor = Descriptor(raw);
    let security = Security {
        size: size_of::<Security>() as u32,
        descriptor: descriptor.0,
        inherit: 0,
    };
    let parent = owned(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            3 | 0x40000000 | 0x80000,
            8,
            1,
            4096,
            4096,
            0,
            &security,
        )
    })?;
    // 同じprocessで作成済みのinstanceへ同期clientを接続します。
    let child = owned(unsafe {
        CreateFileW(
            name.as_ptr(),
            0xc0000000,
            0,
            ptr::null_mut(),
            3,
            0,
            ptr::null_mut(),
        )
    })?;
    if unsafe { SetHandleInformation(child.as_raw_handle(), 1, 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((parent, child))
}
pub(super) struct Flight {
    transfer: Option<Transfer>,
    ov: Box<Overlapped>,
    event: Option<OwnedHandle>,
    handle: Handle,
    pending: bool,
    immediate: Option<(usize, Completion, Option<i32>)>,
    query_error: Option<i32>,
    query_reported: bool,
    #[cfg(test)]
    pub(super) inject_query_error: Option<i32>,
}
impl Flight {
    pub fn start(handle: Handle, transfer: Transfer) -> Result<Self, (Transfer, io::Error)> {
        let Ok(event) = owned(unsafe { CreateEventW(ptr::null_mut(), 1, 0, ptr::null()) }) else {
            return Err((transfer, io::Error::last_os_error()));
        };
        let ov = Box::new(Overlapped {
            event: event.as_raw_handle(),
            ..Default::default()
        });
        let mut op = Self {
            transfer: Some(transfer),
            ov,
            event: Some(event),
            handle,
            pending: false,
            immediate: None,
            query_error: None,
            query_reported: false,
            #[cfg(test)]
            inject_query_error: None,
        };
        let transfer = op.transfer.as_mut().unwrap();
        let size = transfer.bytes().len() as u32;
        let result = if transfer.kind() == Kind::Write {
            unsafe {
                WriteFile(
                    handle,
                    transfer.bytes().as_ptr().cast(),
                    size,
                    ptr::null_mut(),
                    (&mut *op.ov as *mut Overlapped).cast(),
                )
            }
        } else {
            unsafe {
                ReadFile(
                    handle,
                    transfer.read_buffer().unwrap().as_mut_ptr().cast(),
                    size,
                    ptr::null_mut(),
                    (&mut *op.ov as *mut Overlapped).cast(),
                )
            }
        };
        let error = if result == 0 {
            io::Error::last_os_error().raw_os_error()
        } else {
            None
        };
        if result != 0 || error == Some(997) {
            op.pending = true;
        } else {
            op.immediate = Some((0, classify(transfer.kind(), error), error));
        }
        Ok(op)
    }
    pub fn ticket(&self) -> Option<Ticket> {
        self.transfer.as_ref().map(Transfer::ticket)
    }
    pub fn cancel(&mut self) -> (CancelReply, Option<i32>) {
        if !self.pending {
            return (CancelReply::TooLate, None);
        }
        if unsafe { CancelIoEx(self.handle, (&mut *self.ov as *mut Overlapped).cast()) } != 0 {
            (CancelReply::Requested, None)
        } else {
            let code = io::Error::last_os_error().raw_os_error();
            (
                if code == Some(1168) {
                    CancelReply::TooLate
                } else {
                    CancelReply::Failed
                },
                code,
            )
        }
    }
    pub fn poll(&mut self) -> Option<(Completed, Option<i32>)> {
        let kind = self.transfer.as_ref()?.kind();
        #[cfg(test)]
        if let Some(error) = self.inject_query_error {
            self.query_error = Some(error);
            return None;
        }
        let (confirmed, result, error) = if let Some(done) = self.immediate.take() {
            done
        } else {
            let mut n = 0;
            let ok = unsafe {
                GetOverlappedResult(
                    self.handle,
                    (&mut *self.ov as *mut Overlapped).cast(),
                    &mut n,
                    0,
                )
            };
            let error = if ok == 0 {
                io::Error::last_os_error().raw_os_error()
            } else {
                None
            };
            if error == Some(996) {
                return None;
            }
            // 問合せ自体の失敗は終端の証拠にしません。OS参照中の可能性があれば保持します。
            if ok == 0 && !matches!(error, Some(109 | 232 | 233 | 995)) {
                if !self.query_reported {
                    self.query_error = error;
                    self.query_reported = true;
                }
                return None;
            }
            self.pending = false;
            if ok != 0 {
                // pipeの0バイト成功は相手の0バイトwriteでも起こり得るため、EOFにしません。
                (n as usize, Completion::Done, None)
            } else {
                (0, classify(kind, error), error)
            }
        };
        Some((
            Completed {
                transfer: self.transfer.take().unwrap(),
                confirmed,
                result,
            },
            error,
        ))
    }
    pub fn close(&mut self) -> bool {
        !self.pending && super::close(&mut self.event)
    }
    pub fn take_query_error(&mut self) -> Option<i32> {
        self.query_error.take()
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
}
fn classify(kind: Kind, error: Option<i32>) -> Completion {
    // byte-mode readのbroken pipeだけが確定したEOFです。他のエラーの進行量は不明です。
    if matches!(kind, Kind::Read(_)) && error == Some(109) {
        Completion::Eof
    } else {
        Completion::ProgressUnknown
    }
}
