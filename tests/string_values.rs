use cerune_lang::{
    bytecode::{self, Instruction, InstructionKind, InstructionOrigin, Type as BytecodeType},
    codegen, compile, compile_to_bytecode, compile_to_bytecode_text, compile_to_ir,
    compile_to_ir_text,
    diagnostic::Diagnostic,
    ir::{self, ExprKind, StatementKind, Type},
    run_vm,
    source::Span,
    types::IntegerType,
    vm::{self, VmErrorKind},
};

#[test]
fn binds_infers_and_reassigns_strings_without_changing_copies() {
    let source = r#"
        mut current: string = "最初";
        saved: infer = current;
        current = "次";
        print(saved);
        print(current);
        print("");
        print("line\nbreak\t\"quote\"\\\0end");
    "#;
    assert_eq!(
        run_vm(source).unwrap(),
        "最初\n次\n\nline\nbreak\t\"quote\"\\\0end\n"
    );
}

#[test]
fn string_equality_compares_contents_without_normalizing_or_truncating() {
    let source = r#"
        print("日本語" == "\u{65e5}本語");
        print("" == "");
        print("a" != "A");
        print("a\0x" == "a\0y");
        print("\u{e9}" == "e\u{301}");
        print("same" != "same");
    "#;
    assert_eq!(
        run_vm(source).unwrap(),
        "true\ntrue\ntrue\nfalse\nfalse\nfalse\n"
    );
}

#[test]
fn strings_compose_with_defaults_products_nested_arrays_and_functions() {
    let source = r#"
        type Label { id: i64, text: string = "既定", }
        fn change(values: [[Label; 1]; 2]) -> [[Label; 1]; 2] {
            mut result: infer = values;
            result[1][0] = Label { id: 1, text: "変更", };
            return result;
        }
        fn choose(value: Label) -> string { return value.text; }
        original: [[Label; 1]; 2] = [[Label { id: 0, }], [Label { id: 1, text: "元", }]];
        copy: infer = change(original);
        print(choose(copy[0][0]));
        print(choose(copy[1][0]));
        print(original[1][0].text);
    "#;
    assert_eq!(run_vm(source).unwrap(), "既定\n変更\n元\n");
}

#[test]
fn supports_string_array_element_replacement_and_returned_local_values() {
    let source = r#"
        fn words() -> [string; 2] {
            mut values: [string; 2] = ["before", "kept"];
            copy: infer = values;
            values[0] = "after";
            print(copy[0]);
            return values;
        }
        print(words()[0]);
        print(words()[1]);
    "#;
    assert_eq!(run_vm(source).unwrap(), "before\nafter\nbefore\nkept\n");
}

#[test]
fn evaluates_string_calls_once_and_preserves_short_circuiting() {
    let source = r#"
        fn mark(text: string) -> string { print(text); return text; }
        print(mark("left") == mark("right"));
        print(false && mark("skipped") == "skipped");
        print(true || mark("skipped") != "skipped");
    "#;
    assert_eq!(run_vm(source).unwrap(), "left\nright\nfalse\nfalse\ntrue\n");
}

#[test]
fn supports_loop_bindings_and_explicit_main() {
    let source = r#"
        fn main() -> void {
            for (mut i: i64 = 0; i < 2; i = i + 1) {
                text: infer = "loop";
                print(text);
            }
        }
    "#;
    assert_eq!(run_vm(source).unwrap(), "loop\nloop\n");
}

#[test]
fn rejects_string_mutation_implicit_conversions_and_unimplemented_operations() {
    for source in [
        r#"value: string = "a"; value = "b";"#,
        r#"mut value: string = "a"; value[0] = "b";"#,
        r#"values: [string; 1] = ["a"]; values[0] = "b";"#,
        r#"print("abc"[0]);"#,
        r#"print("a" + "b");"#,
        r#"print("a" < "b");"#,
        r#"print(-"a");"#,
        r#"print(!"a");"#,
        r#"print(~"a");"#,
        r#"print("a" & "b");"#,
        r#"print("a" && true);"#,
        r#"if "a" { print(1); }"#,
        r#"value: string = 1;"#,
        r#"value: i64 = "1";"#,
        r#"print("1" == 1);"#,
        r#"values: infer = ["a", 1];"#,
        r#"fn f(text: string) -> string { return 1; }"#,
        r#"fn f(text: string) -> string { return text; } print(f(1));"#,
        r#"type Entry { text: string = 1, }"#,
        r#"type string { value: i64, }"#,
        r#"fn string() -> void {}"#,
        r#"print(i64("1"));"#,
        r#"print(string(1));"#,
        r#"print(convert<string>(1));"#,
    ] {
        let error = compile(source).expect_err(source);
        let span = error.primary_span().expect("positioned diagnostic");
        assert!(span.end() <= source.len(), "{source}");
    }
}

