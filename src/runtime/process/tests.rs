use super::transfers::{CancelReply, Completion, Kind, Ticket, Transfer};
use super::*;
use crate::{ir::NodeId, source::Span};
use std::collections::VecDeque;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn origin() -> Origin {
    Origin {
        node_id: NodeId(17),
        span: Span::new(30, 41),
    }
}
fn limits() -> Limits {
    Limits {
        arguments: 8,
        argument_bytes: 128,
        stdin_bytes: 8192,
        output_bytes: 16384,
        timeout: ms(100),
        cleanup_timeout: ms(10),
    }
}
fn request() -> Request<'static> {
    Request {
        target: "tool",
        arguments: &[],
        stdin: &[],
        stdout_limit: 32,
        stderr_limit: 32,
        timeout: ms(100),
    }
}
fn complete() -> Cleanup {
    Cleanup::Complete {
        termination_requested: false,
    }
}
fn code(out: &Outcome) -> Option<Code> {
    out.report.failure.map(|f| f.code)
}
struct Reply {
    channel: usize,
    bytes: Vec<u8>,
    n: usize,
    result: Completion,
}
fn done(channel: usize, bytes: &[u8], n: usize, result: Completion) -> Reply {
    Reply {
        channel,
        bytes: bytes.to_vec(),
        n,
        result,
    }
}
struct Step {
    at: Duration,
    replies: Vec<Reply>,
    exit: Option<Result<Exit, WaitError>>,
    cleanup: Option<Cleanup>,
    wake: Wake,
    acknowledge: bool,
}
fn step(at: u64, replies: Vec<Reply>) -> Step {
    Step {
        at: ms(at),
        replies,
        exit: None,
        cleanup: None,
        wake: Wake::Events,
        acknowledge: false,
    }
}
struct Plan {
    launch: Launch,
    steps: VecDeque<Step>,
    cleanup: Option<VecDeque<Step>>,
    start_time: Option<Duration>,
    submit_time: Option<Duration>,
    begin_time: Option<Duration>,
}
fn plan(steps: Vec<Step>) -> Plan {
    Plan {
        launch: Launch::Started,
        steps: steps.into(),
        cleanup: None,
        start_time: None,
        submit_time: None,
        begin_time: None,
    }
}
fn finished(code: u32) -> Plan {
    let mut s = step(
        1,
        vec![
            done(0, b"", 0, Completion::Done),
            done(1, b"", 0, Completion::Eof),
            done(2, b"", 0, Completion::Eof),
        ],
    );
    s.exit = Some(Ok(Exit::Code(code)));
    plan(vec![s])
}
#[derive(Debug, PartialEq, Eq)]
enum Action {
    Start,
    Submit(Ticket, Kind, usize),
    Cancel(Ticket),
    Cleanup(bool),
    Poll,
}
#[derive(Default)]
struct Script {
    plans: VecDeque<Plan>,
    active: Option<Plan>,
    pending: [Option<Transfer>; 3],
    cancellations: Vec<Ticket>,
    cleaning: bool,
    now: Duration,
    actions: Vec<Action>,
    contexts: Vec<CallContext>,
    sent: Vec<u8>,
    args: Vec<String>,
}
impl Script {
    fn new(plans: Vec<Plan>) -> Self {
        Self {
            plans: plans.into(),
            ..Self::default()
        }
    }
}
impl Backend for Script {
    fn now(&mut self) -> Duration {
        self.now
    }
    fn start(&mut self, request: &Request<'_>, context: CallContext, _: Duration) -> Launch {
        assert!(self.active.is_none());
        assert!(self.pending.iter().all(Option::is_none));
        self.contexts.push(context);
        self.actions.push(Action::Start);
        self.args = request.arguments.iter().map(|s| s.to_string()).collect();
        let next = self.plans.pop_front().expect("unexpected launch");
        if let Some(time) = next.start_time {
            self.now = time;
        }
        self.cleaning = false;
        let launch = next.launch;
        if !matches!(launch, Launch::NotStarted(_)) {
            self.active = Some(next);
        }
        launch
    }
    fn submit(&mut self, transfer: Transfer, _: Duration) {
        let i = transfer.kind().index();
        assert!(self.pending[i].is_none());
        self.actions.push(Action::Submit(
            transfer.ticket(),
            transfer.kind(),
            transfer.bytes().len(),
        ));
        self.pending[i] = Some(transfer);
        if let Some(time) = self.active.as_mut().unwrap().submit_time.take() {
            self.now = time;
        }
    }
    fn poll(&mut self, deadline: Duration) -> Batch {
        self.actions.push(Action::Poll);
        let active = self.active.as_mut().unwrap();
        let next = if self.cleaning {
            match active.cleanup.as_mut() {
                Some(steps) => steps.pop_front(),
                None => {
                    let mut s = step(self.now.as_millis() as u64, vec![]);
                    for (i, pending) in self.pending.iter().enumerate() {
                        if pending.is_some() {
                            s.replies.push(done(i, b"", 0, Completion::Cancelled));
                        }
                    }
                    s.cleanup = Some(complete());
                    s.acknowledge = true;
                    Some(s)
                }
            }
        } else {
            active.steps.pop_front()
        };
        let Some(step) = next else {
            self.now = deadline;
            return Batch {
                wake: Wake::Deadline,
                ..Batch::default()
            };
        };
        self.now = step.at;
        let mut batch = Batch {
            exit: step.exit,
            cleanup: step.cleanup,
            wake: step.wake,
            ..Batch::default()
        };
        for (slot, reply) in step.replies.into_iter().enumerate() {
            let mut transfer = self.pending[reply.channel]
                .take()
                .expect("reply without submission");
            if let Some(buffer) = transfer.read_buffer() {
                assert!(reply.bytes.len() <= buffer.len());
                buffer[..reply.bytes.len()].copy_from_slice(&reply.bytes);
            } else if reply.n <= transfer.bytes().len() {
                self.sent.extend_from_slice(&transfer.bytes()[..reply.n]);
            }
            batch.transfers[slot] = Some(Completed {
                transfer,
                confirmed: reply.n,
                result: reply.result,
            });
        }
        if step.acknowledge {
            for (slot, ticket) in self.cancellations.drain(..).enumerate() {
                batch.cancellations[slot] = Some((ticket, CancelReply::Requested));
            }
        }
        if batch.cleanup.is_some() && self.pending.iter().all(Option::is_none) {
            self.active = None;
        }
        batch
    }
    fn cancel(&mut self, ticket: Ticket, _: Duration) {
        assert!(!self.cancellations.contains(&ticket));
        self.actions.push(Action::Cancel(ticket));
        self.cancellations.push(ticket);
    }
    fn begin_cleanup(&mut self, terminate: bool, _: Duration) {
        assert!(!self.cleaning, "cleanup once");
        self.actions.push(Action::Cleanup(terminate));
        self.cleaning = true;
        if let Some(time) = self.active.as_ref().unwrap().begin_time {
            self.now = time;
        }
    }
}
fn run(plan: Plan, request: Request<'_>) -> (Outcome, Script) {
    let mut p = Processes::new(Script::new(vec![plan]), limits());
    let result = p.run(request, 32768, origin());
    (result, p.into_inner().unwrap())
}

