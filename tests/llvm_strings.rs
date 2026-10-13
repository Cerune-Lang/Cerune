#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/termination.rs"]
mod termination;
#[path = "support/u64_cases.rs"]
mod u64_cases;

#[path = "support/string_cases.rs"]
mod string_cases;

use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use cerune_lang::{
    codegen::llvm::Target, compile_to_c, compile_to_llvm, compile_to_llvm_with_target, run_vm,
};

const LINUX: Target = Target::X86_64UnknownLinuxGnu;
const WINDOWS: Target = Target::X86_64PcWindowsMsvc;

#[test]
fn target_is_explicit_and_emission_is_deterministic() {
    let source = "print(\"日\\0\\r\\n\");";
    let error = compile_to_llvm(source).unwrap_err();
    assert!(error.message().contains("explicit --target"));
    assert!(error.primary_span().is_some());
    for target in [LINUX, WINDOWS] {
        let llvm = compile_to_llvm_with_target(source, Some(target)).unwrap();
        assert!(llvm.starts_with(&format!("target triple = \"{}\"", target.triple())));
        assert!(llvm.contains("%cerune.string = type { ptr, i64 }"));
        assert!(llvm.contains("[6 x i8] c\"\\E6\\97\\A5\\00\\0D\\0A\""));
        assert_eq!(llvm.contains("@_setmode"), target == WINDOWS);
        assert!(llvm.contains("call void @cerune.output.part"));
        assert!(
            !llvm.contains("@malloc") && !llvm.contains("@strlen") && !llvm.contains("@strcmp")
        );
        assert_eq!(
            llvm,
            compile_to_llvm_with_target(source, Some(target)).unwrap()
        );
    }
    assert!(
        compile_to_llvm("print(1);")
            .unwrap_err()
            .message()
            .contains("output requires an explicit --target")
    );
    for target in [Some(LINUX), Some(WINDOWS)] {
        let llvm = compile_to_llvm_with_target("print(1);", target).unwrap();
        assert_eq!(llvm.contains("@_setmode"), target == Some(WINDOWS));
        assert!(!llvm.contains("%cerune.string"));
        assert_eq!(llvm.contains("target triple"), target.is_some());
    }
}

#[test]
fn numeric_only_output_matches_bytes_with_explicit_targets() {
    let Some(harness) = Native::new() else {
        return;
    };
    for source in [
        "print(1); print(true); print(0.5);",
        "fn send() -> void { print(42); } send();",
        "print([1, 2]);",
    ] {
        harness.matches(source, &run_vm(source).unwrap());
    }
}

#[test]
fn unused_string_types_and_functions_require_a_target() {
    for source in [
        "type Label { text: string, }",
        "type Rows { values: [[string; 1]; 2], }",
        "fn identity(value: string) -> string { return value; }",
        "fn unused() -> string { return \"unused\"; }",
        "fn unused() -> bool { return \"a\" == \"b\"; }",
    ] {
        assert!(
            compile_to_llvm(source)
                .unwrap_err()
                .primary_span()
                .is_some()
        );
        for target in [LINUX, WINDOWS] {
            let llvm = compile_to_llvm_with_target(source, Some(target)).unwrap();
            assert!(llvm.contains("%cerune.string = type"), "{source}");
        }
    }
}

