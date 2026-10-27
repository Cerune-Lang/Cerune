use super::*;
const STYLES: [LetStyle; 3] = [LetStyle::Preserve, LetStyle::Show, LetStyle::Hide];

#[test]
fn contextual_let_preserves_identifiers_and_for_binding_forms() {
    let source = r#"
        fn let(x: i64) -> i64 { return x; }
        let mut let: i64 = let(1);
        let = let + 1;
        for (let mut i: i64 = 0; i < 2; i = i + 1) { print(i); }
        for (let index: i64, let value: infer in [3,4]) { print(index + value); }
        print(let);
    "#;
    for style in STYLES {
        let text = format(source, style).unwrap();
        assert_eq!(format(&text, style).unwrap(), text);
        let p = crate::compile_to_ir(&text).unwrap();
        assert_eq!(crate::ir_executor::run(&p).unwrap(), "0\n1\n3\n5\n2\n");
        assert!(text.contains("fn let("));
        assert!(text.contains("let = let + 1;"));
    }
    for invalid in [
        "let x = 1;",
        "mut let x: i64 = 1;",
        "let const X:i64=1;",
        "for (mut i:i64=0;i<1;let i=1) {}",
        "fn f(let x:i64)->void{}",
        "type T{let x:i64}",
    ] {
        assert!(format(invalid, LetStyle::Preserve).is_err(), "{invalid}");
    }
}

#[test]
fn else_if_chains_format_idempotently_and_preserve_execution() {
    let source =
        "mut x:i64=2;if x<0{print(0);}else if x<2{print(1);}else if x<3{print(2);}else{print(3);}";
    for style in STYLES {
        let formatted = format(source, style).unwrap();
        assert!(formatted.contains("} else if x < 2 {"), "{formatted}");
        assert!(formatted.contains("} else if x < 3 {"), "{formatted}");
        assert_eq!(format(&formatted, style).unwrap(), formatted);
        let original = crate::ir_executor::run(&crate::compile_to_ir(source).unwrap()).unwrap();
        let after = crate::ir_executor::run(&crate::compile_to_ir(&formatted).unwrap()).unwrap();
        assert_eq!(original, "2\n");
        assert_eq!(original, after);
    }
}

#[test]
fn else_if_keeps_its_layout_comments_and_optional_final_else() {
    let compact = "if false{print(1);}else if true{print(2);}else{print(3);}";
    let expected = "if false {\n    print(1);\n} else if true {\n    print(2);\n} else {\n    print(3);\n}\n";
    assert_eq!(format(compact, LetStyle::Preserve).unwrap(), expected);

    let commented = "if false{}else // reasoning\nif true{print(5);}";
    let formatted = format(commented, LetStyle::Preserve).unwrap();
    assert!(formatted.contains("} else // reasoning\nif true {"));
    assert_eq!(format(&formatted, LetStyle::Preserve).unwrap(), formatted);
    assert_eq!(
        crate::ir_executor::run(&crate::compile_to_ir(&formatted).unwrap()).unwrap(),
        "5\n"
    );
}

#[test]
fn exact_layout_and_comments_do_not_rewrite_literals() {
    let source = "// heading\r\nlet// prefix\r\nmut x:i64=001; // tail  \r\nif true{print(\"a\\0\\r\\n//日本語\");}\r\n";
    let expected = "// heading\nlet // prefix\nmut x: i64 = 001; // tail  \nif true {\n    print(\"a\\0\\r\\n//日本語\");\n}\n";
    assert_eq!(format(source, LetStyle::Preserve).unwrap(), expected);
    for style in STYLES {
        let once = format(source, style).unwrap();
        assert_eq!(format(&once, style).unwrap(), once);
        assert!(once.contains("// tail  \n"));
        assert!(once.contains("\"a\\0\\r\\n//日本語\""));
    }
    assert_eq!(
        format("print(match x { X::A{} => 1, });", LetStyle::Show).unwrap(),
        "print(match x {\n    X::A {} => 1,\n});\n"
    );
}

