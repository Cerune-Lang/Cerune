//! fork後に動くため、固定bufferとOS呼出しだけで継承対象を制限します。
use std::io;
use std::os::fd::RawFd;
pub fn prepare(control: RawFd, force_scan: bool) -> io::Result<()> {
    // close_rangeはRustのexec失敗通知用fdも「閉鎖」せずCLOEXECにします。
    let result = if force_scan {
        -1
    } else {
        unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 4u32) }
    };
    if result != 0 {
        if !force_scan {
            let e = io::Error::last_os_error();
            if !matches!(e.raw_os_error(), Some(libc::ENOSYS | libc::EINVAL)) {
                return Err(e);
            }
        }
        scan()?;
    }
    if unsafe { libc::fcntl(control, libc::F_SETFD, 0) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn scan() -> io::Result<()> {
    let dir = unsafe {
        libc::open(
            c"/proc/self/fd".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if dir < 0 {
        return Err(io::Error::last_os_error());
    }
    let result = scan_open(dir);
    unsafe {
        libc::close(dir);
    }
    result
}
fn scan_open(dir: RawFd) -> io::Result<()> {
    let mut buffer = [0u8; 4096];
    loop {
        let n =
            unsafe { libc::syscall(libc::SYS_getdents64, dir, buffer.as_mut_ptr(), buffer.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        if n == 0 {
            return Ok(());
        }
        let mut offset = 0;
        while offset < n as usize {
            if offset + 20 > n as usize {
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            let length = u16::from_ne_bytes([buffer[offset + 16], buffer[offset + 17]]) as usize;
            if length < 20 || offset + length > n as usize {
                return Err(io::Error::from_raw_os_error(libc::EIO));
            }
            let name = &buffer[offset + 19..offset + length];
            let mut fd = 0i32;
            let mut digits = 0;
            for &byte in name {
                if byte == 0 {
                    break;
                }
                if !byte.is_ascii_digit() {
                    digits = 0;
                    break;
                }
                fd = fd
                    .checked_mul(10)
                    .and_then(|n| n.checked_add((byte - b'0') as i32))
                    .ok_or_else(|| io::Error::from_raw_os_error(libc::EIO))?;
                digits += 1;
            }
            if digits != 0
                && fd >= 3
                && unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0
            {
                return Err(io::Error::last_os_error());
            }
            offset += length;
        }
    }
}