#[test]
fn ir_and_bytecode_preserve_types_values_spans_and_node_origins() {
    let source = r#"text: infer = "日本語\n\0"; print(text);"#;
    let program = compile_to_ir(source).unwrap();
    let StatementKind::Binding { ty, value, .. } = &program.statements[0].kind else {
        panic!("expected binding");
    };
    assert_eq!(*ty, Type::String);
    assert_eq!(value.ty, Type::String);
    assert_eq!(value.kind, ExprKind::String("日本語\n\0".into()));
    let span = Span::new(source.find('"').unwrap(), source.rfind('"').unwrap() + 1);
    assert_eq!(value.span, span);
    let bytecode = compile_to_bytecode(source).unwrap();
    assert_eq!(bytecode.slots[0].ty, BytecodeType::String);
    assert!(
        matches!(&bytecode.instructions[0].kind, InstructionKind::PushString(text) if text == "日本語\n\0")
    );
    assert_eq!(
        bytecode.instructions[0].origin,
        InstructionOrigin::Source {
            node_id: value.id,
            span
        }
    );
    let text = compile_to_ir_text(source).unwrap();
    assert!(text.contains(r#""日本語\n\0":string"#));
    assert!(!text.contains('\0'));
    assert_eq!(text, compile_to_ir_text(source).unwrap());
    let bytecode_text = compile_to_bytecode_text(source).unwrap();
    assert!(bytecode_text.contains(r#"push.string "日本語\n\0""#));
    assert!(!bytecode_text.contains('\0'));
}

type Emitter = fn(&ir::Program) -> Result<String, Diagnostic>;
const TARGET_REQUIRED: &[(&str, Emitter)] = &[
    ("emit-llvm", codegen::emit_llvm),
    ("emit-qbe", codegen::emit_qbe),
];

#[test]
fn strings_require_explicit_targets_where_runtime_selection_is_needed() {
    for source in [
        r#"print("text");"#,
        r#"print("left" == "right");"#,
        r#"type Unused { text: string, }"#,
        r#"type Unused { values: [[string; 1]; 2], }"#,
        r#"fn unused(text: string) -> void {}"#,
        r#"fn unused() -> string { return "text"; }"#,
        r#"fn unused() -> void { print("text"); }"#,
        r#"if false { print("text"); }"#,
        r#"while false { print("text"); }"#,
        r#"print(true || "a" == "b");"#,
        r#"type Unused { flag: bool = "a" == "b", }"#,
        r#"for (mut flag: bool = "a" == "b"; flag; flag = false) {}"#,
        r#"for (mut flag: bool = true; "a" == "b"; flag = false) {}"#,
        r#"for (mut flag: bool = true; false; flag = "a" == "b") {}"#,
        r#"for (mut i: i64 = 0; false; i = i + 1) { print("text"); }"#,
        r#"mut flag: bool = true; flag = "a" == "b";"#,
        r#"fn use_flag(flag: bool) -> void {} use_flag("a" == "b");"#,
        r#"fn flag() -> bool { return "a" == "b"; }"#,
        r#"type Entry { flag: bool, } value: Entry = Entry { flag: "a" == "b", };"#,
        r#"values: [bool; 1] = ["a" == "b"];"#,
    ] {
        let program = compile_to_ir(source).expect(source);
        for &(route, emit) in TARGET_REQUIRED {
            let error = emit(&program).expect_err(route);
            assert!(error.message().contains("explicit --target"));
            let span = error.primary_span().unwrap();
            assert!(span.start() < span.end() && span.end() <= source.len());
        }
        codegen::emit_wat(&program).unwrap();
        codegen::emit_x86_64_win_asm(&program).unwrap();
        codegen::qbe::emit_qbe_with_target(
            &program,
            Some(codegen::qbe::Target::X86_64UnknownLinuxGnu),
        )
        .unwrap();
    }
}

#[test]
fn numeric_only_programs_still_emit_through_every_backend() {
    let program = compile_to_ir("print(1 + 2);").unwrap();
    codegen::emit_c(&program).unwrap();
    codegen::emit_wat(&program).unwrap();
    codegen::emit_x86_64_win_asm(&program).unwrap();
    codegen::llvm::emit_llvm_with_target(
        &program,
        Some(codegen::llvm::Target::X86_64UnknownLinuxGnu),
    )
    .unwrap();
    codegen::qbe::emit_qbe_with_target(&program, Some(codegen::qbe::Target::X86_64UnknownLinuxGnu))
        .unwrap();
}

fn vm_failure(instructions: Vec<InstructionKind>) -> VmErrorKind {
    let program = bytecode::BytecodeProgram {
        array_heap_limit: cerune_lang::ir::DEFAULT_ARRAY_HEAP_LIMIT,
        string_heap_limit: ir::DEFAULT_STRING_HEAP_LIMIT,
        type_definitions: vec![],
        functions: vec![],
        slots: vec![],
        instructions: instructions
            .into_iter()
            .map(Instruction::synthetic)
            .collect(),
    };
    vm::run(&program).unwrap_err().kind()
}

#[test]
fn malformed_bytecode_cannot_treat_strings_as_numbers_or_order_them() {
    assert_eq!(
        vm_failure(vec![InstructionKind::StringByteLength]),
        VmErrorKind::StackUnderflow
    );
    assert_eq!(
        vm_failure(vec![
            InstructionKind::PushBool(true),
            InstructionKind::StringByteLength
        ]),
        VmErrorKind::TypeMismatch {
            expected: BytecodeType::String,
            actual: BytecodeType::Bool
        }
    );
    assert_eq!(
        vm_failure(vec![InstructionKind::Print(BytecodeType::String)]),
        VmErrorKind::StackUnderflow
    );
    assert!(matches!(
        vm_failure(vec![
            InstructionKind::PushBool(true),
            InstructionKind::Print(BytecodeType::String)
        ]),
        VmErrorKind::TypeMismatch {
            expected: BytecodeType::String,
            actual: BytecodeType::Bool
        }
    ));
    assert!(matches!(
        vm_failure(vec![
            InstructionKind::PushString("text".into()),
            InstructionKind::Print(BytecodeType::Bool)
        ]),
        VmErrorKind::TypeMismatch {
            expected: BytecodeType::Bool,
            actual: BytecodeType::String
        }
    ));
    for operation in [
        InstructionKind::Add(BytecodeType::String),
        InstructionKind::Negate(BytecodeType::String),
    ] {
        assert!(matches!(
            vm_failure(vec![InstructionKind::PushString("a".into()), operation]),
            VmErrorKind::TypeMismatch {
                actual: BytecodeType::String,
                ..
            }
        ));
    }
    assert_eq!(
        vm_failure(vec![
            InstructionKind::PushString("a".into()),
            InstructionKind::PushString("b".into()),
            InstructionKind::Less(BytecodeType::String)
        ]),
        VmErrorKind::InvalidComparisonType {
            ty: BytecodeType::String
        }
    );
    assert!(matches!(
        vm_failure(vec![
            InstructionKind::PushString("a".into()),
            InstructionKind::PushInteger(1, IntegerType::I64),
            InstructionKind::Equal(BytecodeType::String)
        ]),
        VmErrorKind::TypeMismatch {
            expected: BytecodeType::String,
            actual: BytecodeType::Integer(IntegerType::I64)
        }
    ));
}

#[test]
fn byte_length_has_explicit_string_input_and_i64_result() {
    assert_eq!(
        run_vm("length: infer = byte_len(\"日本語\\0\"); print(length);").unwrap(),
        "10\n"
    );
    for source in [
        "print(byte_len());",
        "print(byte_len(\"a\", \"b\"));",
        "print(byte_len(1));",
        "print(byte_len([\"a\"]));",
        "small: i8 = byte_len(\"a\");",
        "byte_len(\"unused\");",
        "fn byte_len(text: string) -> i64 { return 0; }",
        "fn recurse() -> string { print(byte_len(recurse())); return \"\"; }",
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.primary_span().is_some(), "{source}");
    }
}

#[test]
fn byte_length_preserves_ir_and_bytecode_origins_and_requires_native_targets() {
    let source = "print(byte_len(\"日\"));";
    let program = compile_to_ir(source).unwrap();
    let StatementKind::Print { value } = &program.statements[0].kind else {
        panic!()
    };
    assert!(matches!(value.kind, ExprKind::StringByteLength { .. }));
    assert_eq!(value.ty, Type::Integer(IntegerType::I64));
    let bytecode = compile_to_bytecode(source).unwrap();
    let instruction = bytecode
        .instructions
        .iter()
        .find(|i| matches!(i.kind, InstructionKind::StringByteLength))
        .unwrap();
    assert_eq!(
        instruction.origin,
        InstructionOrigin::Source {
            node_id: value.id,
            span: value.span
        }
    );
    assert!(
        compile_to_ir_text(source)
            .unwrap()
            .contains("byte_len.string(")
    );
    assert!(
        compile_to_bytecode_text(source)
            .unwrap()
            .contains("byte_len.string")
    );
    assert!(cerune_lang::compile_to_llvm(source).is_err());
    assert!(cerune_lang::compile_to_qbe(source).is_err());
}
