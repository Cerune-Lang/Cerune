use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(route: &str, source: &Path, trace: Option<&Path>, extra: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg(route).arg(source);
    if let Some(path) = trace {
        cmd.arg("--trace").arg(path);
    }
    cmd.args(extra).output().unwrap()
}
fn rows(path: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap();
    assert!(raw.ends_with('\n'));
    raw.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn same_run(actual: &Output, expected: &Output) {
    assert_eq!(actual.status, expected.status);
    assert_eq!(actual.stdout, expected.stdout);
    assert_eq!(actual.stderr, expected.stderr);
}
fn fresh(name: &str) -> std::path::PathBuf {
    let dir = super::directory(name).join(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string(),
    );
    fs::create_dir(&dir).unwrap();
    dir
}

#[test]
fn aliases_and_exact_jsonl_limits() {
    let dir = fresh("trace-bounds");
    let src = Path::new("examples/recursion/factorial.ceru");
    let normal = run("run", src, None, &[]);
    assert_eq!(normal.stdout, b"120\n1\n");
    let path = dir.join("full");
    same_run(&run("run", src, Some(&path), &[]), &normal);
    let bytes = fs::read(&path).unwrap();
    let full = rows(&path);
    assert_eq!(full.len(), 19);
    for (i, event) in full[..18].iter().enumerate() {
        assert_eq!(event["seq"], i + 1);
        assert_eq!(event["schema"], "trace-v1");
    }
    assert_eq!(full[0]["function"], "factorial");
    assert_eq!(full[1]["taken"], false);
    assert_eq!(full[18]["status"], "complete");
    let exact = dir.join("exact");
    same_run(
        &run(
            "run-ir",
            src,
            Some(&exact),
            &[
                "--trace-byte-limit",
                &bytes.len().to_string(),
                "--trace-event-limit",
                "18",
            ],
        ),
        &normal,
    );
    assert_eq!(fs::read(exact).unwrap(), bytes);
    for (name, opts, count, reason) in [
        (
            "short",
            vec![
                "--trace-byte-limit".to_owned(),
                (bytes.len() - 1).to_string(),
            ],
            17,
            "byte-limit",
        ),
        (
            "small",
            vec!["--trace-byte-limit".into(), "128".into()],
            0,
            "byte-limit",
        ),
        (
            "zero",
            vec!["--trace-event-limit".into(), "0".into()],
            0,
            "event-limit",
        ),
        (
            "prefix",
            vec!["--trace-event-limit".into(), "2".into()],
            2,
            "event-limit",
        ),
    ] {
        let path = dir.join(name);
        same_run(
            &run(
                "run",
                src,
                Some(&path),
                &opts.iter().map(String::as_str).collect::<Vec<_>>(),
            ),
            &normal,
        );
        let saved = rows(&path);
        assert_eq!(saved.len(), count + 1);
        assert_eq!(saved[..count], full[..count]);
        assert_eq!(saved[count]["event"], "end");
        assert_eq!(saved[count]["status"], reason);
        if opts[0] == "--trace-byte-limit" {
            assert!(fs::metadata(path).unwrap().len() <= opts[1].parse::<u64>().unwrap());
        }
    }
}

#[test]
fn stops_and_save_errors_preserve_execution_diagnostics() {
    let dir = fresh("trace-stop");
    let src = dir.join("bad.ceru");
    fs::write(
        &src,
        "// 日本語\nfn crash()->i64 {print(7);return 1 / 0;} print(crash());",
    )
    .unwrap();
    let flags = ["--diagnostic-format", "runtime-v1"];
    let normal = run("run", &src, None, &flags);
    assert!(!normal.status.success());
    let path = dir.join("stop");
    same_run(&run("run", &src, Some(&path), &flags), &normal);
    let saved = rows(&path);
    assert_eq!(saved.len(), 3);
    assert_eq!(saved[1]["event"], "stop");
    assert_eq!(saved[1]["code"], "division-by-zero");
    let range =
        saved[1]["start"].as_u64().unwrap() as usize..saved[1]["end"].as_u64().unwrap() as usize;
    assert_eq!(&fs::read_to_string(&src).unwrap()[range], "1 / 0");
    assert_eq!(saved[2]["status"], "complete");

    let before = fs::read(&path).unwrap();
    let failed = run("run", &src, Some(&path), &flags);
    assert_eq!(failed.status, normal.status);
    assert_eq!(failed.stdout, normal.stdout);
    assert!(failed.stderr.starts_with(&normal.stderr));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("trace-v1: could not save"));
    assert_eq!(fs::read(path).unwrap(), before);
    let bounded = dir.join("depth");
    let flags = [
        "--call-depth-limit",
        "1",
        "--diagnostic-format",
        "runtime-v1",
    ];
    let src = Path::new("examples/recursion/factorial.ceru");
    let normal = run("run", src, None, &flags);
    assert!(String::from_utf8_lossy(&normal.stderr).contains("call-v1"));
    same_run(&run("run", src, Some(&bounded), &flags), &normal);
}

