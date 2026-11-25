use super::generated::Route;
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
fn emit_wat(src: &Path, wat: &Path, trace: bool, extra: &[&str]) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg("emit-wat").arg(src).args(extra).arg("-o").arg(wat);
    if trace {
        cmd.arg("--with-trace");
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
fn executors_match_hir_events_output_and_stops() {
    let dir = fresh("trace-routes");
    let branch = fs::read_to_string("experiments/lean/branch.ceru").unwrap();
    let choose = |name: &str, calls: &str| {
        let path = dir.join(name);
        fs::write(&path, format!("{branch}\n{calls}")).unwrap();
        path
    };
    let calls = (0..=253)
        .map(|i| format!("print(choose({i}));\n"))
        .collect::<String>();
    let values = choose("branch-values.ceru", &calls);
    let overflow = choose("branch-254.ceru", "print(choose(254));");
    let overflow_max = choose("branch-255.ceru", "print(choose(255));");
    let bounds = dir.join("bounds.ceru");
    fs::write(
        &bounds,
        "fn pair()->[i64; 2] { return [4,9]; } a:[i64; 2]=pair(); print(a[1]); print(a[2]);",
    )
    .unwrap();
    let implicit = dir.join("main.ceru");
    fs::write(&implicit, "fn main()->void { if true { print(7); } }").unwrap();
    let factorial = Path::new("examples/recursion/factorial.ceru");
    for (i, (src, extra, success)) in [
        (factorial, &[][..], true),
        (factorial, &["--trace-event-limit", "2"][..], true),
        (factorial, &["--trace-byte-limit", "128"][..], true),
        (factorial, &["--call-depth-limit", "3"][..], false),
        (implicit.as_path(), &[][..], true),
        (bounds.as_path(), &[][..], false),
        (values.as_path(), &[][..], true),
        (overflow_max.as_path(), &[][..], false),
        (overflow.as_path(), &[][..], false),
        (Path::new("examples/loop_control.ceru"), &[][..], true),
        (Path::new("examples/short_circuit.ceru"), &[][..], true),
        (
            Path::new("examples/recursion/owned_generic.ceru"),
            &[][..],
            true,
        ),
        (Path::new("examples/modules/failure.ceru"), &[][..], false),
    ]
    .into_iter()
    .enumerate()
    {
        let flags = [&["--diagnostic-format", "runtime-v1"][..], extra].concat();
        let hir_path = dir.join(format!("{i}-hir"));
        let hir = run("run", src, Some(&hir_path), &flags);
        assert_eq!(hir.status.success(), success, "{src:?}");
        let expected = fs::read(hir_path).unwrap();
        let depth_flag = extra.starts_with(&["--call-depth-limit"]);
        for route in [
            Route::C,
            Route::Llvm,
            Route::Qbe,
            Route::Native,
            Route::NativeSsa,
        ] {
            let Some(exe) = route.compile(
                &dir.join(format!("{i}-{route:?}-build")),
                src,
                if depth_flag { extra } else { &[] },
            ) else {
                continue;
            };
            let path = dir.join(format!("{i}-{route:?}"));
            let mut args = vec!["--cerune-runtime-v1", "--trace", path.to_str().unwrap()];
            if !depth_flag {
                args.extend_from_slice(extra);
            }
            args.push("--");
            let normal = super::generated::run(&exe, &[]);
            let actual = super::generated::run(&exe, &args);
            same_run(&actual, &normal);
            if matches!(route, Route::Native | Route::NativeSsa) {
                let plain = route
                    .compile_mode(
                        &dir.join(format!("{i}-{route:?}-plain")),
                        src,
                        if depth_flag { extra } else { &[] },
                        false,
                    )
                    .unwrap();
                same_run(&actual, &super::generated::run(&plain, &[]));
            }
            assert_eq!(actual.status.success(), success);
            assert_eq!(actual.stdout, hir.stdout);
            let diagnostic = |r: &Output| String::from_utf8_lossy(&r.stderr).replace("\r\n", "\n");
            assert_eq!(
                diagnostic(&actual).trim_start_matches("cerune: "),
                diagnostic(&hir).trim_start_matches("cerune: ")
            );
            assert_eq!(fs::read(path).unwrap(), expected, "{src:?} {route:?}");
        }
        if let Some(host) = std::env::var_os("CERUNE_TEST_WAT_HOST") {
            let wat = dir.join(format!("{i}.wat"));
            emit_wat(src, &wat, true, if depth_flag { extra } else { &[] });
            let path = dir.join(format!("{i}-wat"));
            let invoke = |args: &[&str]| {
                Command::new(&host)
                    .arg(&wat)
                    .args(args)
                    .arg("--")
                    .output()
                    .unwrap()
            };
            let mut options = vec!["--trace", path.to_str().unwrap()];
            if !depth_flag {
                options.extend_from_slice(extra);
            }
            let actual = invoke(&options);
            same_run(&actual, &invoke(&[]));
            assert_eq!(
                actual.status.success(),
                success,
                "{src:?}: {}",
                String::from_utf8_lossy(&actual.stderr)
            );
            assert_eq!(actual.stdout, hir.stdout);
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).trim_start_matches("cerune: "),
                String::from_utf8_lossy(&hir.stderr).trim_start_matches("cerune: ")
            );
            assert_eq!(fs::read(path).unwrap(), expected, "{src:?} WAT");
        }
        for (name, command, options) in [
            ("mir", "run-mir", &[][..]),
            ("ssa", "run-mir", &["--ssa"][..]),
            ("vm", "run-vm", &[][..]),
        ] {
            let route_flags = [flags.as_slice(), options].concat();
            let path = dir.join(format!("{i}-{name}"));
            same_run(&run(command, src, Some(&path), &route_flags), &hir);
            assert_eq!(fs::read(path).unwrap(), expected, "{src:?} {name}");
        }
    }
}

