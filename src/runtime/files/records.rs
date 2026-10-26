//! ファイル内容・名前・OSエラー文を含まない進捗記録です。
use super::Origin;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    ReadText,
    WriteNew,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Validate,
    Reserve,
    Open,
    Read,
    Write,
    Close,
    Decode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    No,
    Yes,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    NotAttempted,
    Closed,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    InvalidLimit,
    InvalidName,
    Unconfigured,
    BudgetExceeded,
    AllocationFailed,
    NotFound,
    AlreadyExists,
    UnsupportedTarget,
    OpenFailed,
    ReadFailed,
    WriteFailed,
    CloseFailed,
    ZeroProgress,
    InvalidResponse,
    InvalidUtf8,
    LimitExceeded,
}
impl Code {
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidLimit => "invalid-limit",
            Self::InvalidName => "invalid-name",
            Self::Unconfigured => "unconfigured",
            Self::BudgetExceeded => "budget-exceeded",
            Self::AllocationFailed => "allocation-failed",
            Self::NotFound => "not-found",
            Self::AlreadyExists => "already-exists",
            Self::UnsupportedTarget => "unsupported-target",
            Self::OpenFailed => "open-failed",
            Self::ReadFailed => "read-failed",
            Self::WriteFailed => "write-failed",
            Self::CloseFailed => "close-failed",
            Self::ZeroProgress => "zero-progress",
            Self::InvalidResponse => "invalid-response",
            Self::InvalidUtf8 => "invalid-utf8",
            Self::LimitExceeded => "limit-exceeded",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Details {
    pub operation: Operation,
    pub phase: Phase,
    pub limit: i64,
    /// readではEOFまで全長が不明です。writeでは要求した本文の長さです。
    pub requested: Option<u64>,
    pub transferred: u64,
    /// falseなら契約外応答の前までに確認できた数です。
    pub progress_known: bool,
    pub created: Created,
    pub cleanup: Cleanup,
    pub origin: Origin,
}
impl Details {
    pub(super) fn new(
        operation: Operation,
        limit: i64,
        requested: Option<u64>,
        origin: Origin,
    ) -> Self {
        Self {
            operation,
            phase: Phase::Validate,
            limit,
            requested,
            transferred: 0,
            progress_known: true,
            created: Created::No,
            cleanup: Cleanup::NotAttempted,
            origin,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub code: Code,
    pub details: Details,
}
impl Failure {
    pub fn record(self) -> String {
        let d = self.details;
        let operation = match d.operation {
            Operation::ReadText => "read-text",
            Operation::WriteNew => "write-new",
        };
        let phase = match d.phase {
            Phase::Validate => "validate",
            Phase::Reserve => "reserve",
            Phase::Open => "open",
            Phase::Read => "read",
            Phase::Write => "write",
            Phase::Close => "close",
            Phase::Decode => "decode",
        };
        let requested = d
            .requested
            .map_or_else(|| "unknown".to_owned(), |n| n.to_string());
        let progress = if d.progress_known { "known" } else { "unknown" };
        let created = match d.created {
            Created::No => "no",
            Created::Yes => "yes",
            Created::Unknown => "unknown",
        };
        let cleanup = match d.cleanup {
            Cleanup::NotAttempted => "not-attempted",
            Cleanup::Closed => "closed",
            Cleanup::Failed => "failed",
        };
        format!(
            "file-v1 operation={operation} phase={phase} code={} limit={} requested={requested} transferred={} progress={progress} created={created} cleanup={cleanup} node={}{} bytes={}..{}",
            self.code.name(),
            d.limit,
            d.transferred,
            d.origin.node_id.0,
            d.origin.span.source_id().record_field(),
            d.origin.span.start(),
            d.origin.span.end()
        )
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.record())
    }
}
impl std::error::Error for Failure {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub details: Details,
    pub failure: Option<Code>,
}
