#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{
    RunError, ast, bytecode, compile, compile_to_bytecode, compile_to_c, compile_to_ir,
    compile_to_ir_text, compile_to_wat, compile_to_x86_64_win_asm, ir, lexer, run_vm, source::Span,
    vm::VmErrorKind,
};

#[test]
fn implements_both_truth_tables() {
    for left in [false, true] {
        for right in [false, true] {
            for (op, expected) in [("&&", left && right), ("||", left || right)] {
                let source = format!("print({left} {op} {right});");
                assert_eq!(run_vm(&source).unwrap(), format!("{expected}\n"));
            }
        }
    }
}

#[test]
fn comparisons_bind_before_and_then_or_and_parentheses_override_them() {
    let source = "print(true || false && 1 + 2 < 4 == !false);";
    let program = compile(source).unwrap();
    let ast::StmtKind::Print { value } = &program.statement(0).kind else {
        panic!()
    };
    let ast::ExprKind::Logical {
        op: ast::LogicalOp::Or,
        right,
        ..
    } = &value.kind
    else {
        panic!()
    };
    assert!(matches!(
        right.kind,
        ast::ExprKind::Logical {
            op: ast::LogicalOp::And,
            ..
        }
    ));
    assert_eq!(run_vm(source).unwrap(), "true\n");
    assert_eq!(
        run_vm("print((true || false) && false); print(true || false && false);").unwrap(),
        "false\ntrue\n"
    );
}

