#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{
    RunError, bytecode, compile, compile_to_bytecode, compile_to_bytecode_text, compile_to_c,
    compile_to_ir, compile_to_ir_text, compile_to_wat, compile_to_x86_64_win_asm, ir, run_vm,
    source::{ConversionSyntax, Span},
    types::{IntegerType, NumericType},
    vm::{self, NumericConversionFailure as Failure, VmErrorKind},
};

fn failure(expression: &str, expected: Failure) {
    let source = format!("print({expression});");
    let RunError::Execution(error) = run_vm(&source).unwrap_err() else {
        panic!("expected execution error: {source}");
    };
    let VmErrorKind::NumericConversionFailed { reason, .. } = error.vm_error().kind() else {
        panic!("expected conversion error: {error:?}");
    };
    assert_eq!(reason, expected, "{source}");
    let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
        panic!("expected source origin");
    };
    assert_eq!(span, Span::new(6, 6 + expression.len()));
}

#[test]
fn every_numeric_pair_supports_both_spellings() {
    for from in [
        "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64",
    ] {
        for to in [
            "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64",
        ] {
            let value = if from.starts_with('f') { "42.0" } else { "42" };
            for spelling in [to.to_owned(), format!("convert<{to}>")] {
                let source = format!("value: {from} = {value}; print(i64({spelling}(value)));");
                assert_eq!(run_vm(&source).unwrap(), "42\n", "{source}");
                for emit in [
                    compile_to_c,
                    llvm::compile,
                    compile_to_qbe,
                    compile_to_wat,
                    compile_to_x86_64_win_asm,
                ] {
                    emit(&source).unwrap();
                }
            }
        }
    }
}

#[test]
fn integer_to_float_checks_precision_including_the_i64_upper_boundary() {
    for (ty, values) in [
        ("f32", &[0, -1, 16777216, 16777218, i64::MIN][..]),
        (
            "f64",
            &[
                0,
                -1,
                9007199254740992,
                9007199254740994,
                i64::MIN,
                9223372036854774784,
            ][..],
        ),
    ] {
        for value in values {
            assert_eq!(
                run_vm(&format!("print(i64({ty}({value}))); ")).unwrap(),
                format!("{value}\n")
            );
        }
    }
    for expression in [
        "f32(16777217)",
        "f32(-16777217)",
        "f64(9007199254740993)",
        "f64(-9007199254740993)",
        "f32(9223372036854775807)",
        "f64(9223372036854775807)",
    ] {
        failure(expression, Failure::Inexact);
    }
}

#[test]
fn float_to_integer_checks_every_destination_range() {
    for ty in [
        IntegerType::I8,
        IntegerType::U8,
        IntegerType::I16,
        IntegerType::U16,
        IntegerType::I32,
        IntegerType::U32,
        IntegerType::I64,
    ] {
        let name = ty.name();
        let max = if ty == IntegerType::I64 {
            9223372036854774784
        } else {
            ty.maximum()
        };
        for value in [ty.minimum(), 0, max] {
            assert_eq!(
                run_vm(&format!("print({name}({value}.0));")).unwrap(),
                format!("{value}\n")
            );
        }
        failure(
            &format!("{name}({}.0)", ty.maximum() + 1),
            Failure::OutOfRange,
        );
        let below = if ty == IntegerType::I64 {
            -9223372036854777856i128
        } else {
            ty.minimum() - 1
        };
        failure(&format!("{name}({below}.0)"), Failure::OutOfRange);
        for float in ["f32", "f64"] {
            failure(&format!("{name}(1.5{float})"), Failure::Inexact);
            failure(&format!("{name}(-0.0{float})"), Failure::NegativeZero);
            failure(
                &format!("{name}(1.0{float} / 0.0{float})"),
                Failure::NotFinite,
            );
            failure(
                &format!("{name}(0.0{float} / 0.0{float})"),
                Failure::NotFinite,
            );
        }
    }
}

#[test]
fn float_width_changes_preserve_exact_values_and_reject_rounding() {
    assert_eq!(
        run_vm("print(f32(1.5)); print(f64(0.1f32) == 0.1); print(f32(f64(0.1f32)) == 0.1f32);")
            .unwrap(),
        "1.5\nfalse\ntrue\n"
    );
    for expression in ["f32(0.1)", "f32(1e-50)", "f32(-1e-50)"] {
        failure(expression, Failure::Inexact);
    }
    for expression in ["f32(1e100)", "f32(-1e100)"] {
        failure(expression, Failure::OutOfRange);
    }
    for expression in ["f64(0.0f32 / 0.0f32)", "f32(0.0 / 0.0)"] {
        failure(expression, Failure::NaN);
    }
}

#[test]
fn float_width_changes_preserve_infinity_and_zero_signs() {
    let source = "
        inf: f64 = 1.0 / 0.0;
        print(f64(f32(inf)) == inf);
        print(f64(f32(-inf)) == -inf);
        print(1.0 / f64(-0.0f32) == -inf);
        print(1.0f32 / f32(-0.0) == f32(-inf));
        print(1.0 / f64(0.0f32) == inf);
        print(1.0f32 / f32(0.0) == f32(inf));
    ";
    assert_eq!(run_vm(source).unwrap(), "true\n".repeat(6));
}

