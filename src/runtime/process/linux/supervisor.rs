//! 専用プロセスの入口です。埋込みhostでは呼び出さないでください。
use super::{configuration, inherit, wire};
use scope::{Exit, Scope};
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd},
    thread,
    time::{Duration, Instant},
};
#[path = "scope.rs"]
mod scope;
fn send(
    fd: i32,
    kind: u16,
    call: u64,
    value: u32,
    body: &[u8],
    fds: &[i32],
    deadline: Instant,
) -> io::Result<()> {
    wire::retry(fd, libc::POLLOUT, deadline, || {
        wire::send(fd, kind, call, value, body, fds)
    })
}
/// 専用の単一threadプロセスで、所有したSOCK_SEQPACKET接続を一度だけ処理します。
/// subreaperを設定するため、一般のアプリケーション内で呼び出すAPIではありません。
pub fn serve(control: OwnedFd) -> io::Result<()> {
    let fd = control.as_raw_fd();
    if fd < 3 {
        return Err(wire::invalid());
    }
    let mut kind = 0i32;
    let mut size = std::mem::size_of::<i32>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut kind as *mut i32).cast(),
            &mut size,
        )
    } != 0
        || kind != libc::SOCK_SEQPACKET
    {
        return Err(wire::invalid());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // 設定変更は専用監督だけに限定します。
    scope::init()?;
    let setup = Instant::now() + Duration::from_secs(5);
    let hello = wire::retry(fd, libc::POLLIN, setup, || wire::receive(fd))?;
    hello.expect(wire::HELLO, hello.call, 0)?;
    if hello.value != 0 {
        return Err(wire::invalid());
    }
    let call = hello.call;
    send(fd, wire::HELLO, call, 0, &[], &[], setup)?;
    let start = wire::retry(fd, libc::POLLIN, setup, || wire::receive(fd))?;
    if start.kind != wire::START
        || start.call != call
        || !start.fds.is_empty()
        || start.value == 0
        || start.value > 86_400_000
    {
        return Err(wire::invalid());
    }
    let (target, args) = configuration::decode(&start.body)?;
    // 監督のwatchdogは親喪失時の補助です。共通runtimeの期限判定は置き換えません。
    let watchdog =
        Instant::now() + Duration::from_millis(start.value as u64) + Duration::from_secs(5);
    let launched = Scope::spawn(
        &target.executable,
        &args,
        &target.directory,
        &target.environment,
    );
    let (mut managed, pipes) = match launched {
        Ok(pair) => pair,
        Err(e) => {
            send(
                fd,
                wire::FAILED,
                call,
                e.raw_os_error().unwrap_or(libc::EIO) as u32,
                &[],
                &[],
                setup,
            )?;
            return Ok(());
        }
    };
    let exchange = (|| {
        send(
            fd,
            wire::READY,
            call,
            0,
            &[],
            &[
                pipes.input.as_raw_fd(),
                pipes.output.as_raw_fd(),
                pipes.error.as_raw_fd(),
            ],
            watchdog,
        )?;
        drop(pipes);
        let mut reported = false;
        loop {
            match wire::receive(fd) {
                Ok(p) => {
                    p.expect(wire::FINISH, call, 0)?;
                    if p.value != 0 {
                        return Err(wire::invalid());
                    }
                    return Ok(());
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(e),
            }
            if !reported && let Some(exit) = managed.peek()? {
                let (kind, value) = match exit {
                    Exit::Code(n) => (wire::EXIT_CODE, n),
                    Exit::Signal(n) => (wire::EXIT_SIGNAL, n),
                };
                send(fd, kind, call, value, &[], &[], watchdog)?;
                reported = true;
            }
            if Instant::now() >= watchdog {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "supervisor watchdog",
                ));
            }
            thread::sleep(Duration::from_millis(1));
        }
    })();
    // 切断・不正通信も同じ有限回収を通します。数値PGIDの使用はleaderのreapより前だけです。
    managed.terminate()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !managed.collect()? {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "supervisor cleanup",
            ));
        }
        thread::sleep(Duration::from_millis(1));
    }
    if exchange.is_ok() {
        send(fd, wire::CLEANED, call, 1, &[], &[], deadline)?;
    }
    Ok(())
}
