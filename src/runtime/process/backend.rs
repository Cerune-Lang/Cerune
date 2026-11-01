//! 所有権付き転送を共通runtimeへ接続します。実OS接続は後続です。
use super::transfers::{CancelReply, Completion, Ticket, Transfer};
use super::{Cleanup, Exit, Request};
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
    NotStarted(StartError),
    Uncertain(StartError),
}

#[derive(Debug)]
pub struct Completed {
    pub transfer: Transfer,
    pub confirmed: usize,
    pub result: Completion,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wake {
    #[default]
    Events,
    Interrupted,
    Deadline,
}
/// 一度の待機で取り出す有限の通知群です。配列位置には意味を持たせません。
/// bufferを返すのはOSが参照しなくなった終端通知だけです。
#[derive(Debug, Default)]
pub struct Batch {
    pub transfers: [Option<Completed>; 3],
    pub cancellations: [Option<(Ticket, CancelReply)>; 3],
    pub exit: Option<Result<Exit, WaitError>>,
    pub cleanup: Option<Cleanup>,
    pub wake: Wake,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitError {
    Failed,
}

/// 一つの接続で同時に一つの子プロセス操作を管理します。
/// nowは単調時計です。start以外の制御操作は待機せず、pollだけが期限まで待機します。
/// submitはbufferを所有し、提出失敗も終端通知で返します。不明なら保持し続けます。
/// cancelは一度だけ要求し、その応答と転送完了は別の通知にします。
/// begin_cleanupは起動済み/不明の呼出しで一度だけ実行し、成功時も管理範囲を回収します。
/// 後始末中もpollで転送・取消し・終了の通知を返し、最後にcleanupを返します。
/// 期限後に新規転送を開始せず、回収不能なbuffer・OS管理情報は接続が所有し続けます。
/// Dropに無期限待機を持ち込まず、観測APIからこれらの操作を呼び出しません。
pub trait Backend {
    fn now(&mut self) -> Duration;
    fn start(&mut self, request: &Request<'_>, deadline: Duration) -> Launch;
    fn submit(&mut self, transfer: Transfer, deadline: Duration);
    fn poll(&mut self, deadline: Duration) -> Batch;
    fn cancel(&mut self, ticket: Ticket, deadline: Duration);
    fn begin_cleanup(&mut self, terminate: bool, deadline: Duration);
}
