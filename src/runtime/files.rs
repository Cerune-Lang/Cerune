//! 実行前に選んだファイル接続と、進捗・失敗の読取り専用記録です。
//! OS接続、言語命令、返却後の文字列の所有管理は呼出し側の責務です。
pub use super::output::Origin;
use std::io::{self, Read, Write};
mod directory;
pub use directory::{ConnectError, Directory, FileHandle};
mod names;
pub use names::{MAX_NAME_BYTES, validate_name};
mod records;
pub use records::{Cleanup, Code, Created, Details, Event, Failure, Operation, Phase};

pub const MAX_FILE_BYTES: i64 = i32::MAX as i64;

/// closeはhandleを消費します。エラーでも再試行・暗黙の再送をしません。
/// Read/Writeのエラーは消費0を意味し、割込みだけが再試行可能です。
pub trait Handle: Read + Write {
    fn close(self) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    NotFound,
    AlreadyExists,
    UnsupportedTarget,
    Failed,
    /// 応答が不正なため、作成が起きたか確認できません。
    InvalidResponse,
}

/// 実行用接続です。観測APIではありません。
/// 通常ファイルの検査と作業範囲の固定、WriteNewの排他的作成を実装します。
/// 成功したhandleは先頭位置から使います。通常のエラーは作成・変更なしを保証し、
/// 保証できない応答はInvalidResponseとします。失敗時の内部handle回収も接続の責務です。
pub trait Backend {
    type Handle: Handle;
    fn open(&mut self, name: &str, operation: Operation) -> Result<Self::Handle, OpenError>;
}

pub struct Files<B> {
    backend: Option<B>,
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

impl<B: Backend> Files<B> {
    pub fn new(backend: B) -> Self {
        Self::with_trace_limit(backend, 0)
    }
    pub fn with_trace_limit(backend: B, trace_limit: usize) -> Self {
        Self {
            backend: Some(backend),
            trace_limit,
            ..Self::unconfigured()
        }
    }
    pub fn unconfigured() -> Self {
        Self {
            backend: None,
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
    /// 接続を返します。close・削除・再送などの追加操作は行いません。
    pub fn into_inner(self) -> Option<B> {
        self.backend
    }

    /// available_bytesは呼出し側の文字列使用量の残量です。
    /// この部品は返却後のStringの寿命や、他の生存文字列の使用量を管理しません。
    pub fn read_text(
        &mut self,
        name: &str,
        max_bytes: i64,
        available_bytes: u64,
        origin: Origin,
    ) -> Result<String, Failure> {
        self.read_text_prepared(name, max_bytes, available_bytes, origin, |_| Ok(()))
            .map(|(text, ())| text)
    }

    /// 所有情報をopen前に準備するための実行用処理です。観測callbackではありません。
    pub(crate) fn read_text_prepared<T>(
        &mut self,
        name: &str,
        max_bytes: i64,
        available_bytes: u64,
        origin: Origin,
        prepare: impl FnOnce(usize) -> Result<T, Code>,
    ) -> Result<(String, T), Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let mut details = Details::new(Operation::ReadText, max_bytes, None, origin);
        let result = self
            .read_contents(name, available_bytes, &mut details, prepare)
            .and_then(|(bytes, prepared)| {
                details.phase = Phase::Decode;
                let text = String::from_utf8(bytes).map_err(|_| Code::InvalidUtf8)?;
                Ok((text, prepared))
            });
        self.complete(details, result)
    }

    /// Read raw bytes within the caller's available byte allowance.
    pub fn read_bytes(
        &mut self,
        name: &str,
        max_bytes: i64,
        available_bytes: u64,
        origin: Origin,
    ) -> Result<Vec<u8>, Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let mut details = Details::new(Operation::ReadBytes, max_bytes, None, origin);
        let result = self
            .read_contents(name, available_bytes, &mut details, |_| Ok(()))
            .map(|(bytes, ())| bytes);
        self.complete(details, result)
    }

    fn read_contents<T>(
        &mut self,
        name: &str,
        available_bytes: u64,
        details: &mut Details,
        prepare: impl FnOnce(usize) -> Result<T, Code>,
    ) -> Result<(Vec<u8>, T), Code> {
        self.validate(name, details.limit)?;
        details.phase = Phase::Reserve;
        if details.limit as u64 > available_bytes {
            return Err(Code::BudgetExceeded);
        }
        let limit = details.limit as usize;
        let prepared = prepare(limit)?;
        #[cfg(test)]
        if std::mem::take(&mut self.fail_allocation) {
            return Err(Code::AllocationFailed);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(limit)
            .map_err(|_| Code::AllocationFailed)?;
        let mut handle = self.open(name, details)?;
        details.phase = Phase::Read;
        let result = read(&mut handle, &mut bytes, limit, details);
        close(handle, result, details)?;
        Ok((bytes, prepared))
    }

    pub fn write_new(
        &mut self,
        name: &str,
        text: &str,
        max_bytes: i64,
        origin: Origin,
    ) -> Result<(), Failure> {
        self.write_contents(
            name,
            text.as_bytes(),
            max_bytes,
            origin,
            Operation::WriteNew,
        )
    }

    pub fn write_bytes_new(
        &mut self,
        name: &str,
        bytes: &[u8],
        max_bytes: i64,
        origin: Origin,
    ) -> Result<(), Failure> {
        self.write_contents(name, bytes, max_bytes, origin, Operation::WriteBytesNew)
    }

    fn write_contents(
        &mut self,
        name: &str,
        bytes: &[u8],
        max_bytes: i64,
        origin: Origin,
        operation: Operation,
    ) -> Result<(), Failure> {
        let details = Details::new(operation, max_bytes, Some(bytes.len() as u64), origin);
        self.write_prepared(name, details, bytes.len(), || Ok(bytes))
    }

    fn write_prepared<T: AsRef<[u8]>>(
        &mut self,
        name: &str,
        mut details: Details,
        length: usize,
        prepare: impl FnOnce() -> Result<T, Code>,
    ) -> Result<(), Failure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let result = (|| {
            self.validate(name, details.limit)?;
            if length as u64 > details.limit as u64 {
                return Err(Code::LimitExceeded);
            }
            details.phase = Phase::Reserve;
            let bytes = prepare()?;
            let mut handle = self.open(name, &mut details)?;
            details.phase = Phase::Write;
            let result = write(&mut handle, bytes.as_ref(), &mut details);
            close(handle, result, &mut details)
        })();
        self.complete(details, result)
    }

