use super::*;
use crate::runtime::{
    array_heap::ArrayHeap,
    process::{
        language_values::{PreparedOutcome, PreparedRequest, Value},
        report_value::Value as ReportValue,
    },
};
use crate::{ir, types::IntegerType};

fn definitions() -> (Vec<ir::TypeDefinition>, ir::TypeId, ir::TypeId) {
    let p = crate::compile_to_ir(r#"import "std:process" as p; v:p::Started=p::Started::No{};"#)
        .unwrap();
    let find = |name: &str| {
        p.type_definitions
            .iter()
            .find(|d| d.name.ends_with(name))
            .unwrap()
            .id
    };
    let req = find("_Request");
    let out = find("_Outcome");
    (p.type_definitions, req, out)
}
fn at<'a, V: Value>(
    v: &'a V,
    d: &[ir::TypeDefinition],
    id: ir::TypeId,
    path: &[&str],
) -> (&'a V, ir::Type) {
    if path.is_empty() {
        return (v, ir::Type::Named(id));
    }
    let index = d[id.0]
        .fields
        .iter()
        .position(|f| f.name == path[0])
        .unwrap();
    let field = &d[id.0].fields[index];
    let v = &v.fields(id).unwrap()[index];
    if path.len() == 1 {
        return (v, field.ty.clone());
    }
    let ir::Type::Named(next) = field.ty else {
        panic!("aggregate path")
    };
    at(v, d, next, &path[1..])
}
fn variant<'a, V: Value>(
    v: &V,
    d: &'a [ir::TypeDefinition],
    id: ir::TypeId,
    path: &[&str],
) -> &'a str {
    let (v, ir::Type::Named(id)) = at(v, d, id, path) else {
        panic!("enum path")
    };
    let tag = v.fields(id).unwrap()[0].unsigned(IntegerType::I64).unwrap() as usize;
    &d[id.0].variants.as_ref().unwrap()[tag]
}
fn bytes<V: Value>(v: &V, id: ir::TypeId, index: usize) -> Vec<u8> {
    v.fields(id).unwrap()[index]
        .array(crate::runtime::process::language_values::Element::Byte)
        .unwrap()
        .borrow_values()
        .unwrap()
        .iter()
        .map(|v| v.unsigned(IntegerType::U8).unwrap() as u8)
        .collect()
}
fn release<V: Value>(heap: &mut ArrayHeap<V>, v: &V, id: ir::TypeId) {
    for f in &v.fields(id).unwrap()[1..] {
        let a = f
            .array(crate::runtime::process::language_values::Element::Byte)
            .unwrap();
        assert!(heap.release_owner(a).unwrap());
        heap.free(a).unwrap();
    }
    heap.assert_empty();
}
fn failure_cases<V: Value + std::fmt::Debug>(trace: usize) -> Vec<String> {
    let (d, _, id) = definitions();
    let mut results = Vec::new();
    for (case, expected) in [
        (0, "InvalidRequest"),
        (1, "OutputBudget"),
        (2, "OutputBudget"),
        (3, "AllocationFailed"),
        (4, "Unconfigured"),
        (5, "ConnectionDisabled"),
    ] {
        let mut heap = ArrayHeap::<V>::new(if case == 2 { 256 } else { 512 });
        let mut p = if case == 4 {
            Processes::<Script>::unconfigured(limits())
        } else {
            Processes::with_trace_limit(Script::new(vec![finished(7)]), limits(), trace)
        };
        if case == 5 {
            p.disabled = true;
        }
        if case == 3 {
            heap.fail_next_allocation();
        }
        let prepared = PreparedOutcome::new(&d, id, &mut heap).unwrap();
        let mut req = request();
        if case == 0 {
            req.target = "bad\0target";
        }
        let v = prepared
            .run(&mut p, req, if case == 1 { 1 } else { 32768 }, origin())
            .unwrap();
        assert_eq!(
            variant(&v, &d, id, &["report", "failure", "$Reason$code"]),
            expected
        );
        assert_eq!(variant(&v, &d, id, &["report", "started"]), "No");
        assert_eq!(variant(&v, &d, id, &["report", "cleanup"]), "Absent");
        assert!(bytes(&v, id, 1).is_empty() && bytes(&v, id, 2).is_empty());
        if let Some(backend) = p.backend.as_ref() {
            assert_eq!(backend.clock_reads.get(), 0);
            assert!(backend.actions.is_empty());
        }
        results.push(format!("{v:?}"));
        release(&mut heap, &v, id);
        if case <= 3 {
            let mut req = request();
            req.stdout_limit = 1;
            req.stderr_limit = 1;
            let v = PreparedOutcome::new(&d, id, &mut heap)
                .unwrap()
                .run(&mut p, req, 32768, origin())
                .unwrap();
            assert_eq!(variant(&v, &d, id, &["report", "failure"]), "None");
            assert_eq!(
                at(&v, &d, id, &["report", "exit", "$Code$value"])
                    .0
                    .unsigned(IntegerType::U32),
                Some(7)
            );
            release(&mut heap, &v, id);
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
    results
}
#[test]
fn language_outcomes_cover_prelaunch_failures_and_explicit_continuation_in_both_values() {
    assert_eq!(
        failure_cases::<crate::runtime::value::Value>(0),
        failure_cases::<crate::runtime::value::Value>(1)
    );
    assert_eq!(
        failure_cases::<crate::vm::Value>(0),
        failure_cases::<crate::vm::Value>(1)
    );
}
fn partial<V: Value + std::fmt::Debug>(trace: usize) -> (String, Vec<Action>) {
    let (d, _, id) = definitions();
    let mut heap = ArrayHeap::<V>::new(512);
    let mut req = request();
    req.stdin = &[255, 0, 13, 10];
    let mut s = step(
        1,
        vec![
            done(0, b"", 2, Completion::Done),
            done(1, &[255, 0, 13, 10], 4, Completion::Done),
            done(2, b"err", 3, Completion::Failed),
        ],
    );
    s.exit = Some(Ok(Exit::Code(u32::MAX)));
    let mut p = Processes::with_trace_limit(Script::new(vec![plan(vec![s])]), limits(), trace);
    let v = PreparedOutcome::new(&d, id, &mut heap)
        .unwrap()
        .run(&mut p, req, 32768, origin())
        .unwrap();
    assert_eq!(bytes(&v, id, 1), [255, 0, 13, 10]);
    assert_eq!(bytes(&v, id, 2), b"err");
    assert_eq!(
        variant(&v, &d, id, &["report", "failure", "$Reason$code"]),
        "TransferFailed"
    );
    assert_eq!(variant(&v, &d, id, &["report", "cleanup"]), "Complete");
    assert_eq!(
        variant(&v, &d, id, &["report", "cleanup_reported"]),
        "Complete"
    );
    assert_eq!(
        at(&v, &d, id, &["report", "stdin_accepted"])
            .0
            .unsigned(IntegerType::U64),
        Some(2)
    );
    assert_eq!(
        at(&v, &d, id, &["report", "exit", "$Code$value"])
            .0
            .unsigned(IntegerType::U32),
        Some(u64::from(u32::MAX))
    );
    assert_eq!(
        at(&v, &d, id, &["report", "origin", "node_id"])
            .0
            .unsigned(IntegerType::U64),
        Some(17)
    );
    assert!(matches!(
        heap.allocate(1, 8),
        Err(crate::runtime::array_heap::ArrayError::AllocationLimitExceeded)
    ));
    let result = format!("{v:?}");
    release(&mut heap, &v, id);
    let b = p.into_inner().unwrap();
    assert_eq!(b.sent, [255, 0]);
    (result, b.actions)
}
#[test]
fn complete_outcome_preserves_partial_bytes_failure_origin_and_reserved_capacity() {
    assert_eq!(
        partial::<crate::runtime::value::Value>(0),
        partial::<crate::runtime::value::Value>(64)
    );
    assert_eq!(
        partial::<crate::vm::Value>(0),
        partial::<crate::vm::Value>(64)
    );
}
#[test]
fn converted_request_reaches_runtime_validation_and_runs_without_rewriting_arguments() {
    use crate::vm::Value as V;
    let (d, req_id, out_id) = definitions();
    let mut heap = ArrayHeap::new(512);
    let args = heap.allocate(2, 16).unwrap();
    for s in ["", "日本語"] {
        heap.initialize(&args, V::String(s.to_owned().into()))
            .unwrap();
    }
    let stdin = heap.allocate(0, 8).unwrap();
    let mut input = V::aggregate(
        req_id,
        vec![
            V::String("tool".to_owned().into()),
            V::DynamicArray {
                element: crate::bytecode::Type::String,
                storage: args.clone(),
            },
            V::byte_array(stdin.clone()),
            V::integer(1, IntegerType::U64),
            V::integer(1, IntegerType::U64),
            V::integer(100, IntegerType::U64),
        ],
    );
    let prepared = PreparedRequest::new(&d, req_id, &input).unwrap();
    input.parts().1[0] = V::String("bad\0target".to_owned().into());
    let invalid = PreparedRequest::new(&d, req_id, &input).unwrap();
    for a in [args, stdin] {
        assert!(heap.release_owner(&a).unwrap());
        heap.free(&a).unwrap();
    }
    let mut p = Processes::new(Script::new(vec![finished(7)]), limits());
    let failure = PreparedOutcome::new(&d, out_id, &mut heap).unwrap();
    let v = failure
        .run_prepared(&mut p, &invalid, 32768, origin())
        .unwrap();
    assert_eq!(
        variant(&v, &d, out_id, &["report", "failure", "$Reason$code"]),
        "InvalidRequest"
    );
    release(&mut heap, &v, out_id);
    let success = PreparedOutcome::new(&d, out_id, &mut heap).unwrap();
    let v = success
        .run_prepared(&mut p, &prepared, 32768, origin())
        .unwrap();
    assert_eq!(variant(&v, &d, out_id, &["report", "failure"]), "None");
    release(&mut heap, &v, out_id);
    let b = p.into_inner().unwrap();
    assert_eq!(b.args, vec![String::new(), "日本語".to_owned()]);
}
