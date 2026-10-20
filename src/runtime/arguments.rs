//! 実行開始前に明示注入する、不変なプログラム引数です。
//! OS接続・言語命令・文字列ヒープへのコピーは各呼出し側が担当します。
use std::fmt;

/// 件数と内容のUTF-8バイト数を別々に制限します。
/// allocatorの管理領域やString/Vecの格納量をバイト上限に含めません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_count: u64,
    pub max_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_count: 4096,
            max_bytes: 1048576,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    InvalidLimit,
    CountLimitExceeded,
    ByteLimitExceeded,
    InvalidUtf8,
    EmbeddedNul,
    AllocationFailed,
    Unconfigured,
    InvalidIndex,
    IndexOutOfBounds,
    BudgetExceeded,
}
impl Code {
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidLimit => "invalid-limit",
            Self::CountLimitExceeded => "count-limit-exceeded",
            Self::ByteLimitExceeded => "byte-limit-exceeded",
            Self::InvalidUtf8 => "invalid-utf8",
            Self::EmbeddedNul => "embedded-nul",
            Self::AllocationFailed => "allocation-failed",
            Self::Unconfigured => "unconfigured",
            Self::IndexOutOfBounds => "index-out-of-bounds",
            Self::BudgetExceeded => "budget-exceeded",
            Self::InvalidIndex => "invalid-index",
        }
    }
}

/// 起動準備の失敗です。まだ存在しないソース命令のNodeId/Spanを付けません。
/// 本文・パス・OSのエラー文字列は保持しません。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureFailure {
    pub code: Code,
    /// 特定の引数の検証・コピーで失敗した場合の0始まりの位置です。
    pub index: Option<usize>,
}
impl fmt::Display for CaptureFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "arguments-v1 phase=capture code={}", self.code.name())?;
        if let Some(index) = self.index {
            write!(f, " index={index}")?;
        }
        Ok(())
    }
}
impl std::error::Error for CaptureFailure {}

struct Snapshot {
    values: Vec<String>,
    bytes: u64,
}

/// 未設定と設定済みの空列を区別します。完成後に引数を書き換えるAPIはありません。
/// Debugも本文を出しません。内容はgetを明示的に呼んだ場合だけ返します。
pub struct Arguments {
    snapshot: Option<Snapshot>,
}
impl fmt::Debug for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("Arguments");
        out.field("configured", &self.snapshot.is_some());
        if let Some(snapshot) = &self.snapshot {
            out.field("count", &snapshot.values.len())
                .field("bytes", &snapshot.bytes);
        }
        out.finish()
    }
}
impl Arguments {
    pub const fn unconfigured() -> Self {
        Self { snapshot: None }
    }

    /// 既に区切られた引数のバイト列を検証し、独立した内容を所有します。
    /// 実行ファイル名等の除外は接続側の責務です。OSや環境変数は読みません。
    /// 全件の検証後に確保し、失敗時は部分的なsnapshotを返しません。
    pub fn capture(values: &[&[u8]], limits: Limits) -> Result<Self, CaptureFailure> {
        Self::capture_prepared(values, limits, || Ok(()))
    }

    // 確保失敗の検証用です。公開の観測callbackやallocator設定ではありません。
    fn capture_prepared(
        values: &[&[u8]],
        limits: Limits,
        mut before_allocation: impl FnMut() -> Result<(), ()>,
    ) -> Result<Self, CaptureFailure> {
        let failure = |code, index| CaptureFailure { code, index };
        if limits.max_count > i64::MAX as u64 {
            return Err(failure(Code::InvalidLimit, None));
        }
        if values.len() as u64 > limits.max_count {
            return Err(failure(Code::CountLimitExceeded, None));
        }
        let mut bytes = 0_u64;
        for (index, value) in values.iter().enumerate() {
            bytes = bytes
                .checked_add(value.len() as u64)
                .filter(|&total| total <= limits.max_bytes)
                .ok_or_else(|| failure(Code::ByteLimitExceeded, Some(index)))?;
            std::str::from_utf8(value).map_err(|_| failure(Code::InvalidUtf8, Some(index)))?;
            if value.contains(&0) {
                return Err(failure(Code::EmbeddedNul, Some(index)));
            }
        }
        let mut copied = Vec::new();
        if !values.is_empty() {
            before_allocation().map_err(|_| failure(Code::AllocationFailed, None))?;
            copied
                .try_reserve_exact(values.len())
                .map_err(|_| failure(Code::AllocationFailed, None))?;
        }
        for (index, value) in values.iter().enumerate() {
            let mut text = String::new();
            if !value.is_empty() {
                before_allocation().map_err(|_| failure(Code::AllocationFailed, Some(index)))?;
                text.try_reserve_exact(value.len())
                    .map_err(|_| failure(Code::AllocationFailed, Some(index)))?;
            }
            // 呼出し中の入力は共有借用で不変です。先の検証結果を再利用します。
            text.push_str(std::str::from_utf8(value).expect("validated argument"));
            copied.push(text);
        }
        Ok(Self {
            snapshot: Some(Snapshot {
                values: copied,
                bytes,
            }),
        })
    }

