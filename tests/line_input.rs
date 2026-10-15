use cerune_lang::{
    ir::NodeId,
    runtime::input::{Code, Input, Line, Origin, Outcome},
    source::{SourceMap, Span},
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, Read},
    rc::Rc,
};

#[derive(Debug, Clone)]
enum Reply {
    Byte(u8),
    End,
    Interrupted,
    Error,
    Invalid,
}
#[derive(Default)]
struct State {
    replies: VecDeque<Reply>,
    calls: usize,
    consumed: usize,
}
#[derive(Clone)]
struct Reader(Rc<RefCell<State>>);
impl Reader {
    fn replies(replies: impl IntoIterator<Item = Reply>) -> Self {
        Self(Rc::new(RefCell::new(State {
            replies: replies.into_iter().collect(),
            ..State::default()
        })))
    }
    fn bytes(bytes: &[u8]) -> Self {
        Self::replies(bytes.iter().copied().map(Reply::Byte).chain([Reply::End]))
    }
    fn calls(&self) -> usize {
        self.0.borrow().calls
    }
    fn consumed(&self) -> usize {
        self.0.borrow().consumed
    }
}
impl Read for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        assert_eq!(buffer.len(), 1, "no hidden block read-ahead");
        let mut state = self.0.borrow_mut();
        state.calls += 1;
        match state.replies.pop_front().expect("unexpected extra read") {
            Reply::Byte(b) => {
                state.consumed += 1;
                buffer[0] = b;
                Ok(1)
            }
            Reply::End => Ok(0),
            Reply::Interrupted => Err(io::ErrorKind::Interrupted.into()),
            Reply::Error => Err(io::Error::other("SECRET OS error text")),
            Reply::Invalid => Ok(2),
        }
    }
}
fn origin() -> Origin {
    let mut sources = SourceMap::new();
    sources.add("main.ceru", "caller");
    let file = sources.add(
        "SECRET/path.ceru",
        "a source operation extending past byte 22",
    );
    Origin {
        node_id: NodeId(7),
        span: Span::in_source(file, 10, 22),
    }
}
fn line(text: &str) -> Line {
    Line::Text(text.to_owned())
}

#[test]
fn lines_preserve_bytes_and_are_distinct_from_eof() {
    for text in [
        "\n",
        "\r\n",
        "\r",
        "\0\n",
        "日本\0\r\n",
        "e\u{301}\n",
        "é\n",
        "x\ry\n",
        "\u{feff}x\n",
        "😀\n",
        "last",
    ] {
        let reader = Reader::bytes(text.as_bytes());
        let mut input = Input::with_trace_limit(reader.clone(), 3);
        let length = text.len();
        assert_eq!(
            input.read_line(length as i64, length as u64, origin()),
            Ok(line(text))
        );
        assert_eq!(reader.consumed(), length);
        let first = input.events()[0];
        assert_eq!(first.consumed, length);
        assert_eq!(first.outcome, Outcome::Line);
        assert!(first.consumption_known);
        assert_eq!(first.origin, origin());
        assert_eq!(input.read_line(1, 1, origin()), Ok(Line::End));
        let calls = reader.calls();
        assert_eq!(input.read_line(1, 0, origin()), Ok(Line::End));
        assert_eq!(reader.calls(), calls, "known EOF must not read");
        assert_eq!(input.events()[2].read_calls, 0);
        assert!(input.failure().is_none());
    }
    let reader = Reader::bytes(b"");
    let mut input = Input::new(reader.clone());
    assert_eq!(input.read_line(1, 1, origin()), Ok(Line::End));
    assert_eq!(reader.calls(), 1);
    assert_eq!(input.read_line(1, 0, origin()), Ok(Line::End));
    assert_eq!(reader.calls(), 1);
}

#[test]
fn exact_limit_checks_eof_once_without_losing_the_final_line() {
    let reader = Reader::bytes(b"abc");
    let mut input = Input::with_trace_limit(reader.clone(), 4);
    assert_eq!(input.read_line(3, 3, origin()), Ok(line("abc")));
    assert_eq!(reader.calls(), 4);
    assert_eq!(input.events()[0].read_calls, 4);
    assert_eq!(input.events()[0].consumed, 3);
    for _ in 0..2 {
        assert_eq!(input.read_line(3, 0, origin()), Ok(Line::End));
    }
    assert_eq!(reader.calls(), 4);
    let calls = reader.calls();
    drop(input);
    assert_eq!(reader.calls(), calls, "drop must not drain or read");
}

