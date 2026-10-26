use cerune_lang::{bytecode, compile, compile_to_ir, ir, run_bytecode, run_vm, source::SourceMap};
#[allow(dead_code)]
#[path = "support/runtime_cases.rs"]
mod runtime_cases;
#[allow(dead_code)]
#[path = "support/string_cases.rs"]
mod string_cases;

#[test]
fn examples_preserve_copies_bytes_order_and_defaults() {
    for &(source, expected) in string_cases::PRODUCT_UPDATES {
        assert_eq!(run_vm(source).unwrap(), expected);
    }
}

#[test]
fn base_only_and_full_replacement_still_evaluate_the_base_once() {
    let source = r#"
        type P { x: i64 = 1 / 0, y: [i64; 1], }
        fn base() -> P { print("base"); return P { x: 1, y: [2] }; }
        a: P = P { ..base() };
        b: P = P { ..base(), x: 3, y: [4] };
        print(a.x); print(a.y[0]); print(b.x); print(b.y[0]);
    "#;
    assert_eq!(run_vm(source).unwrap(), "base\nbase\n1\n2\n3\n4\n");
}

#[test]
fn update_validation_keeps_nominal_types_and_field_rules() {
    let definitions =
        "type P { x: i64, y: i64 = 2, } type Q { x: i64, y: i64, } p: P = P { x: 1 };";
    for (expression, reason, span_text) in [
        ("P { ..1, x: 2 }", "update base expects P, found i64", "1"),
        (
            "P { ..Q { x: 1, y: 2 } }",
            "update base expects P, found Q",
            "Q { x: 1, y: 2 }",
        ),
        ("P { ..[p] }", "update base expects P", "[p]"),
        ("P { ..p, x: true }", "expects i64, found bool", "true"),
        ("P { ..p, z: 2 }", "has no field", "z"),
        ("P { ..p, x: 2, x: 3 }", "specified more than once", "x"),
        ("P { x: 2, ..p }", "update base must appear once", ".."),
        ("P { ..p, ..p }", "update base must appear once", ".."),
        ("P { ..p x: 2 }", "expected Comma", "x"),
    ] {
        let source = format!("{definitions} value: P = {expression};");
        let error = compile(&source).unwrap_err();
        assert!(error.message().contains(reason), "{source}: {error:?}");
        let span = error.primary_span().unwrap();
        assert_eq!(&source[span.start()..span.end()], span_text);
    }
    assert!(compile("type P { x: i64, } p: P = P {};").is_err());
    assert!(compile("type P { x: i64, } mut p: P = P { x: 1 }; p.x = 2;").is_err());
}

#[test]
fn calls_inside_the_base_accept_typed_recursion() {
    for functions in [
        "fn cycle() -> P { return P { ..cycle(), x: 1 }; }",
        "fn cycle() -> P { return P { ..other() }; } fn other() -> P { return cycle(); }",
    ] {
        compile(&format!("type P {{ x: i64, }} {functions}")).unwrap();
    }
}

#[test]
fn ir_and_bytecode_retain_the_base_replacements_and_source_positions() {
    let source =
        "// 日本語\r\ntype P { x: i64, y: i64 = 7, } p: P = P { x: 1 }; q: P = P { ..p, x: 2 };";
    let mut sources = SourceMap::new();
    let file = sources.add("update.ceru", source);
    let program = cerune_lang::compile_source_to_ir(sources.get(file).unwrap()).unwrap();
    let ir::StatementKind::Binding { value, .. } = &program.statements[1].kind else {
        panic!()
    };
    let ir::ExprKind::Construct {
        base: Some(base),
        fields,
        ..
    } = &value.kind
    else {
        panic!()
    };
    assert_eq!(sources.slice(value.span), Some("P { ..p, x: 2 }"));
    assert_eq!(sources.slice(base.span), Some("p"));
    assert_eq!(
        fields.len(),
        1,
        "inherited fields must not become default expressions"
    );
    assert_eq!(fields[0].name, "x");
    assert_eq!(sources.slice(fields[0].value.span), Some("2"));
    assert!(value.id.0 < base.id.0 && base.id.0 < fields[0].value.id.0);
    assert!(ir::text::emit(&program).contains("[copy]"));
    let bytecode = bytecode::lower(&program).unwrap();
    assert!(bytecode::format_program(&bytecode).contains("construct.from_base"));
}

#[test]
fn failures_preserve_prior_output_and_the_expression_that_stopped() {
    for &(source, code, expected, expression) in runtime_cases::UPDATE_FAILURES {
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
fn malformed_bytecode_rejects_a_nonmatching_base() {
    let program = compile_to_ir("type P { x: i64, } p: P = P { x: 1 }; q: P = P { ..p };").unwrap();
    let mut bytecode = bytecode::lower(&program).unwrap();
    let index = bytecode
        .instructions
        .iter()
        .position(|instruction| {
            matches!(
                instruction.kind,
                bytecode::InstructionKind::Construct { has_base: true, .. }
            )
        })
        .unwrap();
    bytecode.instructions[index - 1].kind = bytecode::InstructionKind::PushBool(false);
    assert!(run_bytecode(&bytecode).is_err());
}
