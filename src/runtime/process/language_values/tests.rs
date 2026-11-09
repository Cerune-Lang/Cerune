use super::*;
use report_value::Value as ReportValue;
use std::time::Duration;

trait TestValue: Value + std::fmt::Debug {
    fn string(s: &str) -> Self;
    fn strings(storage: ArrayValue<Self>) -> Self;
}
impl TestValue for crate::runtime::value::Value {
    fn string(s: &str) -> Self {
        Self::String(s.to_owned().into())
    }
    fn strings(storage: ArrayValue<Self>) -> Self {
        Self::DynamicArray {
            element: ir::Type::String,
            storage,
        }
    }
}
impl TestValue for crate::vm::Value {
    fn string(s: &str) -> Self {
        Self::String(s.to_owned().into())
    }
    fn strings(storage: ArrayValue<Self>) -> Self {
        Self::DynamicArray {
            element: crate::bytecode::Type::String,
            storage,
        }
    }
}
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
    let request = find("_Request");
    let outcome = find("_Outcome");
    (p.type_definitions, request, outcome)
}
fn input<V: TestValue>(heap: &mut ArrayHeap<V>, id: ir::TypeId) -> V {
    let args = heap.allocate(5, 16).unwrap();
    for s in ["", "a b", "日本語", "e\u{301}", "nul\0cr\r\n"] {
        heap.initialize(&args, V::string(s)).unwrap();
    }
    let bytes = heap.allocate(4, 8).unwrap();
    for b in [255, 0, 13, 10] {
        heap.initialize(&bytes, V::integer(b, IntegerType::U8))
            .unwrap();
    }
    V::aggregate(
        id,
        vec![
            V::string("tool"),
            V::strings(args),
            V::byte_array(bytes),
            V::integer(32, IntegerType::U64),
            V::integer(16, IntegerType::U64),
            V::integer(i128::from(u64::MAX), IntegerType::U64),
        ],
    )
}
fn release<V: Value>(heap: &mut ArrayHeap<V>, value: &V, id: ir::TypeId) {
    let f = value.fields(id).unwrap();
    for (i, e) in [(1, Element::String), (2, Element::Byte)] {
        let a = f[i].array(e).unwrap();
        assert!(heap.release_owner(a).unwrap());
        heap.free(a).unwrap();
    }
}
fn request_copy<V: TestValue>() {
    let (d, id, _) = definitions();
    let mut heap = ArrayHeap::new(4096);
    let value = input::<V>(&mut heap, id);
    let prepared = PreparedRequest::new(&d, id, &value).unwrap();
    let fields = value.fields(id).unwrap();
    fields[1]
        .array(Element::String)
        .unwrap()
        .set(0, V::string("changed"))
        .unwrap();
    fields[2]
        .array(Element::Byte)
        .unwrap()
        .set(0, V::integer(65, IntegerType::U8))
        .unwrap();
    // 元の言語配列を解放した後もRust側要求は独立した内容を持ちます。
    release(&mut heap, &value, id);
    heap.assert_empty();
    for _ in 0..2 {
        prepared
            .with_request(|r| {
                assert_eq!(r.target, "tool");
                assert_eq!(
                    r.arguments,
                    ["", "a b", "日本語", "e\u{301}", "nul\0cr\r\n"]
                );
                assert_eq!(r.stdin, [255, 0, 13, 10]);
                assert_eq!((r.stdout_limit, r.stderr_limit), (32, 16));
                assert_eq!(r.timeout, Duration::from_millis(u64::MAX));
            })
            .unwrap();
    }
}
#[test]
fn request_conversion_preserves_order_bytes_and_independent_storage_in_both_values() {
    request_copy::<crate::runtime::value::Value>();
    request_copy::<crate::vm::Value>();
}
#[test]
fn malformed_requests_fail_before_copying_and_do_not_change_input() {
    type V = crate::vm::Value;
    let (d, id, _) = definitions();
    for mutation in 0..7 {
        let mut heap = ArrayHeap::new(4096);
        let mut v = input::<V>(&mut heap, id);
        let f = v.parts().1;
        match mutation {
            0 => f[0] = V::Bool(false),
            1 => f[3] = V::integer(-1, IntegerType::U64),
            2 => f[4] = V::integer(1, IntegerType::I64),
            3 => f[5] = V::integer(i128::from(u64::MAX) + 1, IntegerType::U64),
            4 => f[1]
                .array(Element::String)
                .unwrap()
                .set(0, V::Bool(false))
                .unwrap(),
            5 => f[2]
                .array(Element::Byte)
                .unwrap()
                .set(0, V::integer(256, IntegerType::U8))
                .unwrap(),
            _ => f[2]
                .array(Element::Byte)
                .unwrap()
                .set(0, V::integer(0, IntegerType::I64))
                .unwrap(),
        }
        let before = format!("{v:?}");
        let mut calls = 0;
        assert!(matches!(
            PreparedRequest::prepare(&d, id, &v, &mut || {
                calls += 1;
                Ok(())
            }),
            Err(Error::InvalidValue)
        ));
        assert_eq!(calls, 0);
        assert_eq!(format!("{v:?}"), before);
        release(&mut heap, &v, id);
        heap.assert_empty();
    }
}
#[test]
fn every_request_copy_failure_leaves_input_usable_and_retry_is_explicit() {
    type V = crate::runtime::value::Value;
    let (d, id, _) = definitions();
    let mut heap = ArrayHeap::new(4096);
    let v = input::<V>(&mut heap, id);
    let mut count = 0;
    let prepared = PreparedRequest::prepare(&d, id, &v, &mut || {
        count += 1;
        Ok(())
    })
    .unwrap();
    for fail in 0..count {
        let mut n = 0;
        assert!(matches!(
            PreparedRequest::prepare(&d, id, &v, &mut || {
                n += 1;
                if n == fail + 1 {
                    Err(Error::Allocation)
                } else {
                    Ok(())
                }
            }),
            Err(Error::Allocation)
        ));
        let mut called = false;
        PreparedRequest::new(&d, id, &v)
            .unwrap()
            .with_request(|r| {
                called = true;
                assert_eq!(r.stdin, [255, 0, 13, 10]);
            })
            .unwrap();
        assert!(called);
    }
    drop(prepared);
    release(&mut heap, &v, id);
    heap.assert_empty();
}
#[test]
fn request_and_outcome_layout_mismatches_are_rejected_without_heap_changes() {
    type V = crate::vm::Value;
    let (d, id, outcome) = definitions();
    let mut heap = ArrayHeap::new(4096);
    let v = input::<V>(&mut heap, id);
    let mut wrong = d.clone();
    wrong[id.0].fields.swap(1, 2);
    assert!(matches!(
        PreparedRequest::new(&wrong, id, &v),
        Err(Error::InvalidLayout)
    ));
    release(&mut heap, &v, id);
    heap.assert_empty();
    for mutation in 0..4 {
        let mut wrong = d.clone();
        match mutation {
            0 => wrong[outcome.0].fields[0].ty = ir::Type::Bool,
            1 => wrong[outcome.0].fields[1].ty = ir::Type::String,
            2 => wrong[outcome.0].fields.swap(1, 2),
            _ => wrong[outcome.0].fields[2].id = ir::FieldId(0),
        }
        assert!(PreparedOutcome::<V>::new(&wrong, outcome, &mut heap).is_err());
        heap.assert_empty();
    }
    drop(PreparedOutcome::<V>::new(&d, outcome, &mut heap).unwrap());
    heap.assert_empty();
}