#[test]
fn nonzero_exit_and_explicit_next_call_have_distinct_ids() {
    let mut p = Processes::new(Script::new(vec![finished(7), finished(0)]), limits());
    let mut req = request();
    req.arguments = &["", "日本語", "a b", "\"quoted\""];
    let first = p.run(req, 32768, origin());
    let second = p.run(req, 32768, origin());
    assert!(first.completed() && second.completed());
    assert_eq!(first.report.exit, Some(Exit::Code(7)));
    assert_eq!(second.report.exit, Some(Exit::Code(0)));
    assert_ne!(
        first.transfers.unwrap().call,
        second.transfers.unwrap().call
    );
    assert_eq!(first.report.origin, origin());
    let backend = p.into_inner().unwrap();
    assert_eq!(backend.args, req.arguments);
    assert_eq!(
        backend.contexts,
        vec![
            CallContext {
                call: first.transfers.unwrap().call,
                origin: origin()
            },
            CallContext {
                call: second.transfers.unwrap().call,
                origin: origin()
            }
        ]
    );
    assert_eq!(
        backend
            .actions
            .iter()
            .filter(|a| matches!(a, Action::Start))
            .count(),
        2
    );
    assert_eq!(
        backend
            .actions
            .iter()
            .filter(|a| matches!(a, Action::Cleanup(false)))
            .count(),
        2
    );
}