#[test]
fn lf_at_the_limit_does_not_probe_or_consume_the_next_line() {
    let reader = Reader::bytes(b"a\nb\nlast");
    let mut input = Input::new(reader.clone());
    for (limit, text, calls) in [(2, "a\n", 2), (2, "b\n", 4), (4, "last", 9)] {
        assert_eq!(
            input.read_line(limit, limit as u64, origin()),
            Ok(line(text))
        );
        assert_eq!(reader.calls(), calls);
    }
    assert_eq!(input.read_line(1, 0, origin()), Ok(Line::End));
    assert_eq!(reader.calls(), 9);
}

#[test]
fn overflow_counts_the_probe_byte_and_never_drains_or_retries() {
    for bytes in [&b"abc\nTAIL"[..], &b"abcdef"[..], &b"abc\xff"[..]] {
        let reader = Reader::bytes(bytes);
        let mut input = Input::with_trace_limit(reader.clone(), 10);
        let failure = input.read_line(3, 3, origin()).unwrap_err();
        assert_eq!(failure.code, Code::LimitExceeded);
        assert_eq!(failure.consumed, 4);
        assert!(failure.consumption_known);
        assert_eq!(reader.calls(), 4);
        assert_eq!(reader.consumed(), 4);
        assert_eq!(failure.origin, origin());
        assert_eq!(
            failure.record(),
            "io-v1 operation=read-line code=limit-exceeded limit=3 consumed=4 consumption=known node=7 file=2 bytes=10..22"
        );
        // A later invalid request cannot replace the original failure.
        let other = Origin {
            node_id: NodeId(99),
            span: Span::new(1, 2),
        };
        assert_eq!(input.read_line(0, 0, other), Err(failure));
        assert_eq!(input.failure(), Some(failure));
        assert_eq!(input.events().len(), 1);
        assert_eq!(reader.calls(), 4);
        drop(input.into_inner());
        assert_eq!(reader.calls(), 4);
    }
}

#[test]
fn strict_utf8_rejects_invalid_completed_lines_without_leaking_contents() {
    for bytes in [
        &b"\xff\n"[..],
        &b"\xc0\x80\n"[..],
        &b"\xed\xa0\x80\n"[..],
        &b"\xf4\x90\x80\x80\n"[..],
        &b"\xe3\x81"[..],
        &b"\x80\n"[..],
        &b"SECRET\xff\n"[..],
    ] {
        let reader = Reader::bytes(bytes);
        let mut input = Input::new(reader.clone());
        let failure = input
            .read_line(bytes.len() as i64, 100, origin())
            .unwrap_err();
        assert_eq!(failure.code, Code::InvalidUtf8);
        assert_eq!(failure.consumed, bytes.len());
        assert_eq!(reader.consumed(), bytes.len());
        assert!(!failure.record().contains("SECRET"));
        assert!(!failure.to_string().contains("SECRET"));
        assert!(!format!("{failure:?}").contains("SECRET"));
    }
}

#[test]
fn framing_failure_precedes_utf8_and_partial_lines_never_succeed() {
    let reader = Reader::bytes(b"\xffLONG");
    let mut input = Input::new(reader);
    assert_eq!(
        input.read_line(2, 2, origin()).unwrap_err().code,
        Code::LimitExceeded
    );
    for (bytes, limit) in [(&b"\xff"[..], 4), (&b"abc"[..], 3)] {
        let reader = Reader::replies(bytes.iter().copied().map(Reply::Byte).chain([Reply::Error]));
        let mut input = Input::with_trace_limit(reader.clone(), 10);
        let failure = input.read_line(limit, 100, origin()).unwrap_err();
        assert_eq!(failure.code, Code::ReadFailed);
        assert_eq!(failure.consumed, bytes.len());
        assert_eq!(reader.calls(), bytes.len() + 1);
        assert!(!failure.to_string().contains("SECRET"));
        assert_eq!(input.read_line(limit, 100, origin()), Err(failure));
        assert_eq!(reader.calls(), bytes.len() + 1);
    }
}

#[test]
fn only_interrupted_responses_are_retried() {
    let reader = Reader::replies([
        Reply::Interrupted,
        Reply::Interrupted,
        Reply::Byte(b'a'),
        Reply::Interrupted,
        Reply::Byte(b'\n'),
        Reply::Error,
    ]);
    let mut input = Input::with_trace_limit(reader.clone(), 4);
    assert_eq!(input.read_line(2, 2, origin()), Ok(line("a\n")));
    assert_eq!(input.events()[0].interruptions, 3);
    assert_eq!(input.events()[0].read_calls, 5);
    assert_eq!(input.events()[0].consumed, 2);
    let failure = input.read_line(2, 2, origin()).unwrap_err();
    assert_eq!(failure.code, Code::ReadFailed);
    assert_eq!(failure.consumed, 0);
    assert_eq!(input.read_line(2, 2, origin()), Err(failure));
    assert_eq!(reader.calls(), 6);
}

