//! 実行用の出力接続と、その要求・結果の読み取り専用記録です。
use crate::{ir::NodeId, source::Span};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub node_id: NodeId,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Write,
    Flush,
}

/// 一つの出力操作の結果です。acceptedは接続先が受理したバイト数です。
/// OSの表示・永続保存を保証せず、UTF-8の途中で止まる場合もあります。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub operation: Operation,
    pub kind: io::ErrorKind,
    pub requested: usize,
    pub accepted: usize,
    pub origin: Option<Origin>,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "output {:?} failed: {:?} (accepted {} of {} bytes)",
            self.operation, self.kind, self.accepted, self.requested
        )
    }
}
impl std::error::Error for Failure {}

/// 内容を含まない記録です。実際のバイト列は明示的に選んだ出力先で取得します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub operation: Operation,
    pub origin: Option<Origin>,
    pub requested: usize,
    pub accepted: usize,
    pub failure: Option<io::ErrorKind>,
}

/// 実行前に選んだwriterへ出力します。記録には実行を制御するcallbackがありません。
/// flushは呼出し側が明示的に行います。言語のflush命令ではありません。
pub struct Output<W> {
    writer: W,
    events: Vec<Event>,
    limit: usize,
    sequence: u64,
    truncated: bool,
    failure: Option<Failure>,
}
impl<W: Write> Output<W> {
    pub fn new(writer: W) -> Self {
        Self::with_trace_limit(writer, 0)
    }

    /// limitは記録する操作数です。0では記録しません。
    pub fn with_trace_limit(writer: W, limit: usize) -> Self {
        Self {
            writer,
            events: Vec::new(),
            limit,
            sequence: 0,
            truncated: false,
            failure: None,
        }
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn trace_truncated(&self) -> bool {
        self.truncated
    }

    /// 最初のI/O失敗を保持します。失敗後に追加の書込みやflushは行いません。
    pub fn failure(&self) -> Option<Failure> {
        self.failure
    }

    /// writerを返します。暗黙にflushしたり、既に受理された内容を再送しません。
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// 埋め込み側の明示的なflushです。ソース命令ではないため出自はNoneです。
    pub fn flush(&mut self) -> Result<(), Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let result = self.writer.flush();
        self.complete(Operation::Flush, None, 0, 0, result.err().map(|e| e.kind()))
    }

    fn complete(
        &mut self,
        operation: Operation,
        origin: Option<Origin>,
        requested: usize,
        accepted: usize,
        kind: Option<io::ErrorKind>,
    ) -> Result<(), Failure> {
        let event = Event {
            sequence: self.sequence,
            operation,
            origin,
            requested,
            accepted,
            failure: kind,
        };
        self.sequence = self.sequence.saturating_add(1);
        if self.limit != 0 {
            if self.events.len() < self.limit && self.events.try_reserve(1).is_ok() {
                self.events.push(event);
            } else {
                self.truncated = true;
            }
        }
        if let Some(kind) = kind {
            let failure = Failure {
                operation,
                kind,
                requested,
                accepted,
                origin,
            };
            self.failure = Some(failure);
            Err(failure)
        } else {
            Ok(())
        }
    }
}

pub(crate) trait Sink {
    fn write(&mut self, text: &str, origin: Option<Origin>) -> Result<(), Failure>;
    fn captured(&self) -> &str {
        ""
    }
}

impl<W: Write> Sink for Output<W> {
    fn write(&mut self, text: &str, origin: Option<Origin>) -> Result<(), Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let bytes = text.as_bytes();
        let mut accepted = 0;
        let kind = loop {
            if accepted == bytes.len() {
                break None;
            }
            match self.writer.write(&bytes[accepted..]) {
                Ok(0) => break Some(io::ErrorKind::WriteZero),
                Ok(n) if n <= bytes.len() - accepted => accepted += n,
                Ok(_) => break Some(io::ErrorKind::InvalidData),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => break Some(e.kind()),
            }
        };
        self.complete(Operation::Write, origin, bytes.len(), accepted, kind)
    }
}

#[derive(Default)]
pub(crate) struct Capture(pub String);
impl Sink for Capture {
    fn write(&mut self, text: &str, _origin: Option<Origin>) -> Result<(), Failure> {
        self.0.push_str(text);
        Ok(())
    }
    fn captured(&self) -> &str {
        &self.0
    }
}