#[test]
fn all_example_sources_and_standard_modules_format_idempotently() {
    fn files(dir: &std::path::Path, result: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                files(&p, result);
            } else if p.extension().is_some_and(|s| s == "ceru") {
                result.push(p);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = vec![];
    files(&root.join("examples"), &mut paths);
    files(&root.join("src/standard"), &mut paths);
    for path in paths {
        let source = std::fs::read_to_string(&path).unwrap();
        for style in STYLES {
            let text = format(&source, style)
                .unwrap_or_else(|e| panic!("{} {style:?}: {e:?}", path.display()));
            assert_eq!(
                format(&text, style).unwrap(),
                text,
                "{} {style:?}",
                path.display()
            );
        }
    }
}

#[test]
fn all_self_contained_examples_preserve_semantic_ir() {
    // Iteration temporaries include source offsets, but their binding IDs and uses
    // are stable. Ignore only that spelling, outside quoted string values.
    fn without_temporary_origins(line: &str) -> String {
        let mut out = String::new();
        let mut rest = line;
        let mut quoted = false;
        let mut escaped = false;
        while !rest.is_empty() {
            if !quoted {
                let prefix = ["%$for_in_snapshot_", "%$for_in_cursor_", "%$for_in_length_"]
                    .into_iter()
                    .find(|p| rest.starts_with(p));
                if let Some(prefix) = prefix {
                    let suffix = &rest[prefix.len()..];
                    let end = suffix
                        .find(|c: char| !c.is_ascii_digit() && c != '_')
                        .unwrap_or(suffix.len());
                    if suffix[end..].starts_with('@') {
                        out.push_str(prefix.trim_end_matches('_'));
                        rest = &suffix[end..];
                        continue;
                    }
                }
            }
            let c = rest.chars().next().unwrap();
            rest = &rest[c.len_utf8()..];
            out.push(c);
            if escaped {
                escaped = false;
            } else if quoted && c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = !quoted;
            }
        }
        out
    }
    fn semantic_text(program: &crate::ir::Program) -> String {
        crate::ir::text::emit(program)
            .lines()
            .map(|line| {
                if line.starts_with(';') || line.starts_with("  array-length ") {
                    line.split(" [").next().unwrap().to_owned()
                } else {
                    line.to_owned()
                }
            })
            .map(|line| without_temporary_origins(&line))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn visit(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                // These sources are deliberately combined by compile_sources.
                if path.file_name().unwrap() == "source_files" {
                    continue;
                }
                visit(&path);
                continue;
            }
            if !path.extension().is_some_and(|s| s == "ceru") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let module = parser::parse_module(lexer::lex(&source).unwrap()).unwrap();
            if module.imports.iter().any(|i| !i.path.starts_with("std:")) {
                continue;
            }
            let before = crate::modules::from_source(&source)
                .unwrap()
                .to_ir()
                .unwrap();
            for style in STYLES {
                let output = format(&source, style).unwrap();
                let after = crate::modules::from_source(&output)
                    .unwrap()
                    .to_ir()
                    .unwrap();
                assert_eq!(
                    semantic_text(&after),
                    semantic_text(&before),
                    "{} {style:?}",
                    path.display()
                );
            }
        }
    }
    visit(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"));
}

#[test]
fn binding_syntax_retains_source_ids_and_distinguishes_identifier_let() {
    let mut sources = crate::source::SourceMap::new();
    sources.add("unrelated", "");
    let text = "let let:i64=1; for(let i:infer, value:infer in [let]){print(value);}";
    let id = sources.add("bindings.ceru", text);
    let source = sources.get(id).unwrap();
    let (_, syntax) = parser::parse_module_with_syntax(lexer::lex_source(source).unwrap()).unwrap();
    assert_eq!(syntax.bindings.len(), 3);
    for binding in &syntax.bindings {
        assert_eq!(binding.start.source_id(), id);
        if let Some(span) = binding.let_span {
            assert_eq!(span.source_id(), id);
            assert_eq!(&text[span.start()..span.end()], "let");
        }
    }
    assert!(syntax.bindings[0].let_span.is_some());
    assert!(syntax.bindings[1].let_span.is_some());
    assert!(syntax.bindings[2].let_span.is_none());
    assert_eq!(
        format_source(source, LetStyle::Hide).unwrap(),
        format(text, LetStyle::Hide).unwrap()
    );
}