#[test]
fn invalid_response_keeps_confirmed_consumption_and_marks_the_rest_unknown() {
    let reader = Reader::replies([Reply::Byte(b'a'), Reply::Invalid]);
    let mut input = Input::with_trace_limit(reader.clone(), 2);
    let failure = input.read_line(3, 3, origin()).unwrap_err();
    assert_eq!(failure.code, Code::InvalidResponse);
    assert_eq!(failure.consumed, 1);
    assert!(!failure.consumption_known);
    assert!(failure.record().contains("consumed=1 consumption=unknown"));
    assert!(!input.events()[0].consumption_known);
    assert_eq!(input.read_line(3, 3, origin()), Err(failure));
    assert_eq!(reader.calls(), 2);
}

#[test]
fn preflight_errors_never_read_and_have_distinct_codes_and_origins() {
    for (limit, budget, code) in [
        (0, 10, Code::InvalidLimit),
        (-1, 10, Code::InvalidLimit),
        (i64::MIN, 10, Code::InvalidLimit),
        (i32::MAX as i64 + 1, 10, Code::InvalidLimit),
        (i64::MAX, 10, Code::InvalidLimit),
        (4, 3, Code::BudgetExceeded),
        (1, 0, Code::BudgetExceeded),
        (i32::MAX as i64, 0, Code::BudgetExceeded),
    ] {
        let reader = Reader::replies([]);
        let mut input = Input::with_trace_limit(reader.clone(), 2);
        let failure = input.read_line(limit, budget, origin()).unwrap_err();
        assert_eq!(failure.code, code);
        assert_eq!(failure.origin, origin());
        assert_eq!(failure.limit, limit);
        assert_eq!(failure.consumed, 0);
        assert_eq!(input.events()[0].read_calls, 0);
        assert_eq!(input.read_line(1, 100, origin()), Err(failure));
        assert_eq!(reader.calls(), 0);
    }
    let mut input = Input::<Reader>::unconfigured();
    let failure = input.read_line(1, 0, origin()).unwrap_err();
    assert_eq!(failure.code, Code::Unconfigured);
    assert_eq!(failure.consumed, 0);
    assert!(input.into_inner().is_none());
}

#[test]
fn known_eof_skips_reservation_but_does_not_accept_invalid_limits() {
    let reader = Reader::bytes(b"");
    let mut input = Input::new(reader.clone());
    assert_eq!(input.read_line(1, 1, origin()), Ok(Line::End));
    assert_eq!(input.read_line(i32::MAX as i64, 0, origin()), Ok(Line::End));
    assert_eq!(reader.calls(), 1);
    let failure = input.read_line(0, 0, origin()).unwrap_err();
    assert_eq!(failure.code, Code::InvalidLimit);
    assert_eq!(input.read_line(1, 1, origin()), Err(failure));
    assert_eq!(reader.calls(), 1);
}

#[test]
fn observation_capacity_does_not_change_reads_results_or_failures() {
    let mut expected = None;
    for capacity in [0, 1, 20] {
        let reader = Reader::replies([
            Reply::Interrupted,
            Reply::Byte(b'a'),
            Reply::Byte(b'\n'),
            Reply::Byte(b'b'),
            Reply::Error,
        ]);
        let mut input = Input::with_trace_limit(reader.clone(), capacity);
        let outcomes = [
            input.read_line(2, 2, origin()),
            input.read_line(2, 2, origin()),
            input.read_line(2, 2, origin()),
        ];
        let observed = (outcomes, reader.calls(), reader.consumed(), input.failure());
        if let Some(expected) = &expected {
            assert_eq!(&observed, expected);
        } else {
            expected = Some(observed);
        }
        assert_eq!(input.trace_truncated(), capacity == 1);
        assert_eq!(input.events().len(), capacity.min(2));
        if capacity > 1 {
            assert_eq!(input.events()[0].sequence, 0);
            assert_eq!(input.events()[1].sequence, 1);
            assert_eq!(input.events()[1].outcome, Outcome::Failed(Code::ReadFailed));
        }
        let before = reader.calls();
        // Immutable observation must not drive the connection.
        let _ = (input.events(), input.failure(), input.trace_truncated());
        assert_eq!(reader.calls(), before);
    }
}

#[test]
fn anonymous_origin_diagnostic_has_no_file_or_input_payload() {
    let mut input = Input::new(Reader::bytes(b"SECRET\n"));
    let failure = input
        .read_line(
            1,
            1,
            Origin {
                node_id: NodeId(2),
                span: Span::new(6, 11),
            },
        )
        .unwrap_err();
    assert_eq!(
        failure.record(),
        "io-v1 operation=read-line code=limit-exceeded limit=1 consumed=2 consumption=known node=2 bytes=6..11"
    );
}
