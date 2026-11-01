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
fn incomplete() -> Cleanup {
    Cleanup::Incomplete {
        child_reaped: false,
        handles_closed: true,
        scope_finished: false,
        error: CleanupError::Reap,
    }
}
enum Action {
    Event(PollEvent),
    Output(Channel, Vec<u8>),
}
struct Plan {
    launch: Launch,
    steps: VecDeque<(Duration, Action)>,
    cleanup: Cleanup,
    start_time: Option<Duration>,
    cleanup_time: Option<Duration>,
}
fn plan(events: Vec<PollEvent>) -> Plan {
    Plan {
        launch: Launch::Started,
        steps: events
            .into_iter()
            .enumerate()
            .map(|(i, e)| (ms(i as u64 + 1), Action::Event(e)))
            .collect(),
        cleanup: complete(),
        start_time: None,
        cleanup_time: None,
    }
}
fn finished(code: u32) -> Plan {
    plan(vec![
        PollEvent::StdinClosed,
        PollEvent::Read(Channel::Stdout, 0),
        PollEvent::Read(Channel::Stderr, 0),
        PollEvent::Exited(Exit::Code(code)),
    ])
}
#[derive(Default)]
struct Script {
    plans: VecDeque<Plan>,
    active: Option<Plan>,
    unreclaimed: Option<Plan>,
    now: Duration,
    starts: usize,
    polls: usize,
    cleanups: Vec<(bool, Duration)>,
    sent: Vec<u8>,
    args: Vec<String>,
    offers: Vec<(usize, usize, usize)>,
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
    fn start(&mut self, request: &Request<'_>, _: Duration) -> Launch {
        assert!(
            self.active.is_none() && self.unreclaimed.is_none(),
            "no overlapping starts"
        );
        self.starts += 1;
        self.args = request.arguments.iter().map(|s| (*s).to_owned()).collect();
        let next = self.plans.pop_front().expect("unexpected launch");
        let launch = next.launch;
        if let Some(time) = next.start_time {
            self.now = time;
        }
        if !matches!(launch, Launch::NotStarted(_)) {
            self.active = Some(next);
        }
        launch
    }
    fn poll(&mut self, request: Poll<'_>) -> PollEvent {
        self.polls += 1;
        self.offers.push((
            request.stdin.len(),
            request.stdout.len(),
            request.stderr.len(),
        ));
        let Some((time, action)) = self.active.as_mut().unwrap().steps.pop_front() else {
            self.now = request.deadline;
            return PollEvent::Deadline;
        };
        self.now = time;
        match action {
            Action::Event(PollEvent::Written(n)) => {
                if n <= request.stdin.len() {
                    self.sent.extend_from_slice(&request.stdin[..n]);
                }
                PollEvent::Written(n)
            }
            Action::Event(event) => event,
            Action::Output(channel, bytes) => {
                let into = match channel {
                    Channel::Stdout => request.stdout,
                    Channel::Stderr => request.stderr,
                    _ => panic!(),
                };
                assert!(bytes.len() <= into.len(), "fixture exceeds requested read");
                into[..bytes.len()].copy_from_slice(&bytes);
                PollEvent::Read(channel, bytes.len())
            }
        }
    }
    fn cleanup(&mut self, terminate: bool, deadline: Duration) -> Cleanup {
        self.cleanups.push((terminate, deadline));
        let plan = self.active.take().expect("cleanup exactly once");
        if let Some(time) = plan.cleanup_time {
            self.now = time;
        }
        let cleanup = plan.cleanup;
        if matches!(cleanup, Cleanup::Incomplete { .. }) {
            self.unreclaimed = Some(plan);
        }
        cleanup
    }
}
fn run(script: Script, request: Request<'_>) -> (Outcome, Script) {
    let mut p = Processes::new(script, limits());
    let out = p.run(request, 16384, origin());
    (out, p.into_inner().unwrap())
}
fn code(out: &Outcome) -> Option<Code> {
    out.report.failure.map(|f| f.code)
}

#[test]
fn nonzero_exit_is_a_completed_operation_and_another_call_can_follow() {
    let mut second = finished(0);
    for (time, _) in &mut second.steps {
        *time += ms(10);
    }
    let mut p =
        Processes::with_trace_limit(Script::new(vec![finished(u32::MAX), second]), limits(), 100);
    let first = p.run(request(), 64, origin());
    assert!(first.completed());
    assert_eq!(first.report.exit, Some(Exit::Code(u32::MAX)));
    assert!(p.run(request(), 64, origin()).completed());
    assert_eq!(p.events().last().unwrap().sequence, 2);
    assert!(p.events().iter().all(|e| e.report.origin == origin()));
    let backend = p.into_inner().unwrap();
    assert_eq!(backend.starts, 2);
    assert_eq!(
        backend.cleanups.iter().map(|x| x.0).collect::<Vec<_>>(),
        [false, false]
    );
}