#[test]
fn simultaneous_notifications_use_stream_order_and_keep_all_partial_results() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let replies = order
            .into_iter()
            .map(|i| match i {
                0 => done(0, b"", 2, Completion::Done),
                1 => done(1, b"ab!", 3, Completion::Done),
                _ => done(2, b"e", 1, Completion::Failed),
            })
            .collect();
        let mut req = request();
        req.stdin = b"abc";
        req.stdout_limit = 2;
        let (out, backend) = run(plan(vec![step(1, replies)]), req);
        assert_eq!(code(&out), Some(Code::OutputLimit));
        assert_eq!(
            out.report.failure.unwrap().phase,
            Phase::Transfer(Channel::Stdout)
        );
        assert_eq!(out.report.stdin_accepted, 2);
        assert_eq!(out.report.stdout_read, 3);
        assert_eq!(out.stdout, b"ab");
        assert_eq!(out.stderr, b"e");
        assert_eq!(
            backend
                .actions
                .iter()
                .filter(|a| matches!(a, Action::Submit(..)))
                .count(),
            3
        );
    }
}

#[test]
fn operation_deadline_wins_over_same_batch_failure_but_preserves_bytes_and_exit() {
    let mut s = step(
        100,
        vec![
            done(2, b"err", 3, Completion::Failed),
            done(0, b"", 1, Completion::Done),
            done(1, b"\0\r\n", 3, Completion::Done),
        ],
    );
    s.exit = Some(Ok(Exit::Code(9)));
    let mut req = request();
    req.stdin = b"abc";
    let (out, backend) = run(plan(vec![s]), req);
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.report.stdin_accepted, 1);
    assert_eq!(out.report.exit, Some(Exit::Code(9)));
    assert_eq!(out.stdout, b"\0\r\n");
    assert_eq!(out.stderr, b"err");
    assert_eq!(
        backend
            .actions
            .iter()
            .filter(|a| matches!(a, Action::Cleanup(true)))
            .count(),
        1
    );
}

#[test]
fn timeout_cancels_once_then_collects_late_partial_transfers_and_replies() {
    let mut late = step(
        101,
        vec![
            done(1, b"late", 4, Completion::Done),
            done(2, &[0xff], 1, Completion::Failed),
            done(0, b"", 2, Completion::Cancelled),
        ],
    );
    late.acknowledge = true;
    late.exit = Some(Ok(Exit::Signal(9)));
    late.cleanup = Some(complete());
    let mut next = plan(vec![]);
    next.cleanup = Some(vec![late].into());
    let mut req = request();
    req.stdin = b"abc";
    let mut p = Processes::with_trace_limit(Script::new(vec![next]), limits(), 100);
    let out = p.run(req, 32768, origin());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.stdout, b"late");
    assert_eq!(out.stderr, [0xff]);
    assert_eq!(out.report.stdin_accepted, 2);
    assert!(out.report.progress_known);
    assert!(!p.is_disabled());
    assert!(p.events().iter().any(|e| !e.report.progress_known));
    let b = p.into_inner().unwrap();
    assert_eq!(
        b.actions
            .iter()
            .filter(|a| matches!(a, Action::Cancel(_)))
            .count(),
        3
    );
    assert!(b.pending.iter().all(Option::is_none));
}