#[test]
fn retains_operator_token_spans_and_rejects_incomplete_forms() {
    let tokens = lexer::lex("true&&false||true").unwrap();
    assert_eq!(tokens[1].kind, lexer::TokenKind::AndAnd);
    assert_eq!(tokens[1].span, Span::new(4, 6));
    assert_eq!(tokens[3].kind, lexer::TokenKind::OrOr);
    assert_eq!(tokens[3].span, Span::new(11, 13));
    for source in [
        "print(true & false);",
        "print(true | false);",
        "print(true &&);",
        "print(|| true);",
        "print(true ||| false);",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn checks_both_operands_even_when_rhs_would_be_skipped() {
    for (source, bad) in [
        ("print(false && 1);", "1"),
        ("print(true || 0.0);", "0.0"),
        ("print(1 && true);", "1"),
        ("print([true] || false);", "[true]"),
    ] {
        let error = compile(source).unwrap_err();
        assert!(
            error.message().contains("requires bool operands"),
            "{error:?}"
        );
        let start = source.find(bad).unwrap();
        assert_eq!(
            error.primary_span(),
            Some(Span::new(start, start + bad.len()))
        );
    }
    for source in [
        "print(false && missing);",
        "print(true || missing());",
        "fn cycle() -> bool { return true || cycle(); } print(cycle());",
        "type Flag { value: bool, } print(true || Flag { value: true, });",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn skips_division_overflow_conversion_and_index_failures() {
    for rhs in [
        "1 / 0 > 0",
        "9223372036854775807 + 1 > 0",
        "i32(2147483648) == 0i32",
        "[true][1]",
    ] {
        assert_eq!(
            run_vm(&format!("print(false && ({rhs})); print(true || ({rhs}));")).unwrap(),
            "false\ntrue\n"
        );
        for op in ["true &&", "false ||"] {
            assert!(matches!(
                run_vm(&format!("print({op} ({rhs}));")),
                Err(RunError::Execution(_))
            ));
        }
    }
    assert!(matches!(
        run_vm("print(1 / 0 > 0 && false);"),
        Err(RunError::Execution(_))
    ));
}

#[test]
fn evaluates_only_needed_operands_once_in_source_order() {
    let source = r#"
        fn mark(id: i64, value: bool) -> bool { print(id); return value; }
        print(mark(1, false) && mark(2, true));
        print(mark(3, true) || mark(4, false));
        print(mark(5, true) && mark(6, true) && mark(7, false));
        print(mark(8, false) || mark(9, false) || mark(10, true));
        print(mark(11, true) && (mark(12, false) || mark(13, true)));
    "#;
    assert_eq!(
        run_vm(source).unwrap(),
        "1\nfalse\n3\ntrue\n5\n6\n7\nfalse\n8\n9\n10\ntrue\n11\n12\n13\ntrue\n"
    );
}

#[test]
fn supports_values_defaults_calls_loops_and_nested_array_index_expressions() {
    let source = r#"
        fn index(ok: bool) -> i64 { if ok { return 0; } return 1; }
        fn keep(value: bool) -> bool { return value && (false || true); }
        type Flag { tag: i64, value: bool = false || keep(true), }
        flag: Flag = Flag { tag: 0, };
        mut values: [bool; 2] = [flag.value && true, false || false];
        values[index(false || true)] = keep(false || true);
        print(values[0] && !values[1]);
        mut i: i64 = 0;
        while i < 2 && values[i] { i = i + 1; }
        print(i);
        for (i = 0; i < 2 && (values[i] || i == 1); i = i + 1) { continue; }
        print(i);
        print(true == (false || keep(true)));
    "#;
    assert_eq!(run_vm(source).unwrap(), "true\n1\n2\ntrue\n");
    for compile in [
        compile_to_c,
        llvm::compile,
        compile_to_qbe,
        compile_to_wat,
        compile_to_x86_64_win_asm,
    ] {
        compile(source).unwrap();
    }
}

#[test]
fn retains_logical_nodes_and_branch_origins() {
    let source = "print(false || true && false);";
    let program = compile_to_ir(source).unwrap();
    let ir::StatementKind::Print { value } = &program.statements[0].kind else {
        panic!()
    };
    let ir::ExprKind::Logical { left, right, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(value.ty, ir::Type::Bool);
    assert_eq!(value.span, Span::new(6, 28));
    assert_ne!(value.id, left.id);
    assert_ne!(value.id, right.id);
    let code = compile_to_bytecode(source).unwrap();
    assert!(code.instructions.iter().any(|instruction| matches!(
        instruction.kind,
        bytecode::InstructionKind::JumpIfFalse(_)
    ) && instruction.origin
        == bytecode::InstructionOrigin::Source {
            node_id: value.id,
            span: left.span
        }));
    let text = compile_to_ir_text(source).unwrap();
    assert!(text.contains("and.short_circuit.bool("));
    assert!(text.contains("or.short_circuit.bool("));
}

#[test]
fn attributes_rhs_runtime_failure_to_the_failing_expression() {
    let source = "print(false || 1 / 0 > 0);";
    let Err(RunError::Execution(error)) = run_vm(source) else {
        panic!()
    };
    assert_eq!(error.vm_error().kind(), VmErrorKind::DivisionByZero);
    let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
        panic!()
    };
    let start = source.find("1 / 0").unwrap();
    assert_eq!(span, Span::new(start, start + 5));
}

#[test]
fn emits_conditional_control_flow_in_every_backend() {
    let source = "a: bool = true; print(a && (false || 1 / 2 == 0));";
    assert!(compile_to_c(source).unwrap().contains(" && "));
    assert!(compile_to_c(source).unwrap().contains(" || "));
    assert!(llvm::compile(source).unwrap().contains("br i1"));
    assert!(compile_to_qbe(source).unwrap().contains("jnz"));
    assert!(compile_to_wat(source).unwrap().contains("if (result i32)"));
    let asm = cerune_lang::compile_to_asm_with_origins(
        source,
        cerune_lang::codegen::x86_64::Target::X86_64PcWindowsMsvc,
    )
    .unwrap();
    assert!(asm.contains("  je .Lcerune_block_"));
    assert!(asm.contains("(short-circuit-rhs)"));
    assert!(asm.contains("(short-circuit-join)"));
}
