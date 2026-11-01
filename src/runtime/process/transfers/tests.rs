use super::*;
use crate::{ir::NodeId, source::Span};

fn transfers(call: u64, limits: [usize; 3]) -> Transfers {
    Transfers::new(
        NonZeroU64::new(call).unwrap(),
        Origin {
            node_id: NodeId(47),
            span: Span::new(10, 20),
        },
        limits,
        usize::MAX,
    )
    .unwrap()
}
fn cleanup() -> Cleanup {
    Cleanup::Complete {
        termination_requested: true,
    }
}
fn timeout(t: &mut Transfers) {
    t.stop(Failure {
        code: Code::Timeout,
        phase: Phase::Wait,
    });
}
fn read(t: &mut Transfers, channel: Channel, bytes: &[u8]) -> Transfer {
    let mut transfer = t.prepare_read(channel).unwrap();
    transfer.read_buffer().unwrap()[..bytes.len()].copy_from_slice(bytes);
    t.submit(transfer.ticket()).unwrap();
    transfer
}
fn eof(t: &mut Transfers, channel: Channel) {
    let transfer = read(t, channel, &[]);
    t.complete(transfer, 0, Completion::Eof).unwrap();
}
fn close(t: &mut Transfers) {
    let transfer = t.prepare_close_stdin().unwrap();
    t.submit(transfer.ticket()).unwrap();
    t.complete(transfer, 0, Completion::Done).unwrap();
}

#[test]
fn completion_order_preserves_each_stream_and_origin() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut t = transfers(1, [2, 4, 4]);
        let input = t.prepare_write(b"ab").unwrap();
        assert_eq!(input.bytes(), b"ab");
        t.submit(input.ticket()).unwrap();
        let output = read(&mut t, Channel::Stdout, b"\0\r\n");
        let error = read(&mut t, Channel::Stderr, &[0xff, b'e']);
        let mut pending = [Some(input), Some(output), Some(error)];
        for i in order {
            t.complete(pending[i].take().unwrap(), [2, 3, 2][i], Completion::Done)
                .unwrap();
        }
        close(&mut t);
        eof(&mut t, Channel::Stdout);
        eof(&mut t, Channel::Stderr);
        let result = t.finish(cleanup());
        assert!(result.reusable);
        assert_eq!(result.snapshot.failure, None);
        assert_eq!(result.snapshot.origin.node_id, NodeId(47));
        assert_eq!(result.snapshot.origin.span, Span::new(10, 20));
        assert_eq!(result.stdout, b"\0\r\n");
        assert_eq!(result.stderr, [0xff, b'e']);
        assert_eq!(result.snapshot.progress.map(|p| p.confirmed), [2, 3, 2]);
    }
}

#[test]
fn cancellation_acknowledges_request_but_late_completion_counts() {
    for reply in [
        CancelReply::Requested,
        CancelReply::TooLate,
        CancelReply::Failed,
    ] {
        for completion in [Completion::Done, Completion::Failed, Completion::Cancelled] {
            let mut t = transfers(1, [0, 8, 0]);
            let transfer = read(&mut t, Channel::Stdout, b"late");
            let ticket = transfer.ticket();
            timeout(&mut t);
            assert!(t.request_cancel(ticket).unwrap());
            assert!(!t.request_cancel(ticket).unwrap());
            t.cancel_reply(ticket, reply).unwrap();
            let pending = t.snapshot();
            assert_eq!(pending.progress[1].confirmed, 0);
            assert!(!pending.progress[1].known);
            assert_eq!(pending.operations[1].unwrap().state, State::Submitted);
            t.complete(transfer, 4, completion).unwrap();
            let result = t.finish(cleanup());
            assert_eq!(result.stdout, b"late");
            assert_eq!(result.snapshot.progress[1].confirmed, 4);
            assert!(result.snapshot.progress[1].known);
            assert_eq!(result.snapshot.failure.unwrap().code, Code::Timeout);
            assert_eq!(
                result.snapshot.operations[1].unwrap().completion,
                Some(completion)
            );
            assert!(result.reusable);
        }
    }
}

#[test]
fn cancellation_reply_may_arrive_after_terminal_completion() {
    let mut t = transfers(1, [0, 4, 0]);
    let transfer = read(&mut t, Channel::Stdout, b"ok");
    let ticket = transfer.ticket();
    timeout(&mut t);
    t.request_cancel(ticket).unwrap();
    t.complete(transfer, 2, Completion::Done).unwrap();
    t.cancel_reply(ticket, CancelReply::TooLate).unwrap();
    assert!(t.finish(cleanup()).reusable);
}

