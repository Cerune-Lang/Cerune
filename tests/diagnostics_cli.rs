use std::{fs, path::PathBuf, process::Command};

#[test]
fn runtime_records_preserve_prints_and_identify_the_failing_expression() {
    use cerune_lang::RunError;
    for (name, expression, expected_output, code) in [
        (
            "overflow",
            "counter + 1",
            "カウンタを更新\n",
            "integer-overflow",
        ),
        ("array_update", "[2]", "4\n", "array-index-out-of-bounds"),
        (
            "function_division",
            "value / divisor",
            "false\n除算を開始\n",
            "division-by-zero",
        ),
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("examples/runtime_failures/{name}.ceru"));
        let source = fs::read_to_string(&path).unwrap();
        let RunError::Execution(error) = cerune_lang::run_vm(&source).unwrap_err() else {
            panic!()
        };
        let failure = error.runtime_failure().unwrap();
        assert_eq!(failure.code.name(), code);
        assert_eq!(
            &source[failure.span.start()..failure.span.end()],
            expression
        );
        assert_eq!(error.vm_error().output(), expected_output);
        for structured in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command.arg("run").arg(&path);
            if structured {
                command.args(["--diagnostic-format", "runtime-v1"]);
            }
            let result = command.output().unwrap();
            assert_eq!(result.status.code(), Some(1));
            assert_eq!(result.stdout, expected_output.as_bytes());
            if structured {
                assert_eq!(
                    String::from_utf8(result.stderr)
                        .unwrap()
                        .replace("\r\n", "\n"),
                    format!("cerune: {}\n", failure.record())
                );
            } else {
                assert!(!result.stderr.windows(10).any(|s| s == b"runtime-v1"));
            }
        }
    }
    for args in [
        vec!["--diagnostic-format"],
        vec!["--diagnostic-format", "json"],
        vec!["--diagnostic-format", "runtime-v1", "extra"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .args(["run", "unused.ceru"])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8(result.stderr).unwrap().contains("usage:"));
    }
}

fn fixture_path(case_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("diagnostics")
        .join(case_name)
}

fn expected_output(case_name: &str, file_name: &str) -> String {
    let path = fixture_path(case_name).join("expected").join(file_name);

    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn assert_diagnostic(case_name: &str, command: &str, expected_file: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
        .arg(command)
        .arg(fixture_path(case_name).join("source.ceru"))
        .output()
        .unwrap_or_else(|error| panic!("failed to run cerune {command}: {error}"));

    assert_eq!(
        output.status.code(),
        Some(1),
        "cerune {command} returned an unexpected status for case `{case_name}`"
    );

    assert!(
        output.stdout.is_empty(),
        "cerune {command} wrote to stdout for case `{case_name}`:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let actual = String::from_utf8(output.stderr)
        .unwrap_or_else(|error| panic!("cerune {command} emitted non-UTF-8 stderr: {error}"));

    let expected = expected_output(case_name, expected_file);

    // OSやGitの設定による改行コードの違いを診断内容の差として扱わない
    assert_eq!(
        normalize_line_endings(&actual),
        normalize_line_endings(&expected),
        "unexpected stderr from cerune {command} for case `{case_name}`"
    );
}

#[test]
fn lexer_unexpected_character_matches_expected_diagnostic() {
    assert_diagnostic("lexer-unexpected-character", "check", "check.stderr");
}

#[test]
fn parser_missing_semicolon_matches_expected_diagnostic() {
    assert_diagnostic("parser-missing-semicolon", "check", "check.stderr");
}

#[test]
fn parser_product_field_assignment_matches_expected_diagnostic() {
    assert_diagnostic("parser-product-field-assignment", "check", "check.stderr");
}

#[test]
fn parser_for_missing_left_parenthesis_matches_expected_diagnostic() {
    assert_diagnostic(
        "parser-for-missing-left-parenthesis",
        "check",
        "check.stderr",
    );
}

#[test]
fn parser_for_missing_right_parenthesis_matches_expected_diagnostic() {
    assert_diagnostic(
        "parser-for-missing-right-parenthesis",
        "check",
        "check.stderr",
    );
}

#[test]
fn semantic_unknown_binding_matches_expected_diagnostic() {
    assert_diagnostic("semantic-unknown-binding", "check", "check.stderr");
}

#[test]
fn semantic_unknown_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-unknown-type", "check", "check.stderr");
}

#[test]
fn semantic_unknown_product_field_matches_expected_diagnostic() {
    assert_diagnostic("semantic-unknown-product-field", "check", "check.stderr");
}

#[test]
fn semantic_missing_product_field_matches_expected_diagnostic() {
    assert_diagnostic("semantic-missing-product-field", "check", "check.stderr");
}

#[test]
fn semantic_duplicate_product_field_matches_expected_diagnostic() {
    assert_diagnostic("semantic-duplicate-product-field", "check", "check.stderr");
}

#[test]
fn semantic_recursive_product_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-recursive-product-type", "check", "check.stderr");
}

#[test]
fn semantic_recursive_product_array_matches_expected_diagnostic() {
    assert_diagnostic("semantic-recursive-product-array", "check", "check.stderr");
}

#[test]
fn semantic_array_length_matches_expected_diagnostic() {
    assert_diagnostic("semantic-array-length", "check", "check.stderr");
}