    pub fn count(&self) -> Result<i64, Code> {
        self.snapshot
            .as_ref()
            .map(|s| s.values.len() as i64)
            .ok_or(Code::Unconfigured)
    }

    /// 読取りだけで確保・コピー・消費はしません。空文字はSome("")です。
    /// 未設定を先に判定し、設定済みの場合だけ添字を検査します。
    pub fn get(&self, index: i64) -> Result<Option<&str>, Code> {
        let snapshot = self.snapshot.as_ref().ok_or(Code::Unconfigured)?;
        if index < 0 {
            return Err(Code::InvalidIndex);
        }
        // 32-bit環境でも大きな添字を切り捨てません。
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| snapshot.values.get(index))
            .map(String::as_str))
    }

    /// 本文を含まない使用バイト数です。文字列ヒープの使用量とは別です。
    pub fn byte_len(&self) -> Result<u64, Code> {
        self.snapshot
            .as_ref()
            .map(|s| s.bytes)
            .ok_or(Code::Unconfigured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        max_count: 8,
        max_bytes: 128,
    };

    #[test]
    fn empty_missing_negative_and_unconfigured_are_distinct() {
        let absent = Arguments::unconfigured();
        assert_eq!(absent.count(), Err(Code::Unconfigured));
        assert_eq!(absent.byte_len(), Err(Code::Unconfigured));
        assert_eq!(absent.get(0), Err(Code::Unconfigured));
        assert_eq!(absent.get(-1), Err(Code::Unconfigured));

        let empty = Arguments::capture(
            &[],
            Limits {
                max_count: 0,
                max_bytes: 0,
            },
        )
        .unwrap();
        assert_eq!(empty.count(), Ok(0));
        assert_eq!(empty.byte_len(), Ok(0));
        assert_eq!(empty.get(0), Ok(None));
        assert_eq!(empty.get(-1), Err(Code::InvalidIndex));

        let one = Arguments::capture(
            &[b""],
            Limits {
                max_count: 1,
                max_bytes: 0,
            },
        )
        .unwrap();
        assert_eq!(one.count(), Ok(1));
        assert_eq!(one.get(0), Ok(Some("")));
        assert_eq!(one.get(1), Ok(None));
        assert_eq!(one.get(i64::MAX), Ok(None));
    }

    #[test]
    fn bytes_and_order_survive_without_normalizing_or_parsing_options() {
        let texts = [
            "日本語",
            "e\u{301}",
            "é",
            " a\r\nb ",
            "--ssa",
            "--",
            "\"quoted\"",
            "",
        ];
        let values: Vec<&[u8]> = texts.iter().map(|s| s.as_bytes()).collect();
        let args = Arguments::capture(&values, LIMITS).unwrap();
        let before = format!("{args:?}");
        for _ in 0..3 {
            for (i, expected) in texts.iter().enumerate() {
                assert_eq!(args.get(i as i64), Ok(Some(*expected)));
            }
        }
        assert_eq!(
            args.byte_len(),
            Ok(texts.iter().map(|s| s.len() as u64).sum())
        );
        assert_eq!(format!("{args:?}"), before);
        for secret in ["日本語", "--ssa", "quoted"] {
            assert!(!before.contains(secret));
        }
    }

    #[test]
    fn host_mutation_and_drop_do_not_change_the_snapshot() {
        let mut host = b"original".to_vec();
        let args = Arguments::capture(&[host.as_slice()], LIMITS).unwrap();
        host.fill(b'x');
        drop(host);
        assert_eq!(args.get(0), Ok(Some("original")));
    }

    #[test]
    fn exact_limits_and_competing_failures_have_defined_precedence() {
        let limits = Limits {
            max_count: 2,
            max_bytes: 3,
        };
        let args = Arguments::capture(&["あ".as_bytes(), b""], limits).unwrap();
        assert_eq!(args.byte_len(), Ok(3));
        for (values, limits, code, index) in [
            (
                vec![b"".as_slice(); 3],
                limits,
                Code::CountLimitExceeded,
                None,
            ),
            (
                vec![b"ab".as_slice(), b"cd"],
                limits,
                Code::ByteLimitExceeded,
                Some(1),
            ),
            (
                vec![b"ab".as_slice(), b"\xff\0"],
                limits,
                Code::ByteLimitExceeded,
                Some(1),
            ),
            (
                vec![b"\xff\0".as_slice()],
                limits,
                Code::InvalidUtf8,
                Some(0),
            ),
            (vec![b"a\0".as_slice()], limits, Code::EmbeddedNul, Some(0)),
            (
                vec![b"\xff".as_slice(); 3],
                limits,
                Code::CountLimitExceeded,
                None,
            ),
            (
                vec![],
                Limits {
                    max_count: u64::MAX,
                    max_bytes: 0,
                },
                Code::InvalidLimit,
                None,
            ),
        ] {
            assert_eq!(
                Arguments::capture(&values, limits).unwrap_err(),
                CaptureFailure { code, index }
            );
        }
    }

    #[test]
    fn invalid_text_is_rejected_before_any_allocation() {
        let mut allocations = 0;
        let result = Arguments::capture_prepared(&[b"valid", b"\xff"], LIMITS, || {
            allocations += 1;
            Ok(())
        });
        assert_eq!(result.unwrap_err().code, Code::InvalidUtf8);
        assert_eq!(allocations, 0);
    }

    #[test]
    fn every_allocation_failure_is_atomic_and_redacted() {
        let values: &[&[u8]] = &[b"SECRET-first", b"", b"SECRET-last"];
        for fail_at in 0..3 {
            let mut allocations = 0;
            let failure = Arguments::capture_prepared(values, LIMITS, || {
                let should_fail = allocations == fail_at;
                allocations += 1;
                if should_fail { Err(()) } else { Ok(()) }
            })
            .unwrap_err();
            assert_eq!(failure.code, Code::AllocationFailed);
            assert_eq!(failure.index, [None, Some(0), Some(2)][fail_at]);
            assert_eq!(allocations, fail_at + 1);
            assert!(!failure.to_string().contains("SECRET"));
            assert!(!format!("{failure:?}").contains("SECRET"));
            assert!(!failure.to_string().contains("node="));
            assert_eq!(Arguments::capture(values, LIMITS).unwrap().count(), Ok(3));
        }
        let empty = Arguments::capture_prepared(&[], LIMITS, || panic!("no allocation")).unwrap();
        assert_eq!(empty.count(), Ok(0));
    }

    #[test]
    fn diagnostics_contain_only_phase_reason_and_position() {
        let failure = Arguments::capture(&[b"SECRET\0value"], LIMITS).unwrap_err();
        assert_eq!(
            failure.to_string(),
            "arguments-v1 phase=capture code=embedded-nul index=0"
        );
        assert!(!format!("{failure:?}").contains("SECRET"));
    }
}

