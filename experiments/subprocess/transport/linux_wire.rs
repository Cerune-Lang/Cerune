//! 固定長の試作用制御フレームとSCM_RIGHTSを扱います。
use super::*;
const SIZE: usize = 24;
pub const HELLO: u16 = 1;
pub const READY: u16 = 2;
pub const FINISH: u16 = 3;
pub const EXIT: u16 = 4;
pub fn header(kind: u16, call: u64, value: u32) -> [u8; SIZE] {
    let mut bytes = [0; SIZE];
    bytes[..4].copy_from_slice(b"CSPT");
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
    bytes[6..8].copy_from_slice(&kind.to_le_bytes());
    bytes[8..16].copy_from_slice(&call.to_le_bytes());
    bytes[16..20].copy_from_slice(&(SIZE as u32).to_le_bytes());
    bytes[20..24].copy_from_slice(&value.to_le_bytes());
    bytes
}
pub fn pair() -> io::Result<[OwnedFd; 2]> {
    let mut fds = [-1; 2];
    if unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
            fds.as_mut_ptr(),
        )
    } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(fds.map(|fd| unsafe { OwnedFd::from_raw_fd(fd) }))
}
pub fn ready(fd: RawFd, events: i16, deadline: Instant) -> io::Result<()> {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "transport deadline",
            ));
        }
        let mut item = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let n = unsafe { libc::poll(&mut item, 1, left.as_millis().clamp(1, 100) as i32) };
        if n > 0 {
            return Ok(());
        }
        if n < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return Err(io::Error::last_os_error());
        }
    }
}
pub fn send(fd: RawFd, bytes: &[u8], descriptors: &[RawFd]) -> io::Result<()> {
    require(descriptors.len() <= 16, "probe send fd bound")?;
    let mut control = [0usize; 32];
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    if !descriptors.is_empty() {
        let size = std::mem::size_of_val(descriptors) as u32;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = unsafe { libc::CMSG_SPACE(size) } as usize;
        require(
            message.msg_controllen <= std::mem::size_of_val(&control),
            "control buffer bound",
        )?;
        unsafe {
            let cmsg = libc::CMSG_FIRSTHDR(&message);
            (*cmsg).cmsg_level = libc::SOL_SOCKET;
            (*cmsg).cmsg_type = libc::SCM_RIGHTS;
            (*cmsg).cmsg_len = libc::CMSG_LEN(size) as usize;
            std::ptr::copy_nonoverlapping(
                descriptors.as_ptr().cast::<u8>(),
                libc::CMSG_DATA(cmsg),
                size as usize,
            );
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let n = unsafe { libc::sendmsg(fd, &message, libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT) };
        if n >= 0 {
            return require(n as usize == bytes.len(), "short control packet");
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::WouldBlock && e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
        ready(fd, libc::POLLOUT, deadline)?;
    }
}
pub fn receive(fd: RawFd, kind: u16, call: u64, count: usize) -> io::Result<(u32, Vec<OwnedFd>)> {
    let deadline = Instant::now() + Duration::from_secs(5);
    // 切詰めフラグだけに依存せず、余分な1バイトでも過大フレームを拒否します。
    let mut bytes = [0u8; SIZE + 1];
    // usizeでcmsghdrのalignmentを確保します。最大4fdを回収してから件数を検査します。
    let mut control = [0usize; 8];
    loop {
        ready(fd, libc::POLLIN, deadline)?;
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen =
            unsafe { libc::CMSG_SPACE(3 * std::mem::size_of::<RawFd>() as u32) } as usize;
        let n = unsafe {
            libc::recvmsg(
                fd,
                &mut message,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        let mut descriptors = Vec::with_capacity(4);
        let mut unexpected = false;
        unsafe {
            let mut cmsg = libc::CMSG_FIRSTHDR(&message);
            while !cmsg.is_null() {
                if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                    let size = (*cmsg).cmsg_len - libc::CMSG_LEN(0) as usize;
                    for offset in (0..size).step_by(std::mem::size_of::<RawFd>()) {
                        let received = libc::CMSG_DATA(cmsg)
                            .add(offset)
                            .cast::<RawFd>()
                            .read_unaligned();
                        descriptors.push(OwnedFd::from_raw_fd(received));
                    }
                } else {
                    unexpected = true;
                }
                cmsg = libc::CMSG_NXTHDR(&message, cmsg);
            }
        }
        // 受信済みfdは以降のどの失敗でもOwnedFdにより閉じます。
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "control peer closed",
            ));
        }
        require(
            message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) == 0 && !unexpected,
            "truncated or unexpected control data",
        )?;
        require(n as usize == SIZE, "control size mismatch")?;
        require(
            bytes[..4] == *b"CSPT" && u16::from_le_bytes(bytes[4..6].try_into().unwrap()) == 1,
            "control version mismatch",
        )?;
        require(
            u16::from_le_bytes(bytes[6..8].try_into().unwrap()) == kind,
            "control kind mismatch",
        )?;
        require(
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()) == call && call != 0,
            "control identity mismatch",
        )?;
        require(
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()) == SIZE as u32,
            "declared control size mismatch",
        )?;
        require(descriptors.len() == count, "descriptor count mismatch")?;
        return Ok((
            u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            descriptors,
        ));
    }
}
