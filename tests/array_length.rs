use cerune_lang::{
    bytecode::{self, InstructionKind, InstructionOrigin},
    compile, compile_to_ir, ir, run_bytecode, run_vm,
    source::SourceMap,
    types::IntegerType,
    vm::VmErrorKind,
};
#[path = "support/array_cases.rs"]
mod array_cases;
#[allow(dead_code)]
#[path = "support/runtime_cases.rs"]
mod runtime_cases;

#[test]
fn examples_preserve_lengths_values_and_evaluation_order() {
    for &(source, expected) in array_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
    }
}

#[test]
fn one_array_argument_produces_an_i64_result() {
    for (source, reason) in [
        ("print(array_len());", "expects 1 argument"),
        ("print(array_len([1], [2]));", "expects 1 argument"),
        ("print(array_len(1));", "expects a fixed or dynamic array"),
        (
            "print(array_len(\"日本語\"));",
            "expects a fixed or dynamic array",
        ),
        (
            "type P { n: i64 } print(array_len(P { n: 1 }));",
            "expects a fixed or dynamic array",
        ),
        ("print(array_len([]));", "at least one value"),
        ("small: i8 = array_len([1]);", "type mismatch"),
        ("array_len([1]);", "must be used"),
        ("fn array_len(x: i64) -> i64 { return x; }", "reserved"),
        ("const array_len: i64 = 1;", "built-in"),
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.message().contains(reason), "{source}: {error:?}");
        assert!(error.primary_span().is_some());
    }
}

#[test]
fn constant_lengths_evaluate_the_operand_and_reject_runtime_calls() {
    assert_eq!(
        run_vm("const N: i64 = array_len([[1, 2]][0]); print(N);").unwrap(),
        "2\n"
    );
    assert_eq!(
        run_vm("const OK: bool = false && array_len([1 / 0]) == 1; print(OK);").unwrap(),
        "false\n"
    );
    for (source, reason) in [
        ("const N: i64 = array_len([1 / 0]);", "division-by-zero"),
        (
            "const N: i64 = array_len([[1]][1]);",
            "array-index-out-of-bounds",
        ),
        (
            "fn values() -> [i64; 1] { return [1]; } const N: i64 = array_len(values());",
            "function calls",
        ),
        ("const N: i64 = array_len([N]);", "cyclic"),
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.message().contains(reason), "{source}: {error:?}");
    }
}

#[test]
fn length_and_operand_retain_distinct_source_origins() {
    let source = "// 日本語\r\nprint(array_len([[1, 2]][0]));";
    let mut sources = SourceMap::new();
    let file = sources.add("length.ceru", source);
    let program = cerune_lang::compile_source_to_ir(sources.get(file).unwrap()).unwrap();
    let ir::StatementKind::Print { value: length } = &program.statements[0].kind else {
        panic!()
    };
    let ir::ExprKind::ArrayLength { value } = &length.kind else {
        panic!()
    };
    assert_eq!(length.ty, ir::Type::Integer(IntegerType::I64));
    assert_eq!(sources.slice(length.span), Some("array_len([[1, 2]][0])"));
    assert_eq!(sources.slice(value.span), Some("[[1, 2]][0]"));
    assert_ne!(length.id, value.id);
    let bytecode = bytecode::lower(&program).unwrap();
    let instruction = bytecode
        .instructions
        .iter()
        .find(|i| matches!(i.kind, InstructionKind::ArrayLength { .. }))
        .unwrap();
    assert_eq!(
        instruction.origin,
        InstructionOrigin::Source {
            node_id: length.id,
            span: length.span
        }
    );
    assert!(ir::text::emit(&program).contains("array_len("));
    assert!(bytecode::format_program(&bytecode).contains("array_len [i64; 2]"));
}

#[test]
fn operand_failures_keep_prior_output_and_the_original_expression() {
    for &(source, code, expected, expression) in runtime_cases::ARRAY_LENGTH_FAILURES {
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
fn malformed_bytecode_cannot_supply_a_scalar_or_a_different_array() {
    let program = compile_to_ir("print(array_len([1, 2]));").unwrap();
    let original = bytecode::lower(&program).unwrap();
    let position = original
        .instructions
        .iter()
        .position(|i| matches!(i.kind, InstructionKind::ArrayLength { .. }))
        .unwrap();
    for (element, length) in [
        (bytecode::Type::Bool, 2),
        (bytecode::Type::Integer(IntegerType::I64), 1),
    ] {
        let mut malformed = original.clone();
        malformed.instructions[position].kind = InstructionKind::ArrayLength { element, length };
        assert!(matches!(
            run_bytecode(&malformed).unwrap_err().vm_error().kind(),
            VmErrorKind::TypeMismatch { .. }
        ));
    }
    let mut malformed = original.clone();
    malformed.instructions.drain(..position);
    assert_eq!(
        run_bytecode(&malformed).unwrap_err().vm_error().kind(),
        VmErrorKind::StackUnderflow
    );
    malformed.instructions.insert(
        0,
        bytecode::Instruction::synthetic(InstructionKind::PushBool(true)),
    );
    assert!(matches!(
        run_bytecode(&malformed).unwrap_err().vm_error().kind(),
        VmErrorKind::TypeMismatch { .. }
    ));
}
