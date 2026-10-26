use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cerune-observe-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn cli(command: &str, input: &Path, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cerune"))
        .arg(command)
        .arg(input)
        .args(options)
        .output()
        .unwrap()
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}
const TARGETS: &[&str] = &["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"];
const BUDGETS: &[&str] = &["--string-heap-limit", "512", "--array-heap-limit", "1024"];
fn observe(input: &Path, target: &str, output: &Path) -> Output {
    let mut options = vec!["--target", target, "-o", output.to_str().unwrap()];
    options.extend(BUDGETS);
    cli("observe", input, &options)
}

#[test]
fn lir_bundle_keeps_old_artifacts_and_matches_individual_emits() {
    let scratch = Scratch::new();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/ir_stages/lir_layout.ceru");
    for (ti, target) in TARGETS.iter().enumerate() {
        for ssa in [false, true] {
            let old = scratch.0.join(format!("{ti}-{ssa}-old"));
            let new = scratch.0.join(format!("{ti}-{ssa}-new"));
            let mut options = vec!["--target", target, "-o", old.to_str().unwrap()];
            if ssa {
                options.push("--ssa");
            }
            success(&cli("observe", &input, &options));
            options[3] = new.to_str().unwrap();
            options.push("--lir");
            success(&cli("observe", &input, &options));
            for entry in fs::read_dir(&old).unwrap() {
                let name = entry.unwrap().file_name();
                if name != "manifest.json" {
                    assert_eq!(
                        fs::read(old.join(&name)).unwrap(),
                        fs::read(new.join(&name)).unwrap()
                    );
                }
            }
            let manifest = fs::read_to_string(new.join("manifest.json")).unwrap();
            assert!(manifest.contains("cerune-observation-v4"));
            assert!(manifest.contains(r#""assembly": "lir""#));
            assert!(manifest.contains(r#""optimization_passes": []"#));
            let base = cli("emit-lir", &input, &["--target", target]);
            success(&base);
            assert_eq!(base.stdout, fs::read(new.join("program.lir.txt")).unwrap());
            if ssa {
                let emitted = cli("emit-lir", &input, &["--target", target, "--ssa"]);
                success(&emitted);
                assert_eq!(
                    emitted.stdout,
                    fs::read(new.join("program.ssa.lir.txt")).unwrap()
                );
                assert!(manifest.contains(r#""ssa_assembly": "ssa_lir""#));
            }
        }
    }
    let output = scratch.0.join("invalid");
    for args in [
        vec!["-o", output.to_str().unwrap()],
        vec![
            "--target",
            TARGETS[0],
            "--annotate-origins",
            "-o",
            output.to_str().unwrap(),
        ],
        vec![
            "--target",
            TARGETS[0],
            "--ssa",
            "--ssa",
            "-o",
            output.to_str().unwrap(),
        ],
    ] {
        assert!(!cli("emit-lir", &input, &args).status.success());
        assert!(!output.exists());
    }
    assert!(
        !cli(
            "observe",
            &input,
            &[
                "--target",
                TARGETS[0],
                "--lir",
                "--lir",
                "-o",
                output.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!output.exists());
}

#[test]
fn ssa_bundle_keeps_baseline_and_records_the_separate_transformation() {
    let scratch = Scratch::new();
    for (index, example) in [
        "examples/ir_stages/ssa_values.ceru",
        "examples/ir_stages/owned_values.ceru",
        "examples/modules/failure.ceru",
    ]
    .iter()
    .enumerate()
    {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join(example);
        let mut hir = cerune_lang::modules::load(&input).unwrap().to_ir().unwrap();
        hir.string_heap_limit = 512;
        hir.array_heap_limit = 1024;
        let mir = cerune_lang::mir::lower(&hir).unwrap();
        let ssa = cerune_lang::mir::ssa::construct(&mir).unwrap();
        let mapping = cerune_lang::mir::ssa::mapping::emit(&ssa).unwrap();
        for (ti, target) in TARGETS.iter().enumerate() {
            let baseline = scratch.0.join(format!("{index}-{ti}-baseline"));
            success(&observe(&input, target, &baseline));
            let output = scratch.0.join(format!("{index}-{ti}-ssa"));
            let second = scratch.0.join(format!("{index}-{ti}-again"));
            for dir in [&output, &second] {
                let mut options = vec!["--ssa", "--target", target, "-o", dir.to_str().unwrap()];
                options.extend(BUDGETS);
                let out = cli("observe", &input, &options);
                success(&out);
                assert!(out.stdout.is_empty());
            }
            for file in [
                "sources.json",
                "program.ceir",
                "program.mir.txt",
                "program.origins.s",
            ] {
                assert_eq!(
                    fs::read(output.join(file)).unwrap(),
                    fs::read(baseline.join(file)).unwrap(),
                    "{file}"
                );
            }
            let mut options = vec!["--ssa"];
            options.extend(BUDGETS);
            let emitted = cli("emit-mir", &input, &options);
            success(&emitted);
            assert_eq!(
                fs::read(output.join("program.ssa.txt")).unwrap(),
                emitted.stdout
            );
            assert_eq!(
                fs::read_to_string(output.join("program.ssa-map.txt")).unwrap(),
                mapping
            );
            let manifest = fs::read_to_string(output.join("manifest.json")).unwrap();
            assert!(manifest.contains("\"schema\": \"cerune-observation-v3\""));
            assert!(manifest.contains("\"optimization_passes\": []"));
            assert!(manifest.contains("\"executed\": false"));
            assert!(manifest.contains("\"kind\": \"representation\""));
            assert!(manifest.contains("\"pass\": \"scalar-ssa-v1\""));
            assert!(
                manifest.contains(
                    "\"input\": \"mir\", \"output\": \"ssa\", \"mapping\": \"ssa_mapping\""
                )
            );
            assert!(manifest.contains("\"assembly\": \"mir\""));
            assert!(manifest.contains("\"ssa_assembly\": \"ssa_lowered_mir\""));
            assert!(manifest.contains("\"pass\": \"ssa-lower-v1\""));
            let lowered = cerune_lang::mir::ssa::lower(&ssa).unwrap();
            assert_eq!(
                fs::read_to_string(output.join("program.ssa-lowered.mir.txt")).unwrap(),
                cerune_lang::mir::text::emit(&lowered.program)
            );
            assert_eq!(
                fs::read_to_string(output.join("program.ssa-lowered-map.txt")).unwrap(),
                lowered.mapping
            );
            let mut native_options = vec!["--ssa", "--target", target, "--annotate-origins"];
            native_options.extend(BUDGETS);
            let asm = cli("emit-asm", &input, &native_options);
            success(&asm);
            assert_eq!(
                fs::read(output.join("program.ssa.origins.s")).unwrap(),
                asm.stdout
            );
            let object = output.join("temporary.o");
            native_options.extend(["-o", object.to_str().unwrap()]);
            let obj = cli("emit-obj", &input, &native_options);
            success(&obj);
            assert!(obj.stdout.is_empty());
            assert_eq!(
                fs::read(&object).unwrap(),
                cerune_lang::codegen::x86_64::emit_object_from_lir(
                    &cerune_lang::codegen::x86_64::lower_mir_with_call_limit(
                        &lowered.program,
                        cerune_lang::codegen::x86_64::Target::parse(target).unwrap(),
                        std::num::NonZeroU32::new(128).unwrap()
                    )
                    .unwrap(),
                    true
                )
                .unwrap()
            );
            fs::remove_file(object).unwrap();
            assert_eq!(fs::read_dir(&output).unwrap().count(), 10);
            for entry in fs::read_dir(&output).unwrap() {
                let file = entry.unwrap().file_name();
                assert_eq!(
                    fs::read(output.join(&file)).unwrap(),
                    fs::read(second.join(&file)).unwrap()
                );
            }
            let emit_file = scratch.0.join(format!("{index}-{ti}.ssa.txt"));
            let out = cli(
                "emit-mir",
                &input,
                &[
                    "-o",
                    emit_file.to_str().unwrap(),
                    "--ssa",
                    "--string-heap-limit",
                    "512",
                    "--array-heap-limit",
                    "1024",
                ],
            );
            success(&out);
            assert!(out.stdout.is_empty());
            assert_eq!(fs::read(emit_file).unwrap(), emitted.stdout);
        }
    }
}

#[test]
fn bundle_matches_individual_emits_and_is_deterministic_for_both_targets() {
    let scratch = Scratch::new();
    for (index, example) in [
        "examples/ir_stages/control_flow.ceru",
        "examples/ir_stages/owned_values.ceru",
        "examples/ir_stages/native_calls.ceru",
        "examples/modules/main.ceru",
        "examples/string_origins.ceru",
    ]
    .iter()
    .enumerate()
    {
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join(example);
        for (ti, target) in TARGETS.iter().enumerate() {
            let bundle = scratch.0.join(format!("{index}-{ti}"));
            let second = scratch.0.join(format!("{index}-{ti}-again"));
            let result = observe(&input, target, &bundle);
            success(&result);
            assert!(result.stdout.is_empty());
            success(&observe(&input, target, &second));
            for (command, file) in [
                ("emit-sources", "sources.json"),
                ("emit-ir", "program.ceir"),
                ("emit-mir", "program.mir.txt"),
                ("emit-asm", "program.origins.s"),
            ] {
                let mut options = Vec::new();
                if command != "emit-sources" {
                    options.extend(BUDGETS);
                }
                if command == "emit-asm" {
                    options.extend(["--target", target, "--annotate-origins"]);
                }
                let individual = cli(command, &input, &options);
                success(&individual);
                let actual = fs::read(bundle.join(file)).unwrap();
                assert_eq!(actual, individual.stdout, "{example} {target} {file}");
                assert_eq!(actual, fs::read(second.join(file)).unwrap());
            }
            let manifest = fs::read_to_string(bundle.join("manifest.json")).unwrap();
            assert!(manifest.contains("\"schema\": \"cerune-observation-v1\""));
            assert!(manifest.contains(&format!("\"target\": \"{target}\"")));
            assert!(manifest.contains("\"string_heap_limit\": 512"));
            assert!(manifest.contains("\"array_heap_limit\": 1024"));
            assert!(manifest.contains("\"executed\": false"));
            assert!(manifest.contains("\"optimization_passes\": []"));
            assert_eq!(
                manifest,
                fs::read_to_string(second.join("manifest.json")).unwrap()
            );
            assert_eq!(fs::read_dir(&bundle).unwrap().count(), 5);
        }
    }
}

#[test]
fn observation_preserves_source_bytes_and_does_not_execute_traps() {
    let scratch = Scratch::new();
    let input = scratch.0.join("日本語 source.ceru");
    let text = "print(\"日本語\\0\\r\\n\");\r\nprint(1 / 0);\r\n";
    fs::write(&input, text).unwrap();
    let output = scratch.0.join("bundle");
    let observed = observe(&input, TARGETS[0], &output);
    success(&observed);
    assert!(observed.stdout.is_empty());
    let sources = fs::read_to_string(output.join("sources.json")).unwrap();
    assert!(sources.contains(r#"print(\"日本語\\0\\r\\n\");\u000d\u000a"#));
    let executed = cli("run", &input, &[]);
    assert!(!executed.status.success());
    assert_eq!(executed.stdout, "日本語\0\r\n\n".as_bytes());
    assert!(String::from_utf8_lossy(&executed.stderr).contains("zero"));
}

#[test]
fn rejects_missing_or_duplicate_options_without_creating_output() {
    let scratch = Scratch::new();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/ir_stages/control_flow.ceru");
    let output = scratch.0.join("bundle");
    let path = output.to_str().unwrap();
    for options in [
        vec!["-o", path],
        vec!["--target", TARGETS[0]],
        vec!["--target", "host", "-o", path],
        vec!["--target", TARGETS[0], "--target", TARGETS[1], "-o", path],
        vec!["--target", TARGETS[0], "-o", path, "-o", path],
        vec!["--target", TARGETS[0], "-o", path, "--annotate-origins"],
        vec![
            "--target",
            TARGETS[0],
            "-o",
            path,
            "--string-heap-limit",
            "-1",
        ],
        vec![
            "--target",
            TARGETS[0],
            "-o",
            path,
            "--array-heap-limit",
            "1",
            "--array-heap-limit",
            "2",
        ],
        vec!["--target", TARGETS[0], "-o", path, "--ssa", "--ssa"],
    ] {
        let result = cli("observe", &input, &options);
        assert!(!result.status.success(), "{options:?}");
        assert!(result.stdout.is_empty());
        assert!(!output.exists());
    }
}

#[test]
fn refuses_existing_paths_and_leaves_no_bundle_on_input_errors() {
    let scratch = Scratch::new();
    let input = scratch.0.join("source.ceru");
    fs::write(&input, "print(7);").unwrap();
    let existing = scratch.0.join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("manifest.json"), b"keep").unwrap();
    assert!(!observe(&input, TARGETS[0], &existing).status.success());
    assert_eq!(fs::read(existing.join("manifest.json")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(existing).unwrap().count(), 1);
    assert!(!observe(&input, TARGETS[0], &input).status.success());
    assert_eq!(fs::read_to_string(&input).unwrap(), "print(7);");
    let bundle = scratch.0.join("bundle");
    fs::write(&input, "value: i64 = true;").unwrap();
    assert!(!observe(&input, TARGETS[0], &bundle).status.success());
    assert!(!bundle.exists());
    fs::write(&input, "print(7);").unwrap();
    assert!(
        !observe(
            &input,
            TARGETS[0],
            &scratch.0.join("missing").join("bundle")
        )
        .status
        .success()
    );
    assert!(!scratch.0.join("missing").exists());
}

#[test]
fn ssa_errors_do_not_overwrite_files_or_leave_a_bundle() {
    let scratch = Scratch::new();
    let input = scratch.0.join("source.ceru");
    let output = scratch.0.join("bundle");
    let emitted = scratch.0.join("ssa.txt");
    fs::write(&input, "value:i64=true;").unwrap();
    fs::write(&emitted, "keep").unwrap();
    let failed = cli(
        "emit-mir",
        &input,
        &["--ssa", "-o", emitted.to_str().unwrap()],
    );
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert_eq!(fs::read_to_string(&emitted).unwrap(), "keep");
    let options = [
        "--ssa",
        "--target",
        TARGETS[0],
        "-o",
        output.to_str().unwrap(),
    ];
    assert!(!cli("observe", &input, &options).status.success());
    assert!(!output.exists());
    fs::write(&input, "print(1/0);").unwrap();
    success(&cli("observe", &input, &options));
    let before = fs::read(output.join("manifest.json")).unwrap();
    assert!(!cli("observe", &input, &options).status.success());
    assert_eq!(fs::read(output.join("manifest.json")).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn refuses_symlink_output() {
    let scratch = Scratch::new();
    let input = scratch.0.join("source.ceru");
    fs::write(&input, "print(7);").unwrap();
    let destination = scratch.0.join("destination");
    fs::create_dir(&destination).unwrap();
    let link = scratch.0.join("link");
    std::os::unix::fs::symlink(&destination, &link).unwrap();
    assert!(!observe(&input, TARGETS[0], &link).status.success());
    assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
}
