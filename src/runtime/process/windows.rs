//! Windows実OS接続の初版です。言語API・CLI・生成経路にはまだ公開していません。
//! 最後の所有者を破棄しても終端未確認資源は解放しません。明示reclaimを優先してください。
use super::transfers::{CancelReply, Completion, Kind, Ticket, Transfer};
use super::{
    Backend, Batch, CallContext, Cleanup, CleanupError, Completed, Launch, Request, StartError,
    WaitError, Wake,
};
use std::{
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle},
    },
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
mod configuration;
mod pipe;
mod scope;
use configuration::Prepared;
pub use configuration::{ConfigError, Target};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: pipe::Handle) -> i32;
}
fn close(handle: &mut Option<OwnedHandle>) -> bool {
    let Some(owned) = handle.take() else {
        return true;
    };
    let raw = owned.into_raw_handle();
    if unsafe { CloseHandle(raw) } != 0 {
        true
    } else {
        *handle = Some(unsafe { OwnedHandle::from_raw_handle(raw) });
        false
    }
}
static RETAINED: AtomicUsize = AtomicUsize::new(0);
/// 最終所有者のDropで、終端未確認のためprocess終了まで保持した接続数です。
pub fn retained_connections() -> usize {
    RETAINED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Start,
    Transfer,
    Query,
    Cancel,
    Terminate,
    Reap,
    Close,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsEvent {
    pub context: Option<CallContext>,
    pub stage: Stage,
    pub ticket: Option<Ticket>,
    pub error: Option<i32>,
}
#[derive(Default)]
struct Active {
    context: Option<CallContext>,
    scope: Option<scope::Scope>,
    pipes: [Option<OwnedHandle>; 3],
    operations: [Option<pipe::Flight>; 3],
    ready: [Option<Completed>; 3],
    cancellations: [Option<(Ticket, CancelReply)>; 3],
    exit_reported: bool,
    cleaning: bool,
    termination_requested: bool,
    cleanup_error: Option<CleanupError>,
    finished: bool,
}
impl Active {
    fn pending(&self) -> bool {
        self.operations.iter().flatten().any(pipe::Flight::pending)
    }
    fn close_pipes(&mut self) -> bool {
        let mut ok = true;
        for p in &mut self.pipes {
            ok &= close(p);
        }
        ok
    }
}
pub struct Windows {
    targets: Vec<Prepared>,
    epoch: Instant,
    active: Option<Box<Active>>,
    events: Vec<OsEvent>,
    trace_limit: usize,
    truncated: bool,
}
impl Windows {
    /// 絶対実行パス・絶対cwd・明示環境を起動前に検査します。暗黙の探索・継承はしません。
    pub fn new(targets: Vec<Target>, trace_limit: usize) -> Result<Self, ConfigError> {
        if trace_limit > 4096 {
            return Err(ConfigError::Capacity);
        }
        Ok(Self {
            targets: configuration::prepare(targets)?,
            epoch: Instant::now(),
            active: Some(Box::default()),
            events: Vec::new(),
            trace_limit,
            truncated: false,
        })
    }
    pub fn events(&self) -> &[OsEvent] {
        &self.events
    }
    pub fn trace_truncated(&self) -> bool {
        self.truncated
    }
    pub fn has_unreclaimed_resources(&self) -> bool {
        let a = self.active.as_ref().unwrap();
        a.scope.is_some()
            || a.pipes.iter().any(Option::is_some)
            || a.operations.iter().any(Option::is_some)
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
            });
        }
    }
    fn a(&mut self) -> &mut Active {
        self.active.as_mut().unwrap()
    }
    /// 明示的な実行制御です。過去のOutcomeを成功へ変更せず、残る資源の回収だけを試みます。
    pub fn reclaim(&mut self, timeout: Duration) -> bool {
        let Some(deadline) = self.now().checked_add(timeout) else {
            return false;
        };
        for i in 0..3 {
            if let Some(ticket) = self.a().operations[i]
                .as_ref()
                .and_then(pipe::Flight::ticket)
            {
                self.cancel(ticket, deadline);
            }
        }
        self.a().cleanup_error = None;
        self.begin_cleanup(true, deadline);
        while self.now() < deadline {
            let batch = self.poll(deadline);
            if matches!(batch.cleanup, Some(Cleanup::Complete { .. })) {
                return true;
            }
            if batch.cleanup.is_some() {
                return false;
            }
        }
        !self.has_unreclaimed_resources()
    }
    fn step(&mut self) -> Batch {
        let mut batch = Batch::default();
        for i in 0..3 {
            batch.transfers[i] = self.a().ready[i].take();
            batch.cancellations[i] = self.a().cancellations[i].take();
            if let Some(op) = &mut self.a().operations[i]
                && let Some((done, error)) = op.poll()
            {
                let ticket = done.transfer.ticket();
                let closed = op.close();
                if closed {
                    self.a().operations[i] = None;
                } else {
                    self.a().cleanup_error = Some(CleanupError::Close);
                }
                self.record(Stage::Transfer, Some(ticket), error);
                batch.transfers[i] = Some(done);
            }
            // 終端後のevent閉鎖だけが失敗した場合も、明示reclaimで再試行できます。
            if let Some(op) = &mut self.a().operations[i]
                && !op.pending()
                && op.ticket().is_none()
            {
                if op.close() {
                    self.a().operations[i] = None;
                } else {
                    self.a().cleanup_error = Some(CleanupError::Close);
                }
            }
            if let Some(op) = &mut self.a().operations[i] {
                let ticket = op.ticket();
                if let Some(error) = op.take_query_error() {
                    self.record(Stage::Query, ticket, Some(error));
                }
            }
        }
        if !self.a().exit_reported
            && let Some(scope) = &mut self.a().scope
        {
            match scope.peek() {
                Ok(Some(exit)) => {
                    batch.exit = Some(Ok(exit));
                    self.a().exit_reported = true;
                }
                Ok(None) => {}
                Err(e) => {
                    self.record(Stage::Reap, None, e.raw_os_error());
                    batch.exit = Some(Err(WaitError::Failed));
                    self.a().exit_reported = true;
                }
            }
        }
        if self.a().cleaning {
            let collected = match self.a().scope.as_mut() {
                Some(scope) => scope.collect(),
                None => Ok(true),
            };
            let scope_finished = match collected {
                Ok(done) => done,
                Err(e) => {
                    self.record(Stage::Reap, None, e.raw_os_error());
                    self.a().cleanup_error = Some(CleanupError::Reap);
                    false
                }
            };
            if scope_finished && !self.a().pending() {
                if !self.a().close_pipes() {
                    self.a().cleanup_error = Some(CleanupError::Close);
                }
                if let Some(scope) = &mut self.a().scope {
                    if scope.close() {
                        self.a().scope = None;
                    } else {
                        self.a().cleanup_error = Some(CleanupError::Close);
                    }
                }
                if self.a().cleanup_error.is_none() {
                    self.a().cleaning = false;
                    self.a().finished = true;
                    batch.cleanup = Some(Cleanup::Complete {
                        termination_requested: self.a().termination_requested,
                    });
                }
            }
            if let Some(error) = self.a().cleanup_error {
                batch.cleanup = Some(Cleanup::Incomplete {
                    child_reaped: self.a().scope.as_ref().is_none_or(scope::Scope::reaped),
                    handles_closed: self.a().pipes.iter().all(Option::is_none)
                        && self.a().operations.iter().all(Option::is_none)
                        && self.a().scope.is_none(),
                    scope_finished,
                    error,
                });
                self.a().cleaning = false;
            }
        }
        batch
    }
}
impl Backend for Windows {
    fn now(&mut self) -> Duration {
        self.epoch.elapsed()
    }
    fn start(&mut self, request: &Request<'_>, context: CallContext, deadline: Duration) -> Launch {
        if self.has_unreclaimed_resources() {
            return Launch::Uncertain(StartError::Resources);
        }
        *self.a() = Active {
            context: Some(context),
            ..Active::default()
        };
        if self.now() >= deadline {
            return Launch::NotStarted(StartError::Failed);
        }
        let Some(index) = self.targets.iter().position(|t| t.name == request.target) else {
            return Launch::NotStarted(StartError::Unregistered);
        };
        if request.arguments.len() > 4096
            || request
                .arguments
                .iter()
                .try_fold(0usize, |n, s| n.checked_add(s.len()))
                .is_none_or(|n| n > 16384)
            || request.arguments.iter().any(|s| s.contains('\0'))
        {
            return Launch::NotStarted(StartError::Resources);
        }
        let result = (|| {
            let mut children = Vec::with_capacity(3);
            for i in 0..3 {
                if self.now() >= deadline {
                    return Err(io::Error::from_raw_os_error(1460));
                }
                let (parent, child) = pipe::pair()?;
                self.a().pipes[i] = Some(parent);
                children.push(child);
            }
            let target = &self.targets[index];
            let stdio = children
                .try_into()
                .map_err(|_| io::Error::other("stdio count"))?;
            let scope = scope::Scope::spawn_suspended(
                &target.executable,
                request.arguments,
                &target.directory,
                &target.environment,
                stdio,
            )?;
            self.a().scope = Some(scope);
            // 期限後にはユーザーコードをresumeせず、起動済み資源を後始末へ渡します。
            if self.now() >= deadline {
                return Err(io::Error::from_raw_os_error(1460));
            }
            self.a().scope.as_ref().unwrap().resume()?;
            if self.a().scope.as_mut().unwrap().close_aux() {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        })();
        match result {
            Ok(()) => {
                self.record(Stage::Start, None, None);
                Launch::Started
            }
            Err(e) => {
                self.record(Stage::Start, None, e.raw_os_error());
                let error = match e.raw_os_error() {
                    Some(2 | 3) => StartError::NotFound,
                    Some(5) => StartError::Denied,
                    Some(8 | 14 | 1450) => StartError::Resources,
                    Some(50) => StartError::Unsupported,
                    _ => StartError::Failed,
                };
                if self.a().scope.is_none() && self.a().close_pipes() {
                    Launch::NotStarted(error)
                } else {
                    Launch::Uncertain(error)
                }
            }
        }
    }
    fn submit(&mut self, transfer: Transfer, deadline: Duration) {
        let i = transfer.kind().index();
        assert!(
            self.a().operations[i].is_none() && self.a().ready[i].is_none(),
            "one transfer per channel"
        );
        let complete = |transfer, result| Completed {
            transfer,
            confirmed: 0,
            result,
        };
        if self.now() >= deadline || self.a().cleaning || self.a().finished {
            self.a().ready[i] = Some(complete(transfer, Completion::NotSubmitted));
            return;
        }
        if transfer.kind() == Kind::CloseStdin {
            let result = if close(&mut self.a().pipes[0]) {
                Completion::Done
            } else {
                Completion::Failed
            };
            self.record(
                Stage::Close,
                Some(transfer.ticket()),
                if result == Completion::Done {
                    None
                } else {
                    io::Error::last_os_error().raw_os_error()
                },
            );
            self.a().ready[0] = Some(complete(transfer, result));
            return;
        }
        let Some(handle) = self.a().pipes[i].as_ref().map(AsRawHandle::as_raw_handle) else {
            self.a().ready[i] = Some(complete(transfer, Completion::NotSubmitted));
            return;
        };
        match pipe::Flight::start(handle, transfer) {
            Ok(op) => self.a().operations[i] = Some(op),
            Err((transfer, error)) => {
                self.record(
                    Stage::Transfer,
                    Some(transfer.ticket()),
                    error.raw_os_error(),
                );
                self.a().ready[i] = Some(complete(transfer, Completion::NotSubmitted));
            }
        }
    }
    fn poll(&mut self, deadline: Duration) -> Batch {
        loop {
            let mut batch = self.step();
            if batch.exit.is_some()
                || batch.cleanup.is_some()
                || batch.transfers.iter().any(Option::is_some)
                || batch.cancellations.iter().any(Option::is_some)
            {
                return batch;
            }
            let remaining = deadline.saturating_sub(self.now());
            if remaining.is_zero() {
                batch.wake = Wake::Deadline;
                return batch;
            }
            // 待機はpollだけで行い、共通runtimeから渡された絶対期限を延長しません。
            std::thread::sleep(remaining.min(Duration::from_millis(1)));
        }
    }
    fn cancel(&mut self, ticket: Ticket, _: Duration) {
        for i in 0..3 {
            let queued = self.a().ready[i]
                .as_ref()
                .is_some_and(|c| c.transfer.ticket() == ticket);
            let pending = self.a().operations[i]
                .as_ref()
                .is_some_and(|op| op.ticket() == Some(ticket));
            if queued || pending {
                let (reply, error) = if pending {
                    self.a().operations[i].as_mut().unwrap().cancel()
                } else {
                    (CancelReply::TooLate, None)
                };
                self.a().cancellations[i] = Some((ticket, reply));
                self.record(Stage::Cancel, Some(ticket), error);
                return;
            }
        }
    }
    fn begin_cleanup(&mut self, terminate: bool, _: Duration) {
        self.a().cleaning = true;
        if let Some(scope) = &mut self.a().scope
            && !scope.close_aux()
        {
            self.a().cleanup_error = Some(CleanupError::Close);
        }
        if let Some(scope) = &mut self.a().scope {
            let empty = scope.collect().unwrap_or(false);
            if terminate || !empty {
                self.a().termination_requested = true;
                if let Err(e) = self.a().scope.as_mut().unwrap().terminate() {
                    self.a().cleanup_error = Some(CleanupError::Terminate);
                    self.record(Stage::Terminate, None, e.raw_os_error());
                } else {
                    self.record(Stage::Terminate, None, None);
                }
            }
        }
    }
}
impl Drop for Windows {
    fn drop(&mut self) {
        let Some(mut a) = self.active.take() else {
            return;
        };
        if let Some(scope) = &mut a.scope {
            let _ = scope.terminate();
            let _ = scope.close_aux();
        }
        for op in a.operations.iter_mut().flatten() {
            let _ = op.cancel();
            let _ = op.poll();
        }
        let done = a
            .scope
            .as_mut()
            .is_none_or(|s| s.collect().unwrap_or(false));
        let mut closed = !a.pending();
        if closed {
            for op in a.operations.iter_mut().flatten() {
                closed &= op.close();
            }
            closed &= a.close_pipes();
            if done && let Some(scope) = &mut a.scope {
                closed &= scope.close();
            }
        }
        if !done || !closed {
            // 無期限waitも背景threadも作りません。OS参照中のbuffer・管理情報をprocess終了まで保持します。
            let _ = Box::leak(a);
            RETAINED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ir::NodeId, source::Span};
    use std::num::NonZeroU64;
    #[test]
    fn failed_completion_query_retains_transfer_and_final_owner() {
        let (parent, child) = pipe::pair().unwrap();
        let mut transfers = super::super::transfers::Transfers::new(
            NonZeroU64::new(1).unwrap(),
            super::super::Origin {
                node_id: NodeId(1),
                span: Span::new(0, 1),
            },
            [0, 16, 16],
            16384,
        )
        .unwrap();
        let transfer = transfers
            .prepare_read(super::super::Channel::Stdout)
            .unwrap();
        let mut op = pipe::Flight::start(parent.as_raw_handle(), transfer)
            .unwrap_or_else(|_| panic!("submit"));
        // OS問合せの失敗を注入し、進行中bufferを返却しないことを検査します。
        op.inject_query_error = Some(6);
        assert!(op.poll().is_none());
        assert!(op.pending());
        assert_eq!(op.take_query_error(), Some(6));
        let mut backend = Windows::new(vec![], 0).unwrap();
        backend.a().pipes[1] = Some(parent);
        backend.a().operations[1] = Some(op);
        let before = retained_connections();
        drop(backend);
        assert!(retained_connections() > before);
        // 意図的に約4 KiBとeventをprocess終了まで保持します。成功回収の検査ではありません。
        drop(child);
    }
}
