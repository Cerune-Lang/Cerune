//! 試作用のSIGPIPE処理です。大域handlerを変更せず、呼出しthreadのmaskを復元します。
use super::*;
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
fn mask() -> io::Result<libc::sigset_t> {
    let mut mask: libc::sigset_t = unsafe { std::mem::zeroed() };
    let code = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut mask) };
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code));
    }
    Ok(mask)
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
pub fn probe(preexisting: bool) -> io::Result<()> {
    // Rust起動時のSIG_IGNに頼らないため、独立したfixtureだけを既定handlerにします。
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = libc::SIG_DFL;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
    }
    require(
        unsafe { libc::sigaction(libc::SIGPIPE, &action, std::ptr::null_mut()) } == 0,
        "set fixture handler",
    )?;
    let set = set();
    let original = mask()?;
    let _restore = Restore(original);
    if preexisting {
        require(
            unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) } == 0,
            "block fixture signal",
        )?;
        require(
            unsafe { libc::pthread_kill(libc::pthread_self(), libc::SIGPIPE) } == 0,
            "queue fixture signal",
        )?;
    }
    let before = mask()?;
    require(pending()? == preexisting, "initial pending state")?;
    let mut fds = [-1; 2];
    require(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } == 0,
        "fixture pipe",
    )?;
    let input = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    drop(unsafe { OwnedFd::from_raw_fd(fds[0]) });
    require(
        write(input.as_raw_fd(), b"x").unwrap_err().raw_os_error() == Some(libc::EPIPE),
        "broken pipe result",
    )?;
    let after = mask()?;
    require(
        (1..=64).all(|n| unsafe { libc::sigismember(&before, n) == libc::sigismember(&after, n) }),
        "thread mask changed",
    )?;
    require(pending()? == preexisting, "pending signal changed")?;
    let mut after_handler: libc::sigaction = unsafe { std::mem::zeroed() };
    require(
        unsafe { libc::sigaction(libc::SIGPIPE, std::ptr::null(), &mut after_handler) } == 0
            && after_handler.sa_sigaction == libc::SIG_DFL,
        "handler changed",
    )?;
    if preexisting {
        drain(&set)?;
    }
    Ok(())
}
