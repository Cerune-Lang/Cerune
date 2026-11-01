//! 回復可能な子プロセス操作と、その読取り専用記録です。
//! 言語命令・実OS接続・返却後の配列所有はまだ接続していません。
pub use super::output::Origin;
use std::time::Duration;
mod backend;
mod records;
pub mod transfers;
pub use backend::{Backend, Launch, Poll, PollEvent, StartError};
pub use records::{
    Channel, Cleanup, CleanupError, Code, Event, Exit, Failure, Limits, Offered, Outcome, Phase,
    Report, Request, Started,
};

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

    /// 接続の管理情報を所有者へ返します。暗黙の再起動・cleanupは行いません。
    pub fn into_inner(self) -> Option<B> {
        self.backend
    }

    /// available_output_bytesはこの部品のバイトbuffer上限です。
    /// 言語側の配列領域・返却後の所有管理は、将来の呼出し側で別途検査します。
    pub fn run(
        &mut self,
        request: Request<'_>,
        available_output_bytes: usize,
        origin: Origin,
    ) -> Outcome {
        self.sequence = self.sequence.saturating_add(1);
        let mut outcome = Outcome {
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
            },
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        self.record(Phase::Validate, outcome.report, None);
        if let Err(failure) = self.execute(&request, available_output_bytes, &mut outcome) {
            outcome.report.failure = Some(failure);
        }
        let phase = if outcome.report.cleanup.is_some() {
            Phase::Cleanup
        } else {
            outcome.report.failure.map_or(Phase::Wait, |f| f.phase)
        };
        self.record(phase, outcome.report, None);
        outcome
    }

    fn execute(
        &mut self,
        request: &Request<'_>,
        available: usize,
        out: &mut Outcome,
    ) -> Result<(), Failure> {
        let fail = |code| Failure {
            code,
            phase: Phase::Validate,
        };
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
        self.record(Phase::Reserve, out.report, None);
        if total > available {
            return Err(Failure {
                code: Code::OutputBudget,
                phase: Phase::Reserve,
            });
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_allocation) {
            return Err(Failure {
                code: Code::AllocationFailed,
                phase: Phase::Reserve,
            });
        }
        out.stdout
            .try_reserve_exact(request.stdout_limit)
            .and_then(|()| out.stderr.try_reserve_exact(request.stderr_limit))
            .map_err(|_| Failure {
                code: Code::AllocationFailed,
                phase: Phase::Reserve,
            })?;
        let mut now = self.backend.as_mut().unwrap().now();
        let deadline = now.checked_add(request.timeout).ok_or(Failure {
            code: Code::InvalidRequest,
            phase: Phase::Start,
        })?;
        self.record(Phase::Start, out.report, None);
        match self.backend.as_mut().unwrap().start(request, deadline) {
            Launch::NotStarted(error) => return Err(start_failure(error)),
            Launch::Uncertain(error) => {
                out.report.started = Started::Unknown;
                out.report.failure = Some(start_failure(error));
            }
            Launch::Started => {
                out.report.started = Started::Yes;
                out.report.failure = self.transfer(request, deadline, &mut now, out).err();
            }
        }
        let cleanup_now = self.backend.as_mut().unwrap().now();
        let cleanup_clock_invalid = cleanup_now < now;
        if cleanup_clock_invalid {
            out.report.failure.get_or_insert(Failure {
                code: Code::InvalidResponse,
                phase: Phase::Cleanup,
            });
        }
        let cleanup_deadline = cleanup_now
            .max(now)
            .checked_add(self.limits.cleanup_timeout)
            .unwrap_or(cleanup_now.max(now));
        let mut cleanup = self
            .backend
            .as_mut()
            .unwrap()
            .cleanup(out.report.failure.is_some(), cleanup_deadline);
        let cleanup_end = self.backend.as_mut().unwrap().now();
        if matches!(cleanup, Cleanup::Complete { .. })
            && (cleanup_clock_invalid
                || cleanup_end < cleanup_now.max(now)
                || cleanup_end >= cleanup_deadline)
        {
            cleanup = Cleanup::Incomplete {
                child_reaped: true,
                handles_closed: true,
                scope_finished: true,
                error: if cleanup_clock_invalid || cleanup_end < cleanup_now.max(now) {
                    CleanupError::InvalidResponse
                } else {
                    CleanupError::Timeout
                },
            };
        }
        out.report.cleanup = Some(cleanup);
        if matches!(cleanup, Cleanup::Incomplete { .. }) {
            self.disabled = true;
            out.report.failure.get_or_insert(Failure {
                code: Code::CleanupFailed,
                phase: Phase::Cleanup,
            });
        }
        Ok(())
    }

    fn transfer(
        &mut self,
        request: &Request<'_>,
        deadline: Duration,
        previous: &mut Duration,
        out: &mut Outcome,
    ) -> Result<(), Failure> {
        let (mut closed, mut stdout_eof, mut stderr_eof) = (false, false, false);
        let mut stdout = [0; CHUNK];
        let mut stderr = [0; CHUNK];
        loop {
            self.check_clock(deadline, previous, Phase::Wait)?;
            if closed && stdout_eof && stderr_eof && out.report.exit.is_some() {
                return Ok(());
            }
            let stdin = &request.stdin[out.report.stdin_accepted..];
            let stdin = &stdin[..stdin.len().min(CHUNK)];
            let close_stdin = !closed && stdin.is_empty();
            let stdout_len = if stdout_eof {
                0
            } else {
                (request.stdout_limit - out.stdout.len())
                    .saturating_add(1)
                    .min(CHUNK)
            };
            let stderr_len = if stderr_eof {
                0
            } else {
                (request.stderr_limit - out.stderr.len())
                    .saturating_add(1)
                    .min(CHUNK)
            };
            let offered = Offered {
                stdin: stdin.len(),
                stdout: stdout_len,
                stderr: stderr_len,
                close_stdin,
                wait_for_exit: out.report.exit.is_none(),
            };
            let event = self.backend.as_mut().unwrap().poll(Poll {
                stdin,
                close_stdin,
                stdout: &mut stdout[..stdout_len],
                stderr: &mut stderr[..stderr_len],
                wait_for_exit: out.report.exit.is_none(),
                deadline,
            });
            let phase = event_phase(event);
            let result = match event {
                PollEvent::Written(n) => {
                    if closed || stdin.is_empty() || n > stdin.len() {
                        invalid(out)
                    } else if n == 0 {
                        Err(Code::ZeroProgress)
                    } else {
                        out.report.stdin_accepted += n;
                        Ok(())
                    }
                }
                PollEvent::StdinClosed => {
                    if !close_stdin {
                        invalid(out)
                    } else {
                        closed = true;
                        Ok(())
                    }
                }
                PollEvent::Read(Channel::Stdout, n) => {
                    if stdout_eof || n > stdout_len {
                        invalid(out)
                    } else {
                        stdout_eof = n == 0;
                        retain(
                            &mut out.stdout,
                            &stdout[..n],
                            request.stdout_limit,
                            &mut out.report.stdout_read,
                            &mut out.report.stdout_retained,
                        )
                    }
                }
                PollEvent::Read(Channel::Stderr, n) => {
                    if stderr_eof || n > stderr_len {
                        invalid(out)
                    } else {
                        stderr_eof = n == 0;
                        retain(
                            &mut out.stderr,
                            &stderr[..n],
                            request.stderr_limit,
                            &mut out.report.stderr_read,
                            &mut out.report.stderr_retained,
                        )
                    }
                }
                PollEvent::Read(Channel::Stdin, _) => invalid(out),
                PollEvent::Exited(exit) => {
                    if out.report.exit.is_some() || exit == Exit::Signal(0) {
                        invalid(out)
                    } else {
                        out.report.exit = Some(exit);
                        Ok(())
                    }
                }
                PollEvent::Interrupted => Ok(()),
                PollEvent::Deadline => {
                    if self.backend.as_mut().unwrap().now() < deadline {
                        invalid(out)
                    } else {
                        Err(Code::Timeout)
                    }
                }
                PollEvent::TransferFailed(channel) => {
                    let active = match channel {
                        Channel::Stdin => !closed && !stdin.is_empty(),
                        Channel::Stdout => !stdout_eof,
                        Channel::Stderr => !stderr_eof,
                    };
                    if !active {
                        invalid(out)
                    } else {
                        Err(Code::TransferFailed)
                    }
                }
                PollEvent::CloseFailed => {
                    if !close_stdin {
                        invalid(out)
                    } else {
                        Err(Code::CloseFailed)
                    }
                }
                PollEvent::WaitFailed => {
                    if out.report.exit.is_some() {
                        invalid(out)
                    } else {
                        Err(Code::WaitFailed)
                    }
                }
            };
            // 進行を保存してから時計を検査します。期限超過が同時のI/O失敗より優先です。
            let result = self
                .check_clock(deadline, previous, phase)
                .and(result.map_err(|code| Failure { code, phase }));
            out.report.failure = result.err();
            self.record(phase, out.report, Some(offered));
            result?;
        }
    }
    fn check_clock(
        &mut self,
        deadline: Duration,
        previous: &mut Duration,
        phase: Phase,
    ) -> Result<(), Failure> {
        let now = self.backend.as_mut().unwrap().now();
        if now < *previous {
            return Err(Failure {
                code: Code::InvalidResponse,
                phase,
            });
        }
        *previous = now;
        if now >= deadline {
            return Err(Failure {
                code: Code::Timeout,
                phase,
            });
        }
        Ok(())
    }
    fn record(&mut self, phase: Phase, report: Report, offered: Option<Offered>) {
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
                report,
                offered,
            });
        } else {
            self.truncated = true;
        }
    }
}
fn invalid(out: &mut Outcome) -> Result<(), Code> {
    out.report.progress_known = false;
    Err(Code::InvalidResponse)
}
fn retain(
    into: &mut Vec<u8>,
    bytes: &[u8],
    limit: usize,
    read: &mut usize,
    retained: &mut usize,
) -> Result<(), Code> {
    *read = read.checked_add(bytes.len()).ok_or(Code::InvalidResponse)?;
    let n = bytes.len().min(limit - into.len());
    into.extend_from_slice(&bytes[..n]);
    *retained = into.len();
    if n != bytes.len() {
        Err(Code::OutputLimit)
    } else {
        Ok(())
    }
}
fn start_failure(error: StartError) -> Failure {
    let code = match error {
        StartError::NotFound => Code::NotFound,
        StartError::Denied => Code::Denied,
        StartError::Resources => Code::Resources,
        StartError::Unsupported => Code::Unsupported,
        StartError::Unregistered => Code::Unregistered,
        StartError::Failed => Code::StartFailed,
    };
    Failure {
        code,
        phase: Phase::Start,
    }
}
fn event_phase(event: PollEvent) -> Phase {
    match event {
        PollEvent::Written(_) => Phase::Transfer(Channel::Stdin),
        PollEvent::Read(channel, _) | PollEvent::TransferFailed(channel) => {
            Phase::Transfer(channel)
        }
        PollEvent::StdinClosed | PollEvent::CloseFailed => Phase::CloseStdin,
        PollEvent::Exited(_)
        | PollEvent::Interrupted
        | PollEvent::Deadline
        | PollEvent::WaitFailed => Phase::Wait,
    }
}
#[cfg(test)]
mod tests;
