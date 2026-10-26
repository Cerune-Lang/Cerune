use super::{ROUTES, directory, process};
use std::{fs, process::Command, time::Duration};
fn run(command: &mut Command, dir: &std::path::Path) -> std::process::Output {
    process::bounded_output(command, dir, "call-cli", Duration::from_secs(30)).unwrap()
}
#[test]
fn call_depth_cli_preserves_input_arguments_evaluation_order_and_frames() {
    let dir = directory("call-depth");
    let path = dir.join("source.ceru");
    let source = "import \"std:io\" as io; import \"std:env\" as env; fn leaf(x:i64)->void{print(\"leaf\");} fn mark()->i64{print(\"argument\");return 1;} fn outer()->void{leaf(mark());print(\"after\");} print(env::arg_count()); print(io::read_line(16)); outer();";
    fs::write(&path, source).unwrap();
    let stdin_file = dir.join("stdin.txt");
    fs::write(&stdin_file, b"hello\r\n").unwrap();
    for &(route, ssa) in ROUTES {
        for limit in [None, Some(1), Some(2), Some(128)] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command
                .arg(route)
                .arg(&path)
                .args(["--diagnostic-format", "runtime-v1"]);
            if ssa {
                command.arg("--ssa");
            }
            if let Some(n) = limit {
                command.args(["--call-depth-limit", &n.to_string()]);
            }
            command.args(["--", "--call-depth-limit", "0"]);
            command.stdin(fs::File::open(&stdin_file).unwrap());
            let out = run(&mut command, &dir);
            let stdout = String::from_utf8(out.stdout).unwrap();
            let stderr = String::from_utf8(out.stderr).unwrap().replace("\r\n", "\n");
            if limit == Some(1) {
                assert_eq!(out.status.code(), Some(1), "{route} {stderr}");
                assert!(
                    !stdout.contains("leaf") && !stdout.contains("after"),
                    "{stdout}"
                );
                // Argument call mark itself exceeds depth 1, before leaf can start.
                assert!(!stdout.contains("argument"));
                assert!(
                    stderr.starts_with("cerune: call-v1 reason=depth-limit limit=1"),
                    "{stderr}"
                );
                assert!(
                    stderr.contains("node=") && stderr.contains("bytes="),
                    "{stderr}"
                );
                assert!(stderr.contains("call-v1 frame=0"), "{stderr}");
            } else {
                assert!(out.status.success(), "{route} {stderr}");
                assert!(stderr.is_empty());
                assert!(stdout.starts_with("2\n"), "{stdout}");
                assert!(stdout.ends_with("argument\nleaf\nafter\n"), "{stdout}");
            }
        }
    }
}
#[test]
fn call_depth_cli_checks_default_128_and_explicit_endpoints() {
    let dir = directory("call-endpoints");
    let path = dir.join("chain.ceru");
    for depth in [128, 129] {
        let mut source = String::new();
        for n in 0..depth {
            source.push_str(&format!(
                "fn chain_{n}()->void{{ {} }}\n",
                if n + 1 == depth {
                    "print(42);".to_owned()
                } else {
                    format!("chain_{}();", n + 1)
                }
            ));
        }
        source.push_str("chain_0(); print(99);");
        fs::write(&path, source).unwrap();
        for &(route, ssa) in ROUTES {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command
                .arg(route)
                .arg(&path)
                .args(["--diagnostic-format", "runtime-v1"]);
            if ssa {
                command.arg("--ssa");
            }
            let out = run(&mut command, &dir);
            let err = String::from_utf8_lossy(&out.stderr);
            if depth == 128 {
                assert!(out.status.success(), "{route}: {err}");
                assert_eq!(out.stdout, b"42\n99\n");
            } else {
                assert_eq!(out.status.code(), Some(1));
                assert!(out.stdout.is_empty());
                assert!(err.contains("depth-limit limit=128"), "{err}");
                assert_eq!(err.matches("call-v1 frame=").count(), 128);
            }
        }
    }
}
#[test]
fn call_depth_cli_rejects_invalid_limits_before_writing() {
    let dir = directory("call-options");
    let path = dir.join("input.ceru");
    fs::write(&path, "print(42);").unwrap();
    let artifact = dir.join("keep.c");
    fs::write(&artifact, "keep").unwrap();
    for value in ["0", "129", "4294967295", "-1", "+1", "1.0", ""] {
        let out = run(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("emit-c")
                .arg(&path)
                .args(["--call-depth-limit", value, "-o"])
                .arg(&artifact),
            &dir,
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("from 1 to 128"));
        assert_eq!(fs::read_to_string(&artifact).unwrap(), "keep");
    }
    for options in [
        vec!["--call-depth-limit"],
        vec!["--call-depth-limit", "1", "--call-depth-limit", "2"],
    ] {
        let out = run(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("run")
                .arg(&path)
                .args(options),
            &dir,
        );
        assert_eq!(out.status.code(), Some(1));
    }
    for command in [
        "check",
        "fmt",
        "emit-ir",
        "emit-mir",
        "emit-bytecode",
        "emit-sources",
        "release",
    ] {
        let out = run(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(command)
                .arg(&path)
                .args(["--call-depth-limit", "1"]),
            &dir,
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("not supported"));
    }
    for command in ["emit-llvm", "emit-qbe"] {
        let out = run(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(command)
                .arg(&path),
            &dir,
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("explicit --target"));
    }
}
