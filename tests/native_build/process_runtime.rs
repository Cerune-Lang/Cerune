use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn record(dir: &Path, name: &str) -> Value {
    serde_json::from_slice(&fs::read(dir.join(name)).unwrap()).unwrap()
}
fn save(dir: &Path, name: &str, value: &Value) {
    fs::write(dir.join(name), serde_json::to_vec(value).unwrap()).unwrap();
}
fn release(dir: &Path, out: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_cerune"));
    c.args(["release", "native"])
        .arg(dir)
        .arg("-o")
        .arg(out)
        .env("PATH", "");
    c
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn archive_name() -> &'static str {
    if cfg!(windows) {
        "process-runtime.lib"
    } else {
        "process-runtime.a"
    }
}
#[test]
fn explicit_process_archive_rejects_missing_duplicate_unused_and_non_archive_inputs() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("examples/process_execution/unconfigured.ceru");
    let bad = scratch.0.join("not an archive");
    fs::write(&bad, b"!<thin>\n").unwrap();
    for (i, source, options) in [
        (
            0,
            input.clone(),
            vec!["--process-runtime", bad.to_str().unwrap()],
        ),
        (1, input.clone(), vec!["--process-runtime"]),
        (
            2,
            input,
            vec![
                "--process-runtime",
                bad.to_str().unwrap(),
                "--process-runtime",
                bad.to_str().unwrap(),
            ],
        ),
        (
            3,
            root.join("examples/build_native/main.ceru"),
            vec!["--process-runtime", bad.to_str().unwrap()],
        ),
    ] {
        let dir = scratch.0.join(format!("rejected-{i}"));
        let result = output(
            build(&source, &dir, &clang, false).args(options),
            &scratch.0,
        );
        assert!(!result.status.success(), "{result:?}");
        assert!(!dir.exists());
    }
}
#[test]
#[ignore = "requires CERUNE_TEST_PROCESS_RUNTIME and Linux supervisor; subprocess OS CI supplies them"]
fn process_runtime_build_release_keeps_exact_inputs_and_runs_after_relocation() {
    let clang = driver().expect("Clang required for this explicitly selected test");
    let archive = fs::canonicalize(
        std::env::var_os("CERUNE_TEST_PROCESS_RUNTIME").expect("required runtime archive"),
    )
    .unwrap();
    let original = fs::read(&archive).unwrap();
    let scratch = Scratch::new();
    let child = scratch
        .0
        .join(if cfg!(windows) { "child.exe" } else { "child" });
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut rustc = Command::new("rustc");
    rustc
        .args(["--edition=2024"])
        .arg(root.join("tests/support/process_child.rs"))
        .arg("-o")
        .arg(&child);
    assert!(output(&mut rustc, &scratch.0).status.success());
    fs::write(scratch.0.join("child-directory-marker"), b"registered cwd").unwrap();
    let supervisor = if cfg!(windows) {
        Value::Null
    } else {
        json!(
            fs::canonicalize(
                std::env::var_os("CERUNE_TEST_PROCESS_SUPERVISOR")
                    .expect("Linux supervisor required")
            )
            .unwrap()
        )
    };
    let config = json!({"schema":"cerune-process-v1","platform":if cfg!(windows){"windows"}else{"linux"},"linux_supervisor":supervisor,
 "limits":{"arguments":32,"argument_bytes":16384,"stdin_bytes":1024,"output_bytes":1024,"timeout_ms":10000,"cleanup_timeout_ms":5000},"transfer_bytes":32768,
 "targets":[{"name":"tool","executable":child,"directory":scratch.0,"environment":[["CERUNE_CHILD_VALUE","日本語\r\n"]]}]});
    save(&scratch.0, "private.json", &config);
    let config_path = scratch.0.join("private.json");
    let source = root.join("examples/process_execution/registered.ceru");
    let expected = include_bytes!("../../examples/process_execution/registered.stdout");
    for ssa in [false, true] {
        let selected = scratch.0.join("selected runtime.archive");
        fs::write(&selected, &original).unwrap();
        let built = scratch.0.join(format!("built-{ssa}"));
        let result = output(
            build(&source, &built, &clang, ssa)
                .arg("--process-runtime")
                .arg(&selected),
            &scratch.0,
        );
        success(&result);
        assert!(result.stdout.is_empty());
        let m = record(&built, "manifest.json");
        let r = record(&built, "runtime.json");
        let p = record(&built, "link-plan.json");
        let dependency = &m["process_runtime"];
        assert_eq!(m["schema"], "cerune-native-build-v2");
        assert_eq!(m["executed"], false);
        assert_eq!(m["ssa"], ssa);
        assert_eq!(r["schema"], "cerune-native-runtime-v2");
        assert_eq!(p["schema"], "cerune-link-plan-v2");
        assert_eq!(r["process_runtime"], *dependency);
        assert_eq!(p["process_runtime"], *dependency);
        assert_eq!(dependency["archive"]["sha256"], hash(&original));
        assert_eq!(dependency["archive"]["bytes"], original.len());
        assert_eq!(dependency["required_abi"]["version"], 1);
        assert_eq!(dependency["target"], target());
        assert_eq!(dependency["linux_supervisor"]["required"], !cfg!(windows));
        assert_eq!(dependency["linux_supervisor"]["bundled"], false);
        assert_eq!(dependency["configuration"]["embedded"], false);
        assert_eq!(p["inputs"], json!(["program.o", archive_name()]));
        assert_eq!(fs::read(built.join(archive_name())).unwrap(), original);
        // Replacing/removing the selected input cannot change the saved link input.
        fs::write(&selected, b"changed after build").unwrap();
        fs::remove_file(&selected).unwrap();
        let packaged = scratch.0.join(format!("packaged-{ssa}"));
        success(&output(&mut release(&built, &packaged), &scratch.0));
        assert!(!packaged.join("private.json").exists());
        assert!(!packaged.join("build/private.json").exists());
        let rel = record(&packaged, "release.json");
        let req = record(&packaged, "runtime-requirements.json");
        assert_eq!(rel["schema"], "cerune-native-release-v2");
        assert_eq!(req["schema"], "cerune-native-requirements-v2");
        assert_eq!(rel["process_runtime"], *dependency);
        assert_eq!(req["process_runtime"], *dependency);
        assert_eq!(rel["system_libraries_bundled"], false);
        for item in rel["files"].as_array().unwrap() {
            let data = fs::read(packaged.join(item["path"].as_str().unwrap())).unwrap();
            assert_eq!(item["bytes"], data.len());
            assert_eq!(item["sha256"], hash(&data));
        }
        assert_eq!(
            fs::read(packaged.join("build").join(archive_name())).unwrap(),
            original
        );
        for language in ["README.md", "README.en.md"] {
            let guide = text(&packaged, language);
            assert!(guide.contains("--process-config"));
            assert!(guide.contains("linux_supervisor"));
        }
        let moved = scratch.0.join(format!("moved distribution-{ssa}"));
        fs::rename(&packaged, &moved).unwrap();
        let hidden = scratch.0.join(format!("old build-{ssa}"));
        fs::rename(&built, &hidden).unwrap();
        let executable = exe(&moved.join("build"));
        let got = output(
            Command::new(&executable)
                .args(["--cerune-runtime-v1", "--process-config"])
                .arg(&config_path)
                .arg("--")
                .env("PATH", "")
                .env("CERUNE_PARENT_SENTINEL", "not inherited"),
            &scratch.0,
        );
        success(&got);
        assert_eq!(got.stdout, expected);
        for route in [
            vec!["run"],
            vec!["run-mir"],
            vec!["run-mir", "--ssa"],
            vec!["run-vm"],
        ] {
            let out = output(
                Command::new(env!("CARGO_BIN_EXE_cerune"))
                    .arg(route[0])
                    .arg(&source)
                    .args(&route[1..])
                    .arg("--process-config")
                    .arg(&config_path),
                &scratch.0,
            );
            success(&out);
            assert_eq!(got.stdout, out.stdout);
        }
        let disconnected = output(&mut Command::new(&executable), &scratch.0);
        success(&disconnected);
        assert!(String::from_utf8_lossy(&disconnected.stdout).contains("Unconfigured{}"));
        if !ssa {
            reject_mutations(&hidden, &scratch.0);
        }
    }
}
fn reject_mutations(built: &Path, root: &Path) {
    let names = ["manifest.json", "runtime.json", "link-plan.json"];
    let originals = names.map(|n| record(built, n));
    for i in 0..9 {
        let mut records = originals.clone();
        match i {
            0 => records[0]["process_runtime"]["archive"]["sha256"] = json!("0".repeat(64)),
            1 => {
                for r in &mut records {
                    r["process_runtime"]["required_abi"]["version"] = json!(2);
                }
            }
            2 => {
                for r in &mut records {
                    r["process_runtime"]["target"] = json!("other");
                }
            }
            3 => records[2]["args"][3] = json!("different.archive"),
            4 => records[2]["inputs"] = json!(["program.o"]),
            5 => {
                for r in &mut records {
                    r["process_runtime"]["configuration"]["embedded"] = json!(true);
                }
            }
            6 => records[1]["cerune_runtime"] = json!("in-object"),
            7 => records[0]["schema"] = json!("cerune-native-build-v1"),
            _ => {
                for r in &mut records {
                    r["process_runtime"]["linux_supervisor"]["bundled"] = json!(true);
                }
            }
        }
        for (n, r) in names.iter().zip(&records) {
            save(built, n, r);
        }
        let dest = root.join(format!("reject-record-{i}"));
        let out = output(&mut release(built, &dest), root);
        assert!(!out.status.success(), "{i}: {out:?}");
        assert!(!dest.exists());
    }
    for (n, r) in names.iter().zip(&originals) {
        save(built, n, r);
    }
    // A changed saved archive is rejected even when every JSON record still agrees.
    use std::io::{Seek, SeekFrom, Write};
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(built.join(archive_name()))
        .unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    f.write_all(b"?").unwrap();
    drop(f);
    let dest = root.join("reject-archive-hash");
    assert!(!output(&mut release(built, &dest), root).status.success());
    assert!(!dest.exists());
}

