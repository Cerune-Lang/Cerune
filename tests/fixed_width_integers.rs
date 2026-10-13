#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{
    RunError, bytecode, compile, compile_to_bytecode, compile_to_bytecode_text, compile_to_c,
    compile_to_ir, compile_to_ir_text, compile_to_wat, compile_to_x86_64_win_asm, ir, run_vm,
    types::IntegerType,
    vm::{self, IntegerOperation, VmErrorKind},
};

#[test]
fn widths_ranges_and_literal_types_are_preserved() {
    for ty in IntegerType::ALL {
        for value in [ty.minimum(), 0, ty.maximum()] {
            let source = format!("value: infer = {value}{}; print(value);", ty.name());
            assert_eq!(run_vm(&source).unwrap(), format!("{value}\n"));
            let program = compile_to_ir(&source).unwrap();
            let ir::StatementKind::Binding { ty: actual, .. } = &program.statements[0].kind else {
                panic!()
            };
            assert_eq!(*actual, ir::Type::Integer(ty));
            assert!(
                compile_to_bytecode_text(&source)
                    .unwrap()
                    .contains(&format!("push.{}", ty.name()))
            );
        }
    }
    assert_eq!(IntegerType::I32.bit_width(), 32);
    assert_eq!(IntegerType::U32.bit_width(), 32);
    assert!(!IntegerType::U32.is_signed());
}

#[test]
fn context_reaches_nested_arithmetic_in_both_operand_orders() {
    for ty in ["i32", "u32"] {
        for expression in [
            "value + 1",
            "1 + value",
            "(1 + 2) + value",
            "value + (1 + 2)",
        ] {
            let source = format!("value: {ty} = 4; answer: infer = {expression}; print(answer);");
            let program = compile_to_ir(&source).unwrap();
            let ir::StatementKind::Binding { ty: actual, .. } = &program.statements[1].kind else {
                panic!()
            };
            assert_eq!(
                *actual,
                if ty == "i32" {
                    ir::Type::Integer(IntegerType::I32)
                } else {
                    ir::Type::Integer(IntegerType::U32)
                }
            );
            assert!(run_vm(&source).is_ok());
        }
    }
    let source = "fn small(value: i32) -> i32 { return value + 1; } print((1 + 2) < small(4));";
    assert_eq!(run_vm(source).unwrap(), "true\n");
}

#[test]
fn typed_arrays_fields_parameters_returns_and_indices_work_together() {
    let source = "
        type Counts { values: [u32; 2], offset: i32 = -2, }
        fn increment(values: [u32; 2]) -> [u32; 2] { return [values[0] + 1, values[1] + 1]; }
        counts: Counts = Counts { values: [2147483648, 3], };
        mut values: [u32; 2] = increment(counts.values);
        index: u32 = 0;
        values[i64(index + 1)] = convert<u32>(counts.offset + 7);
        print(values[0]); print(values[1]); print(counts.offset);
    ";
    assert_eq!(run_vm(source).unwrap(), "2147483649\n5\n-2\n");
    for emit in [
        compile_to_c,
        llvm::compile,
        compile_to_qbe,
        compile_to_wat,
        compile_to_x86_64_win_asm,
    ] {
        assert!(emit(source).is_ok());
    }
}

