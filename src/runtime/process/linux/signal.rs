//! SIGPIPE処理です。大域handlerを変更せず、呼出しthreadのmaskを復元します。
use std::{io, os::fd::RawFd};
fn set() -> libc::sigset_t {
    let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGPIPE);
    }
    set
}
fn pending() -> io::Result<bool> {
    let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigpending(&mut set) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { libc::sigismember(&set, libc::SIGPIPE) } == 1)
}
struct Restore(libc::sigset_t);
impl Drop for Restore {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, &self.0, std::ptr::null_mut());
        }
    }
}
fn drain(set: &libc::sigset_t) -> io::Result<()> {
    let timeout = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    loop {
        let n = unsafe { libc::sigtimedwait(set, std::ptr::null_mut(), &timeout) };
        if n == libc::SIGPIPE {
            return Ok(());
        }
        let e = io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EAGAIN) {
            return Ok(());
        }
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}
pub fn write(fd: RawFd, bytes: &[u8]) -> io::Result<usize> {
    let set = set();
    let mut old: libc::sigset_t = unsafe { std::mem::zeroed() };
    let code = unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, &mut old) };
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code));
    }
    let _restore = Restore(old);
    let was_pending = pending()?;
    let n = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
    let error = (n < 0).then(io::Error::last_os_error);
    if error
        .as_ref()
        .is_some_and(|e| e.raw_os_error() == Some(libc::EPIPE))
        && !was_pending
    {
        drain(&set)?;
    }
    match error {
        Some(e) => Err(e),
        None => Ok(n as usize),
    }
}
