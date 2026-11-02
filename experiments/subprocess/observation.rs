//! 独立した実OS実行の比較です。同じ試験用応答での完全一致検証とは区別します。
use cerune_lang::runtime::process::{Cleanup, Outcome, Report};
use serde_json::{Value, json};
use std::io;

fn comparable(report: &Report) -> Option<Report> {
    let Some(Cleanup::Complete { .. }) = report.cleanup else {
        return None;
    };
    // 正常な回収では生報告と共通runtimeの判定が一致している必要があります。
    if report.failure.is_some()
        || !report.progress_known
        || report.cleanup_reported != report.cleanup
    {
        return None;
    }
    let mut copy = *report;
    // Job/groupが空になった時点は別実行で異なります。比較用の写しだけから除きます。
    // 元の記録は変更せず、下のJSONLへ両実行の事実を残します。
    copy.cleanup = Some(Cleanup::Complete {
        termination_requested: false,
    });
    copy.cleanup_reported = copy.cleanup;
    Some(copy)
}

pub fn compare_completed(a: &Outcome, b: &Outcome) -> io::Result<Value> {
    let left = comparable(&a.report);
    let right = comparable(&b.report);
    if left.is_none() || left != right || a.stdout != b.stdout || a.stderr != b.stderr {
        return Err(io::Error::other(format!(
            "observation comparison failed: without={:?}, with={:?}, stdout_equal={}, stderr_equal={}",
            a.report,
            b.report,
            a.stdout == b.stdout,
            a.stderr == b.stderr
        )));
    }
    Ok(json!({
        "semantic_results_equal":true,
        "cleanup_without_observation":format!("{:?}",a.report.cleanup),
        "cleanup_with_observation":format!("{:?}",b.report.cleanup),
        "cleanup_reported_without_observation":format!("{:?}",a.report.cleanup_reported),
        "cleanup_reported_with_observation":format!("{:?}",b.report.cleanup_reported),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cerune_lang::{
        ir::NodeId,
        runtime::process::{CleanupError, Code, Exit, Failure, Origin, Phase, Started},
        source::Span,
    };
    use std::time::Duration;

    fn outcome(termination_requested: bool) -> Outcome {
        let cleanup = Some(Cleanup::Complete {
            termination_requested,
        });
        Outcome {
            report: Report {
                origin: Origin {
                    node_id: NodeId(17),
                    span: Span::new(30, 41),
                },
                started: Started::Yes,
                stdin_requested: 1,
                stdin_accepted: 1,
                stdout_limit: 8,
                stderr_limit: 8,
                timeout: Duration::from_secs(10),
                stdout_read: 1,
                stderr_read: 0,
                stdout_retained: 1,
                stderr_retained: 0,
                progress_known: true,
                exit: Some(Exit::Code(7)),
                failure: None,
                cleanup,
                cleanup_reported: cleanup,
            },
            stdout: vec![255],
            stderr: vec![],
            transfers: None,
        }
    }

    #[test]
    fn independent_cleanup_requests_are_preserved_while_results_are_compared() {
        let a = outcome(false);
        let b = outcome(true);
        let before_a = a.clone();
        let before_b = b.clone();
        let evidence = compare_completed(&a, &b).unwrap();
        assert_ne!(
            evidence["cleanup_without_observation"],
            evidence["cleanup_with_observation"]
        );
        assert_eq!(a, before_a);
        assert_eq!(b, before_b);
    }

    #[test]
    fn comparison_still_rejects_output_exit_progress_origin_and_cleanup_failures() {
        let a = outcome(false);
        for change in 0..9 {
            let mut b = outcome(true);
            match change {
                0 => b.stdout[0] = 0,
                1 => b.report.exit = Some(Exit::Code(0)),
                2 => b.report.stdin_accepted = 0,
                3 => b.report.progress_known = false,
                4 => b.report.origin.node_id = NodeId(99),
                5 => {
                    b.report.cleanup = Some(Cleanup::Incomplete {
                        child_reaped: true,
                        handles_closed: true,
                        scope_finished: false,
                        error: CleanupError::Timeout,
                    })
                }
                6 => b.report.cleanup_reported = None,
                7 => {
                    b.report.failure = Some(Failure {
                        code: Code::Timeout,
                        phase: Phase::Wait,
                    })
                }
                _ => {
                    b.report.cleanup_reported = Some(Cleanup::Complete {
                        termination_requested: false,
                    })
                }
            }
            assert!(compare_completed(&a, &b).is_err(), "change {change}");
        }
    }
}
