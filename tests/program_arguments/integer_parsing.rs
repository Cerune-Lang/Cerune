//! Independent decimal expectations, shared by interpreter and compiled-route tests.
use super::{execute, hir, snapshot};
use cerune_lang::{bytecode, ir, mir, modules};

fn expected(text: &str) -> String {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        "InvalidFormat{}\n".into()
    } else {
        match text.parse::<i64>() {
            Ok(value) => format!("Value{{value: {value}}}\n"),
            Err(_) => "OutOfRange{}\n".into(),
        }
    }
}

pub(super) fn cases() -> Vec<(ir::Program, Vec<&'static str>, String)> {
    let mut values: Vec<String> = [
        "",
        "-",
        "+",
        "+1",
        "--1",
        " 1",
        "1 ",
        "1\r\n",
        "1\0",
        "1.0",
        "1e2",
        "0xff",
        "1_000",
        "１２",
        "١٢",
        "é",
        "e\u{301}",
        "0",
        "-0",
        "0000",
        "-0001",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "-9223372036854775809",
        "999999999999999999999999x",
        "-999999999999999999999999x",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    values.extend([
        "0".repeat(200),
        format!("-{}1", "0".repeat(200)),
        "9".repeat(200),
    ]);
    let mut state = 175u64;
    for _ in 0..32 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = state as i64;
        values.push(value.to_string());
        values.push(if value < 0 {
            format!("-000{}", value.unsigned_abs())
        } else {
            format!("000{value}")
        });
    }
    let literals: Vec<_> = values
        .iter()
        .map(|s| {
            format!(
                "\"{}\"",
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\0', "\\0")
                    .replace('\r', "\\r")
                    .replace('\n', "\\n")
            )
        })
        .collect();
    let source = format!(
        r#"import "std:num" as num;
        values: [string; {}] = [{}];
        for (mut i: i64 = 0; i < {}; i = i + 1) {{ print(num::parse_i64(values[i])); }}"#,
        values.len(),
        literals.join(","),
        values.len()
    );
    let mut cases = vec![(
        hir(&source),
        vec![],
        values.iter().map(|s| expected(s)).collect(),
    )];
    let mut owned = hir(r#"import "std:num" as num;
        fn text() -> string { print("once"); return concat("12", "3"); }
        fn value() -> i64 { match num::parse_i64(text()) {
            num::ParseI64::Value { value: n } => { return n; },
            num::ParseI64::InvalidFormat {} => { return 0; },
            num::ParseI64::OutOfRange {} => { return 0; },
        } }
        print(false && value() == 123);
        print(true || value() == 123);
        print(value());
        mut a: [num::ParseI64; 2] = [num::parse_i64("1"), num::parse_i64("x")];
        b: [num::ParseI64; 2] = a;
        a[0] = num::parse_i64("2"); print(a); print(b);
        for (mut i: i64 = 0; i < 8; i = i + 1) { print(num::parse_i64(concat("12", "3"))); }
    "#);
    owned.string_heap_limit = 3;
    cases.push((owned, vec![], format!("false\ntrue\nonce\n123\n[Value{{value: 2}}, InvalidFormat{{}}]\n[Value{{value: 1}}, InvalidFormat{{}}]\n{}", "Value{value: 123}\n".repeat(8))));
    cases.push((hir(include_str!("../../examples/integer_parsing/values.ceru")), vec![], "Value{value: 42}\nValue{value: -9223372036854775808}\nOutOfRange{}\nInvalidFormat{}\nInvalidFormat{}\n".into()));
    // Build direct IR to exercise the complete internal byte-access contract,
    // including indices the standard parser itself never requests.
    let indices = [0i64, 1, 2, 3, -1, i64::MIN, i64::MAX];
    let mut bytes = hir(&"print(byte_len(\"é\\0\"));".repeat(indices.len()));
    let mut next = indices.into_iter();
    for stmt in &mut bytes.statements {
        if let ir::StatementKind::Print { value } = &mut stmt.kind
            && let ir::ExprKind::StringByteLength { value: string } = &value.kind
        {
            let mut index = value.clone();
            index.id = ir::NodeId(100_000 + next.len());
            index.kind = ir::ExprKind::Integer(i128::from(next.next().unwrap()));
            value.kind = ir::ExprKind::StringByteAt {
                value: string.clone(),
                index: Box::new(index),
            };
        }
    }
    assert!(next.next().is_none());
    cases.push((bytes, vec![], "195\n169\n0\n-1\n-1\n-1\n-1\n".into()));
    let mut order = hir(r#"
        fn byte_text() -> string { print("text"); return "A"; }
        fn byte_index() -> i64 { print("index"); return 0; }
        print(byte_len("placeholder"));
    "#);
    let text_fn = order
        .function_definitions
        .iter()
        .find(|f| f.name.ends_with("byte_text"))
        .unwrap();
    let text_call = ir::ExprKind::Call {
        function_id: text_fn.id,
        function_name: text_fn.name.clone(),
        arguments: vec![],
    };
    let index_fn = order
        .function_definitions
        .iter()
        .find(|f| f.name.ends_with("byte_index"))
        .unwrap();
    let index_call = ir::ExprKind::Call {
        function_id: index_fn.id,
        function_name: index_fn.name.clone(),
        arguments: vec![],
    };
    for stmt in &mut order.statements {
        if let ir::StatementKind::Print { value } = &mut stmt.kind
            && let ir::ExprKind::StringByteLength { value: text } = &value.kind
        {
            let mut text = text.clone();
            text.kind = text_call.clone();
            let mut index = value.clone();
            index.id = ir::NodeId(100_001);
            index.kind = index_call.clone();
            value.kind = ir::ExprKind::StringByteAt {
                value: text,
                index: Box::new(index),
            };
        }
    }
    cases.push((order, vec![], "text\nindex\n65\n".into()));
    let seed = hir(include_str!("../../examples/arguments/seed.ceru"));
    for (args, out) in [
        (vec![], "usage: seed <0..2147483646> <0..100>\n"),
        (vec!["1337", "3"], "64538327\n1478294467\n18110394\n"),
        (vec!["0", "0"], ""),
        (vec!["", "1"], "seed: invalid integer\n"),
        (vec!["9223372036854775808", "1"], "seed: outside i64\n"),
        (vec!["-1", "1"], "seed: outside 0..2147483646\n"),
        (vec!["2147483647", "1"], "seed: outside 0..2147483646\n"),
        (vec!["1", "-1"], "count: outside 0..100\n"),
        (vec!["1", "101"], "count: outside 0..100\n"),
        (vec!["1", "x"], "count: invalid integer\n"),
        (vec!["1", "9223372036854775808"], "count: outside i64\n"),
    ] {
        cases.push((seed.clone(), args, out.into()));
    }
    cases
}

pub(super) fn check(out: &std::process::Output, expected: &str) {
    assert!(out.status.success(), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    assert_eq!(out.stdout, expected.as_bytes());
}

#[test]
fn integer_parsing_matches_independent_expectations_on_four_executors() {
    for (p, args, expected) in cases() {
        for route in 0..4 {
            assert_eq!(
                execute(route, &p, &snapshot(&args)).unwrap(),
                expected,
                "route {route} args {args:?}"
            );
        }
    }
}

#[test]
fn integer_parsing_virtual_sources_types_and_observation() {
    let source = r#"import "std:num" as a; import "std:num" as b;
        result: a::ParseI64 = b::parse_i64("42"); print(result);"#;
    let compilation = modules::from_source(source).unwrap();
    let standards: Vec<_> = compilation
        .sources
        .files()
        .filter(|s| matches!(s.kind(), cerune_lang::source::SourceKind::Standard { .. }))
        .collect();
    assert_eq!(standards.len(), 1);
    assert!(matches!(
        standards[0].kind(),
        cerune_lang::source::SourceKind::Standard {
            identifier: "std:num",
            version: 1
        }
    ));
    assert!(standards[0].text().contains("for (mut i:"));
    let p = compilation.to_ir().unwrap();
    let original = p.clone();
    let text = ir::text::emit(&p);
    assert!(text.contains("byte_at.string("));
    assert!(text.contains("parse_i64"));
    assert!(mir::text::emit(&mir::lower(&p).unwrap()).contains("string.byte_at"));
    assert!(bytecode::format_program(&bytecode::lower(&p).unwrap()).contains("byte_at.string"));
    let code = bytecode::lower(&p).unwrap();
    let mut accesses = 0;
    for instruction in code.functions.iter().flat_map(|f| &f.instructions) {
        if matches!(instruction.kind, bytecode::InstructionKind::StringByteAt) {
            let bytecode::InstructionOrigin::Source { span, .. } = instruction.origin else {
                panic!("missing byte access origin")
            };
            assert!(matches!(
                compilation.sources.get(span.source_id()).unwrap().kind(),
                cerune_lang::source::SourceKind::Standard {
                    identifier: "std:num",
                    ..
                }
            ));
            assert!(
                compilation
                    .sources
                    .slice(span)
                    .unwrap()
                    .starts_with("__cerune_string_byte_at(")
            );
            accesses += 1;
        }
    }
    assert_eq!(accesses, 3);
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    assert!(mir::ssa::text::emit(&s).unwrap().contains("string.byte_at"));
    for t in [
        cerune_lang::codegen::x86_64::Target::X86_64PcWindowsMsvc,
        cerune_lang::codegen::x86_64::Target::X86_64UnknownLinuxGnu,
    ] {
        let lir = cerune_lang::codegen::x86_64::lower_mir(&m, t).unwrap();
        assert!(
            cerune_lang::codegen::x86_64::lir::emit(&lir)
                .unwrap()
                .contains("LoadStringByte")
        );
        assert!(
            !cerune_lang::codegen::x86_64::emit_object_from_lir(&lir, true)
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(p, original);
    for route in 0..4 {
        assert_eq!(
            execute(route, &p, &snapshot(&[])).unwrap(),
            "Value{value: 42}\n"
        );
    }
    for invalid in [
        "print(__cerune_string_byte_at(\"1\",0));",
        "print($string_byte_at(\"1\",0));",
        "import \"std:num\" as n; print(n::parse_i64(1));",
    ] {
        assert!(cerune_lang::compile_to_ir(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn integer_parsing_resource_failure_remains_a_stop_with_user_origin() {
    use cerune_lang::{ir_executor, mir_executor, ssa_executor, vm};
    let source = r#"import "std:num" as num; print("before"); print(num::parse_i64(concat("12", "3"))); print("after");"#;
    let compilation = modules::from_source(source).unwrap();
    let mut p = compilation.to_ir().unwrap();
    p.string_heap_limit = 2;
    let direct = ir_executor::run(&p).unwrap_err();
    let failure = direct.runtime_failure().unwrap();
    assert_eq!(
        failure.code,
        cerune_lang::runtime::FailureCode::AllocationLimitExceeded
    );
    assert_eq!(
        compilation.sources.slice(failure.span).unwrap(),
        "concat(\"12\", \"3\")"
    );
    assert_eq!(direct.output(), "before\n");
    let m = mir::lower(&p).unwrap();
    let a = mir_executor::run(&m).unwrap_err();
    let b = ssa_executor::run(&mir::ssa::construct(&m).unwrap()).unwrap_err();
    let c = vm::run(&bytecode::lower(&p).unwrap()).unwrap_err();
    assert_eq!(a.runtime_failure().unwrap(), failure);
    assert_eq!(b.runtime_failure().unwrap(), failure);
    assert_eq!(c.kind(), vm::VmErrorKind::AllocationLimitExceeded);
    for output in [a.output(), b.output(), c.output()] {
        assert_eq!(output, "before\n");
    }
}

// Compile each distinct program once, then exercise all argument cases.
pub(super) struct CompiledCase {
    pub program: ir::Program,
    pub runs: Vec<(Vec<&'static str>, String)>,
}
pub(super) fn compiled_cases() -> Vec<CompiledCase> {
    let mut groups: Vec<CompiledCase> = Vec::new();
    for (program, args, expected) in cases() {
        if let Some(group) = groups.iter_mut().find(|g| g.program == program) {
            group.runs.push((args, expected));
        } else {
            groups.push(CompiledCase {
                program,
                runs: vec![(args, expected)],
            });
        }
    }
    groups
}

#[test]
fn integer_parsing_seed_example_cli_routes() {
    let dir = super::directory("integer-seed-cli");
    let file =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/arguments/seed.ceru");
    for (route, ssa) in super::ROUTES {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_cerune"));
        command.arg(route).arg(&file);
        if *ssa {
            command.arg("--ssa");
        }
        command.args(["--", "1337", "3"]);
        let output = super::process::bounded_output(
            &mut command,
            &dir,
            "integer-seed",
            std::time::Duration::from_secs(15),
        )
        .unwrap();
        check(&output, "64538327\n1478294467\n18110394\n");
    }
}
