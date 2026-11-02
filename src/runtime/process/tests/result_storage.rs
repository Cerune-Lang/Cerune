use super::*;
use crate::runtime::array_heap::{ArrayError, ArrayHeap};
use std::{cell::Cell, rc::Rc};

fn storage_error(error: ArrayError) -> ReservationError {
    match error {
        ArrayError::AllocationSizeOverflow | ArrayError::AllocationLimitExceeded => {
            ReservationError::Capacity
        }
        ArrayError::AllocationFailed => ReservationError::Allocation,
        _ => panic!("unexpected reservation error: {error:?}"),
    }
}

#[test]
fn reservation_failure_is_observed_without_clock_launch_or_cleanup_and_allows_next_call() {
    for (error, expected) in [
        (ReservationError::Capacity, Code::OutputBudget),
        (ReservationError::Allocation, Code::AllocationFailed),
    ] {
        let script = Script::new(vec![finished(7)]);
        let clocks = script.clock_reads.clone();
        let mut p = Processes::with_trace_limit(script, limits(), 32);
        let mut calls = 0;
        let (out, storage) = p.run_with_reservation(request(), 32768, origin(), || {
            calls += 1;
            Err::<(), _>(error)
        });
        assert_eq!(calls, 1);
        assert!(storage.is_none());
        assert_eq!(code(&out), Some(expected));
        assert_eq!(out.report.failure.unwrap().phase, Phase::Reserve);
        assert_eq!(out.report.origin, origin());
        assert_eq!(out.report.started, Started::No);
        assert!(out.report.cleanup.is_none() && out.transfers.is_none());
        assert_eq!(clocks.get(), 0);
        assert!(p.backend.as_ref().unwrap().actions.is_empty());
        assert!(!p.is_disabled());
        assert_eq!(p.events().last().unwrap().report, out.report);
        let (next, storage) = p.run_with_reservation(request(), 32768, origin(), || {
            assert_eq!(clocks.get(), 0);
            Ok(42u64)
        });
        assert!(next.completed());
        assert_eq!(storage, Some(42));
        assert_eq!(next.report.exit, Some(Exit::Code(7)));
        assert_eq!(next.transfers.unwrap().call.get(), 2);
        assert_eq!(
            p.into_inner()
                .unwrap()
                .actions
                .iter()
                .filter(|a| matches!(a, Action::Start))
                .count(),
            1
        );
    }
}

#[test]
fn validation_and_transfer_reservation_keep_precedence_and_skip_the_callback() {
    let mut cases = Vec::new();
    let mut invalid = request();
    invalid.timeout = Duration::ZERO;
    cases.push((invalid, 32768, Code::InvalidRequest));
    let mut excess = request();
    excess.stdout_limit = limits().output_bytes + 1;
    cases.push((excess, 32768, Code::RequestLimit));
    cases.push((request(), 1, Code::OutputBudget));
    for (req, available, expected) in cases {
        let mut p = Processes::new(Script::default(), limits());
        let (out, storage) = p.run_with_reservation(
            req,
            available,
            origin(),
            || -> Result<(), ReservationError> {
                panic!("reservation must not be attempted");
            },
        );
        assert_eq!(code(&out), Some(expected));
        assert!(storage.is_none());
        let backend = p.into_inner().unwrap();
        assert_eq!(backend.clock_reads.get(), 0);
        assert!(backend.actions.is_empty());
    }
    let mut p = Processes::<Script>::unconfigured(limits());
    let (out, storage) = p.run_with_reservation(
        request(),
        32768,
        origin(),
        || -> Result<(), ReservationError> { panic!("unconfigured") },
    );
    assert_eq!(code(&out), Some(Code::Unconfigured));
    assert!(storage.is_none());
    let mut p = Processes::new(Script::default(), limits());
    p.disabled = true;
    let (out, storage) =
        p.run_with_reservation(invalid, 0, origin(), || -> Result<(), ReservationError> {
            panic!("disabled")
        });
    assert_eq!(code(&out), Some(Code::ConnectionDisabled));
    assert!(storage.is_none());
}

