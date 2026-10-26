use super::*;

fn execute(hir: &ir::Program, name: &str, cap_memory: bool) -> Option<std::process::Output> {
    let (node, wabt) = tools()?;
    let dir = directory(name);
    let wasm = build(hir, &dir, &node, &wabt);
    if cap_memory {
        let path = dir.join("program.wat");
        let wat = fs::read_to_string(&path).unwrap();
        assert!(wat.contains("(memory 1)"));
        fs::write(&path, wat.replace("(memory 1)", "(memory 1 1)")).unwrap();
        let result = run(
            Command::new(&node).arg(wabt).arg(path).arg("-o").arg(&wasm),
            &dir,
            "capped-assemble",
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Some(run(
        Command::new(node)
            .arg(root().join("tests/support/run_wasm.cjs"))
            .arg(wasm),
        &dir,
        "execute",
    ))
}

#[test]
fn reentrant_aggregate_frames_keep_parent_values_and_return_destinations() {
    // This fixture isolates frame layout; source recursion is covered by shared depth tests.
    let mut hir = cerune_lang::compile_to_ir(
        r#"
fn leaf(depth: i64, value: [i64; 2]) -> [i64; 2] { return value; }
fn walk(depth: i64, value: [i64; 2]) -> [i64; 2] {
    if depth == 0 { return value; }
    saved: [i64; 2] = [value[0] + depth, depth];
    child: [i64; 2] = leaf(depth - 1, saved);
    return [saved[0] + child[0], saved[1] + child[1]];
}
print(walk(3, [10, 0]));
"#,
    )
    .unwrap();
    introduce_cycle(&mut hir);
    let Some(result) = execute(&hir, "recursive-frames", false) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[60, 7]\n");
}

#[test]
fn returned_frames_are_reused_after_early_and_normal_returns() {
    let hir = cerune_lang::compile_to_ir(
        r#"
fn work(value: [i64; 2], early: bool) -> [i64; 2] {
    if early { return value; }
    return [value[1], value[0]];
}
mut total: i64 = 0;
for (mut i: i64 = 0; i < 10000; i = i + 1) {
    result: [i64; 2] = work([1, 2], i % 2 == 0);
    total = total + result[0];
}
print(total);
"#,
    )
    .unwrap();
    let Some(result) = execute(&hir, "frame-reuse", true) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"15000\n");
}

#[test]
fn physical_frame_allocation_failure_keeps_call_origin_and_prior_output() {
    let values = vec!["1"; 9000].join(",");
    let source = format!(
        "fn argument(value: i64) -> i64 {{ print(value); return value; }} fn large(left: i64, right: i64) -> void {{ values: [i64; 9000] = [{values}]; print(values[0]); }} print(7); large(argument(2), argument(3)); print(9);"
    );
    let hir = cerune_lang::compile_to_ir(&source).unwrap();
    let call = hir.statements.iter().find(|s| matches!(&s.kind, ir::StatementKind::Call { function_name, .. } if function_name == "large")).unwrap();
    let expected = cerune_lang::runtime::RuntimeFailure {
        code: cerune_lang::runtime::FailureCode::AllocationFailed,
        node_id: call.id,
        span: call.span,
    };
    let Some(result) = execute(&hir, "frame-exhaustion", true) else {
        return;
    };
    assert!(!result.status.success());
    assert_eq!(result.stdout, b"7\n2\n3\n");
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        format!("cerune: {}\n", expected.record())
    );
    let result = execute(&hir, "frame-growth", false).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"7\n2\n3\n1\n9\n");

    let entry = cerune_lang::compile_to_ir(&format!(
        "fn main() -> void {{ values: [i64; 9000] = [{values}]; print(values[0]); }}"
    ))
    .unwrap();
    let result = execute(&entry, "entry-frame-exhaustion", true).unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(
        result.stderr,
        b"cerune: runtime-v1 code=allocation-failed origin=host\n"
    );
}

#[test]
fn owned_arguments_example_keeps_budget_and_returned_values() {
    let mut hir =
        cerune_lang::compile_to_ir(include_str!("../../examples/owned_arguments.ceru")).unwrap();
    hir.string_heap_limit = 22;
    let expected = cerune_lang::ir_executor::run(&hir).unwrap();
    let Some(result) = execute(&hir, "owned-frames", true) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
}

#[test]
fn call_frames_example_matches_all_four_executors() {
    let hir =
        cerune_lang::compile_to_ir(include_str!("../../examples/call_frames/main.ceru")).unwrap();
    let expected = include_bytes!("../../examples/call_frames/main.stdout");
    for route in 0..4 {
        let (output, failure) = reference(&hir, route, usize::MAX, usize::MAX, "error", 0);
        assert_eq!(output, expected);
        assert!(failure.is_none());
    }
    let Some(result) = execute(&hir, "example-frames", true) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, expected);
}

fn introduce_cycle(hir: &mut ir::Program) {
    let leaf = hir
        .function_definitions
        .iter()
        .find(|f| f.name == "leaf")
        .unwrap()
        .id;
    let id = hir
        .function_definitions
        .iter()
        .find(|f| f.name == "walk")
        .unwrap()
        .id;
    let mut replaced = 0;
    for statement in hir
        .function_definitions
        .iter_mut()
        .flat_map(|f| &mut f.body)
    {
        if let ir::StatementKind::Binding { value, .. } = &mut statement.kind
            && let ir::ExprKind::Call {
                function_id,
                function_name,
                ..
            } = &mut value.kind
            && *function_id == leaf
        {
            *function_id = id;
            *function_name = "walk".into();
            replaced += 1;
        }
    }
    assert_eq!(replaced, 1);
}

#[test]
fn recursive_owned_values_keep_parent_storage_and_separate_heap_lifetimes() {
    let mut hir = cerune_lang::compile_to_ir(
        r#"
fn leaf(depth: i64, value: [string; 1]) -> [string; 1] { return value; }
fn walk(depth: i64, value: [string; 1]) -> [string; 1] {
    if depth == 0 { return value; }
    saved: [string; 1] = [concat(value[0], "!")];
    dynamic: [string] = array_copy(saved);
    child: [string; 1] = leaf(depth - 1, saved);
    return [concat(dynamic[0], child[0])];
}
for (mut i: i64 = 0; i < 100; i = i + 1) {
    result: [string; 1] = walk(3, ["x"]);
    if i == 99 { print(result); }
}
"#,
    )
    .unwrap();
    hir.string_heap_limit = 128;
    hir.array_heap_limit = 256;
    introduce_cycle(&mut hir);
    let Some(result) = execute(&hir, "recursive-owned-frames", true) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[\"x!x!!x!!!x!!!\"]\n");
}

#[test]
fn private_frame_names_do_not_collide_with_source_bindings() {
    let hir = cerune_lang::compile_to_ir(
        r#"
fn preserve(frame_base: [i64; 1], next_frame: i64) -> [i64; 1] {
    return [frame_base[0] + next_frame];
}
frame_base: [i64; 1] = [7];
next_frame: [i64; 1] = preserve(frame_base, 2);
print(next_frame);
"#,
    )
    .unwrap();
    let Some(result) = execute(&hir, "frame-names", false) else {
        return;
    };
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[9]\n");
}