#[test]
fn missing_completion_is_unknown_even_when_cleanup_claims_complete() {
    let mut t = transfers(1, [1, 0, 0]);
    let transfer = t.prepare_write(b"x").unwrap();
    let ticket = transfer.ticket();
    t.submit(ticket).unwrap();
    timeout(&mut t);
    t.request_cancel(ticket).unwrap();
    t.cancel_reply(ticket, CancelReply::Requested).unwrap();
    let result = t.finish(cleanup());
    assert!(!result.reusable);
    assert!(!result.snapshot.progress[0].known);
    assert_eq!(result.snapshot.progress[0].confirmed, 0);
    assert_eq!(
        result.snapshot.operations[0].unwrap().state,
        State::Submitted
    );
    assert_eq!(result.cleanup, cleanup());
    // 結果確定後も接続がbufferを所有します。取消し応答では返却されません。
    assert_eq!(transfer.bytes(), b"x");
}

#[test]
fn partial_input_failure_preserves_accepted_prefix_and_unsent_amount() {
    let mut t = transfers(1, [7, 0, 0]);
    let first = t.prepare_write(b"1234567").unwrap();
    t.submit(first.ticket()).unwrap();
    t.complete(first, 2, Completion::Done).unwrap();
    let second = t.prepare_write(b"34567").unwrap();
    assert_eq!(second.bytes(), b"34567");
    t.submit(second.ticket()).unwrap();
    t.complete(second, 1, Completion::Failed).unwrap();
    let result = t.finish(cleanup());
    assert_eq!(result.snapshot.progress[0].confirmed, 3);
    assert_eq!(
        result.snapshot.limits[0] - result.snapshot.progress[0].confirmed,
        4
    );
    assert_eq!(result.snapshot.failure.unwrap().code, Code::TransferFailed);
    assert!(result.reusable);
}

#[test]
fn zero_exact_and_overflow_limits_keep_probe_out_of_output() {
    for limit in [0, 2] {
        for overflow in [false, true] {
            let mut t = transfers(1, [0, limit, 0]);
            if limit != 0 {
                let transfer = read(&mut t, Channel::Stdout, b"ab");
                t.complete(transfer, 2, Completion::Done).unwrap();
            }
            let transfer = read(&mut t, Channel::Stdout, if overflow { b"!" } else { b"" });
            assert_eq!(transfer.bytes().len(), 1);
            t.complete(
                transfer,
                usize::from(overflow),
                if overflow {
                    Completion::Done
                } else {
                    Completion::Eof
                },
            )
            .unwrap();
            if !overflow {
                close(&mut t);
                eof(&mut t, Channel::Stderr);
            }
            let result = t.finish(cleanup());
            assert_eq!(result.stdout, &b"ab"[..limit]);
            assert_eq!(
                result.snapshot.progress[1].confirmed,
                limit + usize::from(overflow)
            );
            assert_eq!(
                result.snapshot.failure.map(|f| f.code),
                overflow.then_some(Code::OutputLimit)
            );
            assert!(result.reusable);
        }
    }
}

#[test]
fn zero_progress_eof_and_cancelled_zero_are_distinct() {
    for completion in [Completion::Done, Completion::Eof, Completion::Cancelled] {
        let mut t = transfers(1, [0, 0, 0]);
        let transfer = read(&mut t, Channel::Stdout, b"");
        if completion == Completion::Cancelled {
            timeout(&mut t);
            t.request_cancel(transfer.ticket()).unwrap();
        }
        t.complete(transfer, 0, completion).unwrap();
        let snapshot = t.snapshot();
        assert_eq!(snapshot.progress[1].ended, completion == Completion::Eof);
        assert_eq!(
            snapshot.failure.map(|f| f.code),
            match completion {
                Completion::Done => Some(Code::ZeroProgress),
                Completion::Cancelled => Some(Code::Timeout),
                _ => None,
            }
        );
    }
}

#[test]
fn unknown_progress_keeps_lower_bound_and_blocks_reuse() {
    let mut t = transfers(1, [0, 8, 0]);
    let transfer = read(&mut t, Channel::Stdout, b"part");
    t.complete(transfer, 4, Completion::ProgressUnknown)
        .unwrap();
    let result = t.finish(cleanup());
    assert_eq!(result.stdout, b"part");
    assert_eq!(result.snapshot.progress[1].confirmed, 4);
    assert!(!result.snapshot.progress[1].known);
    assert_eq!(result.snapshot.failure.unwrap().code, Code::ProgressUnknown);
    assert!(!result.reusable);
}