#[test]
fn failure_retains_binary_output_and_accepted_input_without_automatic_retry() {
    let mut first = plan(vec![
        PollEvent::Written(2),
        PollEvent::TransferFailed(Channel::Stdin),
    ]);
    first.steps.insert(
        1,
        (ms(1), Action::Output(Channel::Stdout, vec![0, 255, 13, 10])),
    );
    let mut second = finished(7);
    for (t, _) in &mut second.steps {
        *t += ms(10);
    }
    let mut p = Processes::new(Script::new(vec![first, second]), limits());
    let req = Request {
        stdin: b"abcdef",
        ..request()
    };
    let first = p.run(req, 64, origin());
    assert_eq!(code(&first), Some(Code::TransferFailed));
    assert_eq!(
        first.report.failure.unwrap().phase,
        Phase::Transfer(Channel::Stdin)
    );
    assert_eq!(first.report.stdin_accepted, 2);
    assert_eq!(first.stdout, [0, 255, 13, 10]);
    assert!(!p.is_disabled());
    let mut copy = first.clone();
    copy.stdout[0] = 42;
    assert_eq!(first.stdout[0], 0);
    assert!(p.run(request(), 64, origin()).completed());
    let backend = p.into_inner().unwrap();
    assert_eq!(backend.sent, b"ab");
    assert_eq!(backend.starts, 2);
    assert_eq!(
        backend.cleanups.iter().map(|x| x.0).collect::<Vec<_>>(),
        [true, false]
    );
}

#[test]
fn exact_limits_require_eof_and_overflow_retains_only_the_limit() {
    for channel in [Channel::Stdout, Channel::Stderr] {
        for (limit, bytes, expected) in [
            (0, vec![], None),
            (3, b"abc".to_vec(), None),
            (0, vec![255], Some(Code::OutputLimit)),
            (3, b"abcd".to_vec(), Some(Code::OutputLimit)),
        ] {
            let mut script = finished(0);
            if !bytes.is_empty() {
                script
                    .steps
                    .push_front((ms(0), Action::Output(channel, bytes.clone())));
            }
            let req = Request {
                stdout_limit: limit,
                stderr_limit: limit,
                ..request()
            };
            let (out, backend) = run(Script::new(vec![script]), req);
            assert_eq!(code(&out), expected);
            let (retained, read) = if channel == Channel::Stdout {
                (&out.stdout, out.report.stdout_read)
            } else {
                (&out.stderr, out.report.stderr_read)
            };
            assert_eq!(retained, &bytes[..bytes.len().min(limit)]);
            assert_eq!(read, bytes.len());
            assert_eq!(backend.cleanups.len(), 1);
            if expected.is_none() {
                assert_eq!(out.report.exit, Some(Exit::Code(0)));
            }
        }
    }
}

#[test]
fn stdin_and_both_outputs_can_progress_before_child_exit() {
    let mut p = plan(vec![]);
    p.steps = [
        (
            ms(1),
            Action::Output(Channel::Stdout, b"before-input".to_vec()),
        ),
        (ms(2), Action::Output(Channel::Stderr, b"warning".to_vec())),
        (ms(3), Action::Event(PollEvent::Written(1))),
        (ms(4), Action::Event(PollEvent::Interrupted)),
        (ms(5), Action::Event(PollEvent::Written(2))),
        (ms(6), Action::Event(PollEvent::StdinClosed)),
        (ms(7), Action::Event(PollEvent::Exited(Exit::Signal(9)))),
        (ms(8), Action::Event(PollEvent::Read(Channel::Stdout, 0))),
        (ms(9), Action::Event(PollEvent::Read(Channel::Stderr, 0))),
    ]
    .into();
    let req = Request {
        stdin: b"abc",
        arguments: &["", "a b", "\"quoted\"", "日本語"],
        ..request()
    };
    let (out, backend) = run(Script::new(vec![p]), req);
    assert!(out.completed());
    assert_eq!(out.stdout, b"before-input");
    assert_eq!(out.stderr, b"warning");
    assert_eq!(out.report.exit, Some(Exit::Signal(9)));
    assert_eq!(backend.sent, b"abc");
    assert_eq!(backend.args, req.arguments);
    assert!(backend.offers[0].0 > 0 && backend.offers[0].1 > 0 && backend.offers[0].2 > 0);
}

