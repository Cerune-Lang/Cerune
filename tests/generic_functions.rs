use cerune_lang::{compile, compile_to_ir, run_vm};
#[path = "support/generic_cases.rs"]
mod generic_cases;

#[test]
fn examples_preserve_values_copies_and_evaluation_order() {
    for &(source, expected) in generic_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
    }
}

#[test]
fn equivalent_arguments_reuse_instances_and_keep_every_call_site() {
    let source = r#"
        const COUNT: u8 = 2;
        fn echo<T>(v: T) -> T { return v; }
        fn first<T, const N: i64>(v: [T; N]) -> T { return v[0]; }
        print(echo::<[i64; COUNT]>([1, 2])[0]);
        print(echo::<[i64; 2]>([3, 4])[1]);
        print(first::<i64, COUNT>([5, 6]));
        print(first::<i64, 2>([7, 8]));
    "#;
    let ir = compile_to_ir(source).unwrap();
    assert_eq!(run_vm(source).unwrap(), "1\n4\n5\n7\n");
    assert_eq!(ir.function_definitions.len(), 2);
    for function in &ir.function_definitions {
        let origin = function.generic_origin.as_ref().unwrap();
        assert_eq!(origin.calls.len(), 2);
        assert!(source[origin.definition.start()..origin.definition.end()].starts_with("fn "));
        for call in &origin.calls {
            assert!(source[call.span.start()..call.span.end()].contains("::<"));
            assert_eq!(call.argument_spans.len(), origin.arguments.len());
        }
    }
    let text = cerune_lang::ir::text::emit(&ir);
    assert!(text.contains("instantiate echo::<[i64; 2]>"));
    assert!(text.contains("instantiate first::<i64, 2>"));
    assert_eq!(text.matches(";   call [").count(), 4);
}

#[test]
fn void_functions_defaults_nested_calls_and_length_forwarding_work() {
    let source = r#"
        fn show<T>(v: T) -> void { print(v); }
        fn echo<T>(v: T) -> T { return v; }
        fn count<T, const N: i64>(v: [T; N]) -> i64 { return N; }
        fn forward<T, const M: i64>(v: [T; M]) -> i64 { return count::<T, M>(v); }
        type Label { text: string = echo::<string>("既定") }
        show::<string>((Label { }).text);
        show::<u64>(echo::<u64>(echo::<u64>(18446744073709551615)));
        print(forward::<bool, 2>([true, false]));
    "#;
    // 構造体の空構築は既存構文に従い、既定値以外のフィールドを明示します。
    let source = source
        .replace("type Label { text:", "type Label { id: i64, text:")
        .replace("Label { }", "Label { id: 0 }");
    assert_eq!(run_vm(&source).unwrap(), "既定\n18446744073709551615\n2\n");
}