#[test]
fn native_trace_preserves_float_results_and_stack_arguments() {
    let dir = fresh("native-trace-abi");
    let floats = dir.join("returns.ceru");
    fs::write(&floats,"fn single(x:f32)->f32{return x;} fn double(x:f64)->f64{return x;} print(single(1.25)); print(double(-2.75));").unwrap();
    for src in [
        floats.as_path(),
        Path::new("examples/function_arguments.ceru"),
    ] {
        for route in [Route::Native, Route::NativeSsa] {
            let case = dir.join(format!(
                "{}-{route:?}",
                src.file_stem().unwrap().to_str().unwrap()
            ));
            let Some(exe) = route.compile(&case, src, &[]) else {
                return;
            };
            let hir = case.join("hir.jsonl");
            let path = case.join("native.jsonl");
            let reference = run("run", src, Some(&hir), &[]);
            assert!(reference.status.success());
            let actual = super::generated::run(
                &exe,
                &[
                    "--cerune-runtime-v1",
                    "--trace",
                    path.to_str().unwrap(),
                    "--",
                ],
            );
            same_run(&actual, &reference);
            assert_eq!(fs::read(hir).unwrap(), fs::read(path).unwrap());
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
    for route in ["run-ir", "run-vm"] {
        let root = dir.join(route);
        fs::create_dir(&root).unwrap();
        let path = root.join("sample.txt");
        let out = run(
            route,
            Path::new("examples/files/roundtrip.ceru"),
            Some(&path),
            &["--file-root", root.to_str().unwrap()],
        );
        assert!(out.status.success());
        assert!(out.stdout.starts_with(b"true\n"));
        assert!(String::from_utf8_lossy(&out.stderr).contains("trace-v1: could not save"));
        assert_eq!(fs::read(path).unwrap(), "日本語\0\r\né\n".as_bytes());
    }
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
fn generated_routes_keep_arguments_trace_limits_and_file_creation_order() {
    for route in [
        Route::C,
        Route::Llvm,
        Route::Qbe,
        Route::Native,
        Route::NativeSsa,
    ] {
        check_generated_files(route);
    }
}
fn check_generated_files(route: Route) {
    let dir = fresh(&format!("trace-{route:?}-arguments-files"));
    let src = Path::new("examples/arguments/list.ceru");
    let Some(exe) = route.compile(&dir.join("arguments"), src, &[]) else {
        return;
    };
    let data = ["--cerune-runtime-v1", "--trace", "private-example-argument"];
    let hir_trace = dir.join("hir");
    let reference = run("run", src, Some(&hir_trace), &[&["--"][..], &data].concat());
    let expected = fs::read(hir_trace).unwrap();
    for (name, limit, want_full) in [
        ("exact", expected.len(), true),
        ("one-short", expected.len() - 1, false),
    ] {
        let path = dir.join(name);
        let flags = [
            "--cerune-runtime-v1",
            "--trace-byte-limit",
            &limit.to_string(),
            "--trace",
            path.to_str().unwrap(),
            "--",
        ];
        let actual = super::generated::run(&exe, &[&flags[..], &data].concat());
        same_run(&actual, &reference);
        let trace = fs::read(&path).unwrap();
        if want_full {
            assert_eq!(trace, expected);
        } else {
            assert!(trace.len() <= limit);
            assert_eq!(rows(&path).last().unwrap()["status"], "byte-limit");
        }
        assert!(
            !String::from_utf8(trace)
                .unwrap()
                .contains("private-example-argument")
        );
    }
    for bad in [
        vec!["--trace-event-limit", "1", "--"],
        vec!["--trace", "never", "--trace-byte-limit", "64", "--"],
        vec!["--trace", "never", "--trace", "other", "--"],
        vec!["--trace", "never"],
    ] {
        let actual = super::generated::run(&exe, &[&["--cerune-runtime-v1"][..], &bad].concat());
        assert!(!actual.status.success());
        assert!(actual.stdout.is_empty());
        assert!(String::from_utf8_lossy(&actual.stderr).contains("trace-v1: invalid"));
    }
    let root = dir.join("files");
    let exe = route
        .compile(&root, Path::new("examples/files/roundtrip.ceru"), &[])
        .unwrap();
    let path = root.join("sample.txt");
    let actual = super::generated::run(
        &exe,
        &[
            "--cerune-runtime-v1",
            "--file-root",
            root.to_str().unwrap(),
            "--trace",
            path.to_str().unwrap(),
            "--",
        ],
    );
    assert!(actual.status.success());
    assert!(actual.stdout.starts_with(b"true\n"));
    assert!(String::from_utf8_lossy(&actual.stderr).contains("trace-v1: could not save"));
    assert_eq!(fs::read(path).unwrap(), "日本語\0\r\né\n".as_bytes());
}

#[test]
fn wat_trace_keeps_program_arguments_and_saves_after_file_operations() {
    let Some(host) = std::env::var_os("CERUNE_TEST_WAT_HOST") else {
        return;
    };
    let dir = fresh("wat-trace-host");
    let module = dir.join("example.wat");
    let invoke = |args: &[&str]| {
        Command::new(&host)
            .arg(&module)
            .args(args)
            .output()
            .unwrap()
    };
    let source = Path::new("examples/arguments/list.ceru");
    let expected = dir.join("hir");
    let reference = run(
        "run",
        source,
        Some(&expected),
        &["--", "--trace", "private-日本語"],
    );
    let bytes = fs::read(expected).unwrap();
    let trace = dir.join("trace.jsonl");
    let dest = trace.to_str().unwrap();
    emit_wat(source, &module, false, &[]);
    let plain = invoke(&["--", "--trace", "private-日本語"]);
    same_run(&plain, &reference);
    let unsupported = invoke(&["--trace", dest, "--"]);
    assert!(!unsupported.status.success() && unsupported.stdout.is_empty() && !trace.exists());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("--with-trace"));
    emit_wat(source, &module, true, &[]);
    let input = fs::read_to_string(&module).unwrap();
    fs::write(
        &module,
        input.replace("(@custom \"cerune.trace.v1\"", "(@custom \"removed\""),
    )
    .unwrap();
    let stripped = invoke(&["--trace", dest, "--"]);
    assert!(!stripped.status.success() && stripped.stdout.is_empty() && !trace.exists());
    assert!(String::from_utf8_lossy(&stripped.stderr).contains("site table"));
    fs::write(&module, input).unwrap();
    for n in [bytes.len(), bytes.len() - 1] {
        let actual = invoke(&[
            "--trace-byte-limit",
            &n.to_string(),
            "--trace",
            dest,
            "--",
            "--trace",
            "private-日本語",
        ]);
        same_run(&actual, &plain);
        if n == bytes.len() {
            assert_eq!(fs::read(&trace).unwrap(), bytes);
        } else {
            assert_eq!(rows(&trace).last().unwrap()["status"], "byte-limit");
        }
        assert!(
            !fs::read_to_string(&trace)
                .unwrap()
                .contains("private-日本語")
        );
        fs::remove_file(&trace).unwrap();
    }
    emit_wat(
        Path::new("examples/files/roundtrip.ceru"),
        &module,
        true,
        &[],
    );
    let file = dir.join("sample.txt");
    let actual = invoke(&[
        "--trace",
        file.to_str().unwrap(),
        "--file-root",
        dir.to_str().unwrap(),
        "--",
    ]);
    assert!(actual.status.success());
    assert!(actual.stdout.starts_with(b"true\n"));
    assert!(String::from_utf8_lossy(&actual.stderr).contains("trace-v1: could not save"));
    assert_eq!(fs::read(file).unwrap(), "日本語\0\r\né\n".as_bytes());
}

#[test]
fn wat_binary_preserves_sites_and_requires_an_observer_import() {
    let (Some(host), Some(wabt)) = (
        std::env::var_os("CERUNE_TEST_WAT_HOST"),
        std::env::var_os("CERUNE_TEST_WAT2WASM_JS"),
    ) else {
        return;
    };
    let node = std::env::var_os("CERUNE_TEST_NODE").unwrap_or_else(|| "node".into());
    let dir = fresh("wat-trace-binary");
    let source = Path::new("examples/recursion/factorial.ceru");
    let wat = dir.join("example.wat");
    let wasm = dir.join("example.wasm");
    emit_wat(source, &wat, true, &[]);
    let compile = Command::new(&node)
        .arg(wabt)
        .arg("--enable-annotations")
        .arg(&wat)
        .arg("-o")
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let before = dir.join("reference");
    let after = dir.join("binary");
    let hir = run("run", source, Some(&before), &[]);
    let actual = Command::new(host)
        .arg(&wasm)
        .arg("--trace")
        .arg(&after)
        .arg("--")
        .output()
        .unwrap();
    same_run(&actual, &hir);
    assert_eq!(fs::read(before).unwrap(), fs::read(after).unwrap());
    let js = r#"
const assert = require('node:assert/strict');
const m = new WebAssembly.Module(require('node:fs').readFileSync(process.argv[1]));
assert.deepEqual(WebAssembly.Module.exports(m).map(e => e.name), ['main']);
assert.equal(WebAssembly.Module.customSections(m, 'cerune.trace.v1').length, 1);
const cerune = {};
for (const i of WebAssembly.Module.imports(m)) {
    if (i.name !== 'trace_event_v1') cerune[i.name] = () => 0;
}
assert.throws(() => new WebAssembly.Instance(m, {cerune}), /trace_event_v1/);
"#;
    let compatibility = Command::new(node)
        .arg("-e")
        .arg(js)
        .arg(wasm)
        .output()
        .unwrap();
    assert!(
        compatibility.status.success(),
        "{}",
        String::from_utf8_lossy(&compatibility.stderr)
    );
}

#[test]
fn malformed_trace_options_do_not_run_the_program() {
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
        (
            "run-vm",
            vec!["--trace", path_arg, "--trace-byte-limit", "0"],
        ),
        ("run-mir", vec!["--ssa", "--trace-event-limit", "1"]),
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
