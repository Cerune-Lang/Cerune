use super::*;
use cerune_lang::{lexer, parser, source::SourceMap};

#[test]
fn parser_depth_child() {
    let Some(path) = std::env::var_os("CERUNE_DEPTH_CASE") else {
        return;
    };
    #[path = "../support/crash_dialogs.rs"]
    mod crash_dialogs;
    crash_dialogs::suppress();
    let text = fs::read_to_string(path).unwrap();
    let mut sources = SourceMap::new();
    let id = sources.add("depth.ceru", &text);
    let result = parser::parse(lexer::lex_source(sources.get(id).unwrap()).unwrap());
    if std::env::var("CERUNE_DEPTH_EXPECT").unwrap() == "ok" {
        assert!(result.is_ok(), "{result:?}");
    } else {
        let error = result.unwrap_err();
        assert!(error.message().contains("exceeds"), "{error:?}");
        let span = error.primary_span().unwrap();
        assert_eq!(span.source_id(), id);
        assert!(span.start() <= span.end() && span.end() <= text.len());
        assert!(!text[span.start()..span.end()].is_empty());
    }
}

#[test]
fn parser_limits_accept_boundaries_and_diagnose_overflow_in_bounded_children() {
    let w = Workspace::new();
    let depth = parser::MAX_EXPRESSION_DEPTH;
    let nesting = parser::MAX_SYNTAX_NESTING;
    for (kind, boundary) in [
        ("paren", nesting - 1),
        ("type", nesting - 1),
        ("block", nesting),
        ("array", nesting - 1),
        ("unary", depth - 1),
        ("binary", depth - 1),
        ("logical", depth - 1),
        ("field", depth - 1),
        ("index", depth - 1),
    ] {
        for (n, expected) in [(boundary, "ok"), (boundary + 1, "error"), (4096, "error")] {
            let source = match kind {
                "paren" => format!("print({}1{});", "(".repeat(n), ")".repeat(n)),
                "type" => format!("type T {{ x:{}i64{} }}", "[".repeat(n), "]".repeat(n)),
                "block" => format!("{}{}", "if true {".repeat(n), "}".repeat(n)),
                "array" => format!("x:infer={}0{};", "[".repeat(n), "]".repeat(n)),
                "unary" => format!("print({}true);", "!".repeat(n)),
                "binary" => format!("print(1{});", "+1".repeat(n)),
                "logical" => format!("print(true{});", "&&true".repeat(n)),
                "field" => format!("print(x{});", ".field".repeat(n)),
                "index" => format!("print(x{});", "[0]".repeat(n)),
                _ => unreachable!(),
            };
            let file = w.put("case.ceru", &source);
            let result = process::bounded_output(
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "frontend_limits::parser_depth_child",
                        "--nocapture",
                    ])
                    .env("CERUNE_DEPTH_CASE", &file)
                    .env("CERUNE_DEPTH_EXPECT", expected),
                &w.0,
                &format!("{kind}-{n}"),
                Duration::from_secs(10),
            )
            .unwrap();
            assert!(result.status.success(), "{kind}-{n}: {result:?}");
            assert!(
                String::from_utf8_lossy(&result.stdout).contains("1 passed"),
                "{result:?}"
            );
        }
    }
}

#[test]
fn parser_checks_delimiter_free_recursion_and_combined_tree_depth() {
    for source in [
        format!("print({}0);", "match ".repeat(64)),
        format!("print({}1{});", "-".repeat(80), "+1".repeat(80)),
        format!("print((1{}){});", "+1".repeat(100), ".field".repeat(40)),
    ] {
        let error = parser::parse(lexer::lex(&source).unwrap()).unwrap_err();
        assert!(error.message().contains("exceeds"), "{error:?}");
    }
    // Width is not depth; quoted delimiters and comments are not nesting.
    let source = format!(
        "print(\"{}\"); // {}\nx:infer=[{}];",
        "(".repeat(100),
        "[".repeat(100),
        vec!["1"; 1024].join(",")
    );
    assert!(parser::parse(lexer::lex(&source).unwrap()).is_ok());
    assert!(cerune_lang::compile("print(!false);").is_ok());
}

