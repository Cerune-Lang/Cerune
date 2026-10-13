#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{
    RunError, bytecode, compile, compile_to_bytecode, compile_to_bytecode_text, compile_to_c,
    compile_to_ir, compile_to_ir_text, compile_to_wat, compile_to_x86_64_win_asm, ir, lexer,
    run_vm,
    source::Span,
    types::IntegerType,
    vm::{self, IntegerOperation, VmErrorKind},
};

fn failure(source: &str) -> VmErrorKind {
    let Err(RunError::Execution(error)) = run_vm(source) else {
        panic!("{source}")
    };
    assert!(error.origin().is_some(), "{source}");
    error.vm_error().kind()
}

fn edge_values(ty: IntegerType) -> Vec<i128> {
    [
        ty.minimum(),
        ty.minimum() + 1,
        -3,
        -1,
        0,
        1,
        3,
        ty.maximum() - 1,
        ty.maximum(),
    ]
    .into_iter()
    .filter(|value| ty.contains(*value))
    .collect()
}

#[test]
fn bitwise_operations_and_remainders_cover_every_integer_kind() {
    for ty in IntegerType::ALL {
        for left in edge_values(ty) {
            for right in edge_values(ty) {
                for (op, expected) in [
                    ("&", left & right),
                    ("|", left | right),
                    ("^", left ^ right),
                ] {
                    let source = format!("print({left}{0} {op} {right}{0});", ty.name());
                    assert_eq!(
                        run_vm(&source).unwrap(),
                        format!("{expected}\n"),
                        "{source}"
                    );
                }
                let source = format!("print({left}{0} % {right}{0});", ty.name());
                if right == 0 {
                    assert_eq!(failure(&source), VmErrorKind::RemainderByZero);
                } else {
                    let expected = left % right;
                    assert_eq!(
                        run_vm(&source).unwrap(),
                        format!("{expected}\n"),
                        "{source}"
                    );
                }
            }
            let expected = if ty.is_signed() {
                !left
            } else {
                ty.maximum() ^ left
            };
            let source = format!("print(~({left}{}));", ty.name());
            assert_eq!(run_vm(&source).unwrap(), format!("{expected}\n"));
        }
    }
}

#[test]
fn shifts_check_the_original_width_before_performing_the_operation() {
    for ty in IntegerType::ALL {
        for left in edge_values(ty) {
            for count in 0..ty.bit_width() {
                let source = format!("print({left}{0} << {count});", ty.name());
                let expected = left * 2i128.pow(u32::from(count));
                if expected >= ty.minimum() && expected <= ty.maximum() {
                    assert_eq!(
                        run_vm(&source).unwrap(),
                        format!("{expected}\n"),
                        "{source}"
                    );
                } else {
                    assert_eq!(
                        failure(&source),
                        VmErrorKind::IntegerOverflow {
                            operation: IntegerOperation::ShiftLeft,
                            ty: bytecode::Type::Integer(ty),
                        },
                        "{source}"
                    );
                }
                let source = format!("print({left}{0} >> {count});", ty.name());
                // 負の奇数も、0ではなく負の無限大へ寄せる右シフトです。
                let expected = left.div_euclid(2i128.pow(u32::from(count)));
                assert_eq!(
                    run_vm(&source).unwrap(),
                    format!("{expected}\n"),
                    "{source}"
                );
            }
        }
        for count in [
            -1,
            i128::from(ty.bit_width()),
            i128::from(ty.bit_width()) + 1,
            ty.maximum(),
        ] {
            if !ty.contains(count) {
                continue;
            }
            for op in ["<<", ">>"] {
                let source = format!("print(0{0} {op} {count}{0});", ty.name());
                assert_eq!(
                    failure(&source),
                    VmErrorKind::InvalidShiftCount { ty, count }
                );
            }
        }
    }
}

#[test]
fn complement_covers_every_eight_bit_pattern() {
    for ty in [IntegerType::I8, IntegerType::U8] {
        for value in ty.minimum()..=ty.maximum() {
            let source = format!(
                "x: {0} = {value}; print(~~x); print(x & ~x); print(x | ~x);",
                ty.name()
            );
            let ones = if ty.is_signed() { -1 } else { 255 };
            assert_eq!(run_vm(&source).unwrap(), format!("{value}\n0\n{ones}\n"));
        }
    }
}