#[test]
fn semantic_array_element_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-array-element-type", "check", "check.stderr");
}

#[test]
fn semantic_array_index_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-array-index-type", "check", "check.stderr");
}

#[test]
fn semantic_immutable_array_assignment_matches_expected_diagnostic() {
    assert_diagnostic(
        "semantic-immutable-array-assignment",
        "check",
        "check.stderr",
    );
}

#[test]
fn semantic_array_assignment_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-array-assignment-type", "check", "check.stderr");
}

#[test]
fn semantic_array_assignment_non_array_matches_expected_diagnostic() {
    assert_diagnostic(
        "semantic-array-assignment-non-array",
        "check",
        "check.stderr",
    );
}

#[test]
fn semantic_index_non_array_matches_expected_diagnostic() {
    assert_diagnostic("semantic-index-non-array", "check", "check.stderr");
}

#[test]
fn semantic_immutable_assignment_matches_expected_diagnostic() {
    assert_diagnostic("semantic-immutable-assignment", "check", "check.stderr");
}

#[test]
fn semantic_boolean_arithmetic_matches_expected_diagnostic() {
    assert_diagnostic("semantic-boolean-arithmetic", "check", "check.stderr");
}

#[test]
fn semantic_integer_literal_range_matches_expected_diagnostic() {
    assert_diagnostic("semantic-integer-literal-range", "check", "check.stderr");
}

#[test]
fn semantic_if_condition_matches_expected_diagnostic() {
    assert_diagnostic("semantic-if-condition", "check", "check.stderr");
}

#[test]
fn semantic_while_condition_matches_expected_diagnostic() {
    assert_diagnostic("semantic-while-condition", "check", "check.stderr");
}

#[test]
fn semantic_for_condition_matches_expected_diagnostic() {
    assert_diagnostic("semantic-for-condition", "check", "check.stderr");
}

#[test]
fn semantic_for_scope_matches_expected_diagnostic() {
    assert_diagnostic("semantic-for-scope", "check", "check.stderr");
}

#[test]
fn semantic_break_outside_loop_matches_expected_diagnostic() {
    assert_diagnostic("semantic-break-outside-loop", "check", "check.stderr");
}

#[test]
fn semantic_continue_outside_loop_matches_expected_diagnostic() {
    assert_diagnostic("semantic-continue-outside-loop", "check", "check.stderr");
}

#[test]
fn semantic_block_scope_matches_expected_diagnostic() {
    assert_diagnostic("semantic-block-scope", "check", "check.stderr");
}

#[test]
fn semantic_unknown_function_matches_expected_diagnostic() {
    assert_diagnostic("semantic-unknown-function", "check", "check.stderr");
}

#[test]
fn semantic_function_argument_count_matches_expected_diagnostic() {
    assert_diagnostic("semantic-function-argument-count", "check", "check.stderr");
}

#[test]
fn semantic_function_argument_type_matches_expected_diagnostic() {
    assert_diagnostic("semantic-function-argument-type", "check", "check.stderr");
}

#[test]
fn semantic_function_missing_return_matches_expected_diagnostic() {
    assert_diagnostic("semantic-function-missing-return", "check", "check.stderr");
}

#[test]
fn semantic_function_invalid_main_matches_expected_diagnostic() {
    assert_diagnostic("semantic-function-invalid-main", "check", "check.stderr");
}

#[test]
fn semantic_function_recursion_is_accepted() {
    let result = Command::new(env!("CARGO_BIN_EXE_cerune"))
        .arg("check")
        .arg(fixture_path("semantic-function-recursion").join("source.ceru"))
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result.stderr);
    assert!(result.stderr.is_empty());
}

#[test]
fn emit_ir_semantic_unknown_binding_matches_expected_diagnostic() {
    assert_diagnostic("semantic-unknown-binding", "emit-ir", "emit-ir.stderr");
}

#[test]
fn run_compilation_error_matches_expected_diagnostic() {
    for command in ["run", "run-ir", "run-vm"] {
        assert_diagnostic("semantic-unknown-binding", command, "run.stderr");
    }
}

#[test]
fn run_vm_error_matches_expected_diagnostic() {
    assert_diagnostic("vm-division-by-zero", "run-vm", "run.stderr");
}

#[test]
fn run_integer_overflow_matches_expected_diagnostic() {
    assert_diagnostic("vm-integer-overflow", "run-vm", "run.stderr");
}

#[test]
fn run_numeric_conversion_failures_match_expected_diagnostics() {
    for reason in ["inexact", "range", "negative-zero", "nonfinite", "nan"] {
        assert_diagnostic(&format!("vm-conversion-{reason}"), "run-vm", "run.stderr");
    }
}

#[test]
fn run_remainder_by_zero_matches_expected_diagnostic() {
    assert_diagnostic("vm-remainder-by-zero", "run-vm", "run.stderr");
}

#[test]
fn run_invalid_shift_count_matches_expected_diagnostic() {
    assert_diagnostic("vm-invalid-shift-count", "run-vm", "run.stderr");
}

#[test]
fn run_left_shift_overflow_matches_expected_diagnostic() {
    assert_diagnostic("vm-left-shift-overflow", "run-vm", "run.stderr");
}

#[test]
fn run_array_index_out_of_bounds_matches_expected_diagnostic() {
    assert_diagnostic("vm-array-index-out-of-bounds", "run-vm", "run.stderr");
}