#[test]
fn cleanup_complete_without_transfer_notifications_disables_connection_and_keeps_buffers() {
    let mut s = step(101, vec![]);
    s.cleanup = Some(complete());
    s.acknowledge = true;
    let mut next = plan(vec![]);
    next.cleanup = Some(vec![s].into());
    let mut p = Processes::new(Script::new(vec![next, finished(0)]), limits());
    let out = p.run(request(), 32768, origin());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert!(!out.report.progress_known);
    assert!(p.is_disabled());
    assert!(
        out.transfers
            .unwrap()
            .operations
            .iter()
            .all(|o| o.unwrap().state == State::Submitted)
    );
    let before = p.backend.as_ref().unwrap().actions.len();
    let again = p.run(request(), 32768, origin());
    assert_eq!(code(&again), Some(Code::ConnectionDisabled));
    assert_eq!(p.backend.as_ref().unwrap().actions.len(), before);
    assert_eq!(
        p.into_inner()
            .unwrap()
            .pending
            .iter()
            .filter(|p| p.is_some())
            .count(),
        3
    );
}

#[test]
fn normal_completion_waits_for_both_eofs_and_child_exit() {
    let mut first = step(1, vec![done(0, b"", 0, Completion::Done)]);
    first.exit = Some(Ok(Exit::Code(0)));
    let (out, _) = run(plan(vec![first]), request());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.report.exit, Some(Exit::Code(0)));
}

#[test]
fn exact_and_zero_output_limits_require_eof_and_probe_is_excluded() {
    for limit in [0, 2] {
        for overflow in [false, true] {
            let mut steps = vec![step(
                1,
                vec![
                    done(0, b"", 0, Completion::Done),
                    done(2, b"", 0, Completion::Eof),
                ],
            )];
            if limit != 0 {
                steps[0].replies.push(done(1, b"ab", 2, Completion::Done));
            }
            let mut end = step(
                2,
                vec![done(
                    1,
                    if overflow { b"!" } else { b"" },
                    usize::from(overflow),
                    if overflow {
                        Completion::Done
                    } else {
                        Completion::Eof
                    },
                )],
            );
            end.exit = Some(Ok(Exit::Code(0)));
            steps.push(end);
            let mut req = request();
            req.stdout_limit = limit;
            let (out, b) = run(plan(steps), req);
            assert_eq!(code(&out), overflow.then_some(Code::OutputLimit));
            assert_eq!(out.stdout, &b"ab"[..limit]);
            assert_eq!(out.report.stdout_read, limit + usize::from(overflow));
            assert!(
                b.actions
                    .iter()
                    .any(|a| matches!(a, Action::Submit(_, Kind::Read(Channel::Stdout), 1)))
            );
        }
    }
}

