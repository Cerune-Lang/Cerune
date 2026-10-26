use cerune_lang::{bytecode, compile, compile_to_ir, ir, run_bytecode, run_vm, source::SourceMap};
#[path = "support/iteration_cases.rs"]
mod iteration_cases;
#[allow(dead_code)]
#[path = "support/runtime_cases.rs"]
mod runtime_cases;

#[test]
fn examples_preserve_values_snapshots_and_control_flow() {
    for &(source, expected) in iteration_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
    }
}

#[test]
fn headers_require_arrays_compatible_types_and_local_bindings() {
    for (source, reason) in [
        ("for (v: infer in 1) {}", "expects a fixed or dynamic array"),
        (
            "for (v: infer in \"text\") {}",
            "expects a fixed or dynamic array",
        ),
        (
            "type P { n: i64 } for (v: infer in (P { n: 1 })) {}",
            "expects a fixed or dynamic array",
        ),
        ("for (v: u8 in [1]) {}", "type mismatch"),
        ("for (i: u64, v: infer in [1]) {}", "type mismatch"),
        ("for (mut i: i64, v: infer in [1]) {}", "must be immutable"),
        ("for (v: infer in [1]) { v = 2; }", "immutable"),
        ("for (i: infer, v: infer in [1]) { i = 2; }", "immutable"),
        ("for (v: infer, v: infer in [1]) {}", "duplicate binding"),
        ("for (v: infer in [1]) { v: i64 = 2; }", "duplicate binding"),
        ("for (v: infer in [1]) {} print(v);", "unknown binding"),
        (
            "for (i: infer, v: infer in [1]) {} print(i);",
            "unknown binding",
        ),
        ("const V: i64 = 1; for (V: infer in [1]) {}", "constant"),
        ("for (v: infer in v) {}", "unknown binding"),
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.message().contains(reason), "{source}: {error:?}");
        assert!(error.primary_span().is_some());
    }
}

#[test]
fn invalid_headers_and_reserved_in_are_rejected() {
    for source in [
        "for v: infer in [1] {}",
        "for (v in [1]) {}",
        "for (v: infer in) {}",
        "for (i: infer, v: infer, x: infer in [1]) {}",
        "for (v: infer in [1];) {}",
        "in: i64 = 1;",
        "for (v: infer in []) {}",
        "fn first() -> i64 { for (v: infer in [1]) { return v; } }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn generated_control_retains_original_operand_and_body_origins() {
    let source = "// 日本語\r\nfor (i: infer, v: infer in [2, 0]) { print(10 / v); }";
    let mut sources = SourceMap::new();
    let file = sources.add("iteration.ceru", source);
    let program = cerune_lang::compile_source_to_ir(sources.get(file).unwrap()).unwrap();
    let ir::StatementKind::Binding { value, .. } = &program.statements[0].kind else {
        panic!()
    };
    assert_eq!(sources.slice(value.span), Some("[2, 0]"));
    let text = ir::text::emit(&program);
    assert!(text.contains("$for_in_snapshot_"));
    assert!(text.contains("$for_in_length_"));
    assert!(text.contains("$for_in_cursor_"));
    assert_eq!(
        text,
        ir::text::emit(&cerune_lang::compile_source_to_ir(sources.get(file).unwrap()).unwrap())
    );
    let error = run_bytecode(&bytecode::lower(&program).unwrap()).unwrap_err();
    let failure = error.runtime_failure().unwrap();
    assert_eq!(failure.span.source_id(), file);
    assert_eq!(sources.slice(failure.span), Some("10 / v"));
    assert_eq!(error.vm_error().output(), "5\n");
}

#[test]
fn operand_and_body_failures_keep_prior_output_and_original_expressions() {
    for &(source, code, expected, expression) in runtime_cases::ITERATION_FAILURES {
        let program = compile_to_ir(source).unwrap();
        let error = run_bytecode(&bytecode::lower(&program).unwrap()).unwrap_err();
        assert_eq!(error.vm_error().output(), expected);
        let failure = error.runtime_failure().unwrap();
        assert_eq!(failure.code.name(), code);
        assert_eq!(
            &source[failure.span.start()..failure.span.end()],
            expression
        );
    }
}

#[test]
fn parsed_headers_keep_exact_spans_and_counted_for_stays_distinct() {
    use cerune_lang::{
        ast::{Item, StmtKind},
        lexer, parser,
    };
    let source = "for (i: i64, mut v: infer in [1]) {} for (mut n: i64 = 0; n < 1; n = n + 1) {}";
    let ast = parser::parse(lexer::lex(source).unwrap()).unwrap();
    let Item::Statement(first) = &ast.items[0] else {
        panic!()
    };
    let StmtKind::ForEach {
        index,
        element,
        value,
        header_span,
        ..
    } = &first.kind
    else {
        panic!()
    };
    let index = index.as_ref().unwrap();
    assert!(!index.mutable);
    assert!(element.mutable);
    assert_eq!(&source[index.span.start()..index.span.end()], "i: i64");
    assert_eq!(
        &source[element.span.start()..element.span.end()],
        "mut v: infer"
    );
    assert_eq!(&source[value.span.start()..value.span.end()], "[1]");
    assert_eq!(
        &source[header_span.start()..header_span.end()],
        "for (i: i64, mut v: infer in [1])"
    );
    let Item::Statement(second) = &ast.items[1] else {
        panic!()
    };
    assert!(matches!(second.kind, StmtKind::For { .. }));
}