#[test]
fn timeout_preserves_progress_and_child_exit_does_not_imply_pipe_eof() {
    let mut p = plan(vec![PollEvent::Exited(Exit::Code(0))]);
    p.steps
        .push_back((ms(99), Action::Output(Channel::Stdout, b"partial".to_vec())));
    let (out, backend) = run(Script::new(vec![p]), request());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(out.stdout, b"partial");
    assert_eq!(out.report.exit, Some(Exit::Code(0)));
    assert_eq!(backend.cleanups, [(true, ms(110))]);
}

#[test]
fn deadline_wins_over_same_poll_error_but_transferred_bytes_are_kept() {
    for action in [
        Action::Output(Channel::Stdout, b"late".to_vec()),
        Action::Event(PollEvent::TransferFailed(Channel::Stdin)),
    ] {
        let mut p = plan(vec![]);
        p.steps.push_back((ms(100), action));
        let req = Request {
            stdin: b"x",
            ..request()
        };
        let (out, _) = run(Script::new(vec![p]), req);
        assert_eq!(code(&out), Some(Code::Timeout));
        if !out.stdout.is_empty() {
            assert_eq!(out.stdout, b"late");
        }
    }
}

#[test]
fn incomplete_cleanup_preserves_original_error_and_disables_only_this_connection() {
    let mut first = plan(vec![PollEvent::TransferFailed(Channel::Stdout)]);
    first.cleanup = incomplete();
    let mut p = Processes::with_trace_limit(Script::new(vec![first]), limits(), 32);
    let result = p.run(request(), 64, origin());
    assert_eq!(code(&result), Some(Code::TransferFailed));
    assert_eq!(result.report.cleanup, Some(incomplete()));
    assert!(p.is_disabled());
    let second = p.run(request(), 64, origin());
    assert_eq!(code(&second), Some(Code::ConnectionDisabled));
    assert_eq!(second.report.started, Started::No);
    assert_eq!(second.report.cleanup, None);
    let backend = p.into_inner().unwrap();
    assert_eq!(backend.starts, 1);
    assert_eq!(backend.cleanups.len(), 1);
    assert!(backend.unreclaimed.is_some());
    assert!(run(Script::new(vec![finished(0)]), request()).0.completed());
}

#[test]
fn cleanup_failure_after_normal_exit_is_not_success() {
    let mut p = finished(0);
    p.cleanup = incomplete();
    let (out, _) = run(Script::new(vec![p]), request());
    assert_eq!(code(&out), Some(Code::CleanupFailed));
    assert_eq!(out.report.exit, Some(Exit::Code(0)));
}

#[test]
fn known_launch_failure_can_be_retried_explicitly_uncertain_launch_is_cleaned_once() {
    for uncertain in [false, true] {
        let mut first = plan(vec![]);
        first.launch = if uncertain {
            Launch::Uncertain(StartError::Resources)
        } else {
            Launch::NotStarted(StartError::Resources)
        };
        let mut p = Processes::new(Script::new(vec![first, finished(0)]), limits());
        let out = p.run(request(), 64, origin());
        assert_eq!(code(&out), Some(Code::Resources));
        assert_eq!(
            out.report.started,
            if uncertain {
                Started::Unknown
            } else {
                Started::No
            }
        );
        assert_eq!(out.report.cleanup.is_some(), uncertain);
        assert!(p.run(request(), 64, origin()).completed());
        let backend = p.into_inner().unwrap();
        assert_eq!(backend.starts, 2);
        assert_eq!(backend.cleanups.len(), if uncertain { 2 } else { 1 });
    }
}

#[test]
fn invalid_responses_are_not_interpreted_as_progress_or_completion() {
    for event in [
        PollEvent::Written(9),
        PollEvent::StdinClosed,
        PollEvent::Read(Channel::Stdin, 1),
        PollEvent::Read(Channel::Stdout, 34),
        PollEvent::Exited(Exit::Signal(0)),
        PollEvent::Deadline,
    ] {
        let (out, backend) = run(
            Script::new(vec![plan(vec![event])]),
            Request {
                stdin: b"abc",
                ..request()
            },
        );
        assert_eq!(code(&out), Some(Code::InvalidResponse), "{event:?}");
        assert!(!out.report.progress_known);
        assert_eq!(out.report.stdin_accepted, 0);
        assert_eq!(backend.cleanups.len(), 1);
    }
    let (out, _) = run(
        Script::new(vec![plan(vec![PollEvent::Written(0)])]),
        Request {
            stdin: b"abc",
            ..request()
        },
    );
    assert_eq!(code(&out), Some(Code::ZeroProgress));
    assert!(out.report.progress_known);
}

