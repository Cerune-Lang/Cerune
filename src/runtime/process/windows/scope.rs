use super::super::Exit;
use std::{
    ffi::{OsStr, c_void},
    io,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr,
};

type Handle = *mut c_void;
#[repr(C)]
struct Startup {
    size: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    columns: u32,
    rows: u32,
    fill: u32,
    flags: u32,
    show: u16,
    reserved_size: u16,
    reserved_bytes: *mut u8,
    input: Handle,
    output: Handle,
    error: Handle,
}
#[repr(C)]
struct ExtendedStartup {
    startup: Startup,
    attributes: Handle,
}
#[repr(C)]
struct ProcessInfo {
    process: Handle,
    thread: Handle,
    process_id: u32,
    thread_id: u32,
}
#[repr(C)]
struct BasicLimits {
    process_time: i64,
    job_time: i64,
    flags: u32,
    minimum_working_set: usize,
    maximum_working_set: usize,
    active_process_limit: u32,
    affinity: usize,
    priority: u32,
    scheduling: u32,
}
#[repr(C)]
struct ExtendedLimits {
    basic: BasicLimits,
    io: [u64; 6],
    process_memory: usize,
    job_memory: usize,
    peak_process_memory: usize,
    peak_job_memory: usize,
}
#[repr(C)]
struct Accounting {
    user: i64,
    kernel: i64,
    period_user: i64,
    period_kernel: i64,
    faults: u32,
    total: u32,
    active: u32,
    terminated: u32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(security: Handle, name: *const u16) -> Handle;
    fn SetInformationJobObject(job: Handle, class: i32, info: Handle, size: u32) -> i32;
    fn QueryInformationJobObject(
        job: Handle,
        class: i32,
        info: Handle,
        size: u32,
        returned: *mut u32,
    ) -> i32;
    fn TerminateJobObject(job: Handle, code: u32) -> i32;
    fn InitializeProcThreadAttributeList(
        list: Handle,
        count: u32,
        flags: u32,
        size: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        list: Handle,
        flags: u32,
        attribute: usize,
        value: Handle,
        size: usize,
        previous: Handle,
        returned: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(list: Handle);
    fn CreateProcessW(
        application: *const u16,
        command: *mut u16,
        process_security: Handle,
        thread_security: Handle,
        inherit: i32,
        flags: u32,
        environment: Handle,
        directory: *const u16,
        startup: *mut Startup,
        info: *mut ProcessInfo,
    ) -> i32;
    fn ResumeThread(thread: Handle) -> u32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn GetExitCodeProcess(process: Handle, code: *mut u32) -> i32;
}
fn check(value: i32) -> io::Result<()> {
    if value == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn owned(raw: Handle) -> io::Result<OwnedHandle> {
    if raw.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
    }
}
pub(super) fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut result: Vec<_> = value.encode_wide().collect();
    if result.contains(&0) {
        return Err(io::Error::other("NUL in OS string"));
    }
    result.push(0);
    Ok(result)
}
/// CRT互換argvの規則です。shellの文字列構文ではありません。
fn quoted(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut result = vec![34];
    let mut slashes = 0;
    for ch in value.encode_wide() {
        if ch == 0 {
            return Err(io::Error::other("NUL in argument"));
        }
        if ch == 92 {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            92,
            slashes * if ch == 34 { 2 } else { 1 },
        ));
        slashes = 0;
        if ch == 34 {
            result.push(92);
        }
        result.push(ch);
    }
    result.extend(std::iter::repeat_n(92, slashes * 2));
    result.push(34);
    Ok(result)
}
struct Attributes(Vec<usize>);
impl Attributes {
    fn new() -> io::Result<Self> {
        let mut size = 0;
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 2, 0, &mut size);
        }
        if size == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut data = vec![0usize; size.div_ceil(size_of::<usize>())];
        check(unsafe {
            InitializeProcThreadAttributeList(data.as_mut_ptr().cast(), 2, 0, &mut size)
        })?;
        Ok(Self(data))
    }
    fn ptr(&mut self) -> Handle {
        self.0.as_mut_ptr().cast()
    }
    fn handles(&mut self, attribute: usize, handles: &mut [Handle]) -> io::Result<()> {
        check(unsafe {
            UpdateProcThreadAttribute(
                self.ptr(),
                0,
                attribute,
                handles.as_mut_ptr().cast(),
                std::mem::size_of_val(handles),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.ptr());
        }
    }
}
pub struct Scope {
    process: Option<OwnedHandle>,
    job: Option<OwnedHandle>,
    signalled: bool,
    reaped: bool,
    auxiliary: [Option<OwnedHandle>; 4],
}
impl Scope {
    /// 起動・Job管理を維持し、標準I/Oの生成方法だけを呼出し側へ分離します。
    pub fn spawn_suspended(
        exe: &Path,
        args: &[&str],
        cwd: &Path,
        env: &[u16],
        stdio: [OwnedHandle; 3],
    ) -> io::Result<Self> {
        let [input_child, output_child, error_child] = stdio;
        let job = owned(unsafe { CreateJobObjectW(ptr::null_mut(), ptr::null()) })?;
        let mut limits: ExtendedLimits = unsafe { zeroed() };
        limits.basic.flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        check(unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                9,
                (&mut limits as *mut ExtendedLimits).cast(),
                size_of::<ExtendedLimits>() as u32,
            )
        })?;
        let mut handles = [
            input_child.as_raw_handle(),
            output_child.as_raw_handle(),
            error_child.as_raw_handle(),
        ];
        let mut jobs = [job.as_raw_handle()];
        let mut attributes = Attributes::new()?;
        attributes.handles(0x20002, &mut handles)?; // PROC_THREAD_ATTRIBUTE_HANDLE_LIST
        attributes.handles(0x2000d, &mut jobs)?; // PROC_THREAD_ATTRIBUTE_JOB_LIST
        let mut startup: ExtendedStartup = unsafe { zeroed() };
        startup.startup.size = size_of::<ExtendedStartup>() as u32;
        startup.startup.flags = 0x100; // STARTF_USESTDHANDLES
        startup.startup.input = handles[0];
        startup.startup.output = handles[1];
        startup.startup.error = handles[2];
        startup.attributes = attributes.ptr();
        let application = wide(exe.as_os_str())?;
        let directory = wide(cwd.as_os_str())?;
        let mut command = quoted(exe.as_os_str())?;
        for arg in args {
            command.push(32);
            command.extend(quoted(OsStr::new(arg))?);
        }
        command.push(0);
        if command.len() > 32767 {
            return Err(io::Error::from_raw_os_error(87));
        }
        let environment = env;
        let mut info: ProcessInfo = unsafe { zeroed() };
        // Jobへ所属した状態で生成し、明示resumeまでユーザーコードを動かしません。
        check(unsafe {
            CreateProcessW(
                application.as_ptr(),
                command.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                0x0808_0404,
                environment.as_ptr().cast_mut().cast(),
                directory.as_ptr(),
                &mut startup.startup,
                &mut info,
            )
        })?;
        let thread = owned(info.thread)?;
        let scope = Self {
            process: Some(owned(info.process)?),
            job: Some(job),
            signalled: false,
            reaped: false,
            auxiliary: [
                Some(input_child),
                Some(output_child),
                Some(error_child),
                Some(thread),
            ],
        };
        Ok(scope)
    }
    pub fn resume(&self) -> io::Result<()> {
        if unsafe { ResumeThread(self.auxiliary[3].as_ref().unwrap().as_raw_handle()) } == u32::MAX
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub fn close_aux(&mut self) -> bool {
        let mut ok = true;
        for handle in &mut self.auxiliary {
            ok &= super::close(handle);
        }
        ok
    }
    pub fn close(&mut self) -> bool {
        let auxiliary = self.close_aux();
        let process = super::close(&mut self.process);
        let job = super::close(&mut self.job);
        process && job && auxiliary
    }
    pub fn reaped(&self) -> bool {
        self.reaped
    }
    pub fn peek(&mut self) -> io::Result<Option<Exit>> {
        let Some(process) = self.process.as_ref() else {
            return Ok(None);
        };
        match unsafe { WaitForSingleObject(process.as_raw_handle(), 0) } {
            0 => {
                let mut code = 0;
                check(unsafe {
                    GetExitCodeProcess(self.process.as_ref().unwrap().as_raw_handle(), &mut code)
                })?;
                Ok(Some(Exit::Code(code)))
            }
            258 => Ok(None),
            _ => Err(io::Error::last_os_error()),
        }
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        if !self.signalled {
            let Some(job) = self.job.as_ref() else {
                return Ok(());
            };
            check(unsafe { TerminateJobObject(job.as_raw_handle(), 1) })?;
            self.signalled = true;
        }
        Ok(())
    }
    pub fn collect(&mut self) -> io::Result<bool> {
        self.reaped |= self.peek()?.is_some();
        let Some(job) = self.job.as_ref() else {
            return Ok(self.reaped);
        };
        let mut accounting: Accounting = unsafe { zeroed() };
        check(unsafe {
            QueryInformationJobObject(
                job.as_raw_handle(),
                1,
                (&mut accounting as *mut Accounting).cast(),
                size_of::<Accounting>() as u32,
                ptr::null_mut(),
            )
        })?;
        Ok(self.reaped && accounting.active == 0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quote_empty_trailing_slash_and_quote_without_shell_rules() {
        for (input, expected) in [("", "\"\""), ("a\\", "\"a\\\\\""), ("a\"b", "\"a\\\"b\"")] {
            assert_eq!(
                String::from_utf16(&quoted(OsStr::new(input)).unwrap()).unwrap(),
                expected
            );
        }
        assert!(quoted(OsStr::new("a\0b")).is_err());
    }
    #[test]
    fn windows_structure_sizes_match_x64_abi() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(size_of::<Startup>(), 104);
            assert_eq!(size_of::<ExtendedStartup>(), 112);
            assert_eq!(size_of::<ExtendedLimits>(), 144);
            assert_eq!(size_of::<Accounting>(), 48);
        }
    }
}
