use std::{
    fs,
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
#[path = "support/process.rs"]
mod process;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn directory(name: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("output-cli-{name}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}
const ROUTES: &[(&str, bool)] = &[
    ("run", false),
    ("run-ir", false),
    ("run-vm", false),
    ("run-mir", false),
    ("run-mir", true),
];

#[test]
fn output_arrives_while_program_is_still_running() {
    let dir = directory("live");
    let input = dir.join("live.ceru");
    fs::write(&input, "print(\"ready\"); while true {}").unwrap();
    for &(route, ssa) in ROUTES {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
        command.arg(route).arg(&input);
        if ssa {
            command.arg("--ssa");
        }
        let mut child = ChildGuard(
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut pipe = child.0.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut bytes = [0; 6];
            let result = pipe.read_exact(&mut bytes).map(|()| bytes);
            let _ = tx.send(result);
        });
        let result = rx.recv_timeout(Duration::from_secs(10));
        let running = child.0.try_wait().unwrap().is_none();
        drop(child);
        reader.join().unwrap();
        assert_eq!(
            result
                .expect("output must arrive before termination")
                .unwrap(),
            *b"ready\n",
            "{route} ssa={ssa}"
        );
        assert!(running, "{route} ssa={ssa}");
    }
}

#[test]
fn successful_and_failed_cli_runs_do_not_reprint_output() {
    let dir = directory("bytes");
    let failure = dir.join("failure.ceru");
    fs::write(&failure, "print(\"before\");print(1/0);print(\"after\");").unwrap();
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/io_protocol/output_sequence.ceru");
    for &(route, ssa) in ROUTES {
        for (path, ok, expected) in [
            (
                sample.as_path(),
                true,
                include_bytes!("../examples/io_protocol/output_sequence.stdout").as_slice(),
            ),
            (failure.as_path(), false, b"before\n".as_slice()),
        ] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command
                .arg(route)
                .arg(path)
                .args(["--diagnostic-format", "runtime-v1"]);
            if ssa {
                command.arg("--ssa");
            }
            let result =
                process::bounded_output(&mut command, &dir, route, Duration::from_secs(15))
                    .unwrap();
            assert_eq!(
                result.status.success(),
                ok,
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, expected, "{route} ssa={ssa}");
            if ok {
                assert!(result.stderr.is_empty());
            } else {
                assert!(
                    String::from_utf8_lossy(&result.stderr)
                        .contains("runtime-v1 code=division-by-zero")
                );
            }
        }
    }
}

#[test]
fn closed_stdout_stops_cleanly_instead_of_panicking_or_looping() {
    let dir = directory("closed");
    let input = dir.join("closed.ceru");
    fs::write(&input, "while true { print(\"output\"); }").unwrap();
    for machine in [false, true] {
        for (index, &(route, ssa)) in ROUTES.iter().enumerate() {
            let stderr = dir.join(format!("stderr-{index}.txt"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_cerune"));
            command.arg(route).arg(&input);
            if ssa {
                command.arg("--ssa");
            }
            if machine {
                command.args(["--diagnostic-format", "runtime-v1"]);
            }
            let mut child = ChildGuard(
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
                    .spawn()
                    .unwrap(),
            );
            drop(child.0.stdout.take());
            let start = std::time::Instant::now();
            let status = loop {
                if let Some(status) = child.0.try_wait().unwrap() {
                    break Some(status);
                }
                if start.elapsed() > Duration::from_secs(10) {
                    break None;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            drop(child);
            assert_eq!(
                status.expect("closed output must stop execution").code(),
                Some(1),
                "{route}"
            );
            let error = fs::read_to_string(&stderr).unwrap();
            assert!(
                error.contains(if machine {
                    "io-v1 operation=write requested="
                } else {
                    "output Write failed"
                }),
                "{route}: {error}"
            );
            assert!(!error.contains("panicked"), "{route}: {error}");
        }
    }
}

#[path = "call_depth/cli.rs"]
mod call_depth_cli;