#[test]
fn backwards_clock_and_start_timeout_stop_without_more_transfers() {
    let mut p = plan(vec![]);
    p.start_time = Some(ms(100));
    let (out, backend) = run(Script::new(vec![p]), request());
    assert_eq!(code(&out), Some(Code::Timeout));
    assert_eq!(backend.polls, 0);
    let mut p = plan(vec![]);
    p.steps
        .push_back((ms(3), Action::Event(PollEvent::Interrupted)));
    p.steps
        .push_back((ms(2), Action::Event(PollEvent::Interrupted)));
    let (out, backend) = run(Script::new(vec![p]), request());
    assert_eq!(code(&out), Some(Code::InvalidResponse));
    assert_eq!(backend.polls, 2);
    assert_eq!(backend.cleanups.len(), 1);
}

#[test]
fn validation_and_reservation_fail_before_any_launch_and_do_not_poison_connection() {
    let cases = [
        (
            Request {
                target: "",
                ..request()
            },
            64,
            Code::InvalidRequest,
        ),
        (
            Request {
                arguments: &["a\0b"],
                ..request()
            },
            64,
            Code::InvalidRequest,
        ),
        (
            Request {
                timeout: Duration::ZERO,
                ..request()
            },
            64,
            Code::InvalidRequest,
        ),
        (
            Request {
                timeout: ms(101),
                ..request()
            },
            64,
            Code::RequestLimit,
        ),
        (
            Request {
                stdout_limit: usize::MAX,
                ..request()
            },
            64,
            Code::RequestLimit,
        ),
        (request(), 63, Code::OutputBudget),
    ];
    for (req, available, expected) in cases {
        let mut p = Processes::new(Script::new(vec![finished(0)]), limits());
        assert_eq!(code(&p.run(req, available, origin())), Some(expected));
        assert!(p.run(request(), 64, origin()).completed());
        assert_eq!(p.into_inner().unwrap().starts, 1);
    }
    let mut p = Processes::<Script>::unconfigured(limits());
    assert_eq!(
        code(&p.run(request(), 64, origin())),
        Some(Code::Unconfigured)
    );
    let mut p = Processes::new(Script::new(vec![finished(0)]), limits());
    p.fail_allocation = true;
    assert_eq!(
        code(&p.run(request(), 64, origin())),
        Some(Code::AllocationFailed)
    );
    assert!(p.run(request(), 64, origin()).completed());
    assert_eq!(p.into_inner().unwrap().starts, 1);
}

#[test]
fn observation_is_read_only_bounded_and_independent_of_results() {
    let mut results = Vec::new();
    for (trace_limit, allocation_failure) in [(0, false), (1, false), (32, true), (32, false)] {
        let mut p =
            Processes::with_trace_limit(Script::new(vec![finished(7)]), limits(), trace_limit);
        p.fail_record_allocation = allocation_failure;
        let outcome = p.run(
            Request {
                target: "private-target",
                arguments: &["private-argument"],
                ..request()
            },
            64,
            origin(),
        );
        let events = format!("{:?}", p.events());
        assert!(!events.contains("private"));
        assert!(p.events().len() <= trace_limit);
        assert_eq!(p.trace_truncated(), trace_limit == 1 || allocation_failure);
        let backend = p.into_inner().unwrap();
        results.push((outcome, backend.starts, backend.polls, backend.cleanups));
    }
    assert!(results.windows(2).all(|r| r[0] == r[1]));
}

#[test]
fn large_requests_use_bounded_chunks_and_capacity_includes_both_outputs() {
    let input = vec![42; CHUNK + 1];
    let mut p = finished(0);
    p.steps
        .push_front((ms(0), Action::Event(PollEvent::Written(1))));
    p.steps
        .push_front((ms(0), Action::Event(PollEvent::Written(CHUNK))));
    let req = Request {
        stdin: &input,
        stdout_limit: 8192,
        stderr_limit: 8192,
        ..request()
    };
    let (out, backend) = run(Script::new(vec![p]), req);
    assert!(out.completed());
    assert_eq!(backend.sent, input);
    assert!(
        backend
            .offers
            .iter()
            .all(|&(i, o, e)| i <= CHUNK && o <= CHUNK && e <= CHUNK)
    );
}