#[test]
fn files_and_program_arguments_keep_their_meaning() {
    let dir = fresh("trace-resources");
    let path = dir.join("sample.txt");
    let out = run(
        "run-ir",
        Path::new("examples/files/roundtrip.ceru"),
        Some(&path),
        &["--file-root", dir.to_str().unwrap()],
    );
    assert!(out.status.success());
    assert!(out.stdout.starts_with(b"true\n"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("trace-v1: could not save"));
    assert_eq!(fs::read(path).unwrap(), "日本語\0\r\né\n".as_bytes());
    let src = Path::new("examples/arguments/list.ceru");
    let args = [
        "--",
        "--trace",
        "private-example-argument",
        "--trace-event-limit",
    ];
    let normal = run("run", src, None, &args);
    let path = dir.join("args");
    same_run(&run("run", src, Some(&path), &args), &normal);
    assert!(String::from_utf8_lossy(&normal.stdout).contains("private-example-argument"));
    assert!(
        !fs::read_to_string(path)
            .unwrap()
            .contains("private-example-argument")
    );
}

#[test]
fn malformed_options_and_other_routes_do_not_run_the_program() {
    let dir = fresh("trace-options");
    let path = dir.join("output");
    let path_arg = path.to_str().unwrap();
    let src = Path::new("examples/recursion/factorial.ceru");
    for (route, flags) in [
        ("run", vec!["--trace"]),
        ("run", vec!["--trace", path_arg, "--trace", path_arg]),
        (
            "run",
            vec!["--trace", path_arg, "--trace-event-limit", "-1"],
        ),
        ("run", vec!["--trace", path_arg, "--trace-byte-limit", "64"]),
        ("run", vec!["--trace-byte-limit", "128"]),
        (
            "run",
            vec![
                "--trace",
                path_arg,
                "--trace-event-limit",
                "2",
                "--trace-event-limit",
                "1",
            ],
        ),
        ("run", vec!["--trace", "--call-depth-limit", "1", path_arg]),
        ("run-vm", vec!["--trace", path_arg]),
        ("run-mir", vec!["--trace", path_arg]),
    ] {
        let out = run(route, src, None, &flags);
        assert!(!out.status.success(), "{route} {flags:?}");
        assert!(out.stdout.is_empty());
        assert!(!path.exists());
    }
}

#[test]
fn piped_utf8_input_is_consumed_once_and_stays_out_of_the_trace() {
    use std::{io::Write, process::Stdio};
    let dir = fresh("trace-stdin");
    let path = dir.join("lines");
    let invoke = |trace: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
        cmd.args(["run", "examples/stdin/lines.ceru"]);
        if trace {
            cmd.arg("--trace").arg(&path);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all("private-input-日本語\n".as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    };
    let normal = invoke(false);
    assert!(normal.status.success());
    assert!(String::from_utf8_lossy(&normal.stdout).contains("private-input-日本語"));
    same_run(&invoke(true), &normal);
    assert!(!fs::read_to_string(&path).unwrap().contains("private-input"));
    assert_eq!(rows(&path).last().unwrap()["status"], "complete");
}