#[test]
fn invalid_generic_programs_fail_before_execution() {
    for (source, reason) in [
        (
            "fn f<T, T>(v:T)->T{return v;}",
            "conflicting generic parameter",
        ),
        (
            "fn f<T>(T:T)->T{return T;}",
            "conflicting function parameter",
        ),
        (
            "fn f<T>(x:T,x:T)->T{return x;}",
            "conflicting function parameter",
        ),
        (
            "fn f<i64>(v:i64)->i64{return v;}",
            "conflicting generic parameter",
        ),
        ("fn main<T>()->void{}", "cannot have generic parameters"),
        ("fn f<const N: u8>()->i64{return 1;}", "const N: i64"),
        ("fn f<T>(v:T)->T{return v;} print(f(1));", "explicit"),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<i64, 2>(1));",
            "generic arguments",
        ),
        (
            "fn f(v:i64)->i64{return v;} print(f::<i64>(1));",
            "not a generic",
        ),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<2>(1));",
            "requires a type",
        ),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<infer>(1));",
            "concrete type",
        ),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<Missing>(1));",
            "concrete type",
        ),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<u8>(256));",
            "does not fit",
        ),
        (
            "fn f<T>(v:T)->T{return v;} print(f::<string>(1));",
            "expects string",
        ),
        (
            "fn f<const N: i64>()->i64{return N;} print(f::<0>());",
            "greater than zero",
        ),
        (
            "fn f<const N: i64>()->i64{return N;} n:i64=2; print(f::<n>());",
            "constant",
        ),
        (
            "fn f<const N: i64>()->i64{return N;} const SIZE:bool=true; print(f::<SIZE>());",
            "integer type",
        ),
        (
            "fn f<T,const N:i64>(v:[T;N])->T{return v[0];} print(f::<i64,2>([1]));",
            "length mismatch",
        ),
        (
            "fn f<T>(v:T)->T{mut T:i64=1;return v;} print(f::<i64>(1));",
            "conflicts with generic",
        ),
        (
            "fn f<T>(v:T)->i64{return T;} print(f::<i64>(1));",
            "type parameter used as a value",
        ),
        (
            "fn f<const N:i64>(v:N)->i64{return 1;} print(f::<2>(1));",
            "length parameter used as a type",
        ),
        (
            "fn f<T>(v:[i64;T])->i64{return 1;} print(f::<i64>([1]));",
            "type parameter used as an array length",
        ),
        (
            "fn f<T>(v:T)->T{return v;} const N:i64=f::<i64>(2);",
            "function calls",
        ),
        (
            "fn f<T>()->i64{return 1;} print(f::<[i64;100001]>());",
            "storage",
        ),
    ] {
        for error in [
            compile(source).unwrap_err(),
            compile_to_ir(source).unwrap_err(),
        ] {
            assert!(error.message().contains(reason), "{source}: {error:?}");
            assert!(error.primary_span().is_some());
        }
    }
}

#[test]
fn excessive_specialization_is_rejected() {
    let mut source = "fn size<const N:i64>()->i64{return N;}\n".to_owned();
    for n in 1..=257 {
        source.push_str(&format!("print(size::<{n}>());\n"));
    }
    assert!(
        compile(&source)
            .unwrap_err()
            .message()
            .contains("256 instances")
    );
}

#[test]
fn dependent_operations_are_checked_at_instantiation_even_in_short_circuited_calls() {
    let definition = "fn add<T>(a:T,b:T)->T{return a+b;}";
    assert!(
        compile_to_ir(definition)
            .unwrap()
            .function_definitions
            .is_empty()
    );
    assert_eq!(
        run_vm(&format!("{definition} print(add::<i64>(1,2));")).unwrap(),
        "3\n"
    );
    let source = format!("{definition} print(false && add::<string>(\"a\",\"b\")==\"x\");");
    let error = compile(&source).unwrap_err();
    let span = error.primary_span().unwrap();
    assert_eq!(&source[span.start()..span.end()], "a+b");
    assert!(error.message().contains("add::<string> called at"));
}

#[test]
fn original_ast_generic_calls_remain_queryable_with_the_semantic_model() {
    let ast = compile(
        "fn echo<T>(v:T)->T{return v;} x:infer=echo::<u64>(echo::<u64>(18446744073709551615));",
    )
    .unwrap();
    let model = cerune_lang::semantic::analyze(&ast).unwrap();
    let cerune_lang::ast::StmtKind::Binding { value, .. } = &ast.statement(0).kind else {
        panic!()
    };
    assert_eq!(
        model.type_name(model.type_of_expr(value, &model.bindings).unwrap()),
        "u64"
    );
}

#[allow(dead_code)]
#[path = "support/runtime_cases.rs"]
mod runtime_cases;
#[test]
fn failures_point_to_the_generic_body_and_keep_prior_output() {
    for &(source, code, output, expression) in runtime_cases::GENERIC_FAILURES {
        let ir = compile_to_ir(source).unwrap();
        let error =
            cerune_lang::run_bytecode(&cerune_lang::bytecode::lower(&ir).unwrap()).unwrap_err();
        let failure = error.runtime_failure().unwrap();
        assert_eq!(failure.code.name(), code);
        assert_eq!(error.vm_error().output(), output);
        assert_eq!(
            &source[failure.span.start()..failure.span.end()],
            expression
        );
    }
}