#[test]
fn type_angles_are_distinguished_from_comparisons() {
    let source = "fn id <T>(x:T)->T{return x;} print(id :: <i64>(convert <i64>(1))); print(2>(1));";
    let output = format(source, LetStyle::Preserve).unwrap();
    assert!(output.contains("fn id<T>(x: T) -> T"));
    assert!(output.contains("id::<i64>(convert<i64>(1))"));
    assert!(output.contains("2 > (1)"));
    assert_eq!(format(&output, LetStyle::Preserve).unwrap(), output);
}

#[test]
fn blank_lines_and_comments_stay_in_order_when_prefixes_change() {
    let source = "// first\n\n// second\nlet // binding\nx:i64=1;\n\n\n// next\nprint(x);";
    for style in STYLES {
        let text = format(source, style).unwrap();
        assert!(text.starts_with("// first\n\n// second\n"));
        assert!(text.contains("= 1;\n\n// next\n"));
        assert_eq!(text.matches("// binding").count(), 1);
        assert_eq!(format(&text, style).unwrap(), text);
    }
}

#[test]
fn omitted_infer_keeps_the_same_ast_and_ir_when_offsets_are_held_constant() {
    let source = include_str!("../../examples/formatting/infer.ceru").replace(": =", ": infer =");
    // Padding isolates inference semantics from source-offset changes.
    let omitted = source.replace(": infer =", ":       =");
    assert_eq!(
        parser::parse(lexer::lex(&source).unwrap()).unwrap(),
        parser::parse(lexer::lex(&omitted).unwrap()).unwrap()
    );
    assert_eq!(
        crate::compile_to_ir(&source).unwrap(),
        crate::compile_to_ir(&omitted).unwrap()
    );
    for style in STYLES {
        let output = format(&omitted, style).unwrap();
        assert!(output.contains("total: = 0;"));
        assert!(output.contains("i: = 0;"));
        assert_eq!(format(&output, style).unwrap(), output);
    }
}

#[test]
fn omitted_infer_keeps_declaration_rules_and_inference_errors() {
    for input in [
        "let mut value: = 1; value = true;",
        "let value: = 1; value = 2;",
        "value: = 9223372036854775808;",
        "value: = missing;",
        "fn f()->void{} value: = f();",
    ] {
        let explicit = input.replace(": =", ": infer =");
        let compact_error = crate::compile_to_ir(input).unwrap_err();
        let explicit_error = crate::compile_to_ir(&explicit).unwrap_err();
        assert_eq!(compact_error.message(), explicit_error.message(), "{input}");
        let c = compact_error.primary_span().unwrap();
        let e = explicit_error.primary_span().unwrap();
        assert_eq!(
            input[c.start()..c.end()].replace(": =", ": infer ="),
            explicit[e.start()..e.end()],
            "{input}"
        );
    }
    for invalid in [
        "let value = 1;",
        "value: ;",
        "value: =;",
        "value: in [1];",
        "const VALUE: = 1;",
        "fn f(x: =) -> i64 {return 1;}",
        "fn f() -> {return 1;}",
        "type T { x: = }",
        "enum E{V{x: =}}",
        "for(value: in [1]){}",
        "for(value: = in [1]){}",
        "for(i: =, value: infer in [1]){}",
        "for(i: i64, value: = in [1]){}",
        "for(mut i: = 0; i<1; i: = 1){}",
    ] {
        assert!(format(invalid, LetStyle::Preserve).is_err(), "{invalid}");
    }
    let input = "let mut let: = 1; let = 2; for(let v: infer in [let]){print(v);}";
    assert_eq!(
        crate::ir_executor::run(&crate::compile_to_ir(input).unwrap()).unwrap(),
        "2\n"
    );
}

#[test]
fn formatter_preserves_inference_spelling_comments_and_positions() {
    let input = "let x:// before initializer\r\n=1; let y:infer=x; print(y);";
    for style in STYLES {
        let output = format(input, style).unwrap();
        assert!(output.contains("x: // before initializer\n= 1;"));
        assert!(output.contains("y: infer = x;"));
        assert_eq!(format(&output, style).unwrap(), output);
    }
    let mut sources = crate::source::SourceMap::new();
    let id = sources.add("infer.ceru", "let value: = ;");
    let error = format_source(sources.get(id).unwrap(), LetStyle::Show).unwrap_err();
    let span = error.primary_span().unwrap();
    assert_eq!(span.source_id(), id);
    assert_eq!(
        &sources.get(id).unwrap().text()[span.start()..span.end()],
        ";"
    );
}
