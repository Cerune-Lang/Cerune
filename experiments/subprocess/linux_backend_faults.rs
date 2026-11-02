//! 接続先の不正・喪失を再現する検証用監督です。実際の対象は起動しません。
use std::{
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::Path,
    time::Duration,
};
fn require(ok: bool) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other("fault fixture contract"))
    }
}
fn recv(fd: i32) -> io::Result<Vec<u8>> {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut bytes = vec![0u8; 65560];
        let n = unsafe { libc::recv(fd, bytes.as_mut_ptr().cast(), bytes.len(), 0) };
        if n > 0 {
            bytes.truncate(n as usize);
            return Ok(bytes);
        }
        let e = io::Error::last_os_error();
        if n == 0 {
            return Err(io::Error::other("parent disconnected"));
        }
        if !matches!(
            e.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
        ) {
            return Err(e);
        }
        require(std::time::Instant::now() < end)?;
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn send(fd: i32, kind: u16, call: u64, value: u32, version: u16, fds: &[i32]) -> io::Result<()> {
    let mut bytes = [0u8; 24];
    bytes[..4].copy_from_slice(b"CSRP");
    bytes[4..6].copy_from_slice(&version.to_le_bytes());
    bytes[6..8].copy_from_slice(&kind.to_le_bytes());
    bytes[8..16].copy_from_slice(&call.to_le_bytes());
    bytes[16..20].copy_from_slice(&24u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&value.to_le_bytes());
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: bytes.as_mut_ptr().cast(),
        iov_len: 24,
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
            std::ptr::copy_nonoverlapping(
                fds.as_ptr().cast::<u8>(),
                libc::CMSG_DATA(c),
                size as usize,
            );
        }
    }
    require(unsafe { libc::sendmsg(fd, &msg, libc::MSG_NOSIGNAL) } == 24)
}
fn pipe() -> io::Result<[OwnedFd; 2]> {
    let mut fds = [-1; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(fds.map(|fd| unsafe { OwnedFd::from_raw_fd(fd) }))
}
pub fn helper(args: &[String]) -> Option<io::Result<()>> {
    if args.first().map(String::as_str) != Some("--cerune-process-supervisor-v1") {
        return None;
    }
    Some((|| {
        require(args.len() == 2)?;
        let fd: i32 = args[1].parse().map_err(io::Error::other)?;
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        let mode = std::env::args().next().unwrap();
        let mode = Path::new(&mode).file_name().unwrap().to_str().unwrap();
        let hello = recv(fd)?;
        require(hello.len() == 24)?;
        let call = u64::from_le_bytes(hello[8..16].try_into().unwrap());
        if mode == "bad-version" {
            send(fd, 1, call, 0, 2, &[])?;
            return Ok(());
        }
        send(fd, 1, call, 0, 1, &[])?;
        let _ = recv(fd)?;
        if mode == "disconnect-before-ready" {
            return Ok(());
        }
        let [input_r, input_w] = pipe()?;
        let [output_r, output_w] = pipe()?;
        let [error_r, error_w] = pipe()?;
        send(
            fd,
            3,
            call,
            0,
            1,
            &[
                input_w.as_raw_fd(),
                output_r.as_raw_fd(),
                error_r.as_raw_fd(),
            ],
        )?;
        drop((input_w, output_r, error_r, output_w, error_w));
        if mode == "die-after-ready" {
            std::process::exit(77);
        }
        require(mode == "delayed-cleanup")?;
        send(fd, 5, call, 0, 1, &[])?;
        let _ = recv(fd)?;
        std::thread::sleep(Duration::from_millis(250));
        send(fd, 8, call, 1, 1, &[])?;
        drop((input_r, owned));
        Ok(())
    })())
}
pub fn verify(path: &Path, supervisor: &Path) -> io::Result<Vec<serde_json::Value>> {
    use cerune_lang::{
        ir::NodeId,
        runtime::process::{
            linux::{Linux, Target},
            *,
        },
        source::Span,
    };
    use serde_json::json;
    let exe = std::env::current_exe()?;
    let target = Target {
        name: "fixture".into(),
        executable: exe.clone(),
        directory: exe.parent().unwrap().into(),
        environment: vec![],
    };
    let origin = Origin {
        node_id: NodeId(18),
        span: Span::new(50, 60),
    };
    let limits = Limits {
        arguments: 32,
        argument_bytes: 16384,
        stdin_bytes: 1 << 20,
        output_bytes: 1 << 20,
        timeout: Duration::from_secs(10),
        cleanup_timeout: Duration::from_secs(5),
    };
    let request = Request {
        target: "fixture",
        arguments: &["--fixture", "exact"],
        stdin: &[],
        stdout_limit: 64,
        stderr_limit: 64,
        timeout: Duration::from_secs(5),
    };
    let mut results = Vec::new();
    for mode in [
        "bad-version",
        "disconnect-before-ready",
        "die-after-ready",
        "delayed-cleanup",
    ] {
        let helper = path.join(mode);
        std::os::unix::fs::symlink(&exe, &helper)?;
        let mut selected = limits;
        if mode == "delayed-cleanup" {
            selected.cleanup_timeout = Duration::from_millis(10);
        }
        let mut p = Processes::new(
            Linux::new(helper, vec![target.clone()], 64).unwrap(),
            selected,
        );
        let out = p.run(request, 1 << 20, origin);
        require(out.report.failure.is_some() && p.is_disabled())?;
        require(matches!(
            out.report.cleanup,
            Some(Cleanup::Incomplete { .. })
        ))?;
        let mut b = p.into_inner().unwrap();
        if mode == "die-after-ready" {
            require(
                b.events()
                    .iter()
                    .any(|e| e.supervisor_exit == Some(Exit::Code(77)) && e.error.is_none()),
            )?;
        }
        if mode == "delayed-cleanup" {
            require(out.report.failure.unwrap().code == Code::CleanupFailed)?;
            require(b.reclaim(Duration::from_secs(5)))?;
            require(!b.has_unreclaimed_resources())?;
        } else {
            require(out.stdout.is_empty())?;
        }
        results.push(json!({"case":mode,"passed":true,"evidence":{"disabled":true,"failure":format!("{:?}",out.report.failure),"cleanup":format!("{:?}",out.report.cleanup)}}));
    }
    // 埋込みhostのscope・SIGCHLD・無関係な子のwait権・継承可能fdを維持します。
    let mut subreaper = 0i32;
    require(
        unsafe {
            libc::prctl(
                libc::PR_GET_CHILD_SUBREAPER,
                &mut subreaper as *mut i32,
                0,
                0,
                0,
            )
        } == 0,
    )?;
    let group = unsafe { libc::getpgrp() };
    let mut action: libc::sigaction = unsafe { mem::zeroed() };
    require(unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action) } == 0)?;
    let mut unrelated = std::process::Command::new(&exe)
        .args(["--fixture", "exit"])
        .spawn()?;
    let file = std::fs::File::open("/dev/null")?;
    let raw = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD, 200) };
    require(raw >= 200)?;
    let sentinel = unsafe { OwnedFd::from_raw_fd(raw) };
    let fd_arg = raw.to_string();
    let args = ["--fixture-fd", &fd_arg];
    let mut p = Processes::new(
        Linux::new(supervisor.into(), vec![target], 0).unwrap(),
        limits,
    );
    let out = p.run(
        Request {
            arguments: &args,
            ..request
        },
        1 << 20,
        origin,
    );
    require(out.completed() && out.stdout == b"not-inherited\n")?;
    require(unrelated.wait()?.code() == Some(255))?;
    let mut after = 0i32;
    let mut action_after: libc::sigaction = unsafe { mem::zeroed() };
    require(
        unsafe {
            libc::prctl(
                libc::PR_GET_CHILD_SUBREAPER,
                &mut after as *mut i32,
                0,
                0,
                0,
            )
        } == 0,
    )?;
    require(unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action_after) } == 0)?;
    require(
        after == subreaper
            && unsafe { libc::getpgrp() } == group
            && action_after.sa_sigaction == action.sa_sigaction
            && action_after.sa_flags == action.sa_flags,
    )?;
    require(unsafe { libc::fcntl(sentinel.as_raw_fd(), libc::F_GETFD) } == 0)?;
    results.push(json!({"case":"host-isolation","passed":true,"evidence":{"subreaper_unchanged":true,"group_unchanged":true,"sigchld_unchanged":true,"unrelated_child_exit":255,"host_fd_not_inherited":true}}));
    require(
        std::process::Command::new(&exe)
            .arg("--signal-check")
            .arg(supervisor)
            .status()?
            .success(),
    )?;
    results.push(json!({"case":"sigpipe-preservation","passed":true,"evidence":{"default_handler_survives":true,"thread_mask_preserved":true,"preexisting_pending_preserved":true}}));
    Ok(results)
}

