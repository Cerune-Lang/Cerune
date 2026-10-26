use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn release(input: &Path, dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.args(["release", "native"])
        .arg(input)
        .arg("-o")
        .arg(dir);
    cmd
}
fn record(dir: &Path, name: &str) -> Value {
    serde_json::from_slice(&fs::read(dir.join(name)).unwrap()).unwrap()
}
fn save(dir: &Path, name: &str, value: &Value) {
    fs::write(dir.join(name), serde_json::to_vec(value).unwrap()).unwrap();
}
fn fixture(scratch: &Scratch, clang: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/build_native/main.ceru");
    let dir = scratch.0.join("built");
    success(&output(&mut build(&source, &dir, clang, false), &scratch.0));
    dir
}
#[test]
fn release_preserves_build_bytes_and_runs_after_relocation_without_tools() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    for ssa in [false, true] {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/build_native/main.ceru");
        let built = scratch.0.join(format!("built {ssa}"));
        let mut cmd = build(&source, &built, &clang, ssa);
        if ssa {
            cmd.arg("--include-sources");
        }
        success(&output(&mut cmd, &scratch.0));
        let originals: Vec<_> = fs::read_dir(&built)
            .unwrap()
            .map(|f| {
                let f = f.unwrap();
                (f.file_name(), fs::read(f.path()).unwrap())
            })
            .collect();
        let packaged = scratch.0.join(format!("release {ssa}"));
        // No PATH discovery or subprocess can be required for packaging.
        let packaged_result = output(release(&built, &packaged).env("PATH", ""), &scratch.0);
        success(&packaged_result);
        assert!(packaged_result.stdout.is_empty());
        let m = record(&packaged, "release.json");
        assert_eq!(m["status"], "success");
        assert_eq!(m["sources_included"], ssa);
        assert_eq!(m["compiled"], false);
        assert_eq!(m["linked"], false);
        assert_eq!(m["executed"], false);
        for (name, bytes) in &originals {
            assert_eq!(
                fs::read(built.join(name)).unwrap(),
                *bytes,
                "input was modified"
            );
            assert_eq!(fs::read(packaged.join("build").join(name)).unwrap(), *bytes);
        }
        for entry in m["files"].as_array().unwrap() {
            let bytes = fs::read(packaged.join(entry["path"].as_str().unwrap())).unwrap();
            assert_eq!(entry["bytes"], bytes.len());
            assert_eq!(entry["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        }
        for line in text(&packaged, "SHA256SUMS").lines() {
            let (hash, name) = line.split_once("  ").unwrap();
            assert_eq!(
                hash,
                format!(
                    "{:x}",
                    Sha256::digest(fs::read(packaged.join(name)).unwrap())
                )
            );
        }
        let r = record(&packaged, "runtime-requirements.json");
        assert_eq!(r["target"], target());
        assert_eq!(r["deployment_verified"], false);
        assert_eq!(r["transitive_dependencies_resolved"], false);
        let libs = r["direct"]["libraries"].as_array().unwrap();
        assert!(!libs.is_empty());
        if cfg!(windows) {
            assert!(
                libs.iter()
                    .any(|n| n.as_str().unwrap().eq_ignore_ascii_case("kernel32.dll"))
            );
            let imports = r["direct"]["imports"].as_array().unwrap();
            let write = imports
                .iter()
                .find(|entry| entry["name"] == "WriteFile")
                .unwrap();
            assert_eq!(write["kind"], "name");
            assert!(write["hint"].is_u64());
            assert!(write.get("ordinal").is_none());
        } else {
            assert!(libs.contains(&json!("libc.so.6")));
            assert!(
                r["direct"]["interpreter"]
                    .as_str()
                    .unwrap()
                    .contains("ld-linux")
            );
            assert!(
                r["direct"]["symbol_versions"]
                    .to_string()
                    .contains("GLIBC_")
            );
        }
        fs::rename(&built, scratch.0.join(format!("unavailable build {ssa}"))).unwrap();
        let moved = scratch.0.join(format!("relocated {ssa}"));
        fs::rename(packaged, &moved).unwrap();
        let ran = output(
            Command::new(moved.join(m["entry"].as_str().unwrap()))
                .arg("3")
                .env("PATH", ""),
            &scratch.0,
        );
        success(&ran);
        assert_eq!(ran.stdout, b"[9, 4]\n[2, 4]\n6\n");
        let before = fs::read(moved.join("release.json")).unwrap();
        let retry = output(
            &mut release(&scratch.0.join(format!("unavailable build {ssa}")), &moved),
            &scratch.0,
        );
        assert!(!retry.status.success());
        assert_eq!(fs::read(moved.join("release.json")).unwrap(), before);
    }
}
#[test]
fn release_rejects_incomplete_inconsistent_and_unsafe_inputs_before_output() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let built = fixture(&scratch, &clang);
    let original = record(&built, "manifest.json");
    for (index, (key, value)) in [
        ("status", json!("failed")),
        ("status", json!("building")),
        ("schema", json!("unknown")),
        ("route", json!("vm")),
        ("executable", json!("../outside")),
        ("artifacts", json!(["../outside"])),
        ("ssa", Value::Null),
        ("sources_included", json!(true)),
        ("optimization_passes", json!(["unknown"])),
        ("target", json!("unknown")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut edited = original.clone();
        edited[key] = value;
        save(&built, "manifest.json", &edited);
        let out = scratch.0.join(format!("reject-{index}"));
        assert!(
            !output(&mut release(&built, &out), &scratch.0)
                .status
                .success()
        );
        assert!(!out.exists());
    }
    save(&built, "manifest.json", &original);
    for (file, key, value) in [
        ("link-result.json", "status", json!("failed")),
        ("link.result.json", "exit_code", json!(17)),
        ("link-plan.json", "target", json!("wrong")),
        ("runtime.json", "system_libraries_bundled", json!(true)),
    ] {
        let backup = fs::read(built.join(file)).unwrap();
        let mut edited = record(&built, file);
        edited[key] = value;
        save(&built, file, &edited);
        let out = scratch.0.join(file);
        assert!(
            !output(&mut release(&built, &out), &scratch.0)
                .status
                .success()
        );
        assert!(!out.exists());
        fs::write(built.join(file), backup).unwrap();
    }
    let backup = fs::read(exe(&built)).unwrap();
    fs::write(exe(&built), b"not executable").unwrap();
    let out = scratch.0.join("bad-executable");
    assert!(
        !output(&mut release(&built, &out), &scratch.0)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(exe(&built), backup).unwrap();
    fs::write(built.join("unrelated.txt"), "keep").unwrap();
    let out = scratch.0.join("unexpected");
    assert!(
        !output(&mut release(&built, &out), &scratch.0)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::remove_file(built.join("unrelated.txt")).unwrap();
    fs::rename(built.join("program.o"), scratch.0.join("saved.o")).unwrap();
    let out = scratch.0.join("missing");
    assert!(
        !output(&mut release(&built, &out), &scratch.0)
            .status
            .success()
    );
    assert!(!out.exists());
}
#[test]
fn release_rejects_build_options_nested_output_and_links() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let built = fixture(&scratch, &clang);
    for arg in [
        "--ssa",
        "--include-sources",
        "--target",
        "--linker",
        "--string-heap-limit",
        "--argument-byte-limit",
        "--",
    ] {
        let out = scratch.0.join(arg.replace('-', "_"));
        assert!(
            !output(release(&built, &out).arg(arg), &scratch.0)
                .status
                .success()
        );
        assert!(!out.exists());
    }
    let nested = built.join("nested");
    assert!(
        !output(&mut release(&built, &nested), &scratch.0)
            .status
            .success()
    );
    assert!(!nested.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link = scratch.0.join("linked-build");
        symlink(&built, &link).unwrap();
        let out = scratch.0.join("link-output");
        assert!(
            !output(&mut release(&link, &out), &scratch.0)
                .status
                .success()
        );
        assert!(!out.exists());
        let object = built.join("program.o");
        let saved = scratch.0.join("object");
        fs::rename(&object, &saved).unwrap();
        symlink(&saved, &object).unwrap();
        assert!(
            !output(&mut release(&built, &out), &scratch.0)
                .status
                .success()
        );
        assert!(!out.exists());
    }
}

#[test]
fn release_preserves_runtime_stops_without_executing_during_packaging() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let source = scratch.0.join("failure.ceru");
    fs::write(&source, "print(7); zero: = 0; print(1 / zero); print(99);").unwrap();
    let built = scratch.0.join("built");
    success(&output(
        &mut build(&source, &built, &clang, false),
        &scratch.0,
    ));
    let packaged = scratch.0.join("release");
    let out = output(&mut release(&built, &packaged), &scratch.0);
    success(&out);
    assert!(out.stdout.is_empty());
    let before = output(&mut Command::new(exe(&built)), &scratch.0);
    let after = output(&mut Command::new(exe(&packaged.join("build"))), &scratch.0);
    assert!(!after.status.success());
    assert_eq!(after.status, before.status);
    assert_eq!(after.stdout, b"7\n");
    assert_eq!(after.stderr, before.stderr);
    let error = String::from_utf8_lossy(&after.stderr);
    assert!(error.contains("runtime-v1 code=division-by-zero"));
    assert!(error.contains("node="));
    assert!(error.contains("bytes="));
}

#[test]
fn call_depth_survives_build_release_and_observation() {
    let Some(clang) = driver() else { return };
    let scratch = Scratch::new();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/call_frames/return_paths.ceru");
    for ssa in [false, true] {
        for n in [1, 2] {
            let built = scratch.0.join(format!("calls-{ssa}-{n}"));
            success(&output(
                build(&source, &built, &clang, ssa).args(["--call-depth-limit", &n.to_string()]),
                &scratch.0,
            ));
            assert_eq!(record(&built, "manifest.json")["call_depth_limit"], n);
            assert!(text(&built, "program.lir.txt").contains(&format!("call-depth-limit {n}")));
            let before = output(&mut Command::new(exe(&built)), &scratch.0);
            if n == 1 {
                assert_eq!(before.status.code(), Some(1));
                assert!(String::from_utf8_lossy(&before.stderr).contains("depth-limit limit=1"));
            } else {
                success(&before);
                assert_eq!(
                    before.stdout,
                    include_bytes!("../../examples/call_frames/return_paths.stdout")
                );
            }
            let package = scratch.0.join(format!("calls-release-{ssa}-{n}"));
            success(&output(&mut release(&built, &package), &scratch.0));
            assert_eq!(record(&package, "release.json")["call_depth_limit"], n);
            let after = output(&mut Command::new(exe(&package.join("build"))), &scratch.0);
            assert_eq!(after.status.code(), before.status.code());
            assert_eq!(after.stdout, before.stdout);
            assert_eq!(after.stderr, before.stderr);
            let observed = scratch.0.join(format!("calls-observe-{ssa}-{n}"));
            success(&output(
                Command::new(env!("CARGO_BIN_EXE_cerune"))
                    .arg("observe")
                    .arg(&source)
                    .args([
                        "--lir",
                        "--ssa",
                        "--target",
                        target(),
                        "--call-depth-limit",
                        &n.to_string(),
                        "-o",
                    ])
                    .arg(&observed),
                &scratch.0,
            ));
            assert_eq!(record(&observed, "manifest.json")["call_depth_limit"], n);
            let lir = if ssa {
                "program.ssa.lir.txt"
            } else {
                "program.lir.txt"
            };
            assert_eq!(text(&observed, lir), text(&built, "program.lir.txt"));
        }
    }
}
#[test]
fn release_preserves_absent_legacy_limit_and_rejects_invalid_recorded_limit() {
    let Some(clang) = driver() else { return };
    let scratch = Scratch::new();
    let built = fixture(&scratch, &clang);
    let mut m = record(&built, "manifest.json");
    for (i, value) in [json!(0), json!(129), json!("2"), Value::Null]
        .into_iter()
        .enumerate()
    {
        m["call_depth_limit"] = value;
        save(&built, "manifest.json", &m);
        let packaged = scratch.0.join(format!("invalid-calls-{i}"));
        let out = output(&mut release(&built, &packaged), &scratch.0);
        assert_eq!(out.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("invalid build limit call_depth_limit")
        );
        assert!(!packaged.exists());
    }
    m.as_object_mut().unwrap().remove("call_depth_limit");
    save(&built, "manifest.json", &m);
    let packaged = scratch.0.join("legacy-calls");
    success(&output(&mut release(&built, &packaged), &scratch.0));
    assert!(record(&packaged, "release.json")["call_depth_limit"].is_null());
    assert!(
        record(&packaged.join("build"), "manifest.json")
            .get("call_depth_limit")
            .is_none()
    );
}