#[test]
fn close_wait_and_duplicate_terminal_responses_have_distinct_failures() {
    let cases = [
        (vec![PollEvent::CloseFailed], Code::CloseFailed),
        (vec![PollEvent::WaitFailed], Code::WaitFailed),
        (
            vec![PollEvent::StdinClosed, PollEvent::StdinClosed],
            Code::InvalidResponse,
        ),
        (
            vec![
                PollEvent::Read(Channel::Stdout, 0),
                PollEvent::Read(Channel::Stdout, 0),
            ],
            Code::InvalidResponse,
        ),
        (
            vec![
                PollEvent::Exited(Exit::Code(1)),
                PollEvent::Exited(Exit::Code(2)),
            ],
            Code::InvalidResponse,
        ),
    ];
    for (events, expected) in cases {
        let (out, backend) = run(Script::new(vec![plan(events)]), request());
        assert_eq!(code(&out), Some(expected));
        assert_eq!(backend.cleanups.len(), 1);
    }
}

#[test]
fn request_limits_and_unrepresentable_deadlines_prevent_launch() {
    let args = ["x"; 9];
    let long = "x".repeat(129);
    let input = vec![0; 8193];
    for req in [
        Request {
            target: "a\0b",
            ..request()
        },
        Request {
            arguments: &args,
            ..request()
        },
        Request {
            arguments: &[&long],
            ..request()
        },
        Request {
            stdin: &input,
            ..request()
        },
        Request {
            stdout_limit: 16384,
            ..request()
        },
    ] {
        let (out, backend) = run(Script::default(), req);
        assert!(matches!(
            code(&out),
            Some(Code::RequestLimit | Code::InvalidRequest)
        ));
        assert_eq!(backend.starts, 0);
    }
    let backend = Script {
        now: Duration::MAX,
        ..Script::default()
    };
    let (out, backend) = run(backend, request());
    assert_eq!(code(&out), Some(Code::InvalidRequest));
    assert_eq!(backend.starts, 0);
}

#[test]
fn cleanup_deadline_or_backwards_clock_cannot_claim_success() {
    for time in [ms(0), ms(14)] {
        let mut next = finished(0);
        next.cleanup_time = Some(time);
        let mut p = Processes::new(Script::new(vec![next]), limits());
        let out = p.run(request(), 64, origin());
        assert_eq!(code(&out), Some(Code::CleanupFailed));
        assert!(matches!(
            out.report.cleanup,
            Some(Cleanup::Incomplete {
                child_reaped: true,
                handles_closed: true,
                scope_finished: true,
                ..
            })
        ));
        assert!(p.is_disabled());
        assert_eq!(
            code(&p.run(request(), 64, origin())),
            Some(Code::ConnectionDisabled)
        );
        assert_eq!(p.into_inner().unwrap().starts, 1);
    }
}

#[test]
fn trace_records_offered_counts_before_the_response_changes_state() {
    let mut next = finished(0);
    next.steps
        .push_front((ms(0), Action::Output(Channel::Stdout, b"abc".to_vec())));
    let mut p = Processes::with_trace_limit(Script::new(vec![next]), limits(), 32);
    let out = p.run(
        Request {
            stdout_limit: 3,
            ..request()
        },
        64,
        origin(),
    );
    assert!(out.completed());
    let reads: Vec<_> = p
        .events()
        .iter()
        .filter(|e| e.phase == Phase::Transfer(Channel::Stdout))
        .collect();
    assert_eq!(reads[0].offered.unwrap().stdout, 4);
    assert_eq!(reads[0].report.stdout_retained, 3);
    assert_eq!(reads[1].offered.unwrap().stdout, 1);
    let exit = p
        .events()
        .iter()
        .find(|e| e.offered.is_some() && e.report.exit.is_some())
        .unwrap();
    assert!(exit.offered.unwrap().wait_for_exit);
}

#[test]
fn clock_recovery_during_cleanup_does_not_hide_earlier_regression() {
    let mut next = plan(vec![]);
    next.steps
        .push_back((ms(3), Action::Event(PollEvent::Interrupted)));
    next.steps
        .push_back((ms(2), Action::Event(PollEvent::Interrupted)));
    next.cleanup_time = Some(ms(4));
    let mut p = Processes::new(Script::new(vec![next]), limits());
    let out = p.run(request(), 64, origin());
    assert_eq!(code(&out), Some(Code::InvalidResponse));
    assert!(matches!(
        out.report.cleanup,
        Some(Cleanup::Incomplete {
            error: CleanupError::InvalidResponse,
            ..
        })
    ));
    assert!(p.is_disabled());
}