#[test]
fn cli_validates_targets_before_overwriting_output() {
    let directory = test_directory();
    let source = directory.join("source.ceru");
    let artifact = directory.join("output.ll");
    fs::write(&source, "print(\"test\");").unwrap();
    for flags in [
        vec![],
        vec!["--target"],
        vec!["--target", "unknown"],
        vec!["--target", LINUX.triple(), "--target", WINDOWS.triple()],
    ] {
        fs::write(&artifact, b"existing artifact").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-llvm")
            .arg(&source)
            .arg("-o")
            .arg(&artifact)
            .args(flags)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(fs::read(&artifact).unwrap(), b"existing artifact");
    }
    for target_first in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
        command.arg("emit-llvm").arg(&source);
        if target_first {
            command.args(["--target", LINUX.triple()]);
        }
        command.arg("--output").arg(&artifact);
        if !target_first {
            command.args(["--target", LINUX.triple()]);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert!(
            fs::read_to_string(&artifact)
                .unwrap()
                .contains(LINUX.triple())
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

fn test_directory() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "cerune-llvm-strings-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    directory
}

struct Native {
    directory: PathBuf,
    clang: OsString,
    cc: OsString,
    target: Target,
}

impl Native {
    fn new() -> Option<Self> {
        crash_dialogs::suppress();
        // 実行テスト側がホストに合うターゲットを指定します。生成APIは推測しません。
        let target = if cfg!(all(
            target_os = "windows",
            target_arch = "x86_64",
            target_env = "msvc"
        )) {
            WINDOWS
        } else if cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        )) {
            LINUX
        } else {
            assert!(
                std::env::var_os("CERUNE_TEST_LLVM_CLANG").is_none(),
                "native LLVM test target unavailable"
            );
            eprintln!("native LLVM execution skipped: unsupported test host");
            return None;
        };
        fn compiler(variable: &str, default: &str) -> Option<OsString> {
            let configured = std::env::var_os(variable);
            let compiler = configured.clone().unwrap_or_else(|| default.into());
            if Command::new(&compiler)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
            {
                Some(compiler)
            } else {
                assert!(
                    configured.is_none(),
                    "{variable} compiler unavailable: {compiler:?}"
                );
                eprintln!(
                    "native comparison skipped: {compiler:?} unavailable; set {variable} to require it"
                );
                None
            }
        }
        let clang = compiler("CERUNE_TEST_LLVM_CLANG", "clang")?;
        let cc = compiler("CERUNE_TEST_CC", if cfg!(windows) { "clang" } else { "cc" })?;
        Some(Self {
            directory: test_directory(),
            clang,
            cc,
            target,
        })
    }

    fn run(&self, source: &str, llvm: bool, optimization: &str) -> Output {
        self.run_with_origins(source, llvm, optimization, false)
    }

    fn run_with_origins(
        &self,
        source: &str,
        llvm: bool,
        optimization: &str,
        annotate_origins: bool,
    ) -> Output {
        let input = self
            .directory
            .join(if llvm { "program.ll" } else { "program.c" });
        let executable = self.directory.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        fs::write(
            &input,
            if llvm {
                cerune_lang::compile_to_llvm_with_options(
                    source,
                    cerune_lang::codegen::llvm::Options {
                        target: Some(self.target),
                        annotate_origins,
                    },
                )
                .unwrap()
            } else {
                compile_to_c(source).unwrap()
            },
        )
        .unwrap();
        let mut command = Command::new(if llvm { &self.clang } else { &self.cc });
        command.arg(optimization);
        if llvm {
            command.arg(format!("--target={}", self.target.triple()));
        } else {
            command.args(["-std=c11", "-pedantic-errors"]);
        }
        command.arg(&input).arg("-o").arg(&executable);
        if !cfg!(windows) {
            command.arg("-lm");
        }
        let built = command.output().expect("start native compiler");
        assert!(
            built.status.success(),
            "LLVM={llvm} {optimization}: {}\n{source}",
            String::from_utf8_lossy(&built.stderr)
        );
        Command::new(&executable)
            .current_dir(&self.directory)
            .output()
            .expect("run generated program")
    }

    fn matches(&self, source: &str, expected: &str) {
        assert_eq!(run_vm(source).unwrap(), expected);
        for optimization in ["-O0", "-O2"] {
            for llvm in [false, true] {
                let actual = self.run(source, llvm, optimization);
                assert!(
                    actual.status.success(),
                    "LLVM={llvm}: {}",
                    String::from_utf8_lossy(&actual.stderr)
                );
                assert!(actual.stderr.is_empty(), "{:?}", actual.stderr);
                // 改行・文字コードを正規化せず、既知の期待値、VM、C、LLVMを比較します。
                assert_eq!(
                    actual.stdout,
                    expected.as_bytes(),
                    "LLVM={llvm} {optimization}\n{source}"
                );
            }
        }
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        // このテストが新規作成した専用ディレクトリだけを削除します。
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn bytes_equality_and_mixed_output_match_vm_and_c() {
    let Some(native) = Native::new() else { return };
    native.matches(string_cases::CASES[0].0, string_cases::CASES[0].1);
    native.matches(string_cases::UNUSED_DEFAULT, "1\ntrue\n");
    native.matches(string_cases::BYTE_LENGTH.0, string_cases::BYTE_LENGTH.1);
    for source in [
        include_str!("../examples/string_values.ceru"),
        include_str!("../examples/string_lookup.ceru"),
        include_str!("../examples/native_values.ceru"),
        include_str!("../examples/packet_counter.ceru"),
    ] {
        native.matches(source, &run_vm(source).unwrap());
    }
}

#[test]
fn direct_ir_example_matches_known_output_across_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    native.matches(
        include_str!("../examples/ir_execution.ceru"),
        "Hello, IR\nHello, 世界\n[\"Cerune\", \"世界\"]\n",
    );
}

#[test]
fn return_values_arrays_products_and_copies_remain_independent() {
    let Some(native) = Native::new() else { return };
    native.matches(string_cases::CASES[1].0, string_cases::CASES[1].1);
}

#[test]
fn effects_and_short_circuiting_keep_source_order() {
    let Some(native) = Native::new() else { return };
    native.matches(string_cases::CASES[2].0, string_cases::CASES[2].1);
}

#[test]
fn product_updates_keep_base_copies_and_evaluation_order() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::PRODUCT_UPDATES {
        native.matches(source, expected);
    }
}

