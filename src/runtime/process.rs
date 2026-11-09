//! 回復可能な子プロセス操作と、その読取り専用記録です。
//! Windows/Linux実OS Backendを接続済みです。言語命令・返却後の配列所有は後続です。
pub use super::output::Origin;
use std::{num::NonZeroU64, time::Duration};
pub(crate) mod access;
mod backend;
#[cfg(target_os = "linux")]
pub mod linux;
mod records;
// 共通IR接続に先立つ返却値変換。実行命令からの利用は後続です。
#[allow(dead_code)]
pub(crate) mod report_value;
// 共通IR命令への接続前に要求・結果の所有契約を検証します。
#[allow(dead_code)]
pub(crate) mod language_values;
pub mod transfers;
#[cfg(windows)]
pub mod windows;
pub use backend::{Backend, Batch, CallContext, Completed, Launch, StartError, WaitError, Wake};
pub use records::{
    Channel, Cleanup, CleanupError, Code, Event, Exit, Failure, Limits, Offered, Outcome, Phase,
    Report, Request, ReservationError, Started,
};
use transfers::{Snapshot, State, Transfers};
const CHUNK: usize = 4096;

pub struct Processes<B> {
    backend: Option<B>,
    limits: Limits,
    disabled: bool,
    events: Vec<Event>,
    trace_limit: usize,
    sequence: u64,
    truncated: bool,
    #[cfg(test)]
    fail_allocation: bool,
    #[cfg(test)]
    fail_record_allocation: bool,
}
impl<B: Backend> Processes<B> {
    pub fn new(backend: B, limits: Limits) -> Self {
        Self::with_trace_limit(backend, limits, 0)
    }
    pub fn with_trace_limit(backend: B, limits: Limits, trace_limit: usize) -> Self {
        Self {
            backend: Some(backend),
            trace_limit,
            ..Self::unconfigured(limits)
        }
    }
    pub fn unconfigured(limits: Limits) -> Self {
        Self {
            backend: None,
            limits,
            disabled: false,
            events: Vec::new(),
            trace_limit: 0,
            sequence: 0,
            truncated: false,
            #[cfg(test)]
            fail_allocation: false,
            #[cfg(test)]
            fail_record_allocation: false,
        }
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn trace_truncated(&self) -> bool {
        self.truncated
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    /// 未回収資源があれば接続が所有し続けます。暗黙の待機・再起動は行いません。
    pub fn into_inner(self) -> Option<B> {
        self.backend
    }

    /// available_bytesは両出力と3個の転送bufferの上限です。言語の配列領域とは別です。
    pub fn run(&mut self, request: Request<'_>, available_bytes: usize, origin: Origin) -> Outcome {
        self.run_with_reservation(request, available_bytes, origin, || Ok(()))
            .0
    }

    /// 要求と転送bufferの検査後、時計取得・起動より前に返却領域を予約します。
    /// reserveは領域確保だけを行い、OS操作や観測の制御を行わない契約です。
    /// 予約後は、起動失敗や部分結果の場合も同じ所有物を呼出し元へ返します。
    /// 言語の固定結果形状など、失敗値にも必要な領域は呼出し元で先に確保します。
    pub fn run_with_reservation<R>(
        &mut self,
        request: Request<'_>,
        available_bytes: usize,
        origin: Origin,
        reserve: impl FnOnce() -> Result<R, ReservationError>,
    ) -> (Outcome, Option<R>) {
        let mut out = Outcome {
            report: Report {
                origin,
                started: Started::No,
                stdin_requested: request.stdin.len(),
                stdin_accepted: 0,
                stdout_limit: request.stdout_limit,
                stderr_limit: request.stderr_limit,
                timeout: request.timeout,
                stdout_read: 0,
                stderr_read: 0,
                stdout_retained: 0,
                stderr_retained: 0,
                progress_known: true,
                exit: None,
                failure: None,
                cleanup: None,
                cleanup_reported: None,
            },
            stdout: Vec::new(),
            stderr: Vec::new(),
            transfers: None,
        };
        match self.sequence.checked_add(1) {
            Some(n) => self.sequence = n,
            None => self.disabled = true,
        }
        self.record(Phase::Validate, &out);
        let mut reservation = None;
        if let Err(failure) = self.execute(&request, available_bytes, &mut out, || {
            reservation = Some(reserve().map_err(|error| {
                failure(
                    match error {
                        ReservationError::Capacity => Code::OutputBudget,
                        ReservationError::Allocation => Code::AllocationFailed,
                    },
                    Phase::Reserve,
                )
            })?);
            Ok(())
        }) {
            out.report.failure.get_or_insert(failure);
        }
        self.record(
            if out.report.cleanup.is_some() {
                Phase::Cleanup
            } else {
                out.report.failure.map_or(Phase::Wait, |f| f.phase)
            },
            &out,
        );
        (out, reservation)
    }
    fn execute(
        &mut self,
        request: &Request<'_>,
        available: usize,
        out: &mut Outcome,
        reserve: impl FnOnce() -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.validate(request)?;
        self.record(Phase::Reserve, out);
        #[cfg(test)]
        if std::mem::take(&mut self.fail_allocation) {
            return Err(failure(Code::AllocationFailed, Phase::Reserve));
        }
        let mut transfers = Transfers::new(
            NonZeroU64::new(self.sequence).unwrap(),
            out.report.origin,
            [
                request.stdin.len(),
                request.stdout_limit,
                request.stderr_limit,
            ],
            available,
        )
        .map_err(|code| failure(code, Phase::Reserve))?;
        reserve()?;
        let mut previous = self.backend.as_mut().unwrap().now();
        let deadline = previous
            .checked_add(request.timeout)
            .ok_or(failure(Code::InvalidRequest, Phase::Start))?;
        self.update(&transfers, out);
        self.record(Phase::Start, out);
        let launch = self.backend.as_mut().unwrap().start(
            request,
            CallContext {
                call: NonZeroU64::new(self.sequence).unwrap(),
                origin: out.report.origin,
            },
            deadline,
        );
        self.clock(deadline, &mut previous, &mut transfers, Phase::Start);
        match launch {
            Launch::NotStarted(error) => {
                transfers.stop(start_failure(error));
                self.disabled |= transfers.snapshot().protocol_failed;
                self.update(&transfers, out);
                return Err(transfers.snapshot().failure.unwrap());
            }
            Launch::Uncertain(error) => {
                out.report.started = Started::Unknown;
                transfers.stop(start_failure(error));
            }
            Launch::Started => out.report.started = Started::Yes,
        }
        loop {
            self.clock(deadline, &mut previous, &mut transfers, Phase::Wait);
            let snapshot = transfers.snapshot();
            if snapshot.failure.is_some()
                || (snapshot.progress.iter().all(|p| p.ended) && out.report.exit.is_some())
            {
                break;
            }
            self.submit_ready(request, deadline, &mut previous, &mut transfers, out);
            if transfers.snapshot().failure.is_some() {
                break;
            }
            let batch = self.backend.as_mut().unwrap().poll(deadline);
            let now = self.clock(deadline, &mut previous, &mut transfers, Phase::Wait);
            self.receive(batch, now, deadline, false, &mut transfers, out);
        }
        self.cleanup(&mut transfers, &mut previous, out);
        let settlement = transfers.finish(out.report.cleanup.unwrap());
        self.disabled |= !settlement.reusable;
        out.transfers = Some(settlement.snapshot);
        apply_snapshot(settlement.snapshot, &mut out.report);
        out.stdout = settlement.stdout;
        out.stderr = settlement.stderr;
        Ok(())
    }
    fn validate(&self, request: &Request<'_>) -> Result<(), Failure> {
        let fail = |code| failure(code, Phase::Validate);
        if self.disabled {
            return Err(fail(Code::ConnectionDisabled));
        }
        if request.target.is_empty()
            || request.target.contains('\0')
            || request.arguments.iter().any(|s| s.contains('\0'))
            || request.timeout.is_zero()
            || self.limits.cleanup_timeout.is_zero()
        {
            return Err(fail(Code::InvalidRequest));
        }
        let args = request
            .arguments
            .iter()
            .try_fold(request.target.len(), |n, s| n.checked_add(s.len()))
            .ok_or_else(|| fail(Code::RequestLimit))?;
        let total = request
            .stdout_limit
            .checked_add(request.stderr_limit)
            .ok_or_else(|| fail(Code::RequestLimit))?;
        if request.arguments.len() > self.limits.arguments
            || args > self.limits.argument_bytes
            || request.stdin.len() > self.limits.stdin_bytes
            || total > self.limits.output_bytes
            || request.timeout > self.limits.timeout
        {
            return Err(fail(Code::RequestLimit));
        }
        if self.backend.is_none() {
            return Err(fail(Code::Unconfigured));
        }
        Ok(())
    }
    fn submit_ready(
        &mut self,
        request: &Request<'_>,
        deadline: Duration,
        previous: &mut Duration,
        transfers: &mut Transfers,
        out: &mut Outcome,
    ) {
        for i in 0..3 {
            self.clock(deadline, previous, transfers, Phase::Wait);
            let snapshot = transfers.snapshot();
            if snapshot.failure.is_some() {
                break;
            }
            if snapshot.progress[i].ended
                || snapshot.operations[i].is_some_and(|op| op.state != State::Terminal)
            {
                continue;
            }
            let prepared = match i {
                0 if snapshot.progress[0].confirmed == request.stdin.len() => {
                    transfers.prepare_close_stdin()
                }
                0 => transfers.prepare_write(&request.stdin[snapshot.progress[0].confirmed..]),
                1 => transfers.prepare_read(Channel::Stdout),
                _ => transfers.prepare_read(Channel::Stderr),
            };
            let transfer = match prepared {
                Ok(transfer) => transfer,
                Err(_) => {
                    transfers.protocol_error(Phase::Wait);
                    break;
                }
            };
            let phase = transfer.kind().phase();
            // prepareはI/Oを行いません。submit直前に期限を再確認します。
            self.clock(deadline, previous, transfers, phase);
            if transfers.snapshot().failure.is_some() {
                let _ = transfers.withdraw(transfer);
                break;
            }
            if transfers.submit(transfer.ticket()).is_err() {
                let _ = transfers.withdraw(transfer);
                break;
            }
            self.update(transfers, out);
            self.record(phase, out);
            self.backend.as_mut().unwrap().submit(transfer, deadline);
            self.clock(deadline, previous, transfers, phase);
        }
    }
    fn clock(
        &mut self,
        deadline: Duration,
        previous: &mut Duration,
        transfers: &mut Transfers,
        phase: Phase,
    ) -> Duration {
        let now = self.backend.as_mut().unwrap().now();
        if now < *previous {
            transfers.protocol_error(phase);
        } else {
            *previous = now;
            if now >= deadline {
                transfers.stop(failure(Code::Timeout, phase));
            }
        }
        now
    }
    fn receive(
        &mut self,
        mut batch: Batch,
        now: Duration,
        deadline: Duration,
        cleaning: bool,
        transfers: &mut Transfers,
        out: &mut Outcome,
    ) -> Option<Cleanup> {
        let any = batch.transfers.iter().any(Option::is_some)
            || batch.cancellations.iter().any(Option::is_some)
            || batch.exit.is_some()
            || batch.cleanup.is_some();
        if (batch.wake == Wake::Deadline && now < deadline) || (batch.wake == Wake::Events && !any)
        {
            transfers.protocol_error(Phase::Wait);
        }
        // 固定長配列をその場で整列します。待機通知の配列位置に意味を持たせません。
        batch.transfers.sort_unstable_by_key(|item| {
            item.as_ref().map_or((3, u64::MAX), |c| {
                (c.transfer.kind().index(), c.transfer.ticket().sequence())
            })
        });
        for completed in batch.transfers.into_iter().flatten() {
            let phase = completed.transfer.kind().phase();
            let _ = transfers.complete(completed.transfer, completed.confirmed, completed.result);
            self.update(transfers, out);
            self.record(phase, out);
        }
        batch.cancellations.sort_unstable_by_key(|item| {
            item.map(|(ticket, _)| (ticket.call(), ticket.sequence()))
        });
        for (ticket, reply) in batch.cancellations.into_iter().flatten() {
            let _ = transfers.cancel_reply(ticket, reply);
            self.update(transfers, out);
            self.record(Phase::Cleanup, out);
        }
        if let Some(exit) = batch.exit {
            match exit {
                Ok(exit) if out.report.exit.is_none() && exit != Exit::Signal(0) => {
                    out.report.exit = Some(exit)
                }
                Ok(_) => transfers.protocol_error(Phase::Wait),
                Err(_) => {
                    if out.report.exit.is_some() {
                        transfers.protocol_error(Phase::Wait);
                    } else {
                        transfers.stop(failure(Code::WaitFailed, Phase::Wait));
                    }
                }
            }
            self.update(transfers, out);
            self.record(Phase::Wait, out);
        }
        if batch.cleanup.is_some() && !cleaning {
            transfers.protocol_error(Phase::Cleanup);
        }
        self.update(transfers, out);
        self.record(
            if cleaning {
                Phase::Cleanup
            } else {
                Phase::Wait
            },
            out,
        );
        if cleaning { batch.cleanup } else { None }
    }
    fn cleanup(&mut self, transfers: &mut Transfers, previous: &mut Duration, out: &mut Outcome) {
        let start = self.backend.as_mut().unwrap().now();
        let mut invalid_clock = start < *previous;
        if invalid_clock {
            transfers.protocol_error(Phase::Cleanup);
        }
        *previous = start.max(*previous);
        let deadline = previous
            .checked_add(self.limits.cleanup_timeout)
            .unwrap_or(*previous);
        for op in transfers.snapshot().operations.into_iter().flatten() {
            if op.state == State::Submitted {
                // 正常経路は全転送の終端後にだけここへ到達します。
                if transfers.request_cancel(op.ticket).unwrap_or(false) {
                    self.update(transfers, out);
                    self.record(Phase::Cleanup, out);
                    self.backend.as_mut().unwrap().cancel(op.ticket, deadline);
                }
            }
        }
        let terminate = transfers.snapshot().failure.is_some();
        self.backend
            .as_mut()
            .unwrap()
            .begin_cleanup(terminate, deadline);
        self.update(transfers, out);
        self.record(Phase::Cleanup, out);
        let mut reported = None;
        let mut timed_out;
        loop {
            let now = self.backend.as_mut().unwrap().now();
            if now < *previous {
                invalid_clock = true;
                transfers.protocol_error(Phase::Cleanup);
            }
            *previous = (*previous).max(now);
            if invalid_clock || now >= deadline {
                timed_out = now >= deadline;
                break;
            }
            let batch = self.backend.as_mut().unwrap().poll(deadline);
            let end = self.backend.as_mut().unwrap().now();
            if end < *previous {
                invalid_clock = true;
                transfers.protocol_error(Phase::Cleanup);
            }
            *previous = (*previous).max(end);
            timed_out = end >= deadline;
            if timed_out {
                transfers.stop(failure(Code::CleanupFailed, Phase::Cleanup));
            }
            // 後始末期限の通知も回収してから停止します。元の失敗を上書きしません。
            reported = self.receive(batch, end, deadline, true, transfers, out);
            if reported.is_some() || invalid_clock || timed_out {
                break;
            }
        }
        let cleanup = match reported {
            Some(Cleanup::Complete { .. }) if invalid_clock || timed_out => Cleanup::Incomplete {
                child_reaped: true,
                handles_closed: true,
                scope_finished: true,
                error: if invalid_clock {
                    CleanupError::InvalidResponse
                } else {
                    CleanupError::Timeout
                },
            },
            Some(cleanup) => cleanup,
            None => Cleanup::Incomplete {
                child_reaped: false,
                handles_closed: false,
                scope_finished: false,
                error: if invalid_clock {
                    CleanupError::InvalidResponse
                } else {
                    CleanupError::Timeout
                },
            },
        };
        out.report.cleanup_reported = reported;
        out.report.cleanup = Some(cleanup);
        if transfers
            .snapshot()
            .operations
            .into_iter()
            .flatten()
            .any(|op| op.cancel_requested && op.cancel_reply.is_none())
        {
            transfers.protocol_error(Phase::Cleanup);
        }
        if matches!(cleanup, Cleanup::Incomplete { .. }) {
            transfers.stop(failure(Code::CleanupFailed, Phase::Cleanup));
        }
        self.update(transfers, out);
        self.record(Phase::Cleanup, out);
    }
    fn update(&self, transfers: &Transfers, out: &mut Outcome) {
        let snapshot = transfers.snapshot();
        apply_snapshot(snapshot, &mut out.report);
        out.transfers = Some(snapshot);
    }
    fn record(&mut self, phase: Phase, out: &Outcome) {
        if self.trace_limit == 0 {
            return;
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_record_allocation) {
            self.truncated = true;
            return;
        }
        if self.events.len() < self.trace_limit && self.events.try_reserve(1).is_ok() {
            self.events.push(Event {
                sequence: self.sequence,
                phase,
                report: out.report,
                offered: None,
                transfers: out.transfers,
            });
        } else {
            self.truncated = true;
        }
    }
}
fn apply_snapshot(snapshot: Snapshot, report: &mut Report) {
    report.stdin_accepted = snapshot.progress[0].confirmed;
    report.stdout_read = snapshot.progress[1].confirmed;
    report.stderr_read = snapshot.progress[2].confirmed;
    report.stdout_retained = snapshot.progress[1].retained;
    report.stderr_retained = snapshot.progress[2].retained;
    report.progress_known = snapshot.progress.iter().all(|p| p.known);
    report.failure = snapshot.failure;
}
fn failure(code: Code, phase: Phase) -> Failure {
    Failure { code, phase }
}
fn start_failure(error: StartError) -> Failure {
    failure(
        match error {
            StartError::NotFound => Code::NotFound,
            StartError::Denied => Code::Denied,
            StartError::Resources => Code::Resources,
            StartError::Unsupported => Code::Unsupported,
            StartError::Unregistered => Code::Unregistered,
            StartError::Failed => Code::StartFailed,
        },
        Phase::Start,
    )
}
#[cfg(test)]
mod synchronous;
#[cfg(test)]
mod tests;
