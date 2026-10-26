#[path = "support/qbe.rs"]
mod qbe;
use qbe::compile as compile_to_qbe;

#[path = "support/llvm.rs"]
mod llvm;
use cerune_lang::{
    RunError, ast, bytecode, compile, compile_to_bytecode, compile_to_bytecode_text, compile_to_c,
    compile_to_ir, compile_to_ir_text, compile_to_wat, compile_to_x86_64_win_asm, ir, run_vm,
    source::{ConversionSyntax, Span},
    types::IntegerType,
    vm::{self, VmErrorKind},
};

const SPELLINGS: [(&str, ConversionSyntax); 2] = [
    ("i64", ConversionSyntax::Compact),
    ("convert<i64>", ConversionSyntax::Explicit),
];

#[test]
fn both_spellings_preserve_the_same_conversion_and_their_source_origin() {
    for (spelling, expected_syntax) in SPELLINGS {
        let expression = format!("{spelling}(42)");
        let source = format!("answer: infer = {expression};");
        let start = source.find(&expression).unwrap();
        let expected_span = Span::new(start, start + expression.len());
        let ast = compile(&source).unwrap();
        let ast::StmtKind::Binding { value, .. } = &ast.statement(0).kind else {
            panic!("expected binding");
        };
        let ast::ExprKind::Convert { target, syntax, .. } = &value.kind else {
            panic!("expected AST conversion");
        };
        assert!(target.is_named("i64"));
        assert_eq!(*syntax, expected_syntax);
        assert_eq!(value.span, expected_span);

        let program = compile_to_ir(&source).unwrap();
        let ir::StatementKind::Binding {
            value: conversion, ..
        } = &program.statements[0].kind
        else {
            panic!("expected binding");
        };
        let ir::ExprKind::ConvertInteger {
            from,
            to,
            syntax,
            value,
        } = &conversion.kind
        else {
            panic!("expected IR conversion");
        };
        assert_eq!((*from, *to), (IntegerType::I64, IntegerType::I64));
        assert_eq!(*syntax, expected_syntax);
        assert_eq!(value.kind, ir::ExprKind::Integer(42));
        assert_eq!(conversion.ty, ir::Type::Integer(IntegerType::I64));
        assert_eq!(conversion.span, expected_span);
        assert_ne!(conversion.id, value.id);

        let bytecode = compile_to_bytecode(&source).unwrap();
        let instruction = bytecode
            .instructions
            .iter()
            .find(|instruction| {
                matches!(
                    instruction.kind,
                    bytecode::InstructionKind::ConvertInteger { .. }
                )
            })
            .unwrap();
        assert_eq!(
            instruction.origin,
            bytecode::InstructionOrigin::Source {
                node_id: conversion.id,
                span: expected_span,
            }
        );
        let syntax_name = match expected_syntax {
            ConversionSyntax::Compact => "compact",
            ConversionSyntax::Explicit => "explicit",
        };
        assert!(
            compile_to_ir_text(&source)
                .unwrap()
                .contains(&format!("convert.checked.i64->i64[{syntax_name}]"))
        );
        assert!(
            compile_to_bytecode_text(&source)
                .unwrap()
                .contains("convert.checked i64 -> i64\n")
        );
    }
}

#[test]
fn conversions_preserve_integer_boundaries_and_support_nesting() {
    for (spelling, _) in SPELLINGS {
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            let source = format!("print({spelling}(i64({value}))); ");
            assert_eq!(run_vm(&source).unwrap(), format!("{value}\n"));
        }
        assert_eq!(
            run_vm(&format!("print(-{spelling}(2) * 3 + 1);")).unwrap(),
            "-5\n"
        );
        assert_eq!(run_vm(&format!("print({spelling}(42,));")).unwrap(), "42\n");
    }
}

#[test]
fn both_spellings_evaluate_once_and_emit_identical_artifacts_at_equal_output_positions() {
    let sources = SPELLINGS.map(|(spelling, _)| {
        // 書込み失敗の診断は出力位置を保持するため、変換後の位置を空白で揃えます。
        // 変換自身の異なる位置と構文は上の出自テストで比較します。
        let padding = " ".repeat("convert<i64>".len() - spelling.len());
        format!(
            "
        fn next() -> i64 {{ print(7); return 42; }}
        answer: infer = {spelling}(next());{padding}
        print(answer);
    "
        )
    });
    for source in &sources {
        assert_eq!(run_vm(source).unwrap(), "7\n42\n");
    }
    for emit in [
        compile_to_c,
        llvm::compile,
        compile_to_qbe,
        compile_to_wat,
        compile_to_x86_64_win_asm,
    ] {
        assert_eq!(emit(&sources[0]).unwrap(), emit(&sources[1]).unwrap());
    }
}