#[test]
fn cli_depth_errors_preserve_artifacts_and_boundaries_compile() {
    let w = Workspace::new();
    let limit = parser::MAX_SYNTAX_NESTING;
    let entry = w.put(
        "main.ceru",
        &format!(
            "print({}1{});",
            "(".repeat(limit - 1),
            ")".repeat(limit - 1)
        ),
    );
    let result = w.cli("emit-ir", &entry, &[]);
    assert!(result.status.success(), "{result:?}");
    w.put(
        "main.ceru",
        &format!("print(1{});", "+1".repeat(parser::MAX_EXPRESSION_DEPTH - 1)),
    );
    let result = w.cli("emit-ir", &entry, &[]);
    assert!(result.status.success(), "{result:?}");
    w.put(
        "main.ceru",
        &format!(
            "print({}true);",
            "!".repeat(parser::MAX_EXPRESSION_DEPTH - 1)
        ),
    );
    let result = w.cli("emit-ir", &entry, &[]);
    assert!(result.status.success(), "{result:?}");
    w.put(
        "main.ceru",
        &format!("print({}1{});", "(".repeat(limit), ")".repeat(limit)),
    );
    let artifact = w.put("saved.ceir", "keep");
    let failed = w.cli("emit-ir", &entry, &["-o", artifact.to_str().unwrap()]);
    assert_eq!(failed.status.code(), Some(1), "{failed:?}");
    assert!(String::from_utf8_lossy(&failed.stderr).contains("syntax nesting exceeds"));
    assert_eq!(fs::read_to_string(artifact).unwrap(), "keep");

    let source = format!(
        "{}print(1{});{}",
        "if true {".repeat(limit - 1),
        "+1".repeat(parser::MAX_EXPRESSION_DEPTH - limit),
        "}".repeat(limit - 1)
    );
    w.put("combined.ceru", &source);
    let combined = w.cli("emit-ir", &w.0.join("combined.ceru"), &[]);
    assert!(combined.status.success(), "{combined:?}");
    w.put(
        "combined.ceru",
        &format!(
            "{}print(1{});{}",
            "if true {".repeat(limit - 1),
            "+1".repeat(parser::MAX_EXPRESSION_DEPTH - 1),
            "}".repeat(limit - 1)
        ),
    );
    let rejected = w.cli("emit-ir", &w.0.join("combined.ceru"), &[]);
    assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("exceeds"));
    // Imported diagnostics refer to the definition file.
    w.put("main.ceru", "import \"deep.ceru\" as deep;");
    w.put(
        "deep.ceru",
        &format!(
            "pub const V:i64={}1{};",
            "(".repeat(limit + 1),
            ")".repeat(limit + 1)
        ),
    );
    let error = modules::load(&entry).unwrap_err();
    assert!(
        error.render().contains("deep.ceru:1:"),
        "{}",
        error.render()
    );
}

fn limits(file: usize, total: usize, count: usize) -> modules::LoadLimits {
    modules::LoadLimits {
        max_file_bytes: file,
        max_total_bytes: total,
        max_files: count,
    }
}

#[test]
fn source_byte_budgets_count_utf8_crlf_and_report_import_origin() {
    let w = Workspace::new();
    let dependency = "// 日本語\r\npub const V:i64=1;\r\n";
    let root = "import \"data.ceru\" as data;\r\nprint(data::V);\r\n";
    let entry = w.put("main.ceru", root);
    w.put("data.ceru", dependency);
    let total = root.len() + dependency.len();
    let config = limits(root.len().max(dependency.len()), total, 2);
    let loaded = modules::load_with_limits(&entry, config).unwrap();
    assert_eq!(
        loaded
            .sources
            .files()
            .map(|f| f.text().len())
            .sum::<usize>(),
        total
    );
    assert_eq!(
        run_bytecode(&bytecode::lower(&loaded.to_ir().unwrap()).unwrap()).unwrap(),
        "1\n"
    );
    let error =
        modules::load_with_limits(&entry, limits(config.max_file_bytes, total - 1, 2)).unwrap_err();
    assert!(
        error
            .diagnostic
            .message()
            .contains("total source byte limit exceeded")
    );
    assert!(error.render().contains("main.ceru:1:"));
    assert_eq!(error.sources.files().count(), 1);
    let error = modules::load_with_limits(&entry, limits(root.len() - 1, total, 2)).unwrap_err();
    assert!(
        error
            .diagnostic
            .message()
            .contains("source file byte limit exceeded")
    );
    assert!(error.diagnostic.primary_span().is_none());
    assert_eq!(error.sources.files().count(), 0);
}

