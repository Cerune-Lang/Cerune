use super::*;
use crate::{
    runtime::process::{Failure, Origin},
    source::Span,
};
use std::time::Duration;

fn definitions() -> (Vec<ir::TypeDefinition>, ir::TypeId) {
    let p = crate::compile_to_ir(
        r#"import "std:process" as process;
        type Unrelated { value: u64 }
        value:process::Started=process::Started::No{};"#,
    )
    .unwrap();
    let id = p
        .type_definitions
        .iter()
        .find(|d| d.name.ends_with("_Report"))
        .unwrap()
        .id;
    (p.type_definitions, id)
}
fn report() -> Report {
    Report {
        origin: Origin {
            node_id: ir::NodeId(47),
            span: Span::new(11, 29),
        },
        started: Started::Unknown,
        stdin_requested: 5,
        stdin_accepted: 3,
        stdout_limit: 20,
        stderr_limit: 30,
        timeout: Duration::from_millis(u64::MAX),
        stdout_read: 17,
        stderr_read: 13,
        stdout_retained: 7,
        stderr_retained: 9,
        progress_known: false,
        exit: Some(Exit::Code(u32::MAX)),
        failure: Some(Failure {
            code: Code::ProgressUnknown,
            phase: Phase::Transfer(Channel::Stderr),
        }),
        cleanup: Some(Cleanup::Incomplete {
            child_reaped: true,
            handles_closed: false,
            scope_finished: false,
            error: CleanupError::Timeout,
        }),
        cleanup_reported: Some(Cleanup::Complete {
            termination_requested: true,
        }),
    }
}
trait Inspect: Value {
    fn scalar(&self) -> Option<i128>;
    fn children(&self) -> Option<(ir::TypeId, &[Self])>;
}
impl Inspect for crate::runtime::value::Value {
    fn scalar(&self) -> Option<i128> {
        match self {
            Self::Number(crate::runtime::numeric::Number::Integer(n, _)) => Some(*n),
            Self::Bool(v) => Some(i128::from(*v)),
            _ => None,
        }
    }
    fn children(&self) -> Option<(ir::TypeId, &[Self])> {
        match self {
            Self::Aggregate { type_id, fields } => Some((*type_id, fields)),
            _ => None,
        }
    }
}
impl Inspect for crate::vm::Value {
    fn scalar(&self) -> Option<i128> {
        match self {
            Self::Integer(n, _) => Some(*n),
            Self::Bool(v) => Some(i128::from(*v)),
            _ => None,
        }
    }
    fn children(&self) -> Option<(ir::TypeId, &[Self])> {
        match self {
            Self::Aggregate { type_id, fields } => Some((ir::TypeId(*type_id), fields)),
            _ => None,
        }
    }
}
fn field<'a, V: Inspect>(v: &'a V, d: &[ir::TypeDefinition], path: &[&str]) -> &'a V {
    if path.is_empty() {
        return v;
    }
    let (id, fields) = v.children().unwrap();
    let index = d[id.0]
        .fields
        .iter()
        .position(|f| f.name == path[0])
        .unwrap();
    field(&fields[index], d, &path[1..])
}
fn number<V: Inspect>(v: &V, d: &[ir::TypeDefinition], path: &[&str]) -> i128 {
    field(v, d, path).scalar().unwrap()
}
fn variant<'a, V: Inspect>(v: &V, d: &'a [ir::TypeDefinition], path: &[&str]) -> &'a str {
    let v = field(v, d, path);
    let (id, _) = v.children().unwrap();
    &d[id.0].variants.as_ref().unwrap()[number(v, d, &["$tag"]) as usize]
}
fn addresses<V: Inspect>(v: &V, all: &mut Vec<(usize, usize)>) {
    if let Some((_, fields)) = v.children() {
        all.push((fields.as_ptr() as usize, fields.len()));
        for f in fields {
            addresses(f, all);
        }
    }
}
fn check_fields<V: Inspect>() {
    let (d, id) = definitions();
    let prepared = PreparedReport::<V>::new(&d, id).unwrap();
    let mut before = Vec::new();
    addresses(&prepared.value, &mut before);
    let v = prepared.finish(&report()).unwrap();
    let mut after = Vec::new();
    addresses(&v, &mut after);
    assert_eq!(before, after, "all aggregate buffers must stay in place");
    for (path, n) in [
        (&["origin", "node_id"][..], 47),
        (&["origin", "source_id"][..], 0),
        (&["origin", "start"][..], 11),
        (&["origin", "end"][..], 29),
        (&["stdin_requested"][..], 5),
        (&["stdin_accepted"][..], 3),
        (&["stdout_limit"][..], 20),
        (&["stderr_limit"][..], 30),
        (&["timeout_ms"][..], i128::from(u64::MAX)),
        (&["stdout_read"][..], 17),
        (&["stderr_read"][..], 13),
        (&["stdout_retained"][..], 7),
        (&["stderr_retained"][..], 9),
        (&["progress_known"][..], 0),
        (&["exit", "$Code$value"][..], i128::from(u32::MAX)),
        (&["exit", "$Signal$value"][..], 0),
        (&["cleanup", "$Incomplete$child_reaped"][..], 1),
        (&["cleanup", "$Incomplete$handles_closed"][..], 0),
        (&["cleanup", "$Incomplete$scope_finished"][..], 0),
        (
            &["cleanup_reported", "$Complete$termination_requested"][..],
            1,
        ),
    ] {
        assert_eq!(number(&v, &d, path), n, "{path:?}");
    }
    for (path, name) in [
        (&["started"][..], "Unknown"),
        (&["exit"][..], "Code"),
        (&["failure"][..], "Reason"),
        (&["failure", "$Reason$code"][..], "ProgressUnknown"),
        (&["failure", "$Reason$phase"][..], "Transfer"),
        (
            &["failure", "$Reason$phase", "$Transfer$channel"][..],
            "Stderr",
        ),
        (&["cleanup"][..], "Incomplete"),
        (&["cleanup", "$Incomplete$error"][..], "Timeout"),
        (&["cleanup_reported"][..], "Complete"),
    ] {
        assert_eq!(variant(&v, &d, path), name, "{path:?}");
    }
}
#[test]
fn every_field_and_buffer_is_preserved_in_both_executor_representations() {
    check_fields::<crate::runtime::value::Value>();
    check_fields::<crate::vm::Value>();
}
fn check_variants<V: Inspect>() {
    let (mut d, id) = definitions();
    // 番号・型名はABIではありません。入替え後も名前から対応させます。
    for t in &mut d {
        t.name = "renamed".into();
        if let Some(v) = &mut t.variants {
            v.reverse();
        }
    }
    let cases = [
        Code::InvalidRequest,
        Code::RequestLimit,
        Code::Unconfigured,
        Code::Unregistered,
        Code::ConnectionDisabled,
        Code::OutputBudget,
        Code::AllocationFailed,
        Code::NotFound,
        Code::Denied,
        Code::Resources,
        Code::Unsupported,
        Code::StartFailed,
        Code::TransferFailed,
        Code::CloseFailed,
        Code::WaitFailed,
        Code::ZeroProgress,
        Code::InvalidResponse,
        Code::OutputLimit,
        Code::Timeout,
        Code::CleanupFailed,
        Code::ProgressUnknown,
    ];
    for code in cases {
        let mut r = report();
        r.failure.as_mut().unwrap().code = code;
        let v = PreparedReport::<V>::new(&d, id)
            .unwrap()
            .finish(&r)
            .unwrap();
        assert_eq!(
            variant(&v, &d, &["failure", "$Reason$code"]),
            format!("{code:?}")
        );
    }
    for p in [
        Phase::Validate,
        Phase::Reserve,
        Phase::Start,
        Phase::CloseStdin,
        Phase::Wait,
        Phase::Cleanup,
        Phase::Transfer(Channel::Stdin),
        Phase::Transfer(Channel::Stdout),
        Phase::Transfer(Channel::Stderr),
    ] {
        let mut r = report();
        r.failure.as_mut().unwrap().phase = p;
        let v = PreparedReport::<V>::new(&d, id)
            .unwrap()
            .finish(&r)
            .unwrap();
        let expected = format!("{p:?}");
        assert_eq!(
            variant(&v, &d, &["failure", "$Reason$phase"]),
            expected.split('(').next().unwrap()
        );
        if let Phase::Transfer(c) = p {
            assert_eq!(
                variant(&v, &d, &["failure", "$Reason$phase", "$Transfer$channel"]),
                format!("{c:?}")
            );
        }
    }
    for error in [
        CleanupError::Timeout,
        CleanupError::Terminate,
        CleanupError::Reap,
        CleanupError::Close,
        CleanupError::InvalidResponse,
    ] {
        let mut r = report();
        r.cleanup_reported = Some(Cleanup::Incomplete {
            child_reaped: false,
            handles_closed: true,
            scope_finished: true,
            error,
        });
        let v = PreparedReport::<V>::new(&d, id)
            .unwrap()
            .finish(&r)
            .unwrap();
        assert_eq!(
            variant(&v, &d, &["cleanup_reported", "$Incomplete$error"]),
            format!("{error:?}")
        );
        assert_eq!(
            number(&v, &d, &["cleanup_reported", "$Incomplete$child_reaped"]),
            0
        );
        assert_eq!(
            number(&v, &d, &["cleanup_reported", "$Incomplete$handles_closed"]),
            1
        );
        assert_eq!(
            number(&v, &d, &["cleanup_reported", "$Incomplete$scope_finished"]),
            1
        );
    }
    for started in [Started::No, Started::Yes, Started::Unknown] {
        for exit in [
            None,
            Some(Exit::Code(0)),
            Some(Exit::Code(7)),
            Some(Exit::Signal(u32::MAX)),
        ] {
            let mut r = report();
            r.started = started;
            r.exit = exit;
            r.failure = None;
            r.cleanup = None;
            r.cleanup_reported = None;
            let v = PreparedReport::<V>::new(&d, id)
                .unwrap()
                .finish(&r)
                .unwrap();
            assert_eq!(variant(&v, &d, &["started"]), format!("{started:?}"));
            assert_eq!(variant(&v, &d, &["failure"]), "None");
            assert_eq!(variant(&v, &d, &["cleanup"]), "Absent");
            assert_eq!(variant(&v, &d, &["cleanup_reported"]), "Absent");
            match exit {
                None => assert_eq!(variant(&v, &d, &["exit"]), "Absent"),
                Some(Exit::Code(n)) => {
                    assert_eq!(variant(&v, &d, &["exit"]), "Code");
                    assert_eq!(number(&v, &d, &["exit", "$Code$value"]), i128::from(n));
                }
                Some(Exit::Signal(n)) => {
                    assert_eq!(variant(&v, &d, &["exit"]), "Signal");
                    assert_eq!(number(&v, &d, &["exit", "$Signal$value"]), i128::from(n));
                }
            }
        }
    }
}
#[test]
fn all_variants_use_ir_tags_in_both_representations() {
    check_variants::<crate::runtime::value::Value>();
    check_variants::<crate::vm::Value>();
}
#[test]
fn malformed_layout_is_rejected_before_reservation() {
    type V = crate::runtime::value::Value;
    let (d, id) = definitions();
    for mutation in 0..7 {
        let mut d = d.clone();
        match mutation {
            0 => d[id.0].fields.swap(2, 3),
            1 => d[id.0].fields[2].ty = ir::Type::Integer(IntegerType::I64),
            2 => d[id.0].fields[0].ty = ir::Type::Named(id), // 不正な循環
            3 => d[id.0].fields[0].ty = ir::Type::Named(ir::TypeId(usize::MAX)),
            4 => d[id.0].fields[0].id = ir::FieldId(99),
            5 => {
                let ir::Type::Named(s) = d[id.0].fields[1].ty else {
                    panic!()
                };
                d[s.0].variants.as_mut().unwrap()[1] = "No".into();
            }
            _ => {
                d[id.0].fields.pop();
            }
        }
        let mut allocations = 0;
        assert!(matches!(
            PreparedReport::<V>::prepare(&d, id, &mut |_| {
                allocations += 1;
                Ok(())
            }),
            Err(Error::InvalidLayout)
        ));
        assert_eq!(allocations, 0);
    }
}
#[test]
fn durations_are_exact_and_never_silently_truncated() {
    let (d, id) = definitions();
    for duration in [
        Duration::from_nanos(1),
        Duration::from_micros(1500),
        Duration::MAX,
    ] {
        let mut r = report();
        r.timeout = duration;
        assert!(matches!(
            PreparedReport::<crate::vm::Value>::new(&d, id)
                .unwrap()
                .finish(&r),
            Err(Error::UnrepresentableReport)
        ));
    }
    for duration in [
        Duration::ZERO,
        Duration::from_millis(1),
        Duration::from_millis(u64::MAX),
    ] {
        let mut r = report();
        r.timeout = duration;
        let v = PreparedReport::<crate::vm::Value>::new(&d, id)
            .unwrap()
            .finish(&r)
            .unwrap();
        assert_eq!(
            number(&v, &d, &["timeout_ms"]),
            duration.as_millis() as i128
        );
    }
}
thread_local! {static LIVE:std::cell::Cell<usize>=const {std::cell::Cell::new(0)};}
struct Tracked {
    id: ir::TypeId,
    fields: Vec<Self>,
}
impl Tracked {
    fn new(id: ir::TypeId, fields: Vec<Self>) -> Self {
        LIVE.with(|n| n.set(n.get() + 1));
        Self { id, fields }
    }
}
impl Drop for Tracked {
    fn drop(&mut self) {
        LIVE.with(|n| n.set(n.get() - 1));
    }
}
impl Value for Tracked {
    fn integer(_: i128, _: IntegerType) -> Self {
        Self::new(ir::TypeId(0), Vec::new())
    }
    fn boolean(_: bool) -> Self {
        Self::new(ir::TypeId(0), Vec::new())
    }
    fn aggregate(id: ir::TypeId, fields: Vec<Self>) -> Self {
        Self::new(id, fields)
    }
    fn parts(&mut self) -> (ir::TypeId, &mut [Self]) {
        (self.id, &mut self.fields)
    }
}
#[test]
fn failure_at_each_allocation_and_unused_reservations_release_all_values() {
    let (d, id) = definitions();
    let mut allocations = 0;
    let complete = PreparedReport::<Tracked>::prepare(&d, id, &mut |_| {
        allocations += 1;
        Ok(())
    })
    .unwrap();
    assert!(allocations > 10);
    drop(complete);
    assert_eq!(LIVE.with(|n| n.get()), 0);
    for fail_at in 0..allocations {
        let mut call = 0;
        assert!(matches!(
            PreparedReport::<Tracked>::prepare(&d, id, &mut |_| {
                call += 1;
                if call == fail_at + 1 {
                    Err(Error::Allocation)
                } else {
                    Ok(())
                }
            }),
            Err(Error::Allocation)
        ));
        assert_eq!(LIVE.with(|n| n.get()), 0, "failed allocation {fail_at}");
    }
    let v = PreparedReport::<Tracked>::new(&d, id)
        .unwrap()
        .finish(&report())
        .unwrap();
    drop(v);
    assert_eq!(LIVE.with(|n| n.get()), 0);
}

#[test]
fn virtual_source_origin_and_full_width_counts_are_not_replaced() {
    let (d, id) = definitions();
    let mut r = report();
    r.origin.span = d[id.0].span;
    assert_ne!(r.origin.span.source_id().index(), 0);
    r.stdin_requested = usize::MAX;
    r.progress_known = true;
    r.cleanup = Some(Cleanup::Complete {
        termination_requested: false,
    });
    let v = PreparedReport::<crate::vm::Value>::new(&d, id)
        .unwrap()
        .finish(&r)
        .unwrap();
    assert_eq!(
        number(&v, &d, &["origin", "source_id"]),
        r.origin.span.source_id().index() as i128
    );
    assert_eq!(
        number(&v, &d, &["origin", "start"]),
        r.origin.span.start() as i128
    );
    assert_eq!(
        number(&v, &d, &["origin", "end"]),
        r.origin.span.end() as i128
    );
    assert_eq!(number(&v, &d, &["stdin_requested"]), usize::MAX as i128);
    assert_eq!(number(&v, &d, &["progress_known"]), 1);
    assert_eq!(variant(&v, &d, &["cleanup"]), "Complete");
    assert_eq!(
        number(&v, &d, &["cleanup", "$Complete$termination_requested"]),
        0
    );
}
