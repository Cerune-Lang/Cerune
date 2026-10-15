//! 上限付き行入力の共通接続です。言語命令・実stdinへの接続は別途行います。
//! 読取り結果と、最初の異常で停止する方針を分離します。
use std::io::{self, Read};

pub use super::output::Origin;

pub const MAX_LINE_BYTES: i64 = i32::MAX as i64;

/// 初版の通常結果です。空行にもLFまたはCRLFが含まれます。
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    Text(String),
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    InvalidLimit,
    Unconfigured,
    BudgetExceeded,
    AllocationFailed,
    ReadFailed,
    InvalidResponse,
    InvalidUtf8,
    LimitExceeded,
}
impl Code {
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidLimit => "invalid-limit",
            Self::Unconfigured => "unconfigured",
            Self::BudgetExceeded => "budget-exceeded",
            Self::AllocationFailed => "allocation-failed",
            Self::ReadFailed => "read-failed",
            Self::InvalidResponse => "invalid-response",
            Self::InvalidUtf8 => "invalid-utf8",
            Self::LimitExceeded => "limit-exceeded",
        }
    }
}

/// 本文やOSのエラー文字列を持たず、失敗した操作の出自と実消費を保持します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub code: Code,
    pub limit: i64,
    pub consumed: usize,
    /// falseの場合、契約外応答の前までの消費数だけが確認できています。
    pub consumption_known: bool,
    pub origin: Origin,
}
impl Failure {
    pub fn record(self) -> String {
        format!(
            "io-v1 operation=read-line code={} limit={} consumed={} consumption={} node={}{} bytes={}..{}",
            self.code.name(),
            self.limit,
            self.consumed,
            if self.consumption_known {
                "known"
            } else {
                "unknown"
            },
            self.origin.node_id.0,
            self.origin.span.source_id().record_field(),
            self.origin.span.start(),
            self.origin.span.end(),
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
pub enum Outcome {
    Line,
    End,
    Failed(Code),
}

/// 読取り専用の操作記録です。本文と実行を制御するcallbackは持ちません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub origin: Origin,
    pub limit: i64,
    pub consumed: usize,
    pub consumption_known: bool,
    pub read_calls: u64,
    pub interruptions: u64,
    pub outcome: Outcome,
}

/// 埋め込み用の読取り部品です。言語の公開入力APIや、文字列の所有管理ではありません。
/// readerは要求を超えて先読みせず、エラーではバイトを消費しない契約です。
/// 最初の失敗とEOFを保持し、停止後の接続呼出しを防ぎます。
pub struct Input<R> {
    reader: Option<R>,
    eof: bool,
    failure: Option<Failure>,
    events: Vec<Event>,
    trace_limit: usize,
    sequence: u64,
    truncated: bool,
    #[cfg(test)]
    fail_allocation: bool,
    #[cfg(test)]
    fail_record_allocation: bool,
}
impl<R: Read> Input<R> {
    pub fn new(reader: R) -> Self {
        Self::with_trace_limit(reader, 0)
    }

    /// limitは記録容量です。0では記録せず、読取りには影響しません。
    pub fn with_trace_limit(reader: R, limit: usize) -> Self {
        Self {
            reader: Some(reader),
            trace_limit: limit,
            ..Self::unconfigured()
        }
    }

    /// 未設定の接続をEOFに見せません。
    pub fn unconfigured() -> Self {
        Self {
            reader: None,
            eof: false,
            failure: None,
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
    pub fn failure(&self) -> Option<Failure> {
        self.failure
    }

    /// 暗黙の読取り・巻戻し・読み捨てをせず接続を返します。
    pub fn into_inner(self) -> Option<R> {
        self.reader
    }

    /// 上限はLF/CRを含むバイト数です。未確認のEOFには予約が必要です。
    /// available_bytesは呼出し側が管理する文字列予算の残量です。
    /// この部品は返却後のStringの寿命や他の生存文字列を課金しません。
    /// 実行器は予約から所有登録まで割当てを挟まず、完成文字列を継続して課金します。
    pub fn read_line(
        &mut self,
        max_bytes: i64,
        available_bytes: u64,
        origin: Origin,
    ) -> Result<Line, Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let mut progress = Progress::default();
        let result = self.read(max_bytes, available_bytes, &mut progress);
        let outcome = match &result {
            Ok(Line::Text(_)) => Outcome::Line,
            Ok(Line::End) => Outcome::End,
            Err(code) => Outcome::Failed(*code),
        };
        let event = Event {
            sequence: self.sequence,
            origin,
            limit: max_bytes,
            consumed: progress.consumed,
            consumption_known: progress.consumption_known,
            read_calls: progress.read_calls,
            interruptions: progress.interruptions,
            outcome,
        };
        self.sequence = self.sequence.saturating_add(1);
        self.record(event);
        result.map_err(|code| {
            let failure = Failure {
                code,
                limit: max_bytes,
                consumed: progress.consumed,
                consumption_known: progress.consumption_known,
                origin,
            };
            self.failure = Some(failure);
            failure
        })
    }

    fn read(
        &mut self,
        max_bytes: i64,
        available_bytes: u64,
        progress: &mut Progress,
    ) -> Result<Line, Code> {
        if !(1..=MAX_LINE_BYTES).contains(&max_bytes) {
            return Err(Code::InvalidLimit);
        }
        let reader = self.reader.as_mut().ok_or(Code::Unconfigured)?;
        if self.eof {
            return Ok(Line::End);
        }
        if max_bytes as u64 > available_bytes {
            return Err(Code::BudgetExceeded);
        }
        let limit = max_bytes as usize;
        #[cfg(test)]
        if std::mem::take(&mut self.fail_allocation) {
            return Err(Code::AllocationFailed);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(limit)
            .map_err(|_| Code::AllocationFailed)?;
        read_frame(reader, &mut self.eof, bytes, limit, progress)
    }

    fn record(&mut self, event: Event) {
        if self.trace_limit == 0 {
            return;
        }
        #[cfg(test)]
        if std::mem::take(&mut self.fail_record_allocation) {
            self.truncated = true;
            return;
        }
        if self.events.len() < self.trace_limit && self.events.try_reserve(1).is_ok() {
            self.events.push(event);
        } else {
            self.truncated = true;
        }
    }
}

struct Progress {
    consumed: usize,
    consumption_known: bool,
    read_calls: u64,
    interruptions: u64,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            consumed: 0,
            consumption_known: true,
            read_calls: 0,
            interruptions: 0,
        }
    }
}

/// 読取り機構は停止方針を持ちません。回復可能な別APIもこの分類を利用できます。
/// EOF以外の継続・回復は呼出し側の責務です。
fn read_frame<R: Read>(
    reader: &mut R,
    eof: &mut bool,
    mut bytes: Vec<u8>,
    limit: usize,
    progress: &mut Progress,
) -> Result<Line, Code> {
    loop {
        let mut byte = [0u8; 1];
        progress.read_calls = progress.read_calls.saturating_add(1);
        match reader.read(&mut byte) {
            Ok(0) => {
                *eof = true;
                if bytes.is_empty() {
                    return Ok(Line::End);
                }
                break;
            }
            Ok(1) => {
                progress.consumed += 1;
                if bytes.len() == limit {
                    return Err(Code::LimitExceeded);
                }
                bytes.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Ok(_) => {
                progress.consumption_known = false;
                return Err(Code::InvalidResponse);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                progress.interruptions = progress.interruptions.saturating_add(1);
            }
            Err(_) => return Err(Code::ReadFailed),
        }
    }
    // Vecの領域を移し、完成行を二重確保しません。失敗時は元の領域も解放します。
    String::from_utf8(bytes)
        .map(Line::Text)
        .map_err(|_| Code::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ir::NodeId, source::Span};
    fn origin() -> Origin {
        Origin {
            node_id: NodeId(1),
            span: Span::new(2, 3),
        }
    }
    struct NeverRead;
    impl Read for NeverRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("reservation failure must precede reading");
        }
    }

    #[test]
    fn physical_allocation_failure_is_distinct_and_sticky() {
        let mut input = Input::with_trace_limit(NeverRead, 1);
        input.fail_allocation = true;
        let failure = input
            .read_line(MAX_LINE_BYTES, u64::MAX, origin())
            .unwrap_err();
        assert_eq!(failure.code, Code::AllocationFailed);
        assert_eq!(failure.consumed, 0);
        assert_eq!(input.events()[0].read_calls, 0);
        assert_eq!(input.read_line(1, 1, origin()), Err(failure));
    }

    #[test]
    fn record_allocation_failure_does_not_change_execution() {
        let mut input = Input::with_trace_limit(&b"a\nb"[..], 10);
        input.fail_record_allocation = true;
        assert_eq!(
            input.read_line(2, 2, origin()).unwrap(),
            Line::Text("a\n".into())
        );
        assert!(input.events().is_empty());
        assert!(input.trace_truncated());
        assert_eq!(
            input.read_line(2, 2, origin()).unwrap(),
            Line::Text("b".into())
        );
        assert_eq!(input.events()[0].sequence, 1);
        assert_eq!(input.read_line(1, 0, origin()).unwrap(), Line::End);
        assert!(input.failure().is_none());
    }

    #[test]
    fn completed_string_keeps_the_reserved_allocation() {
        let mut bytes = Vec::with_capacity(10);
        let allocation = bytes.as_mut_ptr();
        let result = read_frame(
            &mut &b"hello\n"[..],
            &mut false,
            bytes,
            10,
            &mut Progress::default(),
        )
        .unwrap();
        let Line::Text(text) = result else {
            panic!("expected line")
        };
        assert_eq!(text.as_ptr(), allocation);
        assert_eq!(text, "hello\n");
    }
}
