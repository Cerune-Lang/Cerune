use super::Workspace;
#[cfg(any(windows, target_os = "linux"))]
use super::process;
use serde_json::{Value, json};
#[cfg(any(windows, target_os = "linux"))]
use std::{
    fs,
    process::{Command, Stdio},
    time::Duration,
};

const EXAMPLE: &str = include_str!("../../examples/process_execution/registered.ceru");
const ROUTES: &[(&str, &[&str])] = &[
    ("run", &[]),
    ("run-ir", &[]),
    ("run-mir", &[]),
    ("run-mir", &["--ssa"]),
    ("run-vm", &[]),
];
fn config(w: &Workspace) -> Value {
    json!({
        "schema":"cerune-process-v1", "platform":std::env::consts::OS,
        "linux_supervisor": if cfg!(target_os="linux") { Some(w.0.join("supervisor")) } else { None },
        "limits":{"arguments":32,"argument_bytes":16384,"stdin_bytes":1024,
            "output_bytes":1024,"timeout_ms":10000,"cleanup_timeout_ms":5000},
        "transfer_bytes":32768,
        "targets":[{"name":"tool","executable":w.0.join("missing-child"),
            "directory":w.0,"environment":[]}]
    })
}
fn reject(w: &Workspace, text: &str, expected: &str) {
    let entry = w.put("main.ceru", "print(999);");
    let config = w.put("config.json", text);
    let out = w.cli(
        "run",
        &entry,
        &["--process-config", config.to_str().unwrap()],
    );
    assert!(!out.status.success(), "{text}");
    assert!(out.stdout.is_empty());
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains(expected), "{error}");
    assert!(!error.contains("secret-env-value"), "{error}");
}
#[test]
fn rejects_invalid_configuration_before_execution() {
    let w = Workspace::new();
    reject(&w, "{", "invalid configuration");
    reject(&w, &" ".repeat(1024 * 1024 + 1), "exceeds 1 MiB");
    let entry = w.put("main.ceru", "print(999);");
    let invalid_utf8 = w.0.join("invalid-utf8.json");
    std::fs::write(&invalid_utf8, b"{\"schema\":\"\xff\"}").unwrap();
    let out = w.cli(
        "run",
        &entry,
        &["--process-config", invalid_utf8.to_str().unwrap()],
    );
    assert!(!out.status.success() && out.stdout.is_empty());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("invalid configuration")
    );
    let base = config(&w);
    reject(
        &w,
        &base.to_string().replace(
            "\"transfer_bytes\":32768",
            "\"transfer_bytes\":18446744073709551616",
        ),
        "invalid configuration",
    );
    for (pointer, value, expected) in [
        ("/schema", json!("future"), "unsupported schema"),
        (
            "/platform",
            json!("different-host"),
            "platform does not match",
        ),
        ("/limits/arguments", json!(4097), "argument limits"),
        ("/limits/argument_bytes", json!(16385), "argument limits"),
        ("/limits/timeout_ms", json!(0), "time limits"),
        ("/limits/cleanup_timeout_ms", json!(0), "time limits"),
        ("/transfer_bytes", json!(-1), "invalid configuration"),
        ("/limits/stdin_bytes", json!(1.5), "invalid configuration"),
        (
            "/targets/0/environment",
            json!([["KEY", "secret-env-value", "extra"]]),
            "invalid configuration",
        ),
    ] {
        let mut v = base.clone();
        *v.pointer_mut(pointer).unwrap() = value;
        reject(&w, &v.to_string(), expected);
    }
    for pointer in ["/schema", "/limits", "/targets"] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(&pointer[1..]);
        reject(&w, &v.to_string(), "invalid configuration");
    }
    let mut v = base.clone();
    v["unknown"] = json!("secret-env-value");
    reject(&w, &v.to_string(), "invalid configuration");
    let duplicated = base
        .to_string()
        .replacen('{', r#"{"schema":"cerune-process-v1","#, 1);
    reject(&w, &duplicated, "invalid configuration");
}
#[test]
fn flags_and_delimiter_remain_explicit() {
    let w = Workspace::new();
    let entry = w.put("main.ceru", "print(999);");
    for args in [
        vec!["--process-config"],
        vec!["--process-config", ""],
        vec!["--process-config", "--file-root"],
        vec!["--process-config", "one", "--process-config", "two"],
        vec!["--process-config", "does-not-exist.json"],
    ] {
        let out = w.cli("run", &entry, &args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    let entry = w.put(
        "arguments.ceru",
        r#"import "std:env" as e; print(e::arg(0)); print(e::arg(1));"#,
    );
    for (route, options) in ROUTES {
        let mut args = options.to_vec();
        args.extend(["--", "--process-config", "literal.json"]);
        let out = w.cli(route, &entry, &args);
        assert!(out.status.success(), "{:?}", out);
        assert_eq!(
            out.stdout,
            b"Value{text: \"--process-config\"}\nValue{text: \"literal.json\"}\n"
        );
    }
    let out = w.cli("emit-ir", &entry, &["--process-config", "config.json"]);
    assert!(!out.status.success());
}
#[test]
#[cfg(any(windows, target_os = "linux"))]
fn registration_validation_and_unregistered_results() {
    let w = Workspace::new();
    let base = config(&w);
    for (pointer, value) in [
        ("/targets/0/executable", json!("relative-tool")),
        ("/targets/0/directory", json!("relative-dir")),
        ("/targets/0/name", json!("")),
        ("/targets/0/name", json!("bad\u{0}name")),
        ("/targets/0/environment", json!([["A", "1"], ["A", "2"]])),
        (
            "/targets/0/environment",
            json!([["A=B", "secret-env-value"]]),
        ),
    ] {
        let mut v = base.clone();
        *v.pointer_mut(pointer).unwrap() = value;
        reject(&w, &v.to_string(), "registration");
    }
    let mut v = base.clone();
    v["targets"] = json!([v["targets"][0].clone(), v["targets"][0].clone()]);
    reject(&w, &v.to_string(), "Duplicate");
    if cfg!(target_os = "linux") {
        v = base.clone();
        v["linux_supervisor"] = Value::Null;
        reject(&w, &v.to_string(), "linux_supervisor is required");
        v["linux_supervisor"] = json!("relative");
        reject(&w, &v.to_string(), "supervisor");
    } else {
        v = base.clone();
        v["linux_supervisor"] = json!("/unused");
        reject(&w, &v.to_string(), "not accepted on Windows");
        v = base.clone();
        v["targets"][0]["environment"] = json!([["A", "1"], ["a", "2"]]);
        reject(&w, &v.to_string(), "Duplicate");
    }
    v = base;
    v["targets"] = json!([]);
    let text = v.to_string();
    let padded = format!("{}{}", text, " ".repeat(1024 * 1024 - text.len()));
    let cfg = w.put("empty.json", &padded);
    let entry = w.put("main.ceru", EXAMPLE);
    for (route, options) in ROUTES {
        let mut args = options.to_vec();
        args.extend(["--process-config", cfg.to_str().unwrap()]);
        let out = w.cli(route, &entry, &args);
        assert!(out.status.success(), "{out:?}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains("Unregistered{}"), "{text}");
        assert!(text.ends_with("continued\n"));
    }
}
#[test]
#[cfg(not(any(windows, target_os = "linux")))]
fn unsupported_host_is_explicit_and_missing_connection_still_works() {
    let w = Workspace::new();
    reject(&w, &config(&w).to_string(), "unsupported host platform");
    let entry = w.put("main.ceru", EXAMPLE);
    let out = w.cli("run", &entry, &[]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("Unconfigured{}")
    );
}
#[test]
#[cfg(any(windows, target_os = "linux"))]
#[cfg_attr(
    target_os = "linux",
    ignore = "requires CERUNE_TEST_PROCESS_SUPERVISOR; run explicitly in subprocess OS CI"
)]
fn os_connections_preserve_results_and_other_execution_resources() {
    let w = Workspace::new();
    let child_source = w.put("child.rs", include_str!("../support/process_child.rs"));
    let child = w.0.join(if cfg!(windows) { "child.exe" } else { "child" });
    let compiled = process::bounded_output(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(&child_source)
            .arg("-o")
            .arg(&child),
        &w.0,
        "compile child",
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    let mut cfg = config(&w);
    cfg["targets"][0]["executable"] = json!(child);
    cfg["targets"][0]["environment"] = json!([["CERUNE_CHILD_VALUE", "日本語\r\n"]]);
    if cfg!(target_os = "linux") {
        cfg["linux_supervisor"] = json!(
            std::env::var("CERUNE_TEST_PROCESS_SUPERVISOR")
                .expect("set the absolute path of the explicitly built supervisor")
        );
    }
    w.put("child-directory-marker", "registered cwd");
    let config_path = w.put("config.json", &cfg.to_string());
    let entry = w.put("example.ceru", EXAMPLE);
    for (route, options) in ROUTES {
        let mut args = options.to_vec();
        args.extend(["--process-config", config_path.to_str().unwrap()]);
        let out = process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(route)
                .arg(&entry)
                .args(&args)
                .env("CERUNE_PARENT_SENTINEL", "not inherited"),
            &w.0,
            "registered example",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(out.status.success(), "{route} {options:?}: {out:?}");
        assert_eq!(
            out.stdout,
            include_bytes!("../../examples/process_execution/registered.stdout")
        );
        assert!(out.stderr.is_empty(), "{out:?}");
    }
    // 起動後の資源とは別に、設定と要求の上限を検査します。
    for (label, change, expected) in [
        ("not-found", "path", "NotFound{}"),
        ("reservation", "transfer", "OutputBudget{}"),
        ("request", "request", "RequestLimit{}"),
        ("output", "output", "OutputLimit{}"),
        ("timeout", "timeout", "Timeout{}"),
    ] {
        let mut value = cfg.clone();
        let mut source = EXAMPLE.to_owned();
        match change {
            "path" => value["targets"][0]["executable"] = json!(w.0.join("missing-child")),
            "transfer" => value["transfer_bytes"] = json!(0),
            "request" => value["limits"]["stdin_bytes"] = json!(0),
            "output" => source = source.replace("stdout_limit: 32", "stdout_limit: 2"),
            "timeout" => {
                source = source
                    .replace(
                        r#"["echo", "", "日本語", "a\"b", "tail\\", "--"]"#,
                        r#"["sleep"]"#,
                    )
                    .replace("timeout_ms: 10000", "timeout_ms: 200")
            }
            _ => unreachable!(),
        }
        let entry = w.put("failure.ceru", &source);
        let config_path = w.put("failure.json", &value.to_string());
        for (route, options) in ROUTES {
            let mut args = options.to_vec();
            args.extend(["--process-config", config_path.to_str().unwrap()]);
            let out = w.cli(route, &entry, &args);
            assert!(out.status.success(), "{label}/{route}: {out:?}");
            let text = String::from_utf8(out.stdout).unwrap();
            assert!(text.contains(expected), "{label}/{route}: {text}");
            assert!(
                text.starts_with("before\n") && text.ends_with("continued\n"),
                "{text}"
            );
        }
    }
    let example_body = EXAMPLE
        .lines()
        .filter(|line| !line.starts_with("import "))
        .collect::<Vec<_>>()
        .join("\n");
    let combined = format!(
        r#"import "std:process" as types;
import "std:process/run" as process;
import "std:io" as io;
import "std:env" as env;
import "std:fs" as fs;
print(io::read_line(64));
print(env::arg(0));
print(fs::read_text("payload",64));
{example_body}
flush();
"#
    );
    w.put("payload", "file value");
    let stdin = w.put("stdin", "input\n");
    let entry = w.put("combined.ceru", &combined);
    for (route, options) in ROUTES {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
        cmd.arg(route).arg(&entry).args(*options).args([
            "--process-config",
            config_path.to_str().unwrap(),
            "--file-root",
            w.0.to_str().unwrap(),
            "--call-depth-limit",
            "8",
            "--",
            "argument",
        ]);
        let out = process::bounded_output_with_stdin(
            &mut cmd,
            &w.0,
            "combined resources",
            Duration::from_secs(30),
            Stdio::from(fs::File::open(&stdin).unwrap()),
        )
        .unwrap();
        assert!(out.status.success(), "{out:?}");
        let mut expected =
            b"Line{text: \"input\\n\"}\nValue{text: \"argument\"}\nfile value\n".to_vec();
        expected.extend(include_bytes!(
            "../../examples/process_execution/registered.stdout"
        ));
        assert_eq!(out.stdout, expected);
        assert!(out.stderr.is_empty(), "{out:?}");
    }
}