#[test]
fn short_input_is_resubmitted_only_from_confirmed_position_then_closed() {
    let mut final_step = step(4, vec![done(0, b"", 0, Completion::Done)]);
    final_step.exit = Some(Ok(Exit::Code(0)));
    let steps = vec![
        step(
            1,
            vec![
                done(0, b"", 1, Completion::Done),
                done(1, b"", 0, Completion::Eof),
                done(2, b"", 0, Completion::Eof),
            ],
        ),
        step(2, vec![done(0, b"", 2, Completion::Done)]),
        final_step,
    ];
    let mut req = request();
    req.stdin = b"abc";
    let (out, b) = run(plan(steps), req);
    assert!(out.completed());
    assert_eq!(b.sent, b"abc");
    let offered: Vec<_> = b
        .actions
        .iter()
        .filter_map(|a| match a {
            Action::Submit(_, Kind::Write, n) => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(offered, [3, 2]);
}

#[test]
fn launch_failure_is_recoverable_and_uncertain_launch_is_cleaned_once() {
    for launch in [
        Launch::NotStarted(StartError::Denied),
        Launch::Uncertain(StartError::Failed),
    ] {
        let mut next = plan(vec![]);
        next.launch = launch;
        let mut p = Processes::new(Script::new(vec![next, finished(0)]), limits());
        let first = p.run(request(), 32768, origin());
        assert_eq!(
            first.report.started,
            if matches!(launch, Launch::Uncertain(_)) {
                Started::Unknown
            } else {
                Started::No
            }
        );
        assert!(p.run(request(), 32768, origin()).completed());
        let b = p.into_inner().unwrap();
        assert_eq!(
            b.actions
                .iter()
                .filter(|a| matches!(a, Action::Cleanup(_)))
                .count(),
            if matches!(launch, Launch::Uncertain(_)) {
                2
            } else {
                1
            }
        );
    }
}

#[test]
fn cleanup_deadline_keeps_returned_progress_and_reclaimed_flags() {
    let mut next = plan(vec![]);
    let mut late = step(
        110,
        vec![
            done(0, b"", 0, Completion::Cancelled),
            done(1, b"late", 4, Completion::Done),
            done(2, b"", 0, Completion::Cancelled),
        ],
    );
    late.cleanup = Some(complete());
    next.cleanup = Some(vec![late].into());
    let mut p = Processes::new(Script::new(vec![next]), limits());
    let out = p.run(request(), 32768, origin());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.stdout, b"late");
    assert_eq!(
        out.report.cleanup,
        Some(Cleanup::Incomplete {
            child_reaped: true,
            handles_closed: true,
            scope_finished: true,
            error: CleanupError::Timeout,
        })
    );
    assert!(p.is_disabled());
}

#[test]
fn cleanup_failure_after_success_is_not_a_successful_operation() {
    let mut next = finished(0);
    let mut clean = step(2, vec![]);
    clean.cleanup = Some(Cleanup::Incomplete {
        child_reaped: true,
        handles_closed: false,
        scope_finished: false,
        error: CleanupError::Close,
    });
    next.cleanup = Some(vec![clean].into());
    let (out, _) = run(next, request());
    assert_eq!(code(&out), Some(Code::CleanupFailed));
    assert_eq!(out.report.exit, Some(Exit::Code(0)));
}

#[test]
fn empty_or_early_deadline_batches_are_protocol_failures() {
    for wake in [Wake::Events, Wake::Deadline] {
        let mut invalid = step(1, vec![]);
        invalid.wake = wake;
        let mut p = Processes::new(Script::new(vec![plan(vec![invalid])]), limits());
        let out = p.run(request(), 32768, origin());
        assert_eq!(code(&out), Some(Code::InvalidResponse));
        assert!(p.is_disabled());
    }
}

#[test]
fn invalid_counts_and_exit_notifications_do_not_block_other_stream_collection() {
    let mut invalid = step(
        1,
        vec![
            done(1, b"", 34, Completion::Done),
            done(2, b"ok", 2, Completion::Done),
        ],
    );
    invalid.exit = Some(Ok(Exit::Signal(0)));
    let mut p = Processes::new(Script::new(vec![plan(vec![invalid])]), limits());
    let out = p.run(request(), 32768, origin());
    assert_eq!(code(&out), Some(Code::InvalidResponse));
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr, b"ok");
    assert!(!out.report.progress_known);
    assert!(p.is_disabled());
}

#[test]
fn wait_failure_and_duplicate_exit_are_distinct_from_nonzero_exit() {
    for duplicate in [false, true] {
        let mut a = step(1, vec![]);
        a.exit = Some(Ok(Exit::Code(7)));
        let mut b = step(2, vec![]);
        b.exit = Some(Ok(Exit::Code(7)));
        let steps = if duplicate {
            vec![a, b]
        } else {
            a.exit = Some(Err(WaitError::Failed));
            vec![a]
        };
        let (out, _) = run(plan(steps), request());
        assert_eq!(
            code(&out),
            Some(if duplicate {
                Code::InvalidResponse
            } else {
                Code::WaitFailed
            })
        );
    }
}

#[test]
fn startup_and_submission_timeouts_prevent_further_submissions() {
    for startup in [false, true] {
        let mut next = plan(vec![]);
        if startup {
            next.start_time = Some(ms(100));
        } else {
            next.submit_time = Some(ms(100));
        }
        let (out, b) = run(next, request());
        assert_eq!(code(&out), Some(Code::Timeout));
        assert_eq!(
            b.actions
                .iter()
                .filter(|a| matches!(a, Action::Submit(..)))
                .count(),
            usize::from(!startup)
        );
        assert_eq!(
            b.actions
                .iter()
                .filter(|a| matches!(a, Action::Cleanup(true)))
                .count(),
            1
        );
    }
}