fn raw_outcome() -> Outcome {
    use crate::runtime::process::{Report, Started};
    Outcome {
        report: Report {
            origin: Origin {
                node_id: ir::NodeId(1),
                span: crate::source::Span::new(2, 3),
            },
            started: Started::Yes,
            stdin_requested: 0,
            stdin_accepted: 0,
            stdout_limit: 4,
            stderr_limit: 4,
            timeout: Duration::from_millis(1),
            stdout_read: 4,
            stderr_read: 0,
            stdout_retained: 4,
            stderr_retained: 0,
            progress_known: true,
            exit: Some(super::super::Exit::Code(7)),
            failure: None,
            cleanup: None,
            cleanup_reported: None,
        },
        transfers: None,
        stdout: vec![255, 0, 13, 10],
        stderr: vec![],
    }
}
#[test]
fn failed_finalization_releases_both_arrays_before_publication() {
    type V = crate::vm::Value;
    let (d, _, id) = definitions();
    for mutation in 0..3 {
        let mut heap = ArrayHeap::<V>::new(64);
        let mut prepared = PreparedOutcome::new(&d, id, &mut heap).unwrap();
        prepared.outputs.reserve_limits(4, 4).unwrap();
        let mut raw = raw_outcome();
        match mutation {
            0 => raw.report.timeout = Duration::from_nanos(1),
            1 => {
                raw.stderr = vec![0; 5];
                raw.report.stderr_retained = 5;
            }
            _ => raw.report.stdout_retained = 3,
        }
        assert!(prepared.finish(&raw).is_err());
        heap.assert_empty();
        let array = heap.allocate(8, 8).unwrap();
        for _ in 0..8 {
            heap.initialize(&array, V::integer(0, IntegerType::U8))
                .unwrap();
        }
        assert!(heap.release_owner(&array).unwrap());
        heap.free(&array).unwrap();
        heap.assert_empty();
    }
}
#[test]
fn outer_storage_is_reserved_and_final_output_uses_existing_array_ownership() {
    type V = crate::vm::Value;
    let (d, _, id) = definitions();
    let mut heap = ArrayHeap::<V>::new(64);
    let mut prepared = PreparedOutcome::new(&d, id, &mut heap).unwrap();
    prepared.outputs.reserve_limits(4, 4).unwrap();
    let address = prepared.fields.as_ptr();
    let value = prepared.finish(&raw_outcome()).unwrap();
    let f = value.fields(id).unwrap();
    assert_eq!(address, f.as_ptr());
    let stdout = f[1].array(Element::Byte).unwrap();
    let stderr = f[2].array(Element::Byte).unwrap();
    assert_eq!(
        stdout
            .borrow_values()
            .unwrap()
            .iter()
            .map(|v| v.unsigned(IntegerType::U8).unwrap())
            .collect::<Vec<_>>(),
        [255, 0, 13, 10]
    );
    assert_eq!(stderr.len(), 0);
    heap.retain(stdout).unwrap();
    assert!(!heap.release_owner(stdout).unwrap());
    assert!(matches!(
        heap.allocate(1, 8),
        Err(ArrayError::AllocationLimitExceeded)
    ));
    for array in [stdout, stderr] {
        assert!(heap.release_owner(array).unwrap());
        heap.free(array).unwrap();
    }
    heap.assert_empty();
}