#[test]
fn unicode_crlf_sources_keep_instantiation_and_body_spans() {
    let source = "// 日本語\r\nfn echo<T>(v:T)->T{return v;}\r\nprint(echo::<i64>(1));\r\nprint(echo::<string>(\"日\"));";
    let mut sources = cerune_lang::source::SourceMap::new();
    let id = sources.add("generic.ceru", source);
    let ir = cerune_lang::compile_source_to_ir(sources.get(id).unwrap()).unwrap();
    assert_eq!(ir.function_definitions.len(), 2);
    assert_ne!(
        ir.function_definitions[0].body[0].id,
        ir.function_definitions[1].body[0].id
    );
    for f in &ir.function_definitions {
        let origin = f.generic_origin.as_ref().unwrap();
        assert_eq!(origin.definition.source_id(), id);
        assert_eq!(
            sources.slice(origin.definition),
            Some("fn echo<T>(v:T)->T{return v;}")
        );
        assert_eq!(sources.slice(f.body[0].span), Some("return v;"));
        assert!(
            sources
                .slice(origin.calls[0].span)
                .unwrap()
                .starts_with("echo::<")
        );
    }
}

#[test]
fn constant_lengths_in_type_arguments_remain_queryable_and_observable() {
    let source =
        "const COUNT:u8=2; fn echo<T>(v:T)->T{return v;} values:infer=echo::<[i64;COUNT]>([1,2]);";
    let ast = compile(source).unwrap();
    let model = cerune_lang::semantic::analyze(&ast).unwrap();
    let cerune_lang::ast::StmtKind::Binding { value, .. } = &ast.statement(0).kind else {
        panic!()
    };
    let cerune_lang::ast::ExprKind::GenericCall(call) = &value.kind else {
        panic!()
    };
    let cerune_lang::ast::GenericArgument::Type(ty) = &call.generic_arguments[0] else {
        panic!()
    };
    assert_eq!(
        model.type_name(model.resolve_type_ref(ty).unwrap()),
        "[i64; 2]"
    );
    let ir = compile_to_ir(source).unwrap();
    let spans = &ir.constant_definitions[0].array_length_uses;
    assert_eq!(spans.len(), 1);
    assert_eq!(&source[spans[0].start()..spans[0].end()], "COUNT");
}

#[test]
fn defaults_keep_recursive_generic_calls() {
    for source in [
        "type Box { value:i64=make::<i64>(), marker:bool } fn make<T>()->i64{return (Box{marker:true}).value;} print(make::<i64>());",
        "type Inner { value:i64=make::<i64>(), marker:bool } type Outer { item:Inner=Inner{marker:true}, marker:bool } fn make<T>()->i64{return (Outer{marker:true}).item.value;} print(make::<i64>());",
    ] {
        compile(source).unwrap();
        let p = compile_to_ir(source).unwrap();
        assert!(
            cerune_lang::ir_executor::run_with_call_limit(
                &p,
                std::num::NonZeroU32::new(8).unwrap()
            )
            .unwrap_err()
            .call_failure()
            .is_some()
        );
    }
    assert_eq!(run_vm("type Box { value:i64=make::<i64>(), marker:bool } fn make<T>()->i64{return (Box{value:7,marker:true}).value;} print(make::<i64>());").unwrap(), "7\n");
    assert_eq!(run_vm("type Box { value:i64=make::<i64>(), marker:bool } fn make<T>()->i64{v:Box=Box{value:7,marker:true};return (Box{..v,marker:false}).value;} print(make::<i64>());").unwrap(), "7\n");
}
