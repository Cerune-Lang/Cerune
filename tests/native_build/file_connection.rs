use super::*;
use serde_json::{Value, json};

fn record(dir: &Path, name: &str) -> Value {
    serde_json::from_slice(&fs::read(dir.join(name)).unwrap()).unwrap()
}
fn save(dir: &Path, name: &str, value: &Value) {
    fs::write(dir.join(name), serde_json::to_vec(value).unwrap()).unwrap();
}
fn release(input: &Path, dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.args(["release", "native"])
        .arg(input)
        .arg("-o")
        .arg(dir);
    cmd
}

#[test]
fn file_connection_survives_build_release_relocation_and_matches_executors() {
    let Some(clang) = driver() else { return };
    let scratch = Scratch::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/files/roundtrip.ceru");
    let expected = "日本語\0\r\ne\u{301}\n".as_bytes();
    for ssa in [false, true] {
        let built = scratch.0.join(format!("build-{ssa}"));
        success(&output(
            &mut build(&source, &built, &clang, ssa),
            &scratch.0,
        ));
        let manifest = record(&built, "manifest.json");
        let contract = &manifest["file_connection"];
        assert_eq!(contract["schema"], "cerune-native-file-connection-v1");
        assert_eq!(contract["enabled"], true);
        assert_eq!(contract["startup"]["marker"], "--cerune-runtime-v1");
        assert_eq!(contract["startup"]["file_root_option"], "--file-root");
        assert_eq!(contract["startup"]["argument_delimiter"], "--");
        assert_eq!(contract["startup"]["recognition"], "leading-marker-only");
        assert_eq!(contract["root_binding"], "execution-time-argument");
        assert_eq!(contract["missing_root"], "unconfigured-on-file-operation");
        assert_eq!(contract["diagnostic_schema"], "file-v1");
        if cfg!(windows) {
            assert_eq!(contract["os_requirements"]["dynamic_library"], "ntdll.dll");
            assert_eq!(
                contract["os_requirements"]["dynamic_symbol"],
                "NtCreateFile"
            );
            assert!(contract["os_requirements"].get("read_reopen").is_none());
        } else {
            assert_eq!(contract["os_requirements"]["read_reopen"], "/proc/self/fd");
            assert!(contract["os_requirements"].get("dynamic_library").is_none());
        }
        assert_eq!(record(&built, "runtime.json")["file_connection"], *contract);
        let packaged = scratch.0.join(format!("packaged-{ssa}"));
        success(&output(
            release(&built, &packaged).env("PATH", ""),
            &scratch.0,
        ));
        assert_eq!(
            record(&packaged, "release.json")["file_connection"],
            *contract
        );
        assert_eq!(
            record(&packaged, "runtime-requirements.json")["file_connection"],
            *contract
        );
        assert_eq!(
            record(&packaged, "runtime-requirements.json")["deployment_verified"],
            false
        );
        for name in [
            "manifest.json",
            "runtime.json",
            "program.o",
            "program.lir.txt",
            "program.origins.json",
        ] {
            assert_eq!(
                fs::read(built.join(name)).unwrap(),
                fs::read(packaged.join("build").join(name)).unwrap()
            );
        }
        for name in ["README.md", "README.en.md"] {
            let guide = text(&packaged, name);
            assert!(guide.contains("--cerune-runtime-v1 --file-root <directory> --"));
            assert!(guide.contains("file-v1/unconfigured"));
            assert!(guide.contains(if cfg!(windows) {
                "NtCreateFile"
            } else {
                "/proc/self/fd"
            }));
        }
        // Neither packaging nor build performs the source's write operation.
        assert!(!scratch.0.join("sample.txt").exists());
        assert!(!built.join("sample.txt").exists());
        let moved = scratch.0.join(format!("moved-{ssa}"));
        fs::rename(&packaged, &moved).unwrap();
        fs::rename(&built, scratch.0.join(format!("old-build-{ssa}"))).unwrap();
        let program = exe(&moved.join("build"));
        let unconfigured = output(Command::new(&program).env("PATH", ""), &scratch.0);
        assert!(!unconfigured.status.success());
        let error = String::from_utf8_lossy(&unconfigured.stderr);
        assert!(error.contains("file-v1"));
        assert!(error.contains("code=unconfigured"));
        assert!(error.contains("node="));
        assert!(unconfigured.stdout.is_empty());
        let root = scratch.0.join(format!("data-{ssa}"));
        fs::create_dir(&root).unwrap();
        let args = ["--cerune-runtime-v1", "--file-root"];
        let ran = output(
            Command::new(&program)
                .args(args)
                .arg(&root)
                .arg("--")
                .env("PATH", ""),
            &scratch.0,
        );
        success(&ran);
        assert_eq!(ran.stdout, b"true\n16\n");
        assert_eq!(fs::read(root.join("sample.txt")).unwrap(), expected);
        let duplicate = output(
            Command::new(&program).args(args).arg(&root).arg("--"),
            &scratch.0,
        );
        assert!(!duplicate.status.success());
        assert!(String::from_utf8_lossy(&duplicate.stderr).contains("code=already-exists"));
        assert_eq!(fs::read(root.join("sample.txt")).unwrap(), expected);
        for (index, (route, via_ssa)) in [
            ("run", false),
            ("run-mir", false),
            ("run-mir", true),
            ("run-vm", false),
        ]
        .iter()
        .enumerate()
        {
            let root = scratch.0.join(format!("comparison-{ssa}-{index}"));
            fs::create_dir(&root).unwrap();
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
            cmd.arg(route).arg(&source).arg("--file-root").arg(&root);
            if *via_ssa {
                cmd.arg("--ssa");
            }
            let result = output(&mut cmd, &scratch.0);
            success(&result);
            assert_eq!(result.stdout, ran.stdout);
            assert_eq!(fs::read(root.join("sample.txt")).unwrap(), expected);
        }
    }
}