#[test]
fn language_array_budget_failure_rolls_back_both_outputs_before_launch() {
    let mut heap = ArrayHeap::<u64>::new(256); // stdoutで使い切り、stderrは確保できません。
    let mut p = Processes::new(Script::new(vec![finished(0)]), limits());
    let (out, storage) = p.run_with_reservation(request(), 32768, origin(), || {
        heap.reserve_byte_outputs(32, 32).map_err(storage_error)
    });
    assert_eq!(code(&out), Some(Code::OutputBudget));
    assert!(storage.is_none());
    drop(storage);
    heap.assert_empty();
    assert_eq!(p.backend.as_ref().unwrap().clock_reads.get(), 0);
    assert!(p.backend.as_ref().unwrap().actions.is_empty());
    assert!(p.run(request(), 32768, origin()).completed());
}

#[test]
fn partial_binary_output_uses_prelaunch_storage_and_keeps_original_failure_and_cleanup() {
    let mut heap = ArrayHeap::<u64>::new(512);
    let mut req = request();
    req.stdin = b"ab";
    let mut s = step(
        1,
        vec![
            done(0, b"", 1, Completion::Done),
            done(1, &[255, 0, 13, 10], 4, Completion::Done),
            done(2, b"err", 3, Completion::Failed),
        ],
    );
    s.exit = Some(Ok(Exit::Code(u32::MAX)));
    let script = Script::new(vec![plan(vec![s])]);
    let clocks = script.clock_reads.clone();
    let mut p = Processes::with_trace_limit(script, limits(), 64);
    let (out, storage) = p.run_with_reservation(req, 32768, origin(), || {
        assert_eq!(clocks.get(), 0);
        heap.reserve_byte_outputs(32, 32).map_err(storage_error)
    });
    assert_eq!(code(&out), Some(Code::TransferFailed));
    assert_eq!(out.report.exit, Some(Exit::Code(u32::MAX)));
    assert_eq!(out.report.stdin_accepted, 1);
    assert_eq!(out.report.cleanup, Some(complete()));
    let report = out.report;
    let [stdout, stderr] = storage
        .unwrap()
        .finish([&out.stdout, &out.stderr], u64::from)
        .unwrap();
    drop(out);
    assert_eq!(stdout.len(), 4);
    assert_eq!(stderr.len(), 3);
    assert_eq!(stdout.get(0), Ok(255));
    assert_eq!(stdout.get(3), Ok(10));
    assert_eq!(stderr.get(2), Ok(u64::from(b'r')));
    assert_eq!(report.failure.unwrap().code, Code::TransferFailed);
    assert_eq!(p.events().last().unwrap().report, report);
    for array in [&stdout, &stderr] {
        assert!(heap.release_owner(array).unwrap());
        heap.free(array).unwrap();
    }
    heap.assert_empty();
    assert_eq!(
        p.into_inner()
            .unwrap()
            .actions
            .iter()
            .filter(|a| matches!(a, Action::Start))
            .count(),
        1
    );
}

#[test]
fn abandoned_storage_and_start_failure_do_not_leak_reservations() {
    let mut heap = ArrayHeap::<u64>::new(512);
    let mut not_started = finished(0);
    not_started.launch = Launch::NotStarted(StartError::NotFound);
    let mut p = Processes::new(Script::new(vec![not_started]), limits());
    let (out, storage) = p.run_with_reservation(request(), 32768, origin(), || {
        heap.reserve_byte_outputs(32, 32).map_err(storage_error)
    });
    assert_eq!(code(&out), Some(Code::NotFound));
    assert_eq!(out.report.started, Started::No);
    assert!(storage.is_some());
    drop(storage);
    heap.assert_empty();
    assert!(
        p.into_inner()
            .unwrap()
            .actions
            .iter()
            .all(|a| matches!(a, Action::Start))
    );
}

struct Tracked(Rc<Cell<usize>>);
impl Drop for Tracked {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn reservation_is_owned_once_and_trace_configuration_does_not_change_results() {
    let mut baseline = None;
    for trace in [0, 1, 64] {
        let drops = Rc::new(Cell::new(0));
        let mut p = Processes::with_trace_limit(Script::new(vec![finished(9)]), limits(), trace);
        let (out, storage) =
            p.run_with_reservation(request(), 32768, origin(), || Ok(Tracked(drops.clone())));
        assert_eq!(drops.get(), 0);
        assert!(storage.is_some());
        drop(storage);
        assert_eq!(drops.get(), 1);
        let actions = p.into_inner().unwrap().actions;
        if let Some((previous, previous_actions)) = &baseline {
            assert_eq!(&out, previous);
            assert_eq!(&actions, previous_actions);
        } else {
            baseline = Some((out, actions));
        }
    }
}