// 大域handlerを変更する検査は、検証用の独立プロセスだけで行います。
pub fn signal_check(supervisor: &Path) -> io::Result<()> {
    use cerune_lang::{
        ir::NodeId,
        runtime::process::{
            linux::{Linux, Target},
            *,
        },
        source::Span,
    };
    let mut action: libc::sigaction = unsafe { mem::zeroed() };
    action.sa_sigaction = libc::SIG_DFL;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
    }
    require(unsafe { libc::sigaction(libc::SIGPIPE, &action, std::ptr::null_mut()) } == 0)?;
    let mut set: libc::sigset_t = unsafe { mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGPIPE);
    }
    let exe = std::env::current_exe()?;
    for preexisting in [false, true] {
        if preexisting {
            require(
                unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) } == 0,
            )?;
            require(unsafe { libc::pthread_kill(libc::pthread_self(), libc::SIGPIPE) } == 0)?;
        }
        let mut before: libc::sigset_t = unsafe { mem::zeroed() };
        require(
            unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut before) } == 0,
        )?;
        let target = Target {
            name: "fixture".into(),
            executable: exe.clone(),
            directory: exe.parent().unwrap().into(),
            environment: vec![],
        };
        let limits = Limits {
            arguments: 32,
            argument_bytes: 16384,
            stdin_bytes: 1 << 20,
            output_bytes: 1 << 20,
            timeout: Duration::from_secs(10),
            cleanup_timeout: Duration::from_secs(5),
        };
        let mut p = Processes::new(
            Linux::new(supervisor.into(), vec![target], 0).unwrap(),
            limits,
        );
        let bytes = vec![1u8; 128 * 1024];
        let out = p.run(
            Request {
                target: "fixture",
                arguments: &["--fixture", "early-close"],
                stdin: &bytes,
                stdout_limit: 64,
                stderr_limit: 64,
                timeout: Duration::from_secs(5),
            },
            1 << 20,
            Origin {
                node_id: NodeId(1),
                span: Span::new(0, 1),
            },
        );
        require(
            out.report
                .failure
                .is_some_and(|f| f.code == Code::TransferFailed),
        )?;
        let mut after: libc::sigset_t = unsafe { mem::zeroed() };
        let mut pending: libc::sigset_t = unsafe { mem::zeroed() };
        require(
            unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut after) } == 0,
        )?;
        require(unsafe { libc::sigpending(&mut pending) } == 0)?;
        require(
            (1..=64)
                .all(|n| unsafe { libc::sigismember(&before, n) == libc::sigismember(&after, n) }),
        )?;
        require((unsafe { libc::sigismember(&pending, libc::SIGPIPE) } == 1) == preexisting)?;
        let mut actual: libc::sigaction = unsafe { mem::zeroed() };
        require(
            unsafe { libc::sigaction(libc::SIGPIPE, std::ptr::null(), &mut actual) } == 0
                && actual.sa_sigaction == libc::SIG_DFL,
        )?;
    }
    Ok(())
}
