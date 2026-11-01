//! 実行用接続です。観測APIとは独立し、OS接続は後続で追加します。
use super::{Channel, Cleanup, Exit, Request};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    NotFound,
    Denied,
    Resources,
    Unsupported,
    Unregistered,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    Started,
    /// 起動・保持資源がないことを接続が確認した場合だけ返します。
    NotStarted(StartError),
    /// 起動状態や保持資源が不明な場合もcleanupを一度実行します。
    Uncertain(StartError),
}
pub struct Poll<'a> {
    pub stdin: &'a [u8],
    pub close_stdin: bool,
    pub stdout: &'a mut [u8],
    pub stderr: &'a mut [u8],
    pub wait_for_exit: bool,
    pub deadline: Duration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollEvent {
    Written(usize),
    StdinClosed,
    Read(Channel, usize),
    Exited(Exit),
    /// pollの待機だけが中断された場合です。転送・closeは発生していません。
    Interrupted,
    /// 期限までに転送・終了の準備ができなかった場合です。
    Deadline,
    TransferFailed(Channel),
    CloseFailed,
    WaitFailed,
}

/// 同期契約です。pollの借用bufferを返却後まで保持してはいけません。
/// 複数の進行中転送を扱うtransfers部品との接続は後続です。
/// 一つの接続は同時に一つの操作を管理します。
/// nowは単調時計です。pollは全pipeと終了を多重化し、一つの事象だけを返します。
/// 対応するbuffer/flagが無効な操作は行わず、deadlineを超えて新しい転送を始めません。
/// Read(0)はEOF、Written(0)は0進行です。エラー時の転送量は0でなければなりません。
/// cleanupは成功時にも一度呼ばれ、残る管理対象の終了・回収を担当します。
/// Incompleteを返した接続は未回収資源を保持し、Dropにも無期限の待機を持ち込みません。
/// これらは信頼する実行接続の契約であり、未信頼コードのsandboxではありません。
pub trait Backend {
    fn now(&mut self) -> Duration;
    fn start(&mut self, request: &Request<'_>, deadline: Duration) -> Launch;
    fn poll(&mut self, request: Poll<'_>) -> PollEvent;
    fn cleanup(&mut self, terminate: bool, deadline: Duration) -> Cleanup;
}