    fn validate(&self, name: &str, max_bytes: i64) -> Result<(), Code> {
        if !(0..=MAX_FILE_BYTES).contains(&max_bytes) {
            return Err(Code::InvalidLimit);
        }
        validate_name(name)?;
        self.backend.as_ref().ok_or(Code::Unconfigured)?;
        Ok(())
    }
    fn open(&mut self, name: &str, details: &mut Details) -> Result<B::Handle, Code> {
        details.phase = Phase::Open;
        match self
            .backend
            .as_mut()
            .expect("validated connection")
            .open(name, details.operation)
        {
            Ok(handle) => {
                if details.operation.creates_file() {
                    details.created = Created::Yes;
                }
                Ok(handle)
            }
            Err(error) => Err(match error {
                OpenError::NotFound => Code::NotFound,
                OpenError::AlreadyExists => Code::AlreadyExists,
                OpenError::UnsupportedTarget => Code::UnsupportedTarget,
                OpenError::Failed => Code::OpenFailed,
                OpenError::InvalidResponse => {
                    if details.operation.creates_file() {
                        details.created = Created::Unknown;
                    }
                    Code::InvalidResponse
                }
            }),
        }
    }
    fn complete<T>(&mut self, details: Details, result: Result<T, Code>) -> Result<T, Failure> {
        let event = Event {
            sequence: self.sequence,
            details,
            failure: result.as_ref().err().copied(),
        };
        self.sequence = self.sequence.saturating_add(1);
        if self.trace_limit != 0 {
            #[cfg(test)]
            let failed = std::mem::take(&mut self.fail_record_allocation);
            #[cfg(not(test))]
            let failed = false;
            if !failed && self.events.len() < self.trace_limit && self.events.try_reserve(1).is_ok()
            {
                self.events.push(event);
            } else {
                self.truncated = true;
            }
        }
        result.map_err(|code| {
            let failure = Failure { code, details };
            self.failure = Some(failure);
            failure
        })
    }
}

fn read(
    handle: &mut impl Read,
    bytes: &mut Vec<u8>,
    limit: usize,
    details: &mut Details,
) -> Result<(), Code> {
    let mut buffer = [0u8; 4096];
    loop {
        // 上限後の確認は独立した1バイト。上限+1の本文領域は確保しません。
        let request = (limit - bytes.len()).clamp(1, buffer.len());
        match handle.read(&mut buffer[..request]) {
            Ok(0) => {
                details.requested = Some(details.transferred);
                return Ok(());
            }
            Ok(n) if n <= request => {
                details.transferred += n as u64;
                if n > limit - bytes.len() {
                    return Err(Code::LimitExceeded);
                }
                bytes.extend_from_slice(&buffer[..n]);
            }
            Ok(_) => {
                details.progress_known = false;
                return Err(Code::InvalidResponse);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Code::ReadFailed),
        }
    }
}
fn write(handle: &mut impl Write, bytes: &[u8], details: &mut Details) -> Result<(), Code> {
    while (details.transferred as usize) < bytes.len() {
        let remaining = &bytes[details.transferred as usize..];
        match handle.write(remaining) {
            Ok(0) => return Err(Code::ZeroProgress),
            Ok(n) if n <= remaining.len() => details.transferred += n as u64,
            Ok(_) => {
                details.progress_known = false;
                return Err(Code::InvalidResponse);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Code::WriteFailed),
        }
    }
    Ok(())
}
fn close(handle: impl Handle, result: Result<(), Code>, details: &mut Details) -> Result<(), Code> {
    let closed = handle.close();
    details.cleanup = if closed.is_ok() {
        Cleanup::Closed
    } else {
        Cleanup::Failed
    };
    result?;
    details.phase = Phase::Close;
    closed.map_err(|_| Code::CloseFailed)
}

#[cfg(test)]
mod tests;

mod binary;
pub(crate) use binary::{byte_storage, read_array, write_array};
mod access;
pub(crate) use access::{Access, unconfigured};
