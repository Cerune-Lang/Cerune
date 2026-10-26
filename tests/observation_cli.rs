use std::{fs, path::PathBuf, process::Command};

const OBSERVATION_CASES: &[&str] = &[
    "i64-addition",
    "checked-i64-arithmetic",
    "i64-minimum-literal",
    "integer-conversions",
    "fixed-width-integers",
    "small-integers",
    "integer-bit-operations",
    "numeric-conversions",
    "truncating-conversions",
    "rounding-conversions",
    "aggregate-values",
    "u64-values",
    "float-types",
    "float-output",
    "mutable-assignment",
    "boolean-comparisons",
    "short-circuit",
    "conditional-scope",
    "while-loop",
    "loop-control",
    "for-loop",
    "product-types",
    "functions",
    "aggregate-functions",
    "fixed-arrays",
    "array-length",
    "array-iteration",
    "constant-array-lengths",
    "generic-functions",
    "product-array-field",
    "product-array-elements",
    "nested-fixed-arrays",
];

#[test]
fn owned_call_observations_match_ir_bytecode_and_execution() {
    for (command, expected) in [
        ("emit-ir", "ir.ceir"),
        ("emit-bytecode", "bytecode.cebc"),
        ("run", "run.stdout"),
        ("run-ir", "run.stdout"),
        ("run-vm", "run.stdout"),
    ] {
        assert_observation("owned-arguments", command, expected);
        assert_observation("borrowed-reads", command, expected);
    }
}

#[test]
fn rounding_origins_are_visible_in_llvm_and_both_assemblies() {
    for (command, target, file) in [
        ("emit-llvm", "x86_64-unknown-linux-gnu", "llvm.annotated.ll"),
        ("emit-asm", "x86_64-unknown-linux-gnu", "linux.annotated.s"),
        ("emit-asm", "x86_64-pc-windows-msvc", "windows.annotated.s"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(command)
            .arg(source_path("rounding-conversions"))
            .args(["--target", target, "--annotate-origins"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected_output("rounding-conversions", file).replace("\r\n", "\n")
        );
    }
}

fn fixture_path(case_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("observation")
        .join(case_name)
}

fn source_path(case_name: &str) -> PathBuf {
    fixture_path(case_name).join("source.ceru")
}

fn expected_output(case_name: &str, file_name: &str) -> String {
    let path = fixture_path(case_name).join("expected").join(file_name);

    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_observation(case_name: &str, command: &str, expected_file: &str) {
    let mut process = Command::new(env!("CARGO_BIN_EXE_cerune"));
    process.arg(command).arg(source_path(case_name));
    if matches!(command, "emit-llvm" | "emit-qbe") {
        process.args(["--target", "x86_64-unknown-linux-gnu"]);
    }
    let output = process
        .output()
        .unwrap_or_else(|error| panic!("failed to run cerune {command}: {error}"));

    assert!(
        output.status.success(),
        "cerune {command} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.stderr.is_empty(),
        "cerune {command} wrote to stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("cerune {command} emitted non-UTF-8 output: {error}"));

    let expected = expected_output(case_name, expected_file);

    // Gitのチェックアウト設定に左右されないように、期待値の改行コードを LF に揃える
    let expected = expected.replace("\r\n", "\n");

    assert_eq!(
        actual, expected,
        "unexpected output from cerune {command} for case `{case_name}`"
    );
}

fn assert_observation_cases(command: &str, expected_file: &str) {
    for &case_name in OBSERVATION_CASES {
        assert_observation(case_name, command, expected_file);
    }
}

#[test]
fn string_observations_match_ir_bytecode_and_vm_output() {
    // 同じ文字列ソースから全経路の変換結果を固定します。
    for (command, file) in [
        ("emit-ir", "ir.ceir"),
        ("emit-c", "c.c"),
        ("emit-llvm", "llvm.ll"),
        ("emit-qbe", "qbe.ssa"),
        ("emit-wat", "wat.wat"),
        ("emit-asm", "asm.s"),
        ("emit-bytecode", "bytecode.cebc"),
        ("run", "run.stdout"),
        ("run-ir", "run.stdout"),
        ("run-vm", "run.stdout"),
    ] {
        assert_observation("string-values", command, file);
        assert_observation("string-byte-length", command, file);
        assert_observation("string-concat", command, file);
    }
}

#[test]
fn missing_string_targets_fail_without_overwriting_an_artifact() {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cerune-string-output-{}-{id}.tmp",
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    use std::io::Write;
    file.write_all(b"existing artifact").unwrap();
    drop(file);
    for command in ["emit-llvm", "emit-qbe"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(command)
            .arg(source_path("string-values"))
            .arg("-o")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("explicit --target"), "{message}");
        assert_eq!(fs::read(&path).unwrap(), b"existing artifact");
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn native_origins_match_ir_and_both_target_assemblies() {
    assert_observation("native-values", "emit-ir", "ir.ceir");
    for (target, file) in [
        ("x86_64-pc-windows-msvc", "windows.s"),
        ("x86_64-unknown-linux-gnu", "linux.s"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-asm")
            .arg(source_path("native-values"))
            .args(["--target", target, "--annotate-origins"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected_output("native-values", file).replace("\r\n", "\n")
        );
    }
}

#[test]
fn emit_ir_matches_expected_output() {
    assert_observation_cases("emit-ir", "ir.ceir");
}

#[test]
fn emit_c_matches_expected_output() {
    assert_observation_cases("emit-c", "c.c");
}

#[test]
fn emit_llvm_matches_expected_output() {
    assert_observation_cases("emit-llvm", "llvm.ll");
}

#[test]
fn emit_qbe_matches_expected_output() {
    assert_observation_cases("emit-qbe", "qbe.ssa");
}

#[test]
fn emit_wat_matches_expected_output() {
    assert_observation_cases("emit-wat", "wat.wat");
}

#[test]
fn emit_asm_matches_expected_output() {
    assert_observation_cases("emit-asm", "asm.s");
}

#[test]
fn emit_bytecode_matches_expected_output() {
    assert_observation_cases("emit-bytecode", "bytecode.cebc");
}

#[test]
fn run_matches_expected_output() {
    for command in ["run", "run-ir", "run-vm"] {
        assert_observation_cases(command, "run.stdout");
    }
}

#[test]
fn aggregate_and_match_origins_remain_visible_in_native_outputs() {
    for (command, target, file) in [
        ("emit-llvm", "x86_64-unknown-linux-gnu", "llvm.annotated.ll"),
        ("emit-asm", "x86_64-unknown-linux-gnu", "linux.annotated.s"),
        ("emit-asm", "x86_64-pc-windows-msvc", "windows.annotated.s"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(command)
            .arg(source_path("aggregate-values"))
            .args(["--target", target, "--annotate-origins"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected_output("aggregate-values", file).replace("\r\n", "\n")
        );
    }
}

#[test]
fn concatenation_origins_and_generated_ownership_are_stable() {
    for (command, target, file) in [
        ("emit-llvm", "x86_64-unknown-linux-gnu", "llvm.annotated.ll"),
        ("emit-asm", "x86_64-unknown-linux-gnu", "linux.annotated.s"),
        ("emit-asm", "x86_64-pc-windows-msvc", "windows.annotated.s"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(command)
            .arg(source_path("string-concat"))
            .args(["--target", target, "--annotate-origins"])
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            expected_output("string-concat", file).replace("\r\n", "\n")
        );
    }
}