#[test]
fn conversions_work_in_fields_arrays_indices_and_function_returns() {
    let source = "
        type Boxed { value: i64 = convert<i64>(3), tag: bool, }
        fn identity(value: i64) -> i64 { return i64(value); }
        boxed: Boxed = Boxed { tag: true, };
        mut values: [i64; 2] = [i64(1), convert<i64>(2)];
        values[convert<i64>(0)] = identity(i64(boxed.value));
        print(i64(values[0]));
        print(convert<i64>([4, 5][1]));
    ";
    assert_eq!(run_vm(source).unwrap(), "3\n5\n");
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
fn input_failures_keep_their_original_operation_and_span() {
    for (spelling, _) in SPELLINGS {
        let source = format!("print({spelling}(1 / 0));");
        let RunError::Execution(error) = run_vm(&source).unwrap_err() else {
            panic!("expected runtime failure");
        };
        assert_eq!(error.vm_error().kind(), VmErrorKind::DivisionByZero);
        let start = source.find("1 / 0").unwrap();
        let Some(bytecode::InstructionOrigin::Source { span, .. }) = error.origin() else {
            panic!("expected source origin");
        };
        assert_eq!(span, Span::new(start, start + 5));
        let source = format!("print({spelling}(9223372036854775807 + 1));");
        let RunError::Execution(error) = run_vm(&source).unwrap_err() else {
            panic!("expected runtime overflow");
        };
        assert!(matches!(
            error.vm_error().kind(),
            VmErrorKind::IntegerOverflow { .. }
        ));
    }
}

#[test]
fn conversions_reject_nonnumeric_inputs_without_contextual_retyping() {
    for (spelling, _) in SPELLINGS {
        for input in ["true", "[1, 2]"] {
            let source = format!("print({spelling}({input}));");
            let error = compile(&source).unwrap_err();
            assert!(
                error
                    .message()
                    .starts_with("numeric conversion requires a numeric value, found ")
            );
            let start = source.find(input).unwrap();
            assert_eq!(
                error.primary_span(),
                Some(Span::new(start, start + input.len()))
            );
        }
        let source = format!("result: f32 = {spelling}(1);");
        assert_eq!(
            compile(&source).unwrap_err().message(),
            "type mismatch for `result`: expected f32, found i64"
        );
    }
    for target in ["bool", "[i64; 2]"] {
        let source = format!("print(convert<{target}>(1));");
        assert_eq!(
            compile(&source).unwrap_err().message(),
            "conversion target must be a numeric type"
        );
    }
    assert_eq!(
        compile("print(convert<u128>(1));").unwrap_err().message(),
        "unknown type `u128`"
    );
}

#[test]
fn conversions_require_one_argument_and_accept_typed_recursive_calls() {
    for (spelling, _) in SPELLINGS {
        for arguments in ["", "1, 2"] {
            let source = format!("print({spelling}({arguments}));");
            assert_eq!(
                compile(&source).unwrap_err().message(),
                "conversion requires exactly one value"
            );
        }
        let source = format!("fn again() -> i64 {{ return {spelling}(again()); }} print(again());");
        compile(&source).unwrap();
    }
}

#[test]
fn builtin_type_names_cannot_be_redefined_as_functions_or_types() {
    for name in ["i32", "u32", "i64", "bool", "f32", "f64"] {
        let source = format!("fn {name}(value: i64) -> i64 {{ return value; }}");
        let error = compile(&source).unwrap_err();
        assert_eq!(
            error.message(),
            format!("function name `{name}` is reserved for a built-in type")
        );
        assert_eq!(error.primary_span(), Some(Span::new(3, 3 + name.len())));
        let source = format!("type {name} {{ value: i64, }}");
        let error = compile(&source).unwrap_err();
        assert_eq!(
            error.message(),
            format!("type name `{name}` is reserved for a built-in type")
        );
    }
}

#[test]
fn convert_remains_available_for_ordinary_calls_and_comparisons() {
    let source = "
        fn convert(value: i64) -> i64 { return value + 1; }
        convert: i64 = 1;
        limit: i64 = 3;
        print(convert < limit);
        print(convert(2));
        print(convert < 4);
        print(convert<i64>(5));
        i64: i64 = 6;
        print(i64);
        print(i64(7));
    ";
    assert_eq!(run_vm(source).unwrap(), "true\n3\ntrue\n5\n6\n7\n");
}

#[test]
fn vm_conversion_checks_the_input_type_and_stack() {
    let mut program = compile_to_bytecode("print(i64(1));").unwrap();
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
    let error = vm::run(&program).unwrap_err();
    assert_eq!(error.kind(), VmErrorKind::StackUnderflow);
    assert_eq!(error.instruction_index(), 0);
}
