use super::*;
use crate::{bytecode, ir_executor, mir, mir_executor, ssa_executor, vm};

const REQUEST: &str = r#"
import "std:process" as p;
import "std:process/run" as process;
fn request() -> p::Request {
    print("request");
    return p::Request {
        target: "tool", arguments: array_copy(["", "a b", "日本語"]),
        stdin: array_copy([u8(255),u8(0),u8(13),u8(10)]),
        stdout_limit: 32, stderr_limit: 16, timeout_ms: 100,
    };
}
"#;
const BODY: &str = r#"
fn invoke() -> bool {
    out:p::Outcome=process::run(request());
    print(out.report.failure);
    print(out.report.exit);
    print(out.stdout); print(out.stderr);
    print(out.report.stdin_accepted);
    mut changed:[u8]=out.stdout;
    if array_len(changed)>0 {changed[0]=u8(65);print(out.stdout[0]);print(changed[0]);}
    return true;
}
print(false && invoke());
print(true || invoke());
print(invoke());
print("continued");
"#;
fn execute(route: usize, source: &str, p: &mut Processes<Script>, available: usize) -> String {
    let ir = crate::compile_to_ir(source).unwrap();
    match route {
        0 => ir_executor::run_with_processes(&ir, p, available).unwrap(),
        1 => mir_executor::run_with_processes(&mir::lower(&ir).unwrap(), p, available).unwrap(),
        2 => ssa_executor::run_with_processes(
            &mir::ssa::construct(&mir::lower(&ir).unwrap()).unwrap(),
            p,
            available,
        )
        .unwrap(),
        3 => vm::run_with_processes(&bytecode::lower(&ir).unwrap(), p, available).unwrap(),
        _ => unreachable!(),
    }
}
fn binary_plan() -> Plan {
    let mut result = step(
        1,
        vec![
            done(0, b"", 4, Completion::Done),
            done(1, &[255, 0, 13, 10], 4, Completion::Done),
            done(2, &[0, 255], 2, Completion::Done),
        ],
    );
    result.exit = Some(Ok(Exit::Code(u32::MAX)));
    plan(vec![
        result,
        step(
            2,
            vec![
                done(1, b"", 0, Completion::Eof),
                done(2, b"", 0, Completion::Eof),
            ],
        ),
    ])
}
#[test]
fn four_executors_preserve_order_bytes_origin_and_independent_copies() {
    let source = format!("{REQUEST}{BODY}");
    let mut baseline = None;
    for route in 0..4 {
        let mut p = Processes::new(Script::new(vec![binary_plan()]), limits());
        let text = execute(route, &source, &mut p, 32768);
        assert!(text.starts_with("false\ntrue\nrequest\n"), "{text}");
        assert!(
            text.contains("[255, 0, 13, 10]\n[0, 255]\n4\n255\n65\n"),
            "{text}"
        );
        assert!(text.contains("4294967295"), "{text}");
        assert!(text.ends_with("true\ncontinued\n"));
        let script = p.into_inner().unwrap();
        assert_eq!(script.sent, [255, 0, 13, 10]);
        assert_eq!(script.args, ["", "a b", "日本語"]);
        assert_eq!(script.contexts.len(), 1);
        let observation = (text, script.contexts, script.actions);
        if let Some(b) = &baseline {
            assert_eq!(&observation, b);
        } else {
            baseline = Some(observation);
        }
    }
}
#[test]
fn failed_launch_then_explicit_retry_continues_in_all_executors() {
    let source = format!("{REQUEST}{}{}", BODY, "print(invoke());");
    let mut baseline = None;
    for route in 0..4 {
        let mut failed = plan(vec![]);
        failed.launch = Launch::NotStarted(StartError::NotFound);
        let mut p = Processes::new(Script::new(vec![failed, binary_plan()]), limits());
        let text = execute(route, &source, &mut p, 32768);
        assert!(text.contains("NotFound"), "{text}");
        assert_eq!(text.matches("request\n").count(), 2);
        assert!(text.contains("continued\nrequest\n"), "{text}");
        assert_eq!(p.into_inner().unwrap().contexts.len(), 2);
        if let Some(b) = &baseline {
            assert_eq!(&text, b);
        } else {
            baseline = Some(text);
        }
    }
}
#[test]
fn transfer_budget_failure_precedes_clock_and_launch() {
    let source = format!("{REQUEST}{BODY}");
    for route in 0..4 {
        let mut p = Processes::new(Script::default(), limits());
        let text = execute(route, &source, &mut p, 0);
        assert!(text.contains("OutputBudget"), "{text}");
        assert!(text.ends_with("true\ncontinued\n"));
        let script = p.into_inner().unwrap();
        assert!(script.actions.is_empty());
        assert_eq!(script.clock_reads.get(), 0);
    }
}
#[test]
fn default_execution_is_unconfigured_in_all_executors() {
    let source = format!("{REQUEST}{BODY}");
    let ir = crate::compile_to_ir(&source).unwrap();
    let m = mir::lower(&ir).unwrap();
    let results = [
        ir_executor::run(&ir).unwrap(),
        mir_executor::run(&m).unwrap(),
        ssa_executor::run(&mir::ssa::construct(&m).unwrap()).unwrap(),
        vm::run(&bytecode::lower(&ir).unwrap()).unwrap(),
    ];
    for text in &results {
        assert_eq!(text, &results[0]);
        assert!(text.contains("Unconfigured"), "{text}");
        assert!(text.ends_with("true\ncontinued\n"));
    }
}

#[test]
fn partial_output_timeout_and_disabled_connection_are_values() {
    let source = format!("{REQUEST}{}{}", BODY, "print(invoke());");
    let mut baseline = None;
    for route in 0..4 {
        let mut partial = plan(vec![step(1, vec![done(1, &[255, 0], 2, Completion::Done)])]);
        let mut cleanup = step(111, vec![]);
        cleanup.cleanup = Some(Cleanup::Incomplete {
            child_reaped: false,
            handles_closed: false,
            scope_finished: false,
            error: CleanupError::Timeout,
        });
        partial.cleanup = Some(vec![cleanup].into());
        let mut p = Processes::new(Script::new(vec![partial]), limits());
        let text = execute(route, &source, &mut p, 32768);
        assert!(text.contains("Timeout"), "{text}");
        assert!(text.contains("[255, 0]"), "{text}");
        assert!(text.contains("ConnectionDisabled"), "{text}");
        assert!(text.contains("continued\nrequest\n"), "{text}");
        assert!(p.is_disabled());
        assert_eq!(p.into_inner().unwrap().contexts.len(), 1);
        if let Some(b) = &baseline {
            assert_eq!(&text, b);
        } else {
            baseline = Some(text);
        }
    }
}

#[test]
fn observation_capacity_does_not_change_execution() {
    let source = format!("{REQUEST}{BODY}");
    for route in 0..4 {
        let mut a = Processes::with_trace_limit(Script::new(vec![binary_plan()]), limits(), 0);
        let mut b = Processes::with_trace_limit(Script::new(vec![binary_plan()]), limits(), 128);
        assert_eq!(
            execute(route, &source, &mut a, 32768),
            execute(route, &source, &mut b, 32768)
        );
        assert!(a.events().is_empty());
        assert!(!b.events().is_empty());
        assert_eq!(
            a.into_inner().unwrap().actions,
            b.into_inner().unwrap().actions
        );
    }
}
