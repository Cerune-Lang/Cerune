//! 固定Report v1。数値タグはRustや標準ソースのenum順から独立しています。
//! SourceIdを復元しても、そのIDが特定のSourceMapに存在する保証にはなりません。
use super::{Channel, Cleanup, CleanupError, Code, Exit, Failure, Origin, Phase, Report, Started};
use crate::{
    ir::NodeId,
    source::{SourceId, Span},
};
use std::time::Duration;

pub const WORD_COUNT: usize = 32;
pub const BYTE_COUNT: usize = WORD_COUNT * 8;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Version,
    Shape,
    Reserved,
    Tag,
    Payload,
    Range,
    TimePrecision,
}
/// 検証済みの32 word。C ABIの関数やmemory配置はまだ提供しません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportV1 {
    words: [u64; WORD_COUNT],
}
impl ReportV1 {
    pub fn encode(report: &Report) -> Result<Self, Error> {
        let mut w = [0; WORD_COUNT];
        w[0] = 1;
        w[1] = WORD_COUNT as u64;
        for (slot, value) in [2, 3, 4, 5, 7, 8, 9, 10, 12, 13, 14, 15].into_iter().zip([
            report.origin.node_id.0,
            report.origin.span.source_id().index(),
            report.origin.span.start(),
            report.origin.span.end(),
            report.stdin_requested,
            report.stdin_accepted,
            report.stdout_limit,
            report.stderr_limit,
            report.stdout_read,
            report.stderr_read,
            report.stdout_retained,
            report.stderr_retained,
        ]) {
            w[slot] = u64::try_from(value).map_err(|_| Error::Range)?;
        }
        w[6] = match report.started {
            Started::No => 0,
            Started::Yes => 1,
            Started::Unknown => 2,
        };
        if !report.timeout.subsec_nanos().is_multiple_of(1_000_000) {
            return Err(Error::TimePrecision);
        }
        w[11] = u64::try_from(report.timeout.as_millis()).map_err(|_| Error::Range)?;
        w[16] = u64::from(report.progress_known);
        match report.exit {
            None => (),
            Some(Exit::Code(value)) => {
                w[17] = 1;
                w[18] = u64::from(value);
            }
            Some(Exit::Signal(value)) => {
                w[17] = 2;
                w[18] = u64::from(value);
            }
        }
        if let Some(failure) = report.failure {
            w[19] = encode_code(failure.code);
            (w[20], w[21]) = match failure.phase {
                Phase::Validate => (1, 0),
                Phase::Reserve => (2, 0),
                Phase::Start => (3, 0),
                Phase::Transfer(c) => (
                    4,
                    match c {
                        Channel::Stdin => 1,
                        Channel::Stdout => 2,
                        Channel::Stderr => 3,
                    },
                ),
                Phase::CloseStdin => (5, 0),
                Phase::Wait => (6, 0),
                Phase::Cleanup => (7, 0),
            };
        }
        w[22..25].copy_from_slice(&encode_cleanup(report.cleanup));
        w[25..28].copy_from_slice(&encode_cleanup(report.cleanup_reported));
        Ok(Self { words: w })
    }
    pub fn from_words(words: &[u64]) -> Result<Self, Error> {
        let words: [u64; WORD_COUNT] = words.try_into().map_err(|_| Error::Shape)?;
        decode_words(&words)?;
        Ok(Self { words })
    }
    pub fn words(&self) -> &[u64; WORD_COUNT] {
        &self.words
    }
    pub fn to_report(&self) -> Report {
        decode_words(&self.words).expect("validated ReportV1")
    }
    pub fn to_le_bytes(&self) -> [u8; BYTE_COUNT] {
        let mut bytes = [0; BYTE_COUNT];
        for (chunk, word) in bytes.as_chunks_mut::<8>().0.iter_mut().zip(self.words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }
    pub fn from_le_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != BYTE_COUNT {
            return Err(Error::Shape);
        }
        let mut words = [0; WORD_COUNT];
        for (word, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0.iter()) {
            *word = u64::from_le_bytes(*chunk);
        }
        Self::from_words(&words)
    }
}
fn size(value: u64) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::Range)
}
fn boolean(value: u64) -> Result<bool, Error> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Payload),
    }
}
fn decode_words(w: &[u64; WORD_COUNT]) -> Result<Report, Error> {
    if w[0] != 1 {
        return Err(Error::Version);
    }
    if w[1] != WORD_COUNT as u64 {
        return Err(Error::Shape);
    }
    if w[28..].iter().any(|v| *v != 0) {
        return Err(Error::Reserved);
    }
    if w[4] > w[5] {
        return Err(Error::Range);
    }
    let started = match w[6] {
        0 => Started::No,
        1 => Started::Yes,
        2 => Started::Unknown,
        _ => return Err(Error::Tag),
    };
    let exit = match w[17] {
        0 if w[18] == 0 => None,
        0 => return Err(Error::Payload),
        1 => Some(Exit::Code(u32::try_from(w[18]).map_err(|_| Error::Range)?)),
        2 => Some(Exit::Signal(
            u32::try_from(w[18]).map_err(|_| Error::Range)?,
        )),
        _ => return Err(Error::Tag),
    };
    let failure = if w[19] == 0 {
        if w[20] != 0 || w[21] != 0 {
            return Err(Error::Payload);
        }
        None
    } else {
        let code = decode_code(w[19])?;
        if w[20] != 4 && w[21] != 0 {
            return Err(Error::Payload);
        }
        let phase = match w[20] {
            1 => Phase::Validate,
            2 => Phase::Reserve,
            3 => Phase::Start,
            4 => Phase::Transfer(match w[21] {
                1 => Channel::Stdin,
                2 => Channel::Stdout,
                3 => Channel::Stderr,
                _ => return Err(Error::Tag),
            }),
            5 => Phase::CloseStdin,
            6 => Phase::Wait,
            7 => Phase::Cleanup,
            _ => return Err(Error::Tag),
        };
        Some(Failure { code, phase })
    };
    Ok(Report {
        origin: Origin {
            node_id: NodeId(size(w[2])?),
            span: Span::in_source(SourceId::from_index(size(w[3])?), size(w[4])?, size(w[5])?),
        },
        started,
        stdin_requested: size(w[7])?,
        stdin_accepted: size(w[8])?,
        stdout_limit: size(w[9])?,
        stderr_limit: size(w[10])?,
        timeout: Duration::from_millis(w[11]),
        stdout_read: size(w[12])?,
        stderr_read: size(w[13])?,
        stdout_retained: size(w[14])?,
        stderr_retained: size(w[15])?,
        progress_known: boolean(w[16])?,
        exit,
        failure,
        cleanup: decode_cleanup(w[22], w[23], w[24])?,
        cleanup_reported: decode_cleanup(w[25], w[26], w[27])?,
    })
}
fn encode_cleanup(value: Option<Cleanup>) -> [u64; 3] {
    match value {
        None => [0, 0, 0],
        Some(Cleanup::Complete {
            termination_requested,
        }) => [1, u64::from(termination_requested), 0],
        Some(Cleanup::Incomplete {
            child_reaped,
            handles_closed,
            scope_finished,
            error,
        }) => [
            2,
            (u64::from(child_reaped) << 1)
                | (u64::from(handles_closed) << 2)
                | (u64::from(scope_finished) << 3),
            encode_cleanup_error(error),
        ],
    }
}
fn decode_cleanup(kind: u64, flags: u64, error: u64) -> Result<Option<Cleanup>, Error> {
    Ok(match kind {
        0 if flags == 0 && error == 0 => None,
        0 => return Err(Error::Payload),
        1 if flags <= 1 && error == 0 => Some(Cleanup::Complete {
            termination_requested: flags == 1,
        }),
        1 => return Err(Error::Payload),
        2 if flags & !14 == 0 => Some(Cleanup::Incomplete {
            child_reaped: flags & 2 != 0,
            handles_closed: flags & 4 != 0,
            scope_finished: flags & 8 != 0,
            error: decode_cleanup_error(error)?,
        }),
        2 => return Err(Error::Payload),
        _ => return Err(Error::Tag),
    })
}

