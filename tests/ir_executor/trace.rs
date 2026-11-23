use cerune_lang::{
    bytecode, compile_to_ir, ir_executor, mir, mir_executor, run_bytecode,
    runtime::trace::{Event, Incomplete},
    ssa_executor,
};

#[test]
fn recursive_example_records_the_executed_path() {
    let source = include_str!("../../examples/recursion/factorial.ceru");
    let program = compile_to_ir(source).unwrap();
    let (output, trace) = ir_executor::run_with_trace(&program, 18);
    assert_eq!(trace.incomplete(), None);
    assert_eq!(output, Ok("120\n1\n".into()));
    let output = output.unwrap();
    assert_eq!(ir_executor::run(&program).unwrap(), output);
    let mir = mir::lower(&program).unwrap();
    assert_eq!(mir_executor::run(&mir).unwrap(), output);
    assert_eq!(
        ssa_executor::run(&mir::ssa::construct(&mir).unwrap()).unwrap(),
        output
    );
    assert_eq!(
        run_bytecode(&bytecode::lower(&program).unwrap()).unwrap(),
        output
    );

    let mut seen = Vec::new();
    for &event in trace.events() {
        let (kind, site) = match event {
            Event::Call { function, site } | Event::Return { function, site } => {
                assert_eq!(program.function_definitions[function.0].name, "factorial");
                (
                    if matches!(event, Event::Call { .. }) {
                        "call"
                    } else {
                        "return"
                    },
                    site,
                )
            }
            Event::Branch { taken, site } => (if taken { "true" } else { "false" }, site),
            Event::Stop(_) => panic!("unexpected stop"),
        };
        assert!(site.node.is_some());
        seen.push((kind, &source[site.span.start()..site.span.end()]));
    }
    let inner = ("call", "factorial(n - 1)");
    let no = ("false", "n <= 1");
    let yes = ("true", "n <= 1");
    let base = ("return", "return 1;");
    let recur = ("return", "return n * factorial(n - 1);");
    assert_eq!(
        seen,
        [
            ("call", "factorial(5)"),
            no,
            inner,
            no,
            inner,
            no,
            inner,
            no,
            inner,
            yes,
            base,
            recur,
            recur,
            recur,
            recur,
            ("call", "factorial(0)"),
            yes,
            base
        ]
    );
}

#[test]
fn truncated_traces_preserve_failure_origin_and_prior_output() {
    let source = "// 日本語\r\nfn value()->i64 { print(7); return 1 / 0; } print(false && value() == 0); print(value());";
    let program = compile_to_ir(source).unwrap();
    let expected = ir_executor::run(&program).unwrap_err();
    assert_eq!(expected.output(), "false\n7\n");
    let stop = expected.runtime_failure().unwrap();
    for limit in [0, 1, 2] {
        let (result, trace) = ir_executor::run_with_trace(&program, limit);
        assert_eq!(result.unwrap_err(), expected);
        assert_eq!(trace.events().len(), limit);
        assert_eq!(
            trace.incomplete(),
            (limit < 2).then_some(Incomplete::EventLimit)
        );
        if limit == 2 {
            assert!(matches!(trace.events()[0], Event::Call { .. }));
            assert_eq!(trace.events()[1], Event::Stop(stop));
        }
    }
}

#[test]
fn implicit_entry_and_fallthrough_use_the_function_declaration() {
    let program = compile_to_ir("fn main()->void { print(42); }").unwrap();
    let f = &program.function_definitions[0];
    let (output, trace) = ir_executor::run_with_trace(&program, 2);
    assert_eq!(output.unwrap(), "42\n");
    let site = cerune_lang::runtime::trace::Site {
        node: None,
        span: f.span,
    };
    assert_eq!(
        trace.events(),
        [
            Event::Call {
                function: f.id,
                site
            },
            Event::Return {
                function: f.id,
                site
            }
        ]
    );
}

#[test]
fn zero_limit_without_events_is_a_complete_trace() {
    let program = compile_to_ir("print(7);").unwrap();
    let (output, trace) = ir_executor::run_with_trace(&program, 0);
    assert_eq!(output.unwrap(), "7\n");
    assert_eq!(trace.incomplete(), None);
    assert!(trace.events().is_empty());
}

#[test]
fn call_arguments_finish_left_to_right_before_entry_and_keep_named_origins() {
    let source = "// 日本語\r\nfn arg(n:i64)->i64 {print(n); return n;} fn sum(a:i64,b:i64)->i64 {return a+b;} print(sum(arg(2),arg(3)));";
    let mut sources = cerune_lang::source::SourceMap::new();
    let id = sources.add("order.ceru", source);
    let program = cerune_lang::compile_source_to_ir(sources.get(id).unwrap()).unwrap();
    let (output, trace) = ir_executor::run_with_trace(&program, 6);
    assert_eq!(output.unwrap(), "2\n3\n5\n");
    assert_eq!(trace.incomplete(), None);
    let mut visits = Vec::new();
    for &event in trace.events() {
        let (function, site) = match event {
            Event::Call { function, site } | Event::Return { function, site } => (function, site),
            _ => panic!("unexpected event: {event:?}"),
        };
        assert_eq!(site.span.source_id(), id);
        visits.push((
            program.function_definitions[function.0].name.as_str(),
            &source[site.span.start()..site.span.end()],
        ));
    }
    assert_eq!(
        visits,
        [
            ("arg", "arg(2)"),
            ("arg", "return n;"),
            ("arg", "arg(3)"),
            ("arg", "return n;"),
            ("sum", "sum(arg(2),arg(3))"),
            ("sum", "return a+b;")
        ]
    );
}

#[test]
fn compiler_generated_helpers_do_not_add_calls_to_the_trace() {
    let program = compile_to_ir("values:[i64]=array_copy([1,2]); print(values);").unwrap();
    assert!(
        program
            .function_definitions
            .iter()
            .any(|f| f.lowering.is_some())
    );
    let (result, trace) = ir_executor::run_with_trace(&program, 50);
    assert_eq!(result, ir_executor::run(&program));
    assert!(result.is_ok());
    assert_eq!(trace.incomplete(), None);
    assert!(
        trace
            .events()
            .iter()
            .all(|event| !matches!(event, Event::Call { .. } | Event::Return { .. }))
    );
}
