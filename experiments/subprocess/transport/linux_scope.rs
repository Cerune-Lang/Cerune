//! 監督・process group・fd受渡しを一つの呼出しで検証します。
use super::*;
#[path = "../os_types.rs"]
#[allow(dead_code)]
mod os_types;
use os_types::{Exit, Pipes};
#[path = "linux_inherit.rs"]
mod inherit;
#[path = "../linux.rs"]
#[allow(dead_code)]
mod scope;
const CLEANED: u16 = 5;
struct Supervisor {
    control: Option<OwnedFd>,
    child: Guard,
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        // 検査失敗時も、先に通信を閉じて監督自身の回収処理を待ちます。
        drop(self.control.take());
        let _ = self.child.wait();
    }
}

fn cleanup(scope: &mut scope::Scope) -> io::Result<()> {
    scope.terminate()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !scope.collect()? {
        require(Instant::now() < deadline, "scope cleanup deadline")?;
        thread::sleep(Duration::from_millis(1));
    }
    require(
        scope.evidence()["adopted_descendants_reaped"] == 1,
        "grandchild not reaped",
    )
}
fn worker(control: RawFd, sentinel: RawFd) -> io::Result<()> {
    let control = unsafe { OwnedFd::from_raw_fd(control) };
    require(
        unsafe { libc::fcntl(control.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == 0,
        "control inheritance",
    )?;
    require(
        unsafe { libc::fcntl(sentinel, libc::F_GETFD) } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::EBADF),
        "host descriptor inherited",
    )?;
    wire::receive(control.as_raw_fd(), wire::HELLO, 1, 0)?;
    scope::init()?;
    let exe = std::env::current_exe()?;
    let cwd = exe
        .parent()
        .ok_or_else(|| io::Error::other("fixture directory"))?;
    let (mut managed, pipes) =
        scope::Scope::spawn(&exe, &["--fixture".into(), "descendant".into()], cwd, &[])?;
    // 通信失敗でも後始末を通ります。leaderはgroup終了要求までreapしません。
    let exchange = (|| {
        wire::send(
            control.as_raw_fd(),
            &wire::header(wire::READY, 1, 0),
            &[
                pipes.input.as_raw_fd(),
                pipes.output.as_raw_fd(),
                pipes.error.as_raw_fd(),
            ],
        )?;
        drop(pipes);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(exit) = managed.peek()? {
                require(exit == Exit::Code(0), "leader exit")?;
                break;
            }
            require(Instant::now() < deadline, "leader observation deadline")?;
            thread::sleep(Duration::from_millis(1));
        }
        wire::send(control.as_raw_fd(), &wire::header(wire::EXIT, 1, 0), &[])?;
        wire::receive(control.as_raw_fd(), wire::FINISH, 1, 0)
    })();
    cleanup(&mut managed)?;
    if exchange.is_ok() {
        wire::send(control.as_raw_fd(), &wire::header(CLEANED, 1, 1), &[])?;
    }
    // 試作workerの0は回収検査の成功です。対象の成功や通信成功を表しません。
    Ok(())
}
pub fn helper(args: &[String]) -> Option<io::Result<()>> {
    if args.first().map(String::as_str) != Some("--managed-worker") {
        return None;
    }
    Some((|| {
        require(args.len() == 3, "managed worker arguments")?;
        worker(
            args[1].parse().map_err(io::Error::other)?,
            args[2].parse().map_err(io::Error::other)?,
        )
    })())
}
fn read_prefix(fd: RawFd, expected: &[u8]) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got = Vec::new();
    while got.len() < expected.len() {
        wire::ready(fd, libc::POLLIN, deadline)?;
        let mut bytes = [0u8; 64];
        let n = unsafe { libc::read(fd, bytes.as_mut_ptr().cast(), expected.len() - got.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if matches!(
                e.kind(),
                io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return Err(e);
        }
        require(n != 0, "early EOF")?;
        got.extend_from_slice(&bytes[..n as usize]);
    }
    require(got == expected, "partial output changed")
}
fn run_case(mode: u8) -> io::Result<serde_json::Value> {
    let host_subreaper = scope::subreaper()?;
    let group = unsafe { libc::getpgrp() };
    let [parent, child] = wire::pair()?;
    let file = File::open("/dev/null")?;
    let raw = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD, 200) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let sentinel = unsafe { OwnedFd::from_raw_fd(raw) };
    let control = child.as_raw_fd();
    let mut cmd = command()?;
    cmd.args(["--managed-worker", &control.to_string(), &raw.to_string()]);
    unsafe {
        cmd.pre_exec(move || inherit::prepare(control, mode == 2));
    }
    let mut supervisor = Supervisor {
        control: Some(parent),
        child: Guard(cmd.spawn()?),
    };
    let parent = supervisor.control.as_ref().unwrap().as_raw_fd();
    drop(child);
    wire::send(parent, &wire::header(wire::HELLO, 1, 0), &[])?;
    let (_, fds) = wire::receive(parent, wire::READY, 1, 3)?;
    let [input, output, error]: [OwnedFd; 3] =
        fds.try_into().map_err(|_| io::Error::other("fd count"))?;
    nonblocking(output.as_raw_fd())?;
    nonblocking(error.as_raw_fd())?;
    let (exit, _) = wire::receive(parent, wire::EXIT, 1, 0)?;
    require(exit == 0, "direct exit lost")?;
    read_prefix(output.as_raw_fd(), b"ready\n")?;
    read_prefix(error.as_raw_fd(), b"partial\0\r\n")?;
    let mut byte = 0u8;
    require(
        unsafe { libc::read(output.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) } == -1
            && io::Error::last_os_error().kind() == io::ErrorKind::WouldBlock,
        "descendant did not retain pipe",
    )?;
    drop(input);
    let reason = match mode {
        0 => {
            wire::send(parent, &wire::header(wire::FINISH, 1, 0), &[])?;
            let (reaped, _) = wire::receive(parent, CLEANED, 1, 0)?;
            require(reaped == 1, "cleanup evidence")?;
            "finish"
        }
        1 => "control-disconnect",
        _ => {
            let mut bytes = wire::header(wire::FINISH, 1, 0);
            bytes[4] = 2;
            wire::send(parent, &bytes, &[])?;
            "invalid-control"
        }
    };
    if mode == 1 {
        drop(supervisor.control.take());
    }
    require(
        supervisor.child.wait()?.success(),
        "supervisor reclamation failed",
    )?;
    drop(supervisor.control.take());
    require(
        collect(output.as_raw_fd())?.is_empty() && collect(error.as_raw_fd())?.is_empty(),
        "EOF or retained output",
    )?;
    require(
        scope::subreaper()? == host_subreaper && unsafe { libc::getpgrp() } == group,
        "host scope changed",
    )?;
    require(
        unsafe { libc::fcntl(sentinel.as_raw_fd(), libc::F_GETFD) } == 0,
        "host fd changed",
    )?;
    Ok(
        json!({"trigger":reason,"direct_exit":exit,"grandchild_reaped":1,"supervisor_reaped":true,
        "stdout_prefix":6,"stderr_prefix":10,"eof_confirmed":true,"host_fd_preserved_not_inherited":true,
        "host_scope_preserved":true,"forced_proc_scan":mode==2}),
    )
}
pub fn run(log: &mut File) -> io::Result<()> {
    for (mode, name) in [
        "managed-finish",
        "managed-disconnect",
        "managed-invalid-control",
    ]
    .iter()
    .enumerate()
    {
        case(log, name, || run_case(mode as u8))?;
    }
    Ok(())
}
