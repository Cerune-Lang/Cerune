//! 版付きの有限packetです。受信済みfdは検査失敗時も所有して閉じます。
use std::{
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    ptr,
    time::Instant,
};
pub(super) const HELLO: u16 = 1;
pub(super) const START: u16 = 2;
pub(super) const READY: u16 = 3;
pub(super) const FAILED: u16 = 4;
pub(super) const EXIT_CODE: u16 = 5;
pub(super) const EXIT_SIGNAL: u16 = 6;
pub(super) const FINISH: u16 = 7;
pub(super) const CLEANED: u16 = 8;
pub(super) const MAX: usize = 24 + super::configuration::MAX_REQUEST;
pub(super) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid supervisor packet")
}
pub(super) struct Packet {
    pub kind: u16,
    pub call: u64,
    pub value: u32,
    pub body: Vec<u8>,
    pub fds: Vec<OwnedFd>,
}
impl Packet {
    pub fn expect(&self, kind: u16, call: u64, count: usize) -> io::Result<()> {
        if self.kind != kind
            || self.call != call
            || self.fds.len() != count
            || !self.body.is_empty()
        {
            Err(invalid())
        } else {
            Ok(())
        }
    }
}
pub(super) fn pair() -> io::Result<[OwnedFd; 2]> {
    let mut fds = [-1; 2];
    if unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
            fds.as_mut_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut owned = fds.map(|fd| unsafe { OwnedFd::from_raw_fd(fd) });
    for fd in &mut owned {
        if fd.as_raw_fd() < 3 {
            let raw = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
            if raw < 0 {
                return Err(io::Error::last_os_error());
            }
            *fd = unsafe { OwnedFd::from_raw_fd(raw) };
        }
    }
    Ok(owned)
}
pub(super) fn send(
    fd: RawFd,
    kind: u16,
    call: u64,
    value: u32,
    body: &[u8],
    fds: &[RawFd],
) -> io::Result<()> {
    if call == 0 || body.len() > MAX - 24 || fds.len() > 3 {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(24 + body.len());
    bytes.extend_from_slice(b"CSRP");
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&call.to_le_bytes());
    bytes.extend_from_slice(&((24 + body.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(&value.to_le_bytes());
    bytes.extend_from_slice(body);
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    if !fds.is_empty() {
        let size = mem::size_of_val(fds) as u32;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = unsafe { libc::CMSG_SPACE(size) } as usize;
        unsafe {
            let c = libc::CMSG_FIRSTHDR(&msg);
            (*c).cmsg_level = libc::SOL_SOCKET;
            (*c).cmsg_type = libc::SCM_RIGHTS;
            (*c).cmsg_len = libc::CMSG_LEN(size) as usize;
            ptr::copy_nonoverlapping(fds.as_ptr().cast::<u8>(), libc::CMSG_DATA(c), size as usize);
        }
    }
    let n = unsafe { libc::sendmsg(fd, &msg, libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT) };
    if n < 0 {
        Err(io::Error::last_os_error())
    } else if n as usize != bytes.len() {
        Err(invalid())
    } else {
        Ok(())
    }
}
pub(super) fn receive(fd: RawFd) -> io::Result<Packet> {
    let mut bytes = vec![0u8; MAX + 1];
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: bytes.len(),
    };
    let mut msg: libc::msghdr = unsafe { mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(3 * mem::size_of::<RawFd>() as u32) } as usize;
    let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT) };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut fds = Vec::with_capacity(4);
    let mut unexpected = false;
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                let size = (*c).cmsg_len.saturating_sub(libc::CMSG_LEN(0) as usize);
                if !size.is_multiple_of(mem::size_of::<RawFd>()) {
                    unexpected = true;
                }
                for offset in
                    (0..size / mem::size_of::<RawFd>()).map(|n| n * mem::size_of::<RawFd>())
                {
                    fds.push(OwnedFd::from_raw_fd(
                        libc::CMSG_DATA(c)
                            .add(offset)
                            .cast::<RawFd>()
                            .read_unaligned(),
                    ));
                }
            } else {
                unexpected = true;
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "supervisor disconnected",
        ));
    }
    if unexpected
        || msg.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
        || !(24..=MAX).contains(&(n as usize))
    {
        return Err(invalid());
    }
    bytes.truncate(n as usize);
    let kind = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
    let call = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let size = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    if bytes[..4] != *b"CSRP"
        || bytes[4..6] != 1u16.to_le_bytes()
        || call == 0
        || size != bytes.len()
    {
        return Err(invalid());
    }
    Ok(Packet {
        kind,
        call,
        value: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        body: bytes[24..].to_vec(),
        fds,
    })
}
pub(super) fn retry<T>(
    fd: RawFd,
    events: i16,
    deadline: Instant,
    mut f: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "supervisor deadline",
            ));
        }
        match f() {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            result => return result,
        }
        let mut p = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let ms = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(10) as i32;
        if unsafe { libc::poll(&mut p, 1, ms) } < 0
            && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
        {
            return Err(io::Error::last_os_error());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_identity_and_fd_ownership() {
        let [a, b] = pair().unwrap();
        send(
            a.as_raw_fd(),
            READY,
            42,
            0,
            &[],
            &[a.as_raw_fd(), a.as_raw_fd(), a.as_raw_fd()],
        )
        .unwrap();
        let p = receive(b.as_raw_fd()).unwrap();
        p.expect(READY, 42, 3).unwrap();
        assert!(p.expect(READY, 41, 3).is_err());
        assert!(
            p.fds
                .iter()
                .all(
                    |fd| unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC
                        != 0
                )
        );
        send(a.as_raw_fd(), START, 42, 0, &[1, 2, 3], &[]).unwrap();
        assert!(
            receive(b.as_raw_fd())
                .unwrap()
                .expect(START, 42, 0)
                .is_err()
        );
        assert!(send(a.as_raw_fd(), HELLO, 0, 0, &[], &[]).is_err());
    }
}
