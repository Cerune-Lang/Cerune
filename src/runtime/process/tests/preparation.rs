use super::*;

fn scenario(index: usize) -> Plan {
    match index {
        0 => finished(7),
        1 => {
            let mut p = plan(vec![]);
            p.launch = Launch::NotStarted(StartError::NotFound);
            p
        }
        2 => {
            let mut s = step(
                1,
                vec![
                    done(0, b"", 1, Completion::Done),
                    done(1, &[255, 0, 13, 10], 4, Completion::Done),
                    done(2, b"err", 3, Completion::Failed),
                ],
            );
            s.exit = Some(Ok(Exit::Code(u32::MAX)));
            plan(vec![s])
        }
        3 => plan(vec![step(100, vec![])]),
        4 => {
            let mut p = finished(0);
            let mut s = step(11, vec![]);
            s.cleanup = Some(complete());
            p.cleanup = Some(vec![s].into());
            p
        }
        5 => {
            let mut p = finished(0);
            p.launch = Launch::Uncertain(StartError::Failed);
            p
        }
        _ => unreachable!(),
    }
}
type Observation = (u64, Phase, Report, Option<Snapshot>, Option<Offered>);
fn observations(p: &Processes<Script>) -> Vec<Observation> {
    p.events()
        .iter()
        .map(|e| (e.sequence, e.phase, e.report, e.transfers, e.offered))
        .collect()
}
#[test]
fn staged_and_existing_entry_points_keep_results_actions_and_observations() {
    for case in 0..6 {
        for trace in [0, 1, 128] {
            for reservation in [
                Ok(42u64),
                Err(ReservationError::Capacity),
                Err(ReservationError::Allocation),
            ] {
                let mut old =
                    Processes::with_trace_limit(Script::new(vec![scenario(case)]), limits(), trace);
                let mut staged =
                    Processes::with_trace_limit(Script::new(vec![scenario(case)]), limits(), trace);
                let mut req = request();
                if case == 2 {
                    req.stdin = b"ab";
                }
                let expected = old.run_with_reservation(req, 32768, origin(), || reservation);
                let clocks = staged.backend.as_ref().unwrap().clock_reads.clone();
                let Preparation::Pending(prepared) = staged.prepare(req, 32768, origin()) else {
                    panic!("expected pending")
                };
                for _ in 0..3 {
                    assert_eq!(prepared.report().origin, origin());
                    assert_eq!(prepared.report().started, Started::No);
                    assert_eq!(
                        report_codec::ReportV1::encode(prepared.report())
                            .unwrap()
                            .to_report(),
                        *prepared.report()
                    );
                    assert_eq!(clocks.get(), 0);
                }
                let actual = prepared.execute(reservation);
                assert_eq!(actual, expected, "case {case}, trace {trace}");
                assert_eq!(observations(&staged), observations(&old));
                assert_eq!(staged.trace_truncated(), old.trace_truncated());
                assert_eq!(staged.is_disabled(), old.is_disabled());
                let a = staged.into_inner().unwrap();
                let b = old.into_inner().unwrap();
                assert_eq!(a.actions, b.actions);
                assert_eq!(a.contexts, b.contexts);
                assert_eq!(a.sent, b.sent);
                assert_eq!(a.args, b.args);
                assert_eq!(a.clock_reads.get(), b.clock_reads.get());
            }
        }
    }
}
#[test]
fn rejected_preparation_preserves_precedence_and_never_reads_clock() {
    for case in 0..6 {
        let mut p = if case == 3 {
            Processes::<Script>::unconfigured(limits())
        } else {
            Processes::new(Script::default(), limits())
        };
        let mut req = request();
        let mut available = 32768;
        let expected = match case {
            0 => {
                p.disabled = true;
                req.timeout = Duration::ZERO;
                Code::ConnectionDisabled
            }
            1 => {
                req.timeout = Duration::ZERO;
                Code::InvalidRequest
            }
            2 => {
                req.stdout_limit = limits().output_bytes + 1;
                Code::RequestLimit
            }
            3 => Code::Unconfigured,
            4 => {
                available = 0;
                Code::OutputBudget
            }
            5 => {
                p.fail_allocation = true;
                Code::AllocationFailed
            }
            _ => unreachable!(),
        };
        let Preparation::Complete(out) = p.prepare(req, available, origin()) else {
            panic!("expected complete")
        };
        assert_eq!(code(&out), Some(expected));
        assert_eq!(out.report.started, Started::No);
        if let Some(b) = p.into_inner() {
            assert!(b.actions.is_empty());
            assert_eq!(b.clock_reads.get(), 0);
        }
    }
}
#[test]
fn dropping_pending_releases_unsubmitted_buffers_and_keeps_next_identity() {
    let mut p = Processes::with_trace_limit(Script::new(vec![finished(9)]), limits(), 32);
    let Preparation::Pending(prepared) = p.prepare(request(), 32768, origin()) else {
        panic!("pending")
    };
    drop(prepared);
    let b = p.backend.as_ref().unwrap();
    assert_eq!(b.clock_reads.get(), 0);
    assert!(b.actions.is_empty());
    assert!(!p.is_disabled());
    assert_eq!(
        p.events().iter().map(|e| e.phase).collect::<Vec<_>>(),
        [Phase::Validate, Phase::Reserve]
    );
    let next = p.run(request(), 32768, origin());
    assert_eq!(next.report.exit, Some(Exit::Code(9)));
    assert_eq!(next.transfers.unwrap().call.get(), 2);
}
#[test]
fn failed_staged_reservation_retains_connection_and_next_call_works() {
    for error in [ReservationError::Capacity, ReservationError::Allocation] {
        let mut p = Processes::new(Script::new(vec![finished(3)]), limits());
        let Preparation::Pending(prepared) = p.prepare(request(), 32768, origin()) else {
            panic!("pending")
        };
        let (out, reservation) = prepared.execute::<()>(Err(error));
        assert!(reservation.is_none());
        assert_eq!(out.report.failure.unwrap().phase, Phase::Reserve);
        let b = p.backend.as_ref().unwrap();
        assert!(b.actions.is_empty());
        assert_eq!(b.clock_reads.get(), 0);
        assert_eq!(
            p.run(request(), 32768, origin()).report.exit,
            Some(Exit::Code(3))
        );
    }
}