#[test]
fn prepared_operation_can_be_withdrawn_after_stop_without_os_effect() {
    let mut t = transfers(1, [1, 0, 0]);
    let transfer = t.prepare_write(b"a").unwrap();
    timeout(&mut t);
    assert_eq!(t.submit(transfer.ticket()), Err(Code::ConnectionDisabled));
    t.withdraw(transfer).unwrap();
    assert!(matches!(
        t.prepare_read(Channel::Stdout),
        Err(PrepareError::Stopped)
    ));
    let result = t.finish(cleanup());
    assert!(result.snapshot.progress[0].known);
    assert_eq!(result.snapshot.progress[0].confirmed, 0);
    assert!(result.reusable);
}

#[test]
fn rejected_submission_is_known_zero_and_does_not_need_cancel_ack() {
    let mut t = transfers(1, [0, 0, 0]);
    let transfer = read(&mut t, Channel::Stdout, b"");
    t.complete(transfer, 0, Completion::NotSubmitted).unwrap();
    let result = t.finish(cleanup());
    assert_eq!(
        result.snapshot.operations[1].unwrap().completion,
        Some(Completion::NotSubmitted)
    );
    assert!(result.snapshot.progress[1].known);
    assert!(result.reusable);
}

#[test]
fn invalid_responses_never_add_bytes_or_prevent_other_completions() {
    for malformed in [
        Completion::Done,
        Completion::Eof,
        Completion::Cancelled,
        Completion::NotSubmitted,
    ] {
        let mut t = transfers(1, [0, 2, 2]);
        let output = read(&mut t, Channel::Stdout, b"ab");
        let error = read(&mut t, Channel::Stderr, b"ok");
        let count = if malformed == Completion::Done { 4 } else { 1 };
        assert_eq!(
            t.complete(output, count, malformed),
            Err(Code::InvalidResponse)
        );
        t.complete(error, 2, Completion::Done).unwrap();
        let result = t.finish(cleanup());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, b"ok");
        assert!(!result.snapshot.progress[1].known);
        assert!(!result.reusable);
    }
}

#[test]
fn duplicate_completion_and_foreign_call_cannot_double_count() {
    let mut t = transfers(1, [0, 4, 0]);
    let transfer = read(&mut t, Channel::Stdout, b"ab");
    // 不正な接続を模倣します。公開APIからTransferの複製はできません。
    let duplicate = Transfer {
        operation: transfer.operation,
        buffer: transfer.buffer.clone(),
    };
    t.complete(transfer, 2, Completion::Done).unwrap();
    assert_eq!(
        t.complete(duplicate, 2, Completion::Done),
        Err(Code::InvalidResponse)
    );
    let result = t.finish(cleanup());
    assert_eq!(result.stdout, b"ab");
    assert_eq!(result.snapshot.progress[1].confirmed, 2);
    assert!(!result.reusable);

    let mut a = transfers(1, [0, 2, 0]);
    let mut b = transfers(2, [0, 2, 0]);
    let own = read(&mut a, Channel::Stdout, b"a");
    let foreign = read(&mut b, Channel::Stdout, b"b");
    assert_eq!(
        a.complete(foreign, 1, Completion::Done),
        Err(Code::InvalidResponse)
    );
    a.complete(own, 1, Completion::Done).unwrap();
    let result = a.finish(cleanup());
    assert_eq!(result.stdout, b"a");
    assert!(!result.reusable);
}

#[test]
fn previous_operation_id_cannot_control_next_operation() {
    let mut t = transfers(1, [0, 4, 0]);
    let first = read(&mut t, Channel::Stdout, b"a");
    let old = first.ticket();
    t.complete(first, 1, Completion::Done).unwrap();
    let next = read(&mut t, Channel::Stdout, b"b");
    assert_ne!(old, next.ticket());
    timeout(&mut t);
    assert_eq!(t.request_cancel(old), Err(Code::InvalidResponse));
    t.complete(next, 1, Completion::Done).unwrap();
    let result = t.finish(cleanup());
    assert_eq!(result.stdout, b"ab");
    assert_eq!(result.snapshot.failure.unwrap().code, Code::Timeout);
    assert!(!result.reusable);
}

#[test]
fn one_operation_per_stream_and_close_only_after_input_is_accepted() {
    let mut t = transfers(1, [1, 2, 2]);
    assert!(matches!(
        t.prepare_close_stdin(),
        Err(PrepareError::InvalidInput)
    ));
    let input = t.prepare_write(b"x").unwrap();
    assert!(matches!(t.prepare_write(b"x"), Err(PrepareError::Busy)));
    let output = t.prepare_read(Channel::Stdout).unwrap();
    assert!(matches!(
        t.prepare_read(Channel::Stdout),
        Err(PrepareError::Busy)
    ));
    let error = t.prepare_read(Channel::Stderr).unwrap();
    t.withdraw(output).unwrap();
    t.withdraw(error).unwrap();
    t.submit(input.ticket()).unwrap();
    t.complete(input, 1, Completion::Done).unwrap();
    close(&mut t);
    assert!(matches!(t.prepare_close_stdin(), Err(PrepareError::Ended)));
}

