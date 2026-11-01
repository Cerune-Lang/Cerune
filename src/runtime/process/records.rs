//! 子プロセス操作の要求・結果と、本文を含まない観測記録です。
use super::Origin;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub target: &'a str,
    pub arguments: &'a [&'a str],
    pub stdin: &'a [u8],
    pub stdout_limit: usize,
    pub stderr_limit: usize,
    pub timeout: Duration,
}

/// OS接続の実測前なので既定値は設けません。転送量と期限を明示します。
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub arguments: usize,
    pub argument_bytes: usize,
    pub stdin_bytes: usize,
    pub output_bytes: usize,
    pub timeout: Duration,
    pub cleanup_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stdin,
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Validate,
    Reserve,
    Start,
    Transfer(Channel),
    CloseStdin,
    Wait,
    Cleanup,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    InvalidRequest,
    RequestLimit,
    Unconfigured,
    Unregistered,
    ConnectionDisabled,
    OutputBudget,
    AllocationFailed,
    NotFound,
    Denied,
    Resources,
    Unsupported,
    StartFailed,
    TransferFailed,
    ProgressUnknown,
    CloseFailed,
    WaitFailed,
    ZeroProgress,
    InvalidResponse,
    OutputLimit,
    Timeout,
    CleanupFailed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub code: Code,
    pub phase: Phase,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    No,
    Yes,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Code(u32),
    Signal(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupError {
    Timeout,
    Terminate,
    Reap,
    Close,
    InvalidResponse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    Complete {
        termination_requested: bool,
    },
    /// 接続は未回収の管理情報を所有し続けます。新規起動は無効になります。
    Incomplete {
        child_reaped: bool,
        handles_closed: bool,
        scope_finished: bool,
        error: CleanupError,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub origin: Origin,
    pub started: Started,
    pub stdin_requested: usize,
    pub stdin_accepted: usize,
    pub stdout_limit: usize,
    pub stderr_limit: usize,
    pub timeout: Duration,
    pub stdout_read: usize,
    pub stderr_read: usize,
    pub stdout_retained: usize,
    pub stderr_retained: usize,
    /// falseなら転送数は確認済みの下限です。進行中・不明・不正な通知を含みます。
    pub progress_known: bool,
    pub exit: Option<Exit>,
    pub failure: Option<Failure>,
    pub cleanup: Option<Cleanup>,
    /// 接続の生の回収報告です。期限などを判定したcleanupと分離します。
    pub cleanup_reported: Option<Cleanup>,
}
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub report: Report,
    pub transfers: Option<super::transfers::Snapshot>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl Outcome {
    /// 子の終了コードが0かどうかではなく、操作を回収できたかを返します。
    pub fn completed(&self) -> bool {
        self.report.failure.is_none()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offered {
    pub stdin: usize,
    pub stdout: usize,
    pub stderr: usize,
    pub close_stdin: bool,
    pub wait_for_exit: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub phase: Phase,
    pub report: Report,
    pub transfers: Option<super::transfers::Snapshot>,
    pub offered: Option<Offered>,
}