#[test]
fn file_connection_records_distinguish_unused_operations_disabled_and_legacy() {
    let Some(clang) = driver() else { return };
    let scratch = Scratch::new();
    for (index, source) in [
        "print(1);",
        r#"import "std:fs" as fs; fn unused()->string {return fs::read_text("x",0);} print(1);"#,
    ]
    .iter()
    .enumerate()
    {
        let source_file = scratch.0.join(format!("{index}.ceru"));
        fs::write(&source_file, source).unwrap();
        let built = scratch.0.join(format!("built-{index}"));
        success(&output(
            &mut build(&source_file, &built, &clang, false),
            &scratch.0,
        ));
        let mut manifest = record(&built, "manifest.json");
        let mut runtime = record(&built, "runtime.json");
        assert_eq!(manifest["file_connection"]["enabled"], index == 1);
        let package = scratch.0.join(format!("package-{index}"));
        success(&output(&mut release(&built, &package), &scratch.0));
        if index == 0 {
            assert_eq!(
                record(&package, "release.json")["file_connection"],
                json!({"schema":"cerune-native-file-connection-v1","enabled":false})
            );
            assert!(!text(&package, "README.en.md").contains("--file-root <directory>"));
        }
        // Older builds may already contain file operations: no inference from missing metadata.
        manifest.as_object_mut().unwrap().remove("file_connection");
        runtime.as_object_mut().unwrap().remove("file_connection");
        save(&built, "manifest.json", &manifest);
        save(&built, "runtime.json", &runtime);
        let old = scratch.0.join(format!("legacy-{index}"));
        success(&output(&mut release(&built, &old), &scratch.0));
        assert!(record(&old, "release.json")["file_connection"].is_null());
        assert!(record(&old, "runtime-requirements.json")["file_connection"].is_null());
        assert!(text(&old, "README.en.md").contains("not recorded"));
        assert!(text(&old, "README.md").contains("未記録"));
        assert_eq!(record(&old.join("build"), "manifest.json"), manifest);
    }
}

#[test]
fn release_rejects_invalid_or_inconsistent_file_contract_before_output() {
    let Some(clang) = driver() else { return };
    let scratch = Scratch::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/files/roundtrip.ceru");
    let built = scratch.0.join("built");
    success(&output(
        &mut build(&source, &built, &clang, false),
        &scratch.0,
    ));
    let manifest = record(&built, "manifest.json");
    let runtime = record(&built, "runtime.json");
    let original = &manifest["file_connection"];
    let mut bad = vec![
        Value::Null,
        json!(false),
        json!({"enabled":true}),
        json!({"schema":"unknown","enabled":false}),
    ];
    for (field, value) in [
        ("enabled", json!("true")),
        ("schema", json!("cerune-native-file-connection-v2")),
        ("startup", json!({"marker":"unversioned"})),
        ("root_binding", json!("build-directory")),
        ("os_requirements", json!({})),
        ("unknown", json!(true)),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        bad.push(changed);
    }
    let mut wrong_target = original.clone();
    wrong_target["os_requirements"] = if cfg!(windows) {
        json!({"connection":"openat-directory-fd","read_reopen":"/proc/self/fd"})
    } else {
        json!({"connection":"nt-root-directory-handle","dynamic_library":"ntdll.dll","dynamic_symbol":"NtCreateFile"})
    };
    bad.push(wrong_target);
    for (index, value) in bad.into_iter().enumerate() {
        let mut m = manifest.clone();
        let mut r = runtime.clone();
        m["file_connection"] = value.clone();
        r["file_connection"] = value;
        save(&built, "manifest.json", &m);
        save(&built, "runtime.json", &r);
        let out = scratch.0.join(format!("invalid-{index}"));
        let result = output(&mut release(&built, &out), &scratch.0);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("file_connection"));
        assert!(!out.exists());
    }
    for index in 0..3 {
        let mut m = manifest.clone();
        let mut r = runtime.clone();
        match index {
            0 => {
                m.as_object_mut().unwrap().remove("file_connection");
            }
            1 => {
                r.as_object_mut().unwrap().remove("file_connection");
            }
            _ => {
                r["file_connection"]["enabled"] = json!(false);
            }
        }
        save(&built, "manifest.json", &m);
        save(&built, "runtime.json", &r);
        let out = scratch.0.join(format!("inconsistent-{index}"));
        let result = output(&mut release(&built, &out), &scratch.0);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("inconsistent file_connection"));
        assert!(!out.exists());
    }
}