fn encode_code(value: Code) -> u64 {
    match value {
        Code::InvalidRequest => 1,
        Code::RequestLimit => 2,
        Code::Unconfigured => 3,
        Code::Unregistered => 4,
        Code::ConnectionDisabled => 5,
        Code::OutputBudget => 6,
        Code::AllocationFailed => 7,
        Code::NotFound => 8,
        Code::Denied => 9,
        Code::Resources => 10,
        Code::Unsupported => 11,
        Code::StartFailed => 12,
        Code::TransferFailed => 13,
        Code::CloseFailed => 14,
        Code::WaitFailed => 15,
        Code::ZeroProgress => 16,
        Code::InvalidResponse => 17,
        Code::OutputLimit => 18,
        Code::Timeout => 19,
        Code::CleanupFailed => 20,
        Code::ProgressUnknown => 21,
    }
}
fn decode_code(value: u64) -> Result<Code, Error> {
    match value {
        1 => Ok(Code::InvalidRequest),
        2 => Ok(Code::RequestLimit),
        3 => Ok(Code::Unconfigured),
        4 => Ok(Code::Unregistered),
        5 => Ok(Code::ConnectionDisabled),
        6 => Ok(Code::OutputBudget),
        7 => Ok(Code::AllocationFailed),
        8 => Ok(Code::NotFound),
        9 => Ok(Code::Denied),
        10 => Ok(Code::Resources),
        11 => Ok(Code::Unsupported),
        12 => Ok(Code::StartFailed),
        13 => Ok(Code::TransferFailed),
        14 => Ok(Code::CloseFailed),
        15 => Ok(Code::WaitFailed),
        16 => Ok(Code::ZeroProgress),
        17 => Ok(Code::InvalidResponse),
        18 => Ok(Code::OutputLimit),
        19 => Ok(Code::Timeout),
        20 => Ok(Code::CleanupFailed),
        21 => Ok(Code::ProgressUnknown),
        _ => Err(Error::Tag),
    }
}

fn encode_cleanup_error(value: CleanupError) -> u64 {
    match value {
        CleanupError::Timeout => 1,
        CleanupError::Terminate => 2,
        CleanupError::Reap => 3,
        CleanupError::Close => 4,
        CleanupError::InvalidResponse => 5,
    }
}
fn decode_cleanup_error(value: u64) -> Result<CleanupError, Error> {
    match value {
        1 => Ok(CleanupError::Timeout),
        2 => Ok(CleanupError::Terminate),
        3 => Ok(CleanupError::Reap),
        4 => Ok(CleanupError::Close),
        5 => Ok(CleanupError::InvalidResponse),
        _ => Err(Error::Tag),
    }
}

#[cfg(test)]
mod tests;