#[test]
fn printed_zero_does_not_hide_the_preserved_sign() {
    assert_eq!(
        run_vm("print(-0.0); print(-0.0f32); print(f32(-0.0)); print(f64(-0.0f32)); print(f64(-0.0)); print(f32(-0.0f32)); print(f64(0)); print(f32(0));").unwrap(),
        "-0\n-0\n-0\n-0\n-0\n-0\n0\n0\n"
    );
}

#[test]
fn destination_does_not_change_input_arithmetic_or_evaluate_it_twice() {
    let compact = "fn next() -> i32 { print(7); return 3; } print(f64(next())); print(f64(1 / 2)); print(f64(1) / f64(2));";
    let explicit = compact.replace("f64(", "convert<f64>(");
    assert_eq!(run_vm(compact).unwrap(), "7\n3\n0\n0.5\n");
    assert_eq!(run_vm(&explicit).unwrap(), run_vm(compact).unwrap());
    for emit in [compile_to_c, llvm::compile, compile_to_qbe, compile_to_wat] {
        // 表記が変わると診断のUTF-8範囲も変わります。各生成物の決定性を確認します。
        for source in [compact, explicit.as_str()] {
            assert_eq!(emit(source).unwrap(), emit(source).unwrap());
        }
    }
    // 全経路が元の表記のバイト範囲を診断に保持します。
    // 両表記の実行結果はnative_assemblyの既知出力のケースで比較します。
    for source in [compact, explicit.as_str()] {
        assert_eq!(
            compile_to_x86_64_win_asm(source)
                .unwrap()
                .matches("  callq cerune_fn_next_0\n")
                .count(),
            1
        );
    }
    let RunError::Execution(error) = run_vm("print(f64(1 / 0));").unwrap_err() else {
        panic!()
    };
    assert_eq!(error.vm_error().kind(), VmErrorKind::DivisionByZero);
    let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
        panic!()
    };
    assert_eq!(span, Span::new(10, 15));
}

#[test]
fn conversions_compose_with_defaults_arrays_returns_and_short_circuiting() {
    let source = "
        type Sample { value: f64 = f64(3), tag: bool, }
        fn read(value: i16) -> f64 { return convert<f64>(value); }
        sample: Sample = Sample { tag: true, };
        mut values: [f64; 2] = [sample.value, read(5)];
        for (mut i: i64 = 0; i < 2; i = i + 1) { values[i] = values[i] / f64(2); }
        print(values[0]); print(values[1]);
        print(false && i32(1.5) == 1);
        print(true || f32(0.1) == 0.0f32);
    ";
    assert_eq!(run_vm(source).unwrap(), "1.5\n2.5\nfalse\ntrue\n");
    for emit in [
        compile_to_c,
        llvm::compile,
        compile_to_qbe,
        compile_to_wat,
        compile_to_x86_64_win_asm,
    ] {
        emit(source).unwrap();
    }
}

#[test]
fn conversion_keeps_resolved_types_spelling_and_source_origin() {
    for (spelling, syntax) in [
        ("f64", ConversionSyntax::Compact),
        ("convert<f64>", ConversionSyntax::Explicit),
    ] {
        let source = format!("result: infer = {spelling}(42);");
        let program = compile_to_ir(&source).unwrap();
        let ir::StatementKind::Binding { value: expr, .. } = &program.statements[0].kind else {
            panic!()
        };
        let ir::ExprKind::ConvertNumeric {
            from,
            to,
            syntax: actual,
            value,
            ..
        } = &expr.kind
        else {
            panic!()
        };
        assert_eq!(
            (*from, *to),
            (NumericType::Integer(IntegerType::I64), NumericType::F64)
        );
        assert_eq!(*actual, syntax);
        assert_eq!(value.ty, ir::Type::Integer(IntegerType::I64));
        assert_eq!(expr.ty, ir::Type::F64);
        let bytecode = compile_to_bytecode(&source).unwrap();
        let instruction = bytecode
            .instructions
            .iter()
            .find(|i| matches!(i.kind, bytecode::InstructionKind::ConvertNumeric { .. }))
            .unwrap();
        assert_eq!(
            instruction.origin,
            bytecode::InstructionOrigin::Source {
                node_id: expr.id,
                span: expr.span
            }
        );
        assert!(
            compile_to_ir_text(&source)
                .unwrap()
                .contains("convert.exact.i64->f64")
        );
        assert!(
            compile_to_bytecode_text(&source)
                .unwrap()
                .contains("convert.exact i64 -> f64")
        );
    }
}

#[test]
fn implicit_conversions_and_nonnumeric_conversions_remain_invalid() {
    for source in [
        "x: f64 = 1;",
        "x: f32 = 1.0f64;",
        "print(1 + 1.0);",
        "print(f64(true));",
        "print(f32([1, 2]));",
        "print(convert<bool>(1.0));",
        "print(false && f64(true) == 0.0);",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn malformed_bytecode_cannot_skip_type_and_stack_checks() {
    let mut program = compile_to_bytecode("print(f64(1));").unwrap();
    program.instructions[0].kind = bytecode::InstructionKind::PushBool(true);
    let error = vm::run(&program).unwrap_err();
    assert_eq!(
        error.kind(),
        VmErrorKind::TypeMismatch {
            expected: bytecode::Type::Integer(IntegerType::I64),
            actual: bytecode::Type::Bool
        }
    );
    assert_eq!(error.instruction_index(), 1);
    program.instructions.remove(0);
    assert_eq!(
        vm::run(&program).unwrap_err().kind(),
        VmErrorKind::StackUnderflow
    );
}