#[test]
fn budgets_are_checked_before_buffers_and_chunks_are_bounded() {
    let origin = Origin {
        node_id: NodeId(1),
        span: Span::new(0, 1),
    };
    for (limits, available) in [([0, usize::MAX, 1], usize::MAX), ([0, 0, 0], 3 * CHUNK - 1)] {
        assert!(matches!(
            Transfers::new(NonZeroU64::new(1).unwrap(), origin, limits, available),
            Err(Code::OutputBudget)
        ));
    }
    let mut t = transfers(1, [CHUNK + 1, CHUNK + 1, 0]);
    let input = t.prepare_write(&vec![b'x'; CHUNK + 1]).unwrap();
    assert_eq!(input.bytes().len(), CHUNK);
    t.submit(input.ticket()).unwrap();
    t.complete(input, CHUNK, Completion::Done).unwrap();
    let rest = t.prepare_write(b"x").unwrap();
    assert_eq!(rest.bytes().len(), 1);
    let output = t.prepare_read(Channel::Stdout).unwrap();
    assert_eq!(output.bytes().len(), CHUNK);
}

#[test]
fn readonly_observation_does_not_change_actions_or_result() {
    fn run(observe: bool) -> Settlement {
        let mut t = transfers(1, [0, 4, 0]);
        let transfer = read(&mut t, Channel::Stdout, b"data");
        let ticket = transfer.ticket();
        timeout(&mut t);
        t.request_cancel(ticket).unwrap();
        if observe {
            for _ in 0..100 {
                let snapshot = t.snapshot();
                assert_eq!(snapshot.operations[1].unwrap().ticket, ticket);
                assert!(!snapshot.progress[1].known);
            }
        }
        t.complete(transfer, 4, Completion::Done).unwrap();
        t.finish(cleanup())
    }
    assert_eq!(run(false), run(true));
}

#[test]
fn incomplete_cleanup_preserves_first_cause_and_resource_report() {
    let mut t = transfers(1, [0, 0, 0]);
    timeout(&mut t);
    let cleanup = Cleanup::Incomplete {
        child_reaped: true,
        handles_closed: false,
        scope_finished: false,
        error: super::super::CleanupError::Close,
    };
    let result = t.finish(cleanup);
    assert_eq!(result.snapshot.failure.unwrap().code, Code::Timeout);
    assert_eq!(result.cleanup, cleanup);
    assert!(!result.reusable);
}

#[test]
fn successful_cleanup_cannot_replace_missing_eof_in_normal_execution() {
    let t = transfers(1, [0, 0, 0]);
    let result = t.finish(cleanup());
    assert_eq!(result.snapshot.failure.unwrap().code, Code::InvalidResponse);
    assert!(!result.reusable);
}

#[test]
fn invalid_withdrawal_returns_the_still_owned_buffer() {
    let mut t = transfers(1, [0, 4, 0]);
    let transfer = read(&mut t, Channel::Stdout, b"late");
    let address = transfer.bytes().as_ptr();
    let transfer = t.withdraw(transfer).unwrap_err();
    assert_eq!(transfer.bytes().as_ptr(), address);
    assert_eq!(t.snapshot().operations[1].unwrap().state, State::Submitted);
    t.complete(transfer, 4, Completion::Done).unwrap();
    let result = t.finish(cleanup());
    assert_eq!(result.stdout, b"late");
    assert_eq!(result.snapshot.failure.unwrap().code, Code::InvalidResponse);
    assert!(!result.reusable);
}

#[test]
fn exhausted_transfer_numbers_never_wrap_or_reuse_an_id() {
    let mut t = transfers(1, [0, 0, 0]);
    t.sequence = u64::MAX - 1;
    let transfer = t.prepare_read(Channel::Stdout).unwrap();
    assert_eq!(transfer.ticket().sequence(), u64::MAX);
    t.withdraw(transfer).unwrap();
    assert!(matches!(
        t.prepare_read(Channel::Stdout),
        Err(PrepareError::Exhausted)
    ));
    assert_eq!(
        t.snapshot().operations[1].unwrap().ticket.sequence(),
        u64::MAX
    );
}