/// Source operation failure; argument contents are never included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub code: Code,
    pub index: Option<i64>,
    pub origin: super::output::Origin,
}
impl Failure {
    pub fn record(self) -> String {
        let operation = if self.index.is_some() { "get" } else { "count" };
        let index = self
            .index
            .map(|i| format!(" index={i}"))
            .unwrap_or_default();
        format!(
            "arguments-v1 phase=execute operation={operation} code={}{} node={}{} bytes={}..{}",
            self.code.name(),
            index,
            self.origin.node_id.0,
            self.origin.span.source_id().record_field(),
            self.origin.span.start(),
            self.origin.span.end()
        )
    }
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.record())
    }
}
impl std::error::Error for Failure {}
impl Arguments {
    pub(crate) fn count_at(&self, origin: super::output::Origin) -> Result<i64, Failure> {
        self.count().map_err(|code| Failure {
            code,
            index: None,
            origin,
        })
    }
    pub(crate) fn get_value(
        &self,
        heap: &mut super::string_heap::StringHeap,
        index: i64,
        origin: super::output::Origin,
    ) -> Result<super::string_heap::StringValue, Failure> {
        let failure = |code| Failure {
            code,
            index: Some(index),
            origin,
        };
        let text = self
            .get(index)
            .map_err(failure)?
            .ok_or_else(|| failure(Code::IndexOutOfBounds))?;
        if text.is_empty() {
            return Ok(String::new().into());
        }
        let reservation = heap.reserve(text.len()).map_err(|e| {
            failure(match e {
                super::string_heap::HeapError::AllocationLimitExceeded => Code::BudgetExceeded,
                _ => Code::AllocationFailed,
            })
        })?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| failure(Code::AllocationFailed))?;
        owned.push_str(text);
        Ok(reservation.finish(owned))
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use crate::{
        ir::NodeId, runtime::output::Origin, runtime::string_heap::StringHeap, source::Span,
    };
    #[test]
    fn returned_value_allocation_failures_keep_origin_and_leave_the_heap_empty() {
        let origin = Origin {
            node_id: NodeId(9),
            span: Span::new(4, 8),
        };
        let args = Arguments::capture(
            &[b"SECRET"],
            Limits {
                max_count: 1,
                max_bytes: 6,
            },
        )
        .unwrap();
        let mut heap = StringHeap::new(6);
        heap.fail_next_allocation();
        let failure = args.get_value(&mut heap, 0, origin).unwrap_err();
        assert_eq!(failure.code, Code::AllocationFailed);
        assert_eq!(failure.origin, origin);
        assert!(!failure.record().contains("SECRET"));
        assert!(heap.is_empty());
        let value = args.get_value(&mut heap, 0, origin).unwrap();
        assert_eq!(value.text(), "SECRET");
        heap.manage(&value, false).unwrap();
        assert!(heap.is_empty());
        let empty = Arguments::capture(
            &[b""],
            Limits {
                max_count: 1,
                max_bytes: 0,
            },
        )
        .unwrap();
        let mut heap = StringHeap::new(0);
        assert_eq!(empty.get_value(&mut heap, 0, origin).unwrap().text(), "");
        assert!(heap.is_empty());
    }
}