#[test]
fn backwards_clock_is_not_hidden_by_cleanup_recovery() {
    let mut next = plan(vec![
        step(3, vec![done(1, b"x", 1, Completion::Done)]),
        step(2, vec![]),
    ]);
    next.begin_time = Some(ms(4));
    let mut p = Processes::new(Script::new(vec![next]), limits());
    let out = p.run(request(), 32768, origin());
    assert_eq!(code(&out), Some(Code::InvalidResponse));
    assert_eq!(out.stdout, b"x");
    assert!(p.is_disabled());
}

#[test]
fn validation_buffer_budget_allocation_and_id_exhaustion_do_not_launch() {
    let mut p = Processes::new(Script::new(vec![]), limits());
    let mut req = request();
    req.target = "";
    assert_eq!(
        code(&p.run(req, 32768, origin())),
        Some(Code::InvalidRequest)
    );
    req = request();
    req.arguments = &["a\0b"];
    assert_eq!(
        code(&p.run(req, 32768, origin())),
        Some(Code::InvalidRequest)
    );
    req = request();
    req.stdout_limit = usize::MAX;
    assert_eq!(
        code(&p.run(req, usize::MAX, origin())),
        Some(Code::RequestLimit)
    );
    assert_eq!(
        code(&p.run(request(), 3 * CHUNK + 63, origin())),
        Some(Code::OutputBudget)
    );
    p.fail_allocation = true;
    assert_eq!(
        code(&p.run(request(), 32768, origin())),
        Some(Code::AllocationFailed)
    );
    p.sequence = u64::MAX;
    assert_eq!(
        code(&p.run(request(), 32768, origin())),
        Some(Code::ConnectionDisabled)
    );
    assert!(p.into_inner().unwrap().actions.is_empty());
    let mut p = Processes::<Script>::unconfigured(limits());
    assert_eq!(
        code(&p.run(request(), 32768, origin())),
        Some(Code::Unconfigured)
    );
}

#[test]
fn traces_are_read_only_bounded_and_do_not_change_results_or_control() {
    let mut baseline = None;
    for (limit, fail) in [(0, false), (1, false), (100, false), (100, true)] {
        let mut p = Processes::with_trace_limit(Script::new(vec![finished(7)]), limits(), limit);
        p.fail_record_allocation = fail;
        let out = p.run(request(), 32768, origin());
        assert_eq!(p.events(), p.events());
        assert!(p.events().len() <= limit);
        if limit == 1 || fail {
            assert!(p.trace_truncated());
        }
        let actions = p.into_inner().unwrap().actions;
        if let Some((expected, expected_actions)) = &baseline {
            assert_eq!(&out, expected);
            assert_eq!(&actions, expected_actions);
        } else {
            baseline = Some((out, actions));
        }
    }
}

#[test]
fn results_match_the_previous_synchronous_model_for_shared_cases() {
    let mut output = finished(0);
    output
        .steps
        .push_front(step(0, vec![done(1, b"ab", 2, Completion::Done)]));
    let mut exited = step(1, vec![]);
    exited.exit = Some(Ok(Exit::Code(0)));
    let overflow = plan(vec![step(
        1,
        vec![done(1, &[b'x'; 33], 33, Completion::Done)],
    )]);
    let plans = [finished(7), output, plan(vec![exited]), overflow];
    for (plan, previous) in plans
        .into_iter()
        .zip(super::synchronous::tests::comparison_cases())
    {
        let (current, _) = run(plan, request());
        assert_eq!(current.report, previous.report);
        assert_eq!(current.stdout, previous.stdout);
        assert_eq!(current.stderr, previous.stderr);
    }
}

