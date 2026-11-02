use super::{case, require};
use serde_json::json;
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
#[path = "linux_signal.rs"]
mod signal;
#[path = "linux_wire.rs"]
mod wire;

struct Guard(Child);
impl Guard {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.0.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "fixture wait"));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.wait();
        }
    }
}
fn command() -> io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}
fn worker(fd: RawFd) -> io::Result<()> {
    let socket = unsafe { OwnedFd::from_raw_fd(fd) };
    require(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == 0,
        "worker control close-on-exec",
    )?;
    wire::receive(socket.as_raw_fd(), wire::HELLO, 1, 0)?;
    let mut target = command()?;
    let mut child = Guard(
        target
            .arg("--transport-fixture")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let input = child.0.stdin.take().unwrap();
    let output = child.0.stdout.take().unwrap();
    let error = child.0.stderr.take().unwrap();
    wire::send(
        socket.as_raw_fd(),
        &wire::header(wire::READY, 1, 0),
        &[input.as_raw_fd(), output.as_raw_fd(), error.as_raw_fd()],
    )?;
    // 複製を保持するとEOFが来ないため、受渡し直後に監督側の親端を閉じます。
    drop((input, output, error));
    wire::receive(socket.as_raw_fd(), wire::FINISH, 1, 0)?;
    let status = child.wait()?;
    require(status.code() == Some(7), "fixture exit mismatch")?;
    wire::send(socket.as_raw_fd(), &wire::header(wire::EXIT, 1, 7), &[])
}
pub fn helper(args: &[String]) -> Option<io::Result<()>> {
    match args.first().map(String::as_str) {
        Some("--transport-worker") => Some((|| {
            require(args.len() == 2, "worker arguments")?;
            let fd: RawFd = args[1].parse().map_err(io::Error::other)?;
            require(fd >= 3, "invalid control descriptor")?;
            worker(fd)
        })()),
        Some("--transport-fixture") => Some((|| {
            let mut bytes = Vec::new();
            io::stdin().take(4097).read_to_end(&mut bytes)?;
            require(bytes.len() <= 4096, "fixture input bound")?;
            io::stdout().write_all(&bytes)?;
            io::stderr().write_all(b"err\0\r\n")?;
            std::process::exit(7)
        })()),
        Some("--sigpipe-fixture") => Some(signal::probe(
            args.get(1).map(String::as_str) == Some("pending"),
        )),
        _ => None,
    }
}
fn nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn collect(fd: RawFd) -> io::Result<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut out = Vec::new();
    loop {
        let mut buffer = [0u8; 64];
        let n = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if n == 0 {
            return Ok(out);
        }
        if n > 0 {
            require(out.len() + n as usize <= 4096, "fixture output bound")?;
            out.extend_from_slice(&buffer[..n as usize]);
            continue;
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::WouldBlock && e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
        wire::ready(fd, libc::POLLIN, deadline)?;
    }
}
fn roundtrip() -> io::Result<serde_json::Value> {
    let [parent, child] = wire::pair()?;
    let control = child.as_raw_fd();
    let mut cmd = command()?;
    cmd.arg("--transport-worker").arg(control.to_string());
    unsafe {
        cmd.pre_exec(move || {
            if libc::fcntl(control, libc::F_SETFD, 0) < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut supervisor = Guard(cmd.spawn()?);
    drop(child);
    wire::send(parent.as_raw_fd(), &wire::header(wire::HELLO, 1, 0), &[])?;
    let (_, fds) = wire::receive(parent.as_raw_fd(), wire::READY, 1, 3)?;
    for (fd, access) in fds
        .iter()
        .zip([libc::O_WRONLY, libc::O_RDONLY, libc::O_RDONLY])
    {
        let raw = fd.as_raw_fd();
        require(
            unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC != 0,
            "received fd not close-on-exec",
        )?;
        require(
            unsafe { libc::fcntl(raw, libc::F_GETFL) } & libc::O_ACCMODE == access,
            "descriptor direction",
        )?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        require(
            unsafe { libc::fstat(raw, &mut stat) } == 0
                && stat.st_mode & libc::S_IFMT == libc::S_IFIFO,
            "descriptor is not pipe",
        )?;
        nonblocking(raw)?;
    }
    let [input, output, error]: [OwnedFd; 3] =
        fds.try_into().map_err(|_| io::Error::other("fd count"))?;
    let bytes = "日本語\0\r\n".as_bytes();
    let n = signal::write(input.as_raw_fd(), bytes)?;
    require(n == bytes.len(), "fixture partial write")?;
    drop(input);
    let stdout = collect(output.as_raw_fd())?;
    let stderr = collect(error.as_raw_fd())?;
    require(
        stdout == bytes && stderr == b"err\0\r\n",
        "descriptor payload mismatch",
    )?;
    drop((output, error));
    wire::send(parent.as_raw_fd(), &wire::header(wire::FINISH, 1, 0), &[])?;
    let (code, _) = wire::receive(parent.as_raw_fd(), wire::EXIT, 1, 0)?;
    require(
        code == 7 && supervisor.wait()?.success(),
        "supervisor completion",
    )?;
    Ok(
        json!({"accepted":n,"stdout_read":stdout.len(),"stderr_read":stderr.len(),
        "eof_confirmed":true,"received_fds":3,"cloexec":true,"exit":code,"supervisor_reaped":true}),
    )
}
fn fd_count() -> io::Result<usize> {
    Ok(std::fs::read_dir("/proc/self/fd")?.count())
}
fn reject_packets() -> io::Result<serde_json::Value> {
    let [a, b] = wire::pair()?;
    let file = File::open("/dev/null")?;
    let before = fd_count()?;
    let mut rejected = 0;
    for mode in 0..8 {
        let mut bytes = wire::header(wire::READY, 1, 0).to_vec();
        let descriptors = vec![
            file.as_raw_fd();
            if mode == 5 {
                16
            } else if mode == 4 {
                4
            } else {
                3
            }
        ];
        match mode {
            0 => bytes[4] = 2,
            1 => bytes[8] = 2,
            2 => bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes()),
            3 => bytes.truncate(8),
            6 => bytes.extend_from_slice(&[0; 128]),
            7 => bytes[6] = wire::HELLO as u8,
            _ => {}
        }
        wire::send(a.as_raw_fd(), &bytes, &descriptors)?;
        require(
            wire::receive(b.as_raw_fd(), wire::READY, 1, 3).is_err(),
            &format!("invalid packet accepted in fixture {mode}"),
        )?;
        require(fd_count()? == before, "rejected packet leaked fd")?;
        rejected += 1;
    }
    Ok(json!({"rejected":rejected,"descriptor_count_unchanged":true}))
}
pub fn run(log: &mut File) -> io::Result<()> {
    case(log, "descriptor-roundtrip", roundtrip)?;
    case(log, "invalid-control-packets", reject_packets)?;
    case(log, "duplicate-control-identity", || {
        let [a, b] = wire::pair()?;
        wire::send(a.as_raw_fd(), &wire::header(wire::HELLO, 1, 0), &[])?;
        wire::receive(b.as_raw_fd(), wire::HELLO, 1, 0)?;
        wire::send(a.as_raw_fd(), &wire::header(wire::HELLO, 1, 0), &[])?;
        require(
            wire::receive(b.as_raw_fd(), wire::HELLO, 2, 0).is_err(),
            "old request accepted",
        )?;
        Ok(json!({"stale_request_rejected":true}))
    })?;
    case(log, "control-peer-closed", || {
        let [a, b] = wire::pair()?;
        drop(a);
        require(
            wire::receive(b.as_raw_fd(), wire::HELLO, 1, 0)
                .unwrap_err()
                .kind()
                == io::ErrorKind::UnexpectedEof,
            "closed peer not detected",
        )?;
        Ok(json!({"closure_detected":true}))
    })?;
    for pending in [false, true] {
        case(
            log,
            if pending {
                "sigpipe-pending-preserved"
            } else {
                "sigpipe-default-handler"
            },
            || {
                let mut child = Guard(
                    command()?
                        .args([
                            "--sigpipe-fixture",
                            if pending { "pending" } else { "empty" },
                        ])
                        .spawn()?,
                );
                require(child.wait()?.success(), "SIGPIPE fixture failed")?;
                Ok(
                    json!({"default_handler_preserved":true,"thread_mask_restored":true,"existing_pending_preserved":pending,"error":"EPIPE"}),
                )
            },
        )?;
    }
    println!("subprocess transport: 6 Linux cases passed");
    Ok(())
}