#[test]
fn precedence_and_associativity_are_c_like() {
    for (expr, expected) in [
        ("17 % 5 * 2", "4"),
        ("20 / 3 % 4", "2"),
        ("1 << 2 + 1", "8"),
        ("32 >> 1 >> 2", "4"),
        ("1 | 3 ^ 2 & 6", "1"),
        ("~1 & 3", "2"),
        ("1 << 2 < 5", "true"),
        ("(3 & 1) == 1 && false || true", "true"),
        ("-7 % 3", "-1"),
        ("7 % -3", "1"),
        ("-3 >> 1", "-2"),
        ("(-1) << 63", "-9223372036854775808"),
    ] {
        assert_eq!(
            run_vm(&format!("print({expr});")).unwrap(),
            format!("{expected}\n"),
            "{expr}"
        );
    }
    // 比較の優先順位が高いので、ビット判定の比較には括弧が必要です。
    assert!(compile("print(3 & 1 == 1);").is_err());
}

#[test]
fn rejects_noninteger_mixed_width_and_incomplete_operands() {
    for op in ["%", "&", "|", "^", "<<", ">>"] {
        for (left, right) in [
            ("true", "false"),
            ("1.0", "2.0"),
            ("1i8", "1u8"),
            ("1u8", "1i64"),
            ("[1]", "[2]"),
        ] {
            assert!(compile(&format!("print({left} {op} {right});")).is_err());
        }
        assert!(compile(&format!("print(1 {op});")).is_err());
    }
    for source in [
        "print(~true);",
        "print(~1.0);",
        "print(~[1]);",
        "print(~);",
        "print(1 <<< 2);",
        "print(1 >>> 2);",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn lexer_preserves_new_operator_spans_and_existing_punctuation() {
    use lexer::TokenKind::*;
    let source = "% & | ^ ~ << >> && || < <= > >=";
    let expected = [
        Percent,
        Ampersand,
        Pipe,
        Caret,
        Tilde,
        ShiftLeft,
        ShiftRight,
        AndAnd,
        OrOr,
        Less,
        LessEqual,
        Greater,
        GreaterEqual,
    ];
    let tokens = lexer::lex(source).unwrap();
    let mut start = 0;
    for ((text, kind), token) in source.split(' ').zip(expected).zip(tokens) {
        assert_eq!(token.kind, kind);
        assert_eq!(token.span, Span::new(start, start + text.len()));
        start += text.len() + 1;
    }
    assert_eq!(run_vm("print(convert<u8>(7) >> 1);").unwrap(), "3\n");
}

#[test]
fn keeps_typed_nodes_bytecode_and_failure_origins() {
    for (op, name, expected_kind) in [
        ("%", "rem", ir::BinaryOp::Remainder),
        ("&", "bit_and", ir::BinaryOp::BitAnd),
        ("|", "bit_or", ir::BinaryOp::BitOr),
        ("^", "bit_xor", ir::BinaryOp::BitXor),
        ("<<", "shl.checked", ir::BinaryOp::ShiftLeft),
        (">>", "shr", ir::BinaryOp::ShiftRight),
    ] {
        let source = format!("print(1u8 {op} 1);");
        let program = compile_to_ir(&source).unwrap();
        let ir::StatementKind::Print { value } = &program.statements[0].kind else {
            panic!()
        };
        assert_eq!(value.ty, ir::Type::Integer(IntegerType::U8));
        assert!(matches!(value.kind, ir::ExprKind::Binary { op, .. } if op == expected_kind));
        let code = compile_to_bytecode(&source).unwrap();
        assert_eq!(
            code.instructions[2].origin,
            bytecode::InstructionOrigin::Source {
                node_id: value.id,
                span: value.span
            }
        );
        assert!(
            compile_to_ir_text(&source)
                .unwrap()
                .contains(&format!("{name}.u8("))
        );
        assert!(
            compile_to_bytecode_text(&source)
                .unwrap()
                .contains(&format!("{name}.u8"))
        );
    }
    for expression in ["128u8 << 1", "1u8 >> 8", "7 % 0"] {
        let source = format!("print({expression});");
        let Err(RunError::Execution(error)) = run_vm(&source) else {
            panic!()
        };
        let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
            panic!()
        };
        assert_eq!(span, Span::new(6, 6 + expression.len()));
    }
    assert!(
        compile_to_ir_text("print(~0u8);")
            .unwrap()
            .contains("bit_not.u8(")
    );
    assert!(
        compile_to_bytecode_text("print(~0u8);")
            .unwrap()
            .contains("bit_not.u8")
    );
}

#[test]
fn bit_operations_are_eager_but_logical_operators_can_skip_them() {
    let source = "fn mark(value: u8) -> u8 { print(value); return value; } print(mark(1) | (mark(2) ^ mark(3))); print(~mark(4));";
    assert_eq!(run_vm(source).unwrap(), "1\n2\n3\n1\n4\n251\n");
    assert_eq!(
        failure("print(0u8 & (1u8 % 0));"),
        VmErrorKind::RemainderByZero
    );
    assert_eq!(
        run_vm("print(false && (128u8 << 1) == 0u8); print(true || (1 % 0) == 0);").unwrap(),
        "false\ntrue\n"
    );
    assert!(compile("print(true || (false & false));").is_err());
}

#[test]
fn works_in_defaults_calls_arrays_loops_and_conversions() {
    let source = r#"
        type Flags { tag: i64, bits: u8 = 1 << 2, }
        fn advance(value: u8) -> u8 { return (value << 1) | 1; }
        flags: Flags = Flags { tag: 0, };
        mut values: [u8; 2] = [flags.bits, ~0];
        for (mut i: i64 = 0; i < 2; i = i + 1) {
            values[(i + 1) % 2] = advance(values[i] & 7);
        }
        print(values[0]); print(values[1]); print(i16(values[0] << 1));
    "#;
    assert_eq!(run_vm(source).unwrap(), "3\n9\n6\n");
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
fn malformed_bytecode_does_not_bypass_operand_types() {
    let mut code = compile_to_bytecode("print(1u8 & 2u8);").unwrap();
    code.instructions[2].kind = bytecode::InstructionKind::BitAnd(IntegerType::I8);
    assert!(matches!(
        vm::run(&code).unwrap_err().kind(),
        VmErrorKind::TypeMismatch { .. }
    ));
    let mut code = compile_to_bytecode("print(~0u8);").unwrap();
    code.instructions[1].kind = bytecode::InstructionKind::BitNot(IntegerType::I8);
    assert!(matches!(
        vm::run(&code).unwrap_err().kind(),
        VmErrorKind::TypeMismatch { .. }
    ));
}

#[test]
fn comparisons_contextualize_the_literal_without_retyping_a_nested_integer_expression() {
    for ty in IntegerType::ALL {
        let source = format!(
            "bits: {} = 3; print((bits & (1 << 1)) != 0); print(0 != (bits & (1 << 1))); print((bits + (1 + bits)) > 0);",
            ty.name()
        );
        assert_eq!(run_vm(&source).unwrap(), "true\ntrue\ntrue\n");
    }
    assert!(compile("bits: u8 = 3; print((bits & 1) == 0i64);").is_err());
}

#[test]
fn function_failures_keep_the_failing_operand_and_operation() {
    let source = "fn bad() -> u8 { return 128u8 << 1; } print(bad() & (1u8 % 0));";
    let Err(RunError::Execution(error)) = run_vm(source) else {
        panic!()
    };
    assert!(error.vm_error().function_id().is_some());
    assert!(matches!(
        error.vm_error().kind(),
        VmErrorKind::IntegerOverflow {
            operation: IntegerOperation::ShiftLeft,
            ..
        }
    ));
    let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
        panic!()
    };
    let start = source.find("128u8 << 1").unwrap();
    assert_eq!(span, Span::new(start, start + "128u8 << 1".len()));
}
