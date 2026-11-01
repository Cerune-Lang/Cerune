//! 進行中の転送と取消しを分ける状態管理です。同期Backendへの接続は後続です。
use super::{CHUNK, Channel, Cleanup, Code, Failure, Origin, Phase};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket {
    call: NonZeroU64,
    sequence: u64,
}
impl Ticket {
    pub fn call(self) -> NonZeroU64 {
        self.call
    }
    pub fn sequence(self) -> u64 {
        self.sequence
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Write,
    Read(Channel),
    CloseStdin,
}
impl Kind {
    fn index(self) -> usize {
        match self {
            Self::Write | Self::CloseStdin => 0,
            Self::Read(c) => index(c),
        }
    }
    fn phase(self) -> Phase {
        match self {
            Self::Write => Phase::Transfer(Channel::Stdin),
            Self::Read(c) => Phase::Transfer(c),
            Self::CloseStdin => Phase::CloseStdin,
        }
    }
}
fn index(channel: Channel) -> usize {
    match channel {
        Channel::Stdin => 0,
        Channel::Stdout => 1,
        Channel::Stderr => 2,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Prepared,
    Submitted,
    Terminal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReply {
    Requested,
    TooLate,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub ticket: Ticket,
    pub kind: Kind,
    pub offered: usize,
    pub state: State,
    pub cancel_requested: bool,
    pub cancel_reply: Option<CancelReply>,
    pub completion: Option<Completion>,
    pub confirmed: usize,
}
/// 接続へ所有権を渡す固定容量bufferです。OSが参照しなくなるまで返してはいけません。
/// 移動してもbufferのアドレスは変わりません。複製・拡張APIは設けません。
#[derive(Debug)]
pub struct Transfer {
    operation: Operation,
    buffer: Vec<u8>,
}
impl Transfer {
    pub fn ticket(&self) -> Ticket {
        self.operation.ticket
    }
    pub fn kind(&self) -> Kind {
        self.operation.kind
    }
    pub fn bytes(&self) -> &[u8] {
        &self.buffer[..self.operation.offered]
    }
    pub fn read_buffer(&mut self) -> Option<&mut [u8]> {
        matches!(self.kind(), Kind::Read(_)).then(|| &mut self.buffer[..self.operation.offered])
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    Done,
    Eof,
    Failed,
    Cancelled,
    /// confirmedは確認済みの下限です。OSによるそれ以上の進行は不明です。
    ProgressUnknown,
    /// OS操作が始まっていないことを確認した場合に限ります。
    NotSubmitted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepareError {
    Stopped,
    Busy,
    Ended,
    InvalidInput,
    Exhausted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub confirmed: usize,
    pub retained: usize,
    pub known: bool,
    pub ended: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub call: NonZeroU64,
    pub origin: Origin,
    /// stdin要求量、stdout保持上限、stderr保持上限の順です。
    pub limits: [usize; 3],
    pub progress: [Progress; 3],
    pub operations: [Option<Operation>; 3],
    pub failure: Option<Failure>,
    pub protocol_failed: bool,
}
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub struct Settlement {
    pub snapshot: Snapshot,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// 接続の報告を保持します。pendingがあればCompleteでも再利用できません。
    pub cleanup: Cleanup,
    pub reusable: bool,
}

/// 1呼出しにつき1部品を作ります。callは接続内で再利用しない識別子です。
/// 各streamは一度に1操作、3stream間は独立して進められます。
pub struct Transfers {
    snapshot: Snapshot,
    buffers: [Option<Vec<u8>>; 3],
    output: [Vec<u8>; 2],
    sequence: u64,
}
impl Transfers {
    /// 起動前に出力領域と3個の転送bufferを確保します。
    /// available_bytesはバイトbufferの上限であり、言語側の配列領域とは別です。
    pub fn new(
        call: NonZeroU64,
        origin: Origin,
        limits: [usize; 3],
        available_bytes: usize,
    ) -> Result<Self, Code> {
        let required = limits[1]
            .checked_add(limits[2])
            .and_then(|n| n.checked_add(3 * CHUNK))
            .ok_or(Code::OutputBudget)?;
        if required > available_bytes {
            return Err(Code::OutputBudget);
        }
        fn allocate(size: usize, initialize: bool) -> Result<Vec<u8>, Code> {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(size)
                .map_err(|_| Code::AllocationFailed)?;
            if initialize {
                bytes.resize(size, 0);
            }
            Ok(bytes)
        }
        Ok(Self {
            snapshot: Snapshot {
                call,
                origin,
                limits,
                progress: [Progress {
                    confirmed: 0,
                    retained: 0,
                    known: true,
                    ended: false,
                }; 3],
                operations: [None; 3],
                failure: None,
                protocol_failed: false,
            },
            buffers: [
                Some(allocate(CHUNK, true)?),
                Some(allocate(CHUNK, true)?),
                Some(allocate(CHUNK, true)?),
            ],
            output: [allocate(limits[1], false)?, allocate(limits[2], false)?],
            sequence: 0,
        })
    }
    /// 副作用・時刻取得・割当てを伴わない読取り専用観測です。本文は含みません。
    pub fn snapshot(&self) -> Snapshot {
        let mut snapshot = self.snapshot;
        for (i, op) in snapshot.operations.iter().enumerate() {
            if op.is_some_and(|op| op.state == State::Submitted) {
                snapshot.progress[i].known = false;
            }
        }
        snapshot
    }
    pub fn stop(&mut self, failure: Failure) {
        self.snapshot.failure.get_or_insert(failure);
    }
    fn fail(&mut self, code: Code, phase: Phase) {
        self.stop(Failure { code, phase });
    }
    fn invalid(&mut self, phase: Phase) -> Code {
        self.snapshot.protocol_failed = true;
        self.fail(Code::InvalidResponse, phase);
        Code::InvalidResponse
    }
    fn prepare(&mut self, kind: Kind, offered: usize) -> Result<Transfer, PrepareError> {
        if self.snapshot.failure.is_some() {
            return Err(PrepareError::Stopped);
        }
        let i = kind.index();
        if self.buffers[i].is_none() {
            return Err(PrepareError::Busy);
        }
        if self.snapshot.progress[i].ended {
            return Err(PrepareError::Ended);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(PrepareError::Exhausted)?;
        let operation = Operation {
            ticket: Ticket {
                call: self.snapshot.call,
                sequence: self.sequence,
            },
            kind,
            offered,
            state: State::Prepared,
            cancel_requested: false,
            cancel_reply: None,
            completion: None,
            confirmed: 0,
        };
        self.snapshot.operations[i] = Some(operation);
        Ok(Transfer {
            operation,
            buffer: self.buffers[i].take().unwrap(),
        })
    }
    /// 未受理の入力先頭を渡します。前回の受理数だけ進めた位置を呼出し側が管理します。
    pub fn prepare_write(&mut self, remaining: &[u8]) -> Result<Transfer, PrepareError> {
        let left = self.snapshot.limits[0] - self.snapshot.progress[0].confirmed;
        if remaining.is_empty() || remaining.len() > left {
            return Err(PrepareError::InvalidInput);
        }
        let n = remaining.len().min(CHUNK);
        let mut transfer = self.prepare(Kind::Write, n)?;
        transfer.buffer[..n].copy_from_slice(&remaining[..n]);
        Ok(transfer)
    }
    pub fn prepare_read(&mut self, channel: Channel) -> Result<Transfer, PrepareError> {
        if channel == Channel::Stdin {
            return Err(PrepareError::InvalidInput);
        }
        let i = index(channel);
        let remaining = self.snapshot.limits[i] - self.output[i - 1].len();
        self.prepare(Kind::Read(channel), remaining.saturating_add(1).min(CHUNK))
    }
    pub fn prepare_close_stdin(&mut self) -> Result<Transfer, PrepareError> {
        if self.snapshot.progress[0].confirmed != self.snapshot.limits[0] {
            return Err(PrepareError::InvalidInput);
        }
        self.prepare(Kind::CloseStdin, 0)
    }
    fn find(&self, ticket: Ticket) -> Option<usize> {
        self.snapshot
            .operations
            .iter()
            .position(|op| op.is_some_and(|op| op.ticket == ticket))
    }
    /// 外部のsubmitを呼ぶ直前に記録します。以後は未送信と仮定してはいけません。
    pub fn submit(&mut self, ticket: Ticket) -> Result<(), Code> {
        if self.snapshot.failure.is_some() {
            return Err(Code::ConnectionDisabled);
        }
        let Some(i) = self.find(ticket) else {
            return Err(self.invalid(Phase::Start));
        };
        let op = self.snapshot.operations[i].as_mut().unwrap();
        if op.state != State::Prepared {
            return Err(self.invalid(Phase::Start));
        }
        op.state = State::Submitted;
        Ok(())
    }
    /// まだsubmitしていないbufferを回収します。停止後にも呼べます。
    pub fn withdraw(&mut self, transfer: Transfer) -> Result<(), Transfer> {
        let i = transfer.kind().index();
        if !self.matches(&transfer, State::Prepared) {
            self.invalid(transfer.kind().phase());
            return Err(transfer);
        }
        self.recycle(i, transfer);
        Ok(())
    }
    /// 一度だけ取消し操作を要求します。完了・受理数・buffer寿命は変えません。
    pub fn request_cancel(&mut self, ticket: Ticket) -> Result<bool, Code> {
        let Some(i) = self.find(ticket) else {
            return Err(self.invalid(Phase::Cleanup));
        };
        let op = self.snapshot.operations[i].as_mut().unwrap();
        if self.snapshot.failure.is_none() || op.state != State::Submitted {
            return Err(self.invalid(Phase::Cleanup));
        }
        let first = !op.cancel_requested;
        op.cancel_requested = true;
        Ok(first)
    }
    /// 完了通知より遅く届く取消し応答も記録できます。応答だけで転送は完了しません。
    pub fn cancel_reply(&mut self, ticket: Ticket, reply: CancelReply) -> Result<(), Code> {
        let Some(i) = self.find(ticket) else {
            return Err(self.invalid(Phase::Cleanup));
        };
        let op = self.snapshot.operations[i].as_mut().unwrap();
        if !op.cancel_requested || op.cancel_reply.is_some() {
            return Err(self.invalid(Phase::Cleanup));
        }
        op.cancel_reply = Some(reply);
        Ok(())
    }
    fn matches(&self, transfer: &Transfer, state: State) -> bool {
        self.snapshot.operations[transfer.kind().index()].is_some_and(|op| {
            op.ticket == transfer.ticket()
                && op.kind == transfer.kind()
                && op.offered == transfer.operation.offered
                && op.state == state
        })
    }
    fn recycle(&mut self, i: usize, transfer: Transfer) {
        self.snapshot.operations[i].as_mut().unwrap().state = State::Terminal;
        self.buffers[i] = Some(transfer.buffer);
    }
    /// OSがbufferを使わなくなった後の、終端通知に限ります。
    /// 失敗・取消しにも確認済み部分を添えられます。中間通知には使いません。
    pub fn complete(
        &mut self,
        transfer: Transfer,
        confirmed: usize,
        completion: Completion,
    ) -> Result<(), Code> {
        let kind = transfer.kind();
        let i = kind.index();
        let phase = kind.phase();
        if !self.matches(&transfer, State::Submitted) {
            return Err(self.invalid(phase));
        }
        let op = self.snapshot.operations[i].unwrap();
        let valid = confirmed <= op.offered
            && match completion {
                Completion::Eof => matches!(kind, Kind::Read(_)) && confirmed == 0,
                Completion::Cancelled => op.cancel_requested,
                Completion::NotSubmitted => confirmed == 0,
                _ => true,
            };
        if !valid {
            self.snapshot.progress[i].known = false;
            self.recycle(i, transfer);
            return Err(self.invalid(phase));
        }
        let Some(total) = self.snapshot.progress[i].confirmed.checked_add(confirmed) else {
            self.snapshot.progress[i].known = false;
            self.recycle(i, transfer);
            return Err(self.invalid(phase));
        };
        self.snapshot.progress[i].confirmed = total;
        let op = self.snapshot.operations[i].as_mut().unwrap();
        op.completion = Some(completion);
        op.confirmed = confirmed;
        if i != 0 {
            let retained = confirmed.min(self.snapshot.limits[i] - self.output[i - 1].len());
            self.output[i - 1].extend_from_slice(&transfer.buffer[..retained]);
            self.snapshot.progress[i].retained = self.output[i - 1].len();
            if retained != confirmed {
                self.fail(Code::OutputLimit, phase);
            }
        }
        match completion {
            Completion::Done if kind == Kind::CloseStdin => self.snapshot.progress[0].ended = true,
            Completion::Done if confirmed == 0 => self.fail(Code::ZeroProgress, phase),
            Completion::Done => {}
            Completion::Eof => self.snapshot.progress[i].ended = true,
            Completion::Failed | Completion::Cancelled => {
                self.fail(
                    if kind == Kind::CloseStdin {
                        Code::CloseFailed
                    } else {
                        Code::TransferFailed
                    },
                    phase,
                );
            }
            Completion::NotSubmitted => self.fail(Code::TransferFailed, phase),
            Completion::ProgressUnknown => {
                self.snapshot.progress[i].known = false;
                self.fail(Code::ProgressUnknown, phase);
            }
        }
        self.recycle(i, transfer);
        Ok(())
    }
    /// 子・管理範囲・ハンドルの回収は接続側が担当します。
    /// 未完了bufferは接続側が所有し続け、ここでは破棄・再利用しません。
    pub fn finish(mut self, cleanup: Cleanup) -> Settlement {
        let snapshot = self.snapshot();
        let settled = snapshot
            .operations
            .iter()
            .all(|op| op.is_none_or(|op| op.state == State::Terminal));
        let known = snapshot.progress.iter().all(|p| p.known);
        let cleaned = matches!(cleanup, Cleanup::Complete { .. });
        if !settled || !known || !cleaned {
            self.fail(Code::CleanupFailed, Phase::Cleanup);
        } else if self.snapshot.failure.is_none() && !snapshot.progress.iter().all(|p| p.ended) {
            self.invalid(Phase::Cleanup);
        }
        let reusable = settled && known && cleaned && !self.snapshot.protocol_failed;
        let snapshot = self.snapshot();
        let [stdout, stderr] = self.output;
        Settlement {
            snapshot,
            stdout,
            stderr,
            cleanup,
            reusable,
        }
    }
}
#[cfg(test)]
mod tests;