// Header validation is not a substitute for resolving the required ABI at link time.
#[test]
fn process_archive_link_failure_keeps_input_and_rejects_release() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("examples/process_execution/unconfigured.ceru");
    let archive = scratch.0.join("missing-abi.a");
    let mut object = vec![0u8; if cfg!(windows) { 20 } else { 64 }];
    if cfg!(windows) {
        object[..2].copy_from_slice(&[0x64, 0x86]);
    } else {
        object[..7].copy_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1]);
        object[16..20].copy_from_slice(&[1, 0, 62, 0]);
    }
    let mut bytes = b"!<arch>\n".to_vec();
    bytes.extend(
        format!(
            "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
            "empty.o/",
            0,
            0,
            0,
            644,
            object.len()
        )
        .bytes(),
    );
    bytes.extend(object);
    fs::write(&archive, &bytes).unwrap();
    let built = scratch.0.join("failed-build");
    let result = output(
        build(&input, &built, &clang, false)
            .arg("--process-runtime")
            .arg(&archive),
        &scratch.0,
    );
    assert!(!result.status.success(), "{result:?}");
    let manifest = record(&built, "manifest.json");
    assert_eq!(manifest["schema"], "cerune-native-build-v2");
    assert_eq!(manifest["status"], "failed");
    assert_eq!(
        manifest["process_runtime"]["archive"]["sha256"],
        hash(&bytes)
    );
    assert_eq!(fs::read(built.join(archive_name())).unwrap(), bytes);
    assert_eq!(record(&built, "link-result.json")["status"], "failed");
    assert!(built.join("program.o").is_file());
    assert!(built.join("link.stderr.log").is_file());
    let packaged = scratch.0.join("rejected-release");
    assert!(
        !output(&mut release(&built, &packaged), &scratch.0)
            .status
            .success()
    );
    assert!(!packaged.exists());
}