#[test]
fn origin_example_executes_identically_with_and_without_annotations() {
    let Some(native) = Native::new() else { return };
    let source = include_str!("../examples/string_origins.ceru");
    let expected = "日本語\0\ntrue\nfalse\n";
    native.matches(source, expected);
    for optimization in ["-O0", "-O2"] {
        let output = native.run_with_origins(source, true, optimization, true);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, expected.as_bytes());
    }
}

#[test]
fn array_bounds_and_u64_failures_stop_at_runtime() {
    let Some(native) = Native::new() else { return };
    for source in string_cases::OUT_OF_BOUNDS
        .iter()
        .chain(u64_cases::FAILURES)
    {
        assert!(run_vm(source).is_err());
        for optimization in ["-O0", "-O2"] {
            for llvm in [false, true] {
                termination::assert_expected(
                    &native.run(source, llvm, optimization),
                    if llvm {
                        termination::Expected::IllegalInstruction
                    } else {
                        termination::Expected::CheckedCFailure
                    },
                    source,
                );
            }
        }
    }
}

#[test]
fn u64_values_match_known_output_on_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    native.matches(u64_cases::EXAMPLE.0, u64_cases::EXAMPLE.1);
    native.matches(u64_cases::BOUNDARIES.0, u64_cases::BOUNDARIES.1);
    let annotated = native.run_with_origins(u64_cases::EXAMPLE.0, true, "-O2", true);
    assert!(annotated.status.success());
    assert!(annotated.stderr.is_empty());
    assert_eq!(annotated.stdout, u64_cases::EXAMPLE.1.as_bytes());
}

#[test]
fn constants_match_known_values_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::constant_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn sums_preserve_choices_and_payloads_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::sum_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn array_length_preserves_evaluation_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::array_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn array_iteration_matches_known_outputs_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::iteration_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn constant_array_lengths_preserve_values_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::length_constant_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn rounding_matches_vm_in_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::rounding_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn truncation_matches_vm_in_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::truncation_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn generic_functions_preserve_values_across_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::generic_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn aggregate_values_match_optimized_c_and_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::aggregate_cases::CASES {
        native.matches(source, expected);
    }
}

#[test]
fn dynamic_strings_match_optimized_llvm() {
    let Some(native) = Native::new() else { return };
    for &(source, expected) in string_cases::concat_cases::CASES {
        native.matches(source, expected);
    }
}