#[test]
fn out_of_range_literals_and_implicit_conversions_are_rejected() {
    for source in [
        "value: i32 = 2147483648;",
        "value: i32 = -2147483649;",
        "value: u32 = 4294967296;",
        "value: u32 = -1;",
        "print(-0u32);",
        "value: i32 = 1u32;",
        "value: infer = 1i32 + 1u32;",
        "value: i32 = 1; copy: i64 = value;",
        "value: f32 = 1i32;",
        "value: infer = [1, 2i32];",
        "values: [i32; 1] = [1]; index: u32 = 0; print(values[index]);",
        "print(1.0i32);",
        "print(1i32abc);",
        "print(18446744073709551616u64);",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    assert_eq!(
        run_vm("values: infer = [1i32, 2]; print(values[1]);").unwrap(),
        "2\n"
    );
}

#[test]
fn unsigned_ordering_and_division_preserve_positive_large_values() {
    let source = "
        value: u32 = 4294967295;
        print(value > 2147483648); print(1 < value); print(value / 2);
        print(-7i32 / 3i32); print(7i32 / -3i32);
    ";
    assert_eq!(run_vm(source).unwrap(), "true\ntrue\n2147483647\n-2\n-2\n");
}

#[test]
fn arithmetic_reports_the_original_integer_type() {
    for (expr, ty, operation) in [
        ("2147483647i32 + 1", IntegerType::I32, IntegerOperation::Add),
        (
            "-2147483648i32 - 1",
            IntegerType::I32,
            IntegerOperation::Subtract,
        ),
        (
            "50000i32 * 50000",
            IntegerType::I32,
            IntegerOperation::Multiply,
        ),
        (
            "-(-2147483648i32)",
            IntegerType::I32,
            IntegerOperation::Negate,
        ),
        ("4294967295u32 + 1", IntegerType::U32, IntegerOperation::Add),
        ("0u32 - 1", IntegerType::U32, IntegerOperation::Subtract),
        (
            "4294967295u32 * 4294967295",
            IntegerType::U32,
            IntegerOperation::Multiply,
        ),
    ] {
        let RunError::Execution(error) = run_vm(&format!("print({expr});")).unwrap_err() else {
            panic!("{expr}")
        };
        assert_eq!(
            error.vm_error().kind(),
            VmErrorKind::IntegerOverflow {
                ty: bytecode::Type::Integer(ty),
                operation
            }
        );
    }
    for expr in ["-2147483648i32 / -1", "-9223372036854775808 / -1"] {
        let RunError::Execution(error) = run_vm(&format!("print({expr});")).unwrap_err() else {
            panic!()
        };
        assert_eq!(error.vm_error().kind(), VmErrorKind::DivisionOverflow);
    }
}

#[test]
fn both_conversion_spellings_agree_for_all_supported_type_pairs() {
    let types = IntegerType::ALL;
    for from in types {
        for to in types {
            let values = [
                from.minimum(),
                from.maximum(),
                0,
                -1,
                1,
                to.minimum(),
                to.maximum(),
                2147483648,
                4294967296,
            ];
            for value in values.into_iter().filter(|value| from.contains(*value)) {
                for spelling in [to.name().to_string(), format!("convert<{}>", to.name())] {
                    let source = format!(
                        "value: {} = {value}; print({spelling}(value));",
                        from.name()
                    );
                    if to.contains(value) {
                        assert_eq!(run_vm(&source).unwrap(), format!("{value}\n"), "{source}");
                    } else {
                        let RunError::Execution(error) = run_vm(&source).unwrap_err() else {
                            panic!("{source}")
                        };
                        assert_eq!(
                            error.vm_error().kind(),
                            VmErrorKind::IntegerConversionOutOfRange { from, to }
                        );
                        assert!(error.origin().is_some());
                    }
                }
            }
        }
    }
}

#[test]
fn widening_does_not_retype_input_arithmetic_or_repeat_input_calls() {
    assert_eq!(
        run_vm("value: i32 = 2147483647; print(i64(value) + 1);").unwrap(),
        "2147483648\n"
    );
    assert!(matches!(
        run_vm("value: i32 = 2147483647; print(i64(value + 1));"),
        Err(RunError::Execution(_))
    ));
    for spelling in ["i32", "convert<i32>"] {
        let source =
            format!("fn once() -> i64 {{ print(7); return 42; }} print({spelling}(once()));");
        assert_eq!(run_vm(&source).unwrap(), "7\n42\n");
    }
    let ir = compile_to_ir_text("value: i32 = 1; print(convert<u32>(value));").unwrap();
    assert!(ir.contains("convert.checked.i32->u32[explicit]"));
}

#[test]
fn invalid_bytecode_cannot_create_out_of_range_integers() {
    let mut program = compile_to_bytecode("print(1u32);").unwrap();
    program.instructions[0].kind = bytecode::InstructionKind::PushInteger(-1, IntegerType::U32);
    assert_eq!(
        vm::run(&program).unwrap_err().kind(),
        VmErrorKind::InvalidIntegerValue {
            ty: IntegerType::U32
        }
    );
}
