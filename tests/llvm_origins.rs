use cerune_lang::{
    codegen::llvm::{self, Options, Target},
    compile_to_llvm_with_options, modules,
};
use std::{fs, path::PathBuf, process::Command};

fn emit(source: &str, annotated: bool) -> String {
    compile_to_llvm_with_options(
        source,
        Options {
            target: Some(Target::X86_64UnknownLinuxGnu),
            annotate_origins: annotated,
        },
    )
    .unwrap()
}

#[test]
fn annotations_preserve_every_example_and_are_deterministic() {
    for entry in fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "ceru") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        let annotated = emit(&source, true);
        let without_comments: String = annotated
            .lines()
            .filter(|line| !line.starts_with("; cerune-origin"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(without_comments, emit(&source, false), "{}", path.display());
        assert_eq!(annotated, emit(&source, true));
    }
}

#[test]
fn annotations_preserve_failure_records_and_instructions() {
    for source in [
        include_str!("../examples/runtime_failures/overflow.ceru"),
        include_str!("../examples/runtime_failures/array_update.ceru"),
        include_str!("../examples/runtime_failures/function_division.ceru"),
    ] {
        let annotated = emit(source, true);
        let without_comments: String = annotated
            .lines()
            .filter(|line| !line.starts_with("; cerune-origin"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(without_comments, emit(source, false));
        assert!(without_comments.contains("@cerune.runtime.fail"));
    }
}

#[test]
fn origin_example_has_reviewable_ir_and_llvm() {
    // fixtureはLFで固定します。利用時のSpanは入力そのもののバイト位置です。
    let source = include_str!("../examples/string_origins.ceru").replace("\r\n", "\n");
    assert_eq!(
        cerune_lang::compile_to_ir_text(&source).unwrap(),
        include_str!("fixtures/observation/string-origins/expected/ir.ceir").replace("\r\n", "\n")
    );
    assert_eq!(
        emit(&source, true),
        include_str!("fixtures/observation/string-origins/expected/llvm.ll").replace("\r\n", "\n")
    );
}

#[test]
fn cli_annotation_option_is_explicit() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/string_origins.ceru");
    let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
        .arg("emit-llvm")
        .arg(&source)
        .args(["--annotate-origins", "--target", "x86_64-unknown-linux-gnu"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        llvm::emit_llvm_with_runtime_limits(
            &modules::load(&source).unwrap().to_ir().unwrap(),
            llvm::Options {
                target: Some(llvm::Target::X86_64UnknownLinuxGnu),
                annotate_origins: true
            },
            Default::default(),
            std::num::NonZeroU32::new(128).unwrap()
        )
        .unwrap()
        .as_bytes()
    );
    for route in ["emit-llvm", "emit-qbe"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(route)
            .arg(&source)
            .args(["--annotate-origins", "--annotate-origins"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