#[test]
fn duplicate_paths_and_virtual_sources_do_not_spend_file_budget_twice() {
    let w = Workspace::new();
    let root = "import \"shared.ceru\" as a; import \"./shared.ceru\" as b; import \"std:io\" as io; print(a::V+b::V);";
    let shared = "pub const V:i64=1;";
    let entry = w.put("main.ceru", root);
    w.put("shared.ceru", shared);
    let loaded =
        modules::load_with_limits(&entry, limits(root.len(), root.len() + shared.len(), 2))
            .unwrap();
    assert_eq!(loaded.sources.files().count(), 3);
    let error = modules::load_with_limits(&entry, limits(root.len(), root.len() + shared.len(), 1))
        .unwrap_err();
    assert!(
        error
            .diagnostic
            .message()
            .contains("source file count exceeds 1")
    );
    let span = error.diagnostic.primary_span().unwrap();
    assert!(root[span.start()..span.end()].contains("shared.ceru"));
    let root = w.put("virtual.ceru", "import \"std:io\" as io;");
    assert!(modules::load_with_limits(&root, limits(64, 64, 1)).is_ok());
    assert!(cerune_lang::compile("import \"std:io\" as io;").is_ok());
}

#[test]
fn source_budgets_handle_zero_exact_eof_invalid_utf8_and_oversize_cli() {
    let w = Workspace::new();
    let entry = w.put("empty.ceru", "");
    assert!(modules::load_with_limits(&entry, limits(0, 0, 1)).is_ok());
    assert!(
        modules::load_with_limits(&entry, limits(0, 0, 0))
            .unwrap_err()
            .diagnostic
            .message()
            .contains("file count")
    );
    w.put("empty.ceru", " ");
    assert!(
        modules::load_with_limits(&entry, limits(0, 0, 1))
            .unwrap_err()
            .diagnostic
            .message()
            .contains("byte limit")
    );
    assert!(modules::load_with_limits(&entry, limits(1, 1, 1)).is_ok());
    fs::write(&entry, [0xff]).unwrap();
    assert!(
        modules::load_with_limits(&entry, limits(1, 1, 1))
            .unwrap_err()
            .diagnostic
            .message()
            .contains("UTF-8")
    );
    // An oversized imported file is never registered as a valid source.
    let root = w.put("main.ceru", "import \"large.ceru\" as large;");
    w.put("large.ceru", &" ".repeat(65));
    let error = modules::load_with_limits(&root, limits(64, 1024, 2)).unwrap_err();
    assert!(
        error
            .diagnostic
            .message()
            .contains("source file byte limit")
    );
    assert!(error.render().contains("main.ceru:1:"));
    let large = w.put(
        "large.ceru",
        &" ".repeat(modules::LoadLimits::default().max_file_bytes + 1),
    );
    let artifact = w.put("saved.ceir", "keep");
    let result = w.cli("emit-ir", &large, &["-o", artifact.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("source file byte limit"));
    assert_eq!(fs::read_to_string(artifact).unwrap(), "keep");
}

#[test]
fn maximum_import_depth_keeps_leaf_parser_budget_independent() {
    let w = Workspace::new();
    for i in 0..128 {
        w.put(
            &format!("depth{i}.ceru"),
            &format!("import \"depth{}.ceru\" as next;", i + 1),
        );
    }
    let n = parser::MAX_SYNTAX_NESTING - 1;
    w.put(
        "depth128.ceru",
        &format!("pub const V:i64={}1{};", "(".repeat(n), ")".repeat(n)),
    );
    let result = w.cli("emit-ir", &w.0.join("depth0.ceru"), &[]);
    assert!(result.status.success(), "{result:?}");
}
