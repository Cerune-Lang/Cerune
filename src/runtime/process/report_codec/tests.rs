use super::*;

fn sample() -> Report {
    Report {
        origin: Origin {
            node_id: NodeId(17),
            span: Span::in_source(SourceId::from_index(3), 30, 41),
        },
        started: Started::Unknown,
        stdin_requested: 20,
        stdin_accepted: 7,
        stdout_limit: 32,
        stderr_limit: 64,
        timeout: Duration::from_millis(9_007_199_254_740_993),
        stdout_read: 33,
        stderr_read: 65,
        stdout_retained: 32,
        stderr_retained: 64,
        progress_known: false,
        exit: Some(Exit::Signal(9)),
        failure: Some(Failure {
            code: Code::ProgressUnknown,
            phase: Phase::Transfer(Channel::Stderr),
        }),
        cleanup: Some(Cleanup::Incomplete {
            child_reaped: true,
            handles_closed: false,
            scope_finished: true,
            error: CleanupError::Timeout,
        }),
        cleanup_reported: Some(Cleanup::Complete {
            termination_requested: true,
        }),
    }
}
#[test]
fn fixed_words_and_little_endian_bytes_match_independent_expected_layout() {
    let report = sample();
    let expected = [
        1,
        32,
        17,
        3,
        30,
        41,
        2,
        20,
        7,
        32,
        64,
        9_007_199_254_740_993,
        33,
        65,
        32,
        64,
        0,
        2,
        9,
        21,
        4,
        3,
        2,
        10,
        1,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
    ];
    let encoded = ReportV1::encode(&report).unwrap();
    assert_eq!(*encoded.words(), expected);
    assert_eq!(encoded.to_report(), report);
    let bytes = encoded.to_le_bytes();
    assert_eq!(bytes.len(), 256);
    assert_eq!(&bytes[11 * 8..12 * 8], &[1, 0, 0, 0, 0, 0, 32, 0]);
    assert_eq!(ReportV1::from_le_bytes(&bytes).unwrap(), encoded);
    assert_eq!(ReportV1::from_words(&expected).unwrap().to_report(), report);
}
#[test]
fn all_failure_codes_phases_started_exit_and_cleanup_alternatives_roundtrip() {
    let codes = [
        (Code::InvalidRequest, 1),
        (Code::RequestLimit, 2),
        (Code::Unconfigured, 3),
        (Code::Unregistered, 4),
        (Code::ConnectionDisabled, 5),
        (Code::OutputBudget, 6),
        (Code::AllocationFailed, 7),
        (Code::NotFound, 8),
        (Code::Denied, 9),
        (Code::Resources, 10),
        (Code::Unsupported, 11),
        (Code::StartFailed, 12),
        (Code::TransferFailed, 13),
        (Code::CloseFailed, 14),
        (Code::WaitFailed, 15),
        (Code::ZeroProgress, 16),
        (Code::InvalidResponse, 17),
        (Code::OutputLimit, 18),
        (Code::Timeout, 19),
        (Code::CleanupFailed, 20),
        (Code::ProgressUnknown, 21),
    ];
    let phases = [
        (Phase::Validate, 1, 0),
        (Phase::Reserve, 2, 0),
        (Phase::Start, 3, 0),
        (Phase::Transfer(Channel::Stdin), 4, 1),
        (Phase::Transfer(Channel::Stdout), 4, 2),
        (Phase::Transfer(Channel::Stderr), 4, 3),
        (Phase::CloseStdin, 5, 0),
        (Phase::Wait, 6, 0),
        (Phase::Cleanup, 7, 0),
    ];
    for (code, number) in codes {
        for (phase, tag, channel) in phases {
            let mut r = sample();
            r.failure = Some(Failure { code, phase });
            let e = ReportV1::encode(&r).unwrap();
            assert_eq!(&e.words()[19..22], &[number, tag, channel]);
            assert_eq!(ReportV1::from_words(e.words()).unwrap().to_report(), r);
        }
    }
    for started in [Started::No, Started::Yes, Started::Unknown] {
        for exit in [
            None,
            Some(Exit::Code(0)),
            Some(Exit::Code(u32::MAX)),
            Some(Exit::Signal(u32::MAX)),
        ] {
            let mut r = sample();
            r.started = started;
            r.exit = exit;
            r.failure = None;
            r.progress_known = true;
            let e = ReportV1::encode(&r).unwrap();
            assert_eq!(&e.words()[19..22], &[0, 0, 0]);
            assert_eq!(ReportV1::from_words(e.words()).unwrap().to_report(), r);
        }
    }
    let mut cleanups = vec![
        None,
        Some(Cleanup::Complete {
            termination_requested: false,
        }),
        Some(Cleanup::Complete {
            termination_requested: true,
        }),
    ];
    for error in [
        CleanupError::Timeout,
        CleanupError::Terminate,
        CleanupError::Reap,
        CleanupError::Close,
        CleanupError::InvalidResponse,
    ] {
        for flags in 0..8 {
            cleanups.push(Some(Cleanup::Incomplete {
                child_reaped: flags & 1 != 0,
                handles_closed: flags & 2 != 0,
                scope_finished: flags & 4 != 0,
                error,
            }));
        }
    }
    for &cleanup in &cleanups {
        for &cleanup_reported in &cleanups {
            let mut r = sample();
            r.cleanup = cleanup;
            r.cleanup_reported = cleanup_reported;
            let e = ReportV1::encode(&r).unwrap();
            assert_eq!(ReportV1::from_words(e.words()).unwrap().to_report(), r);
        }
    }
}
#[test]
fn malformed_records_are_rejected_instead_of_normalized() {
    let original = *ReportV1::encode(&sample()).unwrap().words();
    for (slot, value) in [
        (0, 2),
        (1, 31),
        (4, 42),
        (6, 3),
        (16, 2),
        (17, 3),
        (18, u64::MAX),
        (19, 22),
        (20, 0),
        (20, 8),
        (21, 0),
        (21, 4),
        (22, 3),
        (23, 1),
        (23, 16),
        (24, 0),
        (24, 6),
        (25, 3),
        (26, 2),
        (27, 1),
        (28, 1),
        (29, 1),
        (30, 1),
        (31, 1),
    ] {
        let mut w = original;
        w[slot] = value;
        assert!(
            ReportV1::from_words(&w).is_err(),
            "slot={slot} value={value}"
        );
    }
    for edits in [
        &[(17, 0)][..],
        &[(19, 0)][..],
        &[(20, 1)][..],
        &[(22, 0)][..],
        &[(25, 0)][..],
    ] {
        let mut w = original;
        for &(slot, value) in edits {
            w[slot] = value;
        }
        assert!(ReportV1::from_words(&w).is_err(), "{edits:?}");
    }
    assert_eq!(ReportV1::from_words(&original[..31]), Err(Error::Shape));
    assert_eq!(ReportV1::from_words(&[0; 33]), Err(Error::Shape));
    assert_eq!(ReportV1::from_le_bytes(&[0; 255]), Err(Error::Shape));
    assert_eq!(ReportV1::from_le_bytes(&[0; 257]), Err(Error::Shape));
    // 任意のwordもpanicせず、正規形なら保持し、それ以外を拒否します。
    for slot in 0..WORD_COUNT {
        for value in [0, 1, 2, 3, 255, u32::MAX as u64, u64::MAX] {
            let mut w = original;
            w[slot] = value;
            if let Ok(e) = ReportV1::from_words(&w) {
                assert_eq!(*ReportV1::encode(&e.to_report()).unwrap().words(), w);
            }
        }
    }
}
#[test]
fn counts_and_duration_do_not_round_or_wrap() {
    let mut r = sample();
    r.stdin_requested = usize::MAX;
    r.stdout_read = usize::MAX;
    r.timeout = Duration::from_millis(u64::MAX);
    let e = ReportV1::encode(&r).unwrap();
    assert_eq!(e.to_report(), r);
    r.timeout = Duration::from_nanos(1);
    assert_eq!(ReportV1::encode(&r), Err(Error::TimePrecision));
    r.timeout = Duration::from_secs(u64::MAX);
    assert_eq!(ReportV1::encode(&r), Err(Error::Range));
    #[cfg(target_pointer_width = "32")]
    {
        let mut w = *e.words();
        w[2] = u64::MAX;
        assert_eq!(ReportV1::from_words(&w), Err(Error::Range));
    }
}
