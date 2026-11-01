use super::{Exit, Pipes};
use std::{
    ffi::{OsStr, c_void},
    fs::File,
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
struct Security {
    size: u32,
    descriptor: Handle,
    inherit: i32,
}
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
    fn CreatePipe(
        read: *mut Handle,
        write: *mut Handle,
        security: *const Security,
        size: u32,
    ) -> i32;
    fn SetHandleInformation(handle: Handle, mask: u32, flags: u32) -> i32;
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
fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
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
fn pipe(parent_reads: bool) -> io::Result<(OwnedHandle, OwnedHandle)> {
    let security = Security {
        size: size_of::<Security>() as u32,
        descriptor: ptr::null_mut(),
        inherit: 1,
    };
    let (mut read, mut write) = (ptr::null_mut(), ptr::null_mut());
    check(unsafe { CreatePipe(&mut read, &mut write, &security, 0) })?;
    let read = owned(read)?;
    let write = owned(write)?;
    let parent = if parent_reads { &read } else { &write };
    check(unsafe { SetHandleInformation(parent.as_raw_handle(), 1, 0) })?;
    Ok((read, write))
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
pub fn init() -> io::Result<()> {
    Ok(())
}
pub struct Scope {
    process: OwnedHandle,
    job: OwnedHandle,
    signalled: bool,
    reaped: bool,
}
impl Scope {
    pub fn spawn(
        exe: &Path,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<(Self, Pipes)> {
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
        let (input_child, input_parent) = pipe(false)?;
        let (output_parent, output_child) = pipe(true)?;
        let (error_parent, error_child) = pipe(true)?;
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
        let mut environment = Vec::new();
        let mut sorted = env.to_vec();
        sorted.sort_by_key(|(key, _)| key.to_uppercase());
        for (key, value) in sorted {
            environment.extend(wide(OsStr::new(&format!("{key}={value}")))?);
        }
        if environment.is_empty() {
            environment.push(0);
        }
        environment.push(0);
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
                environment.as_mut_ptr().cast(),
                directory.as_ptr(),
                &mut startup.startup,
                &mut info,
            )
        })?;
        let thread = owned(info.thread)?;
        let scope = Self {
            process: owned(info.process)?,
            job,
            signalled: false,
            reaped: false,
        };
        if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        drop(input_child);
        drop(output_child);
        drop(error_child);
        Ok((
            scope,
            Pipes {
                input: Box::new(File::from(input_parent)),
                output: Box::new(File::from(output_parent)),
                error: Box::new(File::from(error_parent)),
            },
        ))
    }
    pub fn peek(&mut self) -> io::Result<Option<Exit>> {
        match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
            0 => {
                let mut code = 0;
                check(unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) })?;
                Ok(Some(Exit::Code(code)))
            }
            258 => Ok(None),
            _ => Err(io::Error::last_os_error()),
        }
    }
    pub fn terminate(&mut self) -> io::Result<()> {
        if !self.signalled {
            check(unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) })?;
            self.signalled = true;
        }
        Ok(())
    }
    pub fn collect(&mut self) -> io::Result<bool> {
        self.reaped = self.peek()?.is_some();
        let mut accounting: Accounting = unsafe { zeroed() };
        check(unsafe {
            QueryInformationJobObject(
                self.job.as_raw_handle(),
                1,
                (&mut accounting as *mut Accounting).cast(),
                size_of::<Accounting>() as u32,
                ptr::null_mut(),
            )
        })?;
        Ok(self.reaped && accounting.active == 0)
    }
    pub fn evidence(&self) -> serde_json::Value {
        serde_json::json!({"mechanism":"job-list-before-resume", "direct_waited":self.reaped})
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        let _ = self.terminate();
        let _ = self.collect();
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
