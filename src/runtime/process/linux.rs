//! Linuxの専用監督と共通runtimeを接続します。監督の配布同梱は後続です。
use super::transfers::{CancelReply, Completion, Kind, Ticket, Transfer};
use super::{
    Backend, Batch, CallContext, Cleanup, CleanupError, Completed, Exit, Launch, Request,
    StartError, WaitError, Wake,
};
use std::{
    io,
    os::{
        fd::{AsRawFd, IntoRawFd, OwnedFd},
        unix::process::{CommandExt, ExitStatusExt},
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
mod configuration;
mod inherit;
mod signal;
pub mod supervisor;
mod wire;
pub use configuration::{ConfigError, Target};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Start,
    Transfer,
    Control,
    Reap,
    Close,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsEvent {
    pub context: Option<CallContext>,
    pub stage: Stage,
    pub ticket: Option<Ticket>,
    pub error: Option<i32>,
    pub supervisor_exit: Option<Exit>,
}
static RETAINED: AtomicUsize = AtomicUsize::new(0);
/// 最終Dropでも監督をreapできず、process終了まで管理情報を保持した接続数です。
pub fn retained_connections() -> usize {
    RETAINED.load(Ordering::Relaxed)
}
// Linux closeの失敗後に同じ数値fdを再closeしません。再利用されたfdを閉じる危険があります。
fn close(fd: &mut Option<OwnedFd>) -> io::Result<()> {
    if let Some(fd) = fd.take()
        && unsafe { libc::close(fd.into_raw_fd()) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn retryable(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
fn start_error(e: &io::Error) -> StartError {
    match e.raw_os_error() {
        Some(libc::ENOENT | libc::ENOTDIR) => StartError::NotFound,
        Some(libc::EACCES | libc::EPERM) => StartError::Denied,
        Some(libc::ENOMEM | libc::EMFILE | libc::ENFILE | libc::E2BIG) => StartError::Resources,
        Some(libc::ENOSYS | libc::ENOTSUP) => StartError::Unsupported,
        _ => StartError::Failed,
    }
}
#[derive(Default)]
struct Active {
    context: Option<CallContext>,
    child: Option<Child>,
    control: Option<OwnedFd>,
    pipes: [Option<OwnedFd>; 3],
    operations: [Option<(Transfer, Duration)>; 3],
    ready: [Option<Completed>; 3],
    cancellations: [Option<(Ticket, CancelReply)>; 3],
    exit_reported: bool,
    cleaned: bool,
    no_target: bool,
    protocol_failed: bool,
    protocol_reported: bool,
    close_failed: bool,
    cleaning: bool,
    finish_sent: bool,
    helper_success: Option<bool>,
    helper_exit: Option<Exit>,
    finished: bool,
}
pub struct Linux {
    supervisor: PathBuf,
    targets: Vec<Target>,
    epoch: Instant,
    active: Option<Box<Active>>,
    events: Vec<OsEvent>,
    trace_limit: usize,
    truncated: bool,
}
impl Linux {
    /// 監督の絶対パス・対象登録・記録上限を検査します。ファイル探索や起動はしません。
    pub fn new(
        supervisor: PathBuf,
        targets: Vec<Target>,
        trace_limit: usize,
    ) -> Result<Self, ConfigError> {
        if !configuration::path(&supervisor) {
            return Err(ConfigError::Invalid);
        }
        if trace_limit > 4096 {
            return Err(ConfigError::Capacity);
        }
        configuration::validate(&targets)?;
        Ok(Self {
            supervisor,
            targets,
            epoch: Instant::now(),
            active: Some(Box::default()),
            events: Vec::new(),
            trace_limit,
            truncated: false,
        })
    }
    fn a(&mut self) -> &mut Active {
        self.active.as_mut().unwrap()
    }
    pub fn events(&self) -> &[OsEvent] {
        &self.events
    }
    pub fn trace_truncated(&self) -> bool {
        self.truncated
    }
    pub fn has_unreclaimed_resources(&self) -> bool {
        let a = self.active.as_ref().unwrap();
        a.child.is_some()
            || a.control.is_some()
            || a.pipes.iter().any(Option::is_some)
            || (a.protocol_failed && !a.cleaned && !a.no_target)
    }
    fn record(&mut self, stage: Stage, ticket: Option<Ticket>, error: Option<i32>) {
        if self.trace_limit == 0 {
            return;
        }
        if self.events.len() == self.trace_limit || self.events.try_reserve(1).is_err() {
            self.truncated = true;
        } else {
            self.events.push(OsEvent {
                context: self.active.as_ref().and_then(|a| a.context),
                stage,
                ticket,
                error,
                supervisor_exit: if stage == Stage::Reap {
                    self.active.as_ref().and_then(|a| a.helper_exit)
                } else {
                    None
                },
            });
        }
    }
    fn close_pipes(&mut self) {
        for i in 0..3 {
            if let Err(e) = close(&mut self.a().pipes[i]) {
                self.a().close_failed = true;
                self.record(Stage::Close, None, e.raw_os_error());
            }
        }
    }
    fn disconnect(&mut self) {
        if let Err(e) = close(&mut self.a().control) {
            self.a().close_failed = true;
            self.record(Stage::Close, None, e.raw_os_error());
        }
    }
    fn protocol_error(&mut self, e: io::Error) {
        self.record(Stage::Control, None, e.raw_os_error());
        self.a().protocol_failed = true;
        // 切断で監督の有限回収を要求します。hostから身元不明のPGIDへは送信しません。
        self.disconnect();
    }
    fn reap(&mut self) -> io::Result<()> {
        if let Some(child) = &mut self.a().child
            && let Some(status) = child.try_wait()?
        {
            self.a().helper_success = Some(status.success());
            self.a().helper_exit = status
                .code()
                .map(|n| Exit::Code(n as u32))
                .or_else(|| status.signal().map(|n| Exit::Signal(n as u32)));
            self.a().child = None;
            self.record(Stage::Reap, None, None);
        }
        Ok(())
    }
    fn receive_control(&mut self, batch: &mut Batch) {
        for _ in 0..4 {
            let Some(fd) = self.a().control.as_ref().map(AsRawFd::as_raw_fd) else {
                break;
            };
            let call = self.a().context.unwrap().call.get();
            let received = wire::receive(fd);
            let result = match received {
                Ok(p) if p.call == call && p.body.is_empty() && p.fds.is_empty() => match p.kind {
                    wire::EXIT_CODE | wire::EXIT_SIGNAL if !self.a().exit_reported => {
                        self.a().exit_reported = true;
                        batch.exit = Some(Ok(if p.kind == wire::EXIT_CODE {
                            Exit::Code(p.value)
                        } else {
                            Exit::Signal(p.value)
                        }));
                        Ok(())
                    }
                    wire::CLEANED if self.a().cleaning && self.a().finish_sent && p.value == 1 => {
                        self.a().cleaned = true;
                        self.disconnect();
                        Ok(())
                    }
                    _ => Err(wire::invalid()),
                },
                Ok(_) => Err(wire::invalid()),
                Err(e) if retryable(&e) => break,
                Err(e) => Err(e),
            };
            if let Err(e) = result {
                self.protocol_error(e);
                break;
            }
        }
        if self.a().protocol_failed && !self.a().protocol_reported && batch.exit.is_none() {
            self.a().protocol_reported = true;
            self.a().exit_reported = true;
            batch.exit = Some(Err(WaitError::Failed));
        }
    }
    fn progress(&mut self, i: usize) {
        let Some((mut transfer, deadline)) = self.a().operations[i].take() else {
            return;
        };
        let mut error = None;
        let (confirmed, result) = if self.now() >= deadline {
            (0, Completion::NotSubmitted)
        } else if let Some(fd) = self.a().pipes[i].as_ref().map(AsRawFd::as_raw_fd) {
            let result = if transfer.kind() == Kind::Write {
                signal::write(fd, transfer.bytes())
            } else {
                let bytes = transfer.read_buffer().unwrap();
                let n = unsafe { libc::read(fd, bytes.as_mut_ptr().cast(), bytes.len()) };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            };
            match result {
                Ok(n) => (
                    n,
                    if n == 0 && matches!(transfer.kind(), Kind::Read(_)) {
                        Completion::Eof
                    } else {
                        Completion::Done
                    },
                ),
                Err(e) if retryable(&e) => {
                    self.a().operations[i] = Some((transfer, deadline));
                    return;
                }
                Err(e) => {
                    error = e.raw_os_error();
                    (0, Completion::Failed)
                }
            }
        } else {
            (0, Completion::NotSubmitted)
        };
        self.record(Stage::Transfer, Some(transfer.ticket()), error);
        self.a().ready[i] = Some(Completed {
            transfer,
            confirmed,
            result,
        });
    }
    fn step(&mut self) -> Batch {
        let mut batch = Batch::default();
        for i in 0..3 {
            self.progress(i);
            batch.transfers[i] = self.a().ready[i].take();
            batch.cancellations[i] = self.a().cancellations[i].take();
        }
        self.receive_control(&mut batch);
        if let Err(e) = self.reap() {
            self.protocol_error(e);
        }
        if self.a().helper_success.is_some()
            && !self.a().cleaned
            && !self.a().no_target
            && !self.a().protocol_failed
        {
            // CLEANEDは監督終了より前に送信されます。残るpacketを先に読み切ります。
            self.receive_control(&mut batch);
            if !self.a().cleaned {
                self.protocol_error(wire::invalid());
            }
        }
        if self.a().cleaning {
            if !self.a().finish_sent
                && !self.a().protocol_failed
                && !self.a().no_target
                && let Some(fd) = self.a().control.as_ref().map(AsRawFd::as_raw_fd)
            {
                match wire::send(
                    fd,
                    wire::FINISH,
                    self.a().context.unwrap().call.get(),
                    0,
                    &[],
                    &[],
                ) {
                    Ok(()) => self.a().finish_sent = true,
                    Err(e) if retryable(&e) => {}
                    Err(e) => self.protocol_error(e),
                }
            }
            if self.a().operations.iter().all(Option::is_none) && self.a().helper_success.is_some()
            {
                self.close_pipes();
                self.disconnect();
                let good = (self.a().cleaned || self.a().no_target)
                    && self.a().helper_success == Some(true)
                    && !self.a().protocol_failed
                    && !self.a().close_failed;
                batch.cleanup = Some(if good {
                    Cleanup::Complete {
                        termination_requested: !self.a().no_target,
                    }
                } else {
                    Cleanup::Incomplete {
                        child_reaped: self.a().cleaned || self.a().no_target,
                        handles_closed: !self.a().close_failed,
                        scope_finished: self.a().cleaned || self.a().no_target,
                        error: if self.a().close_failed {
                            CleanupError::Close
                        } else {
                            CleanupError::InvalidResponse
                        },
                    }
                });
                self.a().cleaning = false;
                self.a().finished = true;
            }
        }
        batch
    }
    /// 明示的な取消し・回収です。過去のOutcomeや転送量を変更しません。
    pub fn reclaim(&mut self, timeout: Duration) -> bool {
        let Some(deadline) = self.now().checked_add(timeout) else {
            return false;
        };
        for i in 0..3 {
            if let Some((op, _)) = &self.a().operations[i] {
                let ticket = op.ticket();
                self.cancel(ticket, deadline);
            }
        }
        self.begin_cleanup(true, deadline);
        while self.now() < deadline {
            let b = self.poll(deadline);
            if let Some(c) = b.cleanup {
                return matches!(c, Cleanup::Complete { .. });
            }
        }
        false
    }
}
impl Backend for Linux {
    fn now(&mut self) -> Duration {
        self.epoch.elapsed()
    }
    fn start(&mut self, r: &Request<'_>, context: CallContext, deadline: Duration) -> Launch {
        if self.has_unreclaimed_resources() {
            return Launch::Uncertain(StartError::Resources);
        }
        *self.a() = Active {
            context: Some(context),
            ..Default::default()
        };
        if self.now() >= deadline {
            return Launch::NotStarted(StartError::Failed);
        }
        let Some(target) = self.targets.iter().find(|t| t.name == r.target) else {
            return Launch::NotStarted(StartError::Unregistered);
        };
        let body = match configuration::encode(target, r.arguments) {
            Ok(b) => b,
            Err(_) => return Launch::NotStarted(StartError::Resources),
        };
        let Some(absolute) = self.epoch.checked_add(deadline) else {
            return Launch::NotStarted(StartError::Resources);
        };
        let result = (|| {
            let [parent, child] = wire::pair()?;
            let fd = child.as_raw_fd();
            let mut cmd = Command::new(&self.supervisor);
            cmd.args(["--cerune-process-supervisor-v1", &fd.to_string()])
                .env_clear()
                .current_dir("/")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            unsafe {
                cmd.pre_exec(move || inherit::prepare(fd, false));
            }
            let helper = cmd.spawn()?;
            self.a().child = Some(helper);
            self.a().control = Some(parent);
            drop(child);
            let fd = self.a().control.as_ref().unwrap().as_raw_fd();
            let call = context.call.get();
            wire::retry(fd, libc::POLLOUT, absolute, || {
                wire::send(fd, wire::HELLO, call, 0, &[], &[])
            })?;
            let hello = wire::retry(fd, libc::POLLIN, absolute, || wire::receive(fd))?;
            hello.expect(wire::HELLO, call, 0)?;
            if hello.value != 0 {
                return Err(wire::invalid());
            }
            let remaining = deadline.saturating_sub(self.now()).as_millis();
            if remaining == 0 || remaining > 86_400_000 {
                return Err(io::Error::from_raw_os_error(libc::E2BIG));
            }
            wire::retry(fd, libc::POLLOUT, absolute, || {
                wire::send(fd, wire::START, call, remaining as u32, &body, &[])
            })?;
            let p = wire::retry(fd, libc::POLLIN, absolute, || wire::receive(fd))?;
            if p.kind == wire::FAILED {
                p.expect(wire::FAILED, call, 0)?;
                self.a().no_target = true;
                return Err(io::Error::from_raw_os_error(p.value as i32));
            }
            p.expect(wire::READY, call, 3)?;
            if p.value != 0 {
                return Err(wire::invalid());
            }
            for (i, fd) in p.fds.into_iter().enumerate() {
                let raw = fd.as_raw_fd();
                self.a().pipes[i] = Some(fd);
                let mut stat: libc::stat = unsafe { std::mem::zeroed() };
                let flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
                let expected = if i == 0 {
                    libc::O_WRONLY
                } else {
                    libc::O_RDONLY
                };
                if unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC == 0
                    || flags < 0
                    || flags & libc::O_ACCMODE != expected
                    || unsafe { libc::fstat(raw, &mut stat) } < 0
                    || stat.st_mode & libc::S_IFMT != libc::S_IFIFO
                {
                    return Err(wire::invalid());
                }
                if unsafe { libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.record(Stage::Start, None, None);
                Launch::Started
            }
            Err(e) => {
                self.record(Stage::Start, None, e.raw_os_error());
                let why = start_error(&e);
                self.disconnect();
                if self.a().no_target {
                    while self.now() < deadline {
                        if self.reap().is_err() {
                            break;
                        }
                        if self.a().child.is_none() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
                if self.a().child.is_none() && !self.a().close_failed {
                    Launch::NotStarted(why)
                } else {
                    Launch::Uncertain(why)
                }
            }
        }
    }
    fn submit(&mut self, t: Transfer, deadline: Duration) {
        let i = t.kind().index();
        assert!(
            self.a().operations[i].is_none() && self.a().ready[i].is_none(),
            "one transfer per channel"
        );
        if self.now() >= deadline || self.a().cleaning || self.a().finished {
            self.a().ready[i] = Some(Completed {
                transfer: t,
                confirmed: 0,
                result: Completion::NotSubmitted,
            });
            return;
        }
        if t.kind() == Kind::CloseStdin {
            let r = close(&mut self.a().pipes[0]);
            self.record(
                Stage::Close,
                Some(t.ticket()),
                r.as_ref().err().and_then(io::Error::raw_os_error),
            );
            if r.is_err() {
                self.a().close_failed = true;
            }
            self.a().ready[0] = Some(Completed {
                transfer: t,
                confirmed: 0,
                result: if r.is_ok() {
                    Completion::Done
                } else {
                    Completion::Failed
                },
            });
        } else {
            self.a().operations[i] = Some((t, deadline));
            self.progress(i);
        }
    }
    fn cancel(&mut self, ticket: Ticket, _: Duration) {
        for i in 0..3 {
            if self.a().operations[i]
                .as_ref()
                .is_some_and(|(op, _)| op.ticket() == ticket)
            {
                let (transfer, _) = self.a().operations[i].take().unwrap();
                self.a().ready[i] = Some(Completed {
                    transfer,
                    confirmed: 0,
                    result: Completion::Cancelled,
                });
                self.a().cancellations[i] = Some((ticket, CancelReply::Requested));
                return;
            }
            if self.a().ready[i]
                .as_ref()
                .is_some_and(|c| c.transfer.ticket() == ticket)
            {
                self.a().cancellations[i] = Some((ticket, CancelReply::TooLate));
                return;
            }
        }
    }
    fn begin_cleanup(&mut self, _: bool, _: Duration) {
        self.a().cleaning = true;
    }
    fn poll(&mut self, deadline: Duration) -> Batch {
        loop {
            let mut b = self.step();
            if b.exit.is_some()
                || b.cleanup.is_some()
                || b.transfers.iter().any(Option::is_some)
                || b.cancellations.iter().any(Option::is_some)
            {
                return b;
            }
            let remaining = deadline.saturating_sub(self.now());
            if remaining.is_zero() {
                b.wake = Wake::Deadline;
                return b;
            }
            std::thread::sleep(remaining.min(Duration::from_millis(1)));
        }
    }
}
impl Drop for Linux {
    fn drop(&mut self) {
        self.disconnect();
        self.close_pipes();
        let _ = self.reap();
        let a = self.active.take().unwrap();
        // I/Oは非ブロッキングsyscall中だけbufferを参照します。保持するのは未回収監督の管理情報です。
        if a.child.is_some() {
            let _ = Box::leak(a);
            RETAINED.fetch_add(1, Ordering::Relaxed);
        }
    }
}