#[test]
fn terminal_transfers_do_not_hide_missing_cancellation_replies() {
    for acknowledge in [false, true] {
        let late = step(
            101,
            vec![
                done(0, b"", 0, Completion::Cancelled),
                done(1, b"ok", 2, Completion::Done),
                done(2, b"", 0, Completion::Cancelled),
            ],
        );
        let mut settled = step(102, vec![]);
        settled.acknowledge = acknowledge;
        settled.cleanup = Some(complete());
        let mut next = plan(vec![]);
        next.cleanup = Some(vec![late, settled].into());
        let mut p = Processes::new(Script::new(vec![next]), limits());
        let out = p.run(request(), 32768, origin());
        assert_eq!(code(&out), Some(Code::Timeout));
        assert_eq!(out.stdout, b"ok");
        assert!(out.report.progress_known);
        assert_eq!(p.is_disabled(), !acknowledge);
        assert_eq!(out.transfers.unwrap().protocol_failed, !acknowledge);
    }
}

#[test]
fn unknown_progress_keeps_prefix_and_never_allows_reuse() {
    let next = plan(vec![step(
        1,
        vec![done(1, b"part", 4, Completion::ProgressUnknown)],
    )]);
    let mut p = Processes::new(Script::new(vec![next]), limits());
    let out = p.run(request(), 32768, origin());
    assert_eq!(code(&out), Some(Code::ProgressUnknown));
    assert_eq!(out.stdout, b"part");
    assert_eq!(out.report.stdout_read, 4);
    assert!(!out.report.progress_known);
    assert!(p.is_disabled());
}

#[test]
fn rejected_close_is_reported_as_close_failure() {
    let (out, _) = run(
        plan(vec![step(
            1,
            vec![done(0, b"", 0, Completion::NotSubmitted)],
        )]),
        request(),
    );
    assert_eq!(code(&out), Some(Code::CloseFailed));
    assert_eq!(out.report.failure.unwrap().phase, Phase::CloseStdin);
}

#[test]
fn large_stdin_uses_bounded_chunks_and_exactly_one_close() {
    let mut last = step(3, vec![done(0, b"", 0, Completion::Done)]);
    last.exit = Some(Ok(Exit::Code(0)));
    let next = plan(vec![
        step(
            1,
            vec![
                done(0, b"", CHUNK, Completion::Done),
                done(1, b"", 0, Completion::Eof),
                done(2, b"", 0, Completion::Eof),
            ],
        ),
        step(2, vec![done(0, b"", 1, Completion::Done)]),
        last,
    ]);
    let input = vec![b'x'; CHUNK + 1];
    let mut req = request();
    req.stdin = &input;
    let (out, b) = run(next, req);
    assert!(out.completed());
    assert_eq!(b.sent, input);
    assert_eq!(
        b.actions
            .iter()
            .filter(|a| matches!(a, Action::Submit(_, Kind::CloseStdin, 0)))
            .count(),
        1
    );
    assert!(
        b.actions
            .iter()
            .all(|a| !matches!(a, Action::Submit(_, _, n) if *n > CHUNK))
    );
}

#[test]
fn launch_deadline_also_applies_when_non_launch_is_confirmed() {
    let mut next = plan(vec![]);
    next.launch = Launch::NotStarted(StartError::Denied);
    next.start_time = Some(ms(100));
    let (out, b) = run(next, request());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.report.started, Started::No);
    assert_eq!(out.transfers.unwrap().failure, out.report.failure);
    assert_eq!(out.report.cleanup, None);
    assert_eq!(b.actions, [Action::Start]);
}

#[test]
fn cleanup_report_is_preserved_separately_from_deadline_verdict() {
    let mut next = finished(0);
    let mut late = step(11, vec![]);
    late.cleanup = Some(Cleanup::Complete {
        termination_requested: true,
    });
    next.cleanup = Some(vec![late].into());
    let (out, _) = run(next, request());
    assert_eq!(code(&out), Some(Code::CleanupFailed));
    assert_eq!(
        out.report.cleanup_reported,
        Some(Cleanup::Complete {
            termination_requested: true
        })
    );
    assert_eq!(
        out.report.cleanup,
        Some(Cleanup::Incomplete {
            child_reaped: true,
            handles_closed: true,
            scope_finished: true,
            error: CleanupError::Timeout,
        })
    );
}
