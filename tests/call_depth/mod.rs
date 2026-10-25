use cerune_lang::{
    bytecode, compile_to_ir, ir, ir_executor, mir, mir_executor, runtime::call, ssa_executor, vm,
};
use std::num::NonZeroU32;

fn compare(source: &str, limit: u32) -> Result<String, (call::Failure, String)> {
    compare_ir(&compile_to_ir(source).unwrap(), limit)
}
fn compare_ir(p: &ir::Program, limit: u32) -> Result<String, (call::Failure, String)> {
    let limit = NonZeroU32::new(limit).unwrap();
    let before = ir::text::emit(p);
    let m = mir::lower(p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(p).unwrap();
    let h = ir_executor::run_with_call_limit(p, limit).map_err(|e| {
        let origin = e.origin().map(|o| call::Origin {
            node_id: o.node_id,
            span: o.span,
        });
        assert_eq!(origin, e.call_failure().unwrap().attempted().origin);
        (
            e.call_failure().expect("depth failure").clone(),
            e.output().to_owned(),
        )
    });
    let mm = mir_executor::run_with_call_limit(&m, limit).map_err(|e| {
        let origin = e.origin().map(|o| call::Origin {
            node_id: o.node_id,
            span: o.span,
        });
        assert_eq!(origin, e.call_failure().unwrap().attempted().origin);
        (
            e.call_failure().expect("depth failure").clone(),
            e.output().to_owned(),
        )
    });
    let ss = ssa_executor::run_with_call_limit(&s, limit).map_err(|e| {
        let origin = e.origin().map(|o| call::Origin {
            node_id: o.node_id,
            span: o.span,
        });
        assert_eq!(origin, e.call_failure().unwrap().attempted().origin);
        (
            e.call_failure().expect("depth failure").clone(),
            e.output().to_owned(),
        )
    });
    let v = vm::run_with_call_limit(&b, limit).map_err(|e| {
        let instructions = match e.function_id() {
            Some(id) => &b.functions[id].instructions,
            None => &b.instructions,
        };
        let origin = match instructions[e.instruction_index()].origin {
            bytecode::InstructionOrigin::Source { node_id, span } => {
                Some(call::Origin { node_id, span })
            }
            bytecode::InstructionOrigin::Synthetic => None,
        };
        assert_eq!(origin, e.call_failure().unwrap().attempted().origin);
        (
            e.call_failure().expect("depth failure").clone(),
            e.output().to_owned(),
        )
    });
    assert_eq!(h, mm, "MIR");
    assert_eq!(h, ss, "SSA");
    assert_eq!(h, v, "VM");
    assert_eq!(before, ir::text::emit(p));
    if let Err((f, _)) = &h {
        assert_eq!(f.limit(), limit);
        assert!(f.record().is_ascii());
    }
    h
}
#[test]
fn arguments_finish_before_entry_and_failure_preserves_caller() {
    let source = r#"
fn mark(x: i64) -> i64 { print(x); return x; }
fn pair(x: i64, y: i64) -> i64 { print("body"); return x + y; }
fn outer() -> i64 { return pair(mark(10), mark(20)); }
print(outer()); print("after");
"#;
    // Arguments execute at depth 2, then pair enters at depth 2 as well.
    assert_eq!(compare(source, 2).unwrap(), "10\n20\nbody\n30\nafter\n");
    let (failure, output) = compare(source, 1).unwrap_err();
    assert_eq!(output, "");
    assert_eq!(failure.active().len(), 1);
    let o = failure.attempted().origin.unwrap();
    assert_eq!(&source[o.span.start()..o.span.end()], "mark(10)");
    // Inline effects finish before the attempted pair call (no nested source calls).
    let source = r#"fn pair(x:i64,y:i64)->i64 {print("body");return x+y;} fn outer()->i64 {print("before"); return pair(10,20);} print(outer());print("after");"#;
    let (failure, output) = compare(source, 1).unwrap_err();
    assert_eq!(output, "before\n");
    let o = failure.attempted().origin.unwrap();
    assert_eq!(&source[o.span.start()..o.span.end()], "pair(10,20)");
    assert!(
        failure
            .record()
            .starts_with("call-v1 reason=depth-limit limit=1 function=")
    );
}
#[test]
fn explicit_main_counts_but_implicit_entry_does_not() {
    assert_eq!(
        compare("fn leaf()->void {print(7);} leaf();", 1).unwrap(),
        "7\n"
    );
    let (failure, output) =
        compare("fn leaf()->void {print(7);} fn main()->void {leaf();}", 1).unwrap_err();
    assert_eq!(output, "");
    assert_eq!(failure.active().len(), 1);
    assert_eq!(failure.active()[0].origin, None);
    assert!(failure.record().contains("origin=host"));
}
#[test]
fn generated_helpers_and_returned_calls_do_not_use_depth() {
    let source = include_str!("../../examples/call_frames/main.ceru");
    let p = compile_to_ir(source).unwrap();
    assert!(p.function_definitions.iter().any(|f| f.lowering.is_some()));
    assert_eq!(
        compare_ir(&p, 2).unwrap(),
        include_str!("../../examples/call_frames/main.stdout").replace("\r\n", "\n")
    );
    assert!(compare_ir(&p, 1).is_err());
    assert_eq!(compare("fn leaf(x:i64)->i64 {if x==2 {return 20;} return x;} mut i:i64=0; while i<4 {print(leaf(i));i=i+1;}",1).unwrap(),"0\n1\n20\n3\n");
}
#[test]
fn generic_instantiations_are_source_calls_and_short_circuit_skips_calls() {
    let source = r#"fn identity<T>(x:T)->T {return x;} fn forward<T>(x:T)->T {return identity::<T>(x);} print(forward::<i64>(42));"#;
    assert_eq!(compare(source, 2).unwrap(), "42\n");
    assert_eq!(compare(source, 1).unwrap_err().0.active().len(), 1);
    assert_eq!(compare("fn yes()->bool {return true;} fn test()->void {print(false && yes());print(true || yes());} test();",1).unwrap(),"false\ntrue\n");
}
#[test]
fn resource_failure_is_not_depth_failure() {
    let p = compile_to_ir(r#"fn make()->string {return concat("a","b");} print(make());"#).unwrap();
    let mut p = p;
    p.string_heap_limit = 0;
    let limit = NonZeroU32::new(1).unwrap();
    let h = ir_executor::run_with_call_limit(&p, limit).unwrap_err();
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(&p).unwrap();
    let mm = mir_executor::run_with_call_limit(&m, limit).unwrap_err();
    let ss = ssa_executor::run_with_call_limit(&s, limit).unwrap_err();
    let v = vm::run_with_call_limit(&b, limit).unwrap_err();
    assert!(
        h.call_failure().is_none()
            && mm.call_failure().is_none()
            && ss.call_failure().is_none()
            && v.call_failure().is_none()
    );
    assert_eq!(h.runtime_failure(), mm.runtime_failure());
    assert_eq!(h.runtime_failure(), ss.runtime_failure());
    assert_eq!(
        h.runtime_failure().unwrap().code,
        cerune_lang::runtime::FailureCode::AllocationLimitExceeded
    );
    assert_eq!(v.kind(), vm::VmErrorKind::AllocationLimitExceeded);
}

#[test]
fn depth_example_and_module_origins() {
    assert_eq!(
        compare(include_str!("../../examples/call_frames/depth.ceru"), 2).unwrap(),
        include_str!("../../examples/call_frames/depth.stdout").replace("\r\n", "\n")
    );
    let source =
        r#"import "std:num" as num; fn parse()->void {print(num::parse_i64("42"));} parse();"#;
    let p = cerune_lang::modules::from_source(source)
        .unwrap()
        .to_ir()
        .unwrap();
    assert!(compare_ir(&p, 2).is_ok());
    let (failure, _) = compare_ir(&p, 1).unwrap_err();
    let target = &p.function_definitions[failure.attempted().function.0];
    assert!(target.name.ends_with("parse_i64"));
    assert!(target.lowering.is_none());
    assert_ne!(
        failure.attempted().origin.unwrap().span.source_id(),
        cerune_lang::source::SourceId::ANONYMOUS
    );
    assert!(failure.record().contains(" file="));
}
#[test]
fn snapshot_orders_frames_from_entry_to_caller() {
    let source = "fn c()->void {print(3);} fn b()->void {c();} fn a()->void {b();} a();";
    let (failure, output) = compare(source, 2).unwrap_err();
    assert_eq!(output, "");
    let p = compile_to_ir(source).unwrap();
    let names: Vec<_> = failure
        .active()
        .iter()
        .map(|f| p.function_definitions[f.function.0].name.as_str())
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(
        p.function_definitions[failure.attempted().function.0].name,
        "c"
    );
    assert_eq!(failure.record().lines().count(), 3);
    assert_eq!(compare(source, 3).unwrap(), "3\n");
}

#[test]
fn argument_failure_precedes_depth_check_and_skips_later_arguments() {
    let source = r#"fn later()->i64 {print("later");return 1;} fn pair(a:i64,b:i64)->void {print("body");} fn outer()->void {print("before"); pair(1/0,later());} outer();print("after");"#;
    let p = compile_to_ir(source).unwrap();
    let limit = NonZeroU32::new(1).unwrap();
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(&p).unwrap();
    let h = ir_executor::run_with_call_limit(&p, limit).unwrap_err();
    let mm = mir_executor::run_with_call_limit(&m, limit).unwrap_err();
    let ss = ssa_executor::run_with_call_limit(&s, limit).unwrap_err();
    let v = vm::run_with_call_limit(&b, limit).unwrap_err();
    assert!(
        h.call_failure().is_none()
            && mm.call_failure().is_none()
            && ss.call_failure().is_none()
            && v.call_failure().is_none()
    );
    assert_eq!(h.runtime_failure(), mm.runtime_failure());
    assert_eq!(h.runtime_failure(), ss.runtime_failure());
    let failure = h.runtime_failure().unwrap();
    assert_eq!(
        cerune_lang::runtime::FailureCode::from_vm(v.kind()),
        Some(failure.code)
    );
    assert_eq!(&source[failure.span.start()..failure.span.end()], "1/0");
    for output in [h.output(), mm.output(), ss.output(), v.output()] {
        assert_eq!(output, "before\n");
    }
}

mod frames;
