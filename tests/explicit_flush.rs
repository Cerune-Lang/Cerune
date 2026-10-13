use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
#[path = "support/process.rs"]
mod process;
fn run(cmd: &mut Command, dir: &Path, label: &str) -> std::process::Output {
    process::bounded_output(cmd, dir, label, Duration::from_secs(30)).unwrap()
}
fn ok(cmd: &mut Command, dir: &Path, label: &str) -> Vec<u8> {
    let result = run(cmd, dir, label);
    assert!(
        result.status.success(),
        "{label}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}
fn tool(var: &str, default: &str) -> Option<std::ffi::OsString> {
    let configured = std::env::var_os(var);
    let name = configured.clone().unwrap_or_else(|| default.into());
    if Command::new(&name)
        .arg(if var == "CERUNE_TEST_QBE" {
            "-h"
        } else {
            "--version"
        })
        .output()
        .is_ok_and(|o| o.status.success())
    {
        Some(name)
    } else {
        assert!(configured.is_none(), "{var} unavailable");
        eprintln!("flush route tool unavailable: {name:?}");
        None
    }
}
// Replace only the external fflush symbol, keeping COFF/ELF string lengths.
// The shim forwards accepted output, then reports failure; this is a controlled
// runtime connection, not a claim that an actual OS failed in this way.
fn inject_failure(bytes: &mut [u8]) {
    let mut count = 0;
    for i in 0..bytes.len().saturating_sub(5) {
        if &bytes[i..i + 6] == b"fflush" {
            bytes[i] = b'c';
            count += 1;
        }
    }
    assert!(count > 0, "artifact must retain a flush call");
}
#[test]
fn flush_example_and_failure_match_generated_routes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root
        .join("target")
        .join(format!("flush-routes-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let clang = tool("CERUNE_TEST_CC", "clang");
    let llvm = tool("CERUNE_TEST_LLVM_CLANG", "clang");
    let qbe = tool("CERUNE_TEST_QBE", "qbe");
    let node = tool("CERUNE_TEST_NODE", "node");
    let wabt = std::env::var_os("CERUNE_TEST_WAT2WASM_JS")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/wasm-tools/node_modules/wabt/bin/wat2wasm"));
    let target = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let shim = dir.join("failure.c");
    fs::write(&shim, "#include <stdio.h>\nint cflush(FILE *s) { int result = fflush(s); return s == stdout ? EOF : result; }\n").unwrap();
    let failure_source = "print(\"before\"); flush(); print(\"after\");";
    let hir = cerune_lang::compile_to_ir(failure_source).unwrap();
    let statement = &hir.statements[1];
    let record = format!(
        "cerune: io-v1 operation=flush requested=0 accepted=0 node={} bytes={}..{}\n",
        statement.id.0,
        statement.span.start(),
        statement.span.end()
    );
    fs::write(
        dir.join("helper.ceru"),
        format!("pub fn send() -> void {{ {failure_source} }}"),
    )
    .unwrap();
    for mode in ["normal", "failure", "module", "live"] {
        let failure = matches!(mode, "failure" | "module");
        let live = mode == "live";
        let source = dir.join("source.ceru");
        fs::write(
            &source,
            if live {
                "print(\"ready\"); flush(); while true {}"
            } else if mode == "module" {
                "import \"helper.ceru\" as helper; helper::send();"
            } else if failure {
                failure_source
            } else {
                include_str!("../examples/io_protocol/explicit_flush.ceru")
            },
        )
        .unwrap();
        let record = if mode == "module" {
            let program = cerune_lang::modules::load(&source)
                .unwrap()
                .to_ir()
                .unwrap();
            let statement = program
                .function_definitions
                .iter()
                .flat_map(|f| &f.body)
                .find(|s| matches!(s.kind, cerune_lang::ir::StatementKind::Flush))
                .unwrap();
            assert!(!statement.span.source_id().record_field().is_empty());
            format!(
                "cerune: io-v1 operation=flush requested=0 accepted=0 node={}{} bytes={}..{}\n",
                statement.id.0,
                statement.span.source_id().record_field(),
                statement.span.start(),
                statement.span.end()
            )
        } else {
            record.clone()
        };
        for (route, ssa) in [
            ("c", false),
            ("llvm", false),
            ("qbe", false),
            ("asm", false),
            ("obj", false),
            ("asm", true),
            ("obj", true),
            ("wat", false),
        ] {
            if route == "wat" {
                if live {
                    continue;
                }
                if node.is_none() || !wabt.is_file() {
                    eprintln!("flush WAT skipped");
                    continue;
                }
            } else if clang.is_none()
                || (route == "qbe" && qbe.is_none())
                || (route == "llvm" && llvm.is_none())
            {
                continue;
            }
            let ext = match route {
                "llvm" => "ll",
                "qbe" => "ssa",
                "asm" => "s",
                "obj" => "o",
                other => other,
            };
            let artifact = dir.join(format!("program.{ext}"));
            let mut emit = Command::new(env!("CARGO_BIN_EXE_cerune"));
            emit.arg(format!("emit-{route}"))
                .arg(&source)
                .arg("-o")
                .arg(&artifact);
            if matches!(route, "llvm" | "qbe" | "asm" | "obj") {
                emit.args(["--target", target]);
            }
            if ssa {
                emit.arg("--ssa");
            }
            ok(&mut emit, &dir, "emit");
            let actual = if route == "wat" {
                let wasm = dir.join("program.wasm");
                ok(
                    Command::new(node.as_ref().unwrap())
                        .arg(&wabt)
                        .arg(&artifact)
                        .arg("-o")
                        .arg(&wasm),
                    &dir,
                    "wat-build",
                );
                let host = dir.join("host.cjs");
                let mut host_source =
                    fs::read_to_string(root.join("tests/support/run_wasm.cjs")).unwrap();
                if failure {
                    host_source =
                        host_source.replace("flush_stdout_v1: () => 0", "flush_stdout_v1: () => 1");
                }
                fs::write(&host, host_source).unwrap();
                run(
                    Command::new(node.as_ref().unwrap()).arg(&host).arg(&wasm),
                    &dir,
                    "wat-run",
                )
            } else {
                if failure {
                    let mut bytes = fs::read(&artifact).unwrap();
                    inject_failure(&mut bytes);
                    if route == "c" {
                        bytes.splice(
                            0..0,
                            b"#include <stdio.h>\nint cflush(FILE *);\n".iter().copied(),
                        );
                    }
                    fs::write(&artifact, bytes).unwrap();
                }
                let input = if route == "qbe" {
                    let asm = dir.join("qbe.s");
                    ok(
                        Command::new(qbe.as_ref().unwrap())
                            .args([
                                "-t",
                                if cfg!(windows) {
                                    "amd64_win"
                                } else {
                                    "amd64_sysv"
                                },
                            ])
                            .arg("-o")
                            .arg(&asm)
                            .arg(&artifact),
                        &dir,
                        "qbe",
                    );
                    asm
                } else {
                    artifact
                };
                let exe = dir.join(if cfg!(windows) {
                    "program.exe"
                } else {
                    "program"
                });
                let mut link = Command::new(if route == "llvm" {
                    llvm.as_ref().unwrap()
                } else {
                    clang.as_ref().unwrap()
                });
                link.arg(&input);
                if failure {
                    link.arg(&shim);
                }
                link.arg("-o").arg(&exe);
                if cfg!(unix) {
                    link.arg("-lm");
                }
                ok(&mut link, &dir, "link");
                if live {
                    output_arrives_before_exit(&exe, route);
                    continue;
                }
                #[cfg(unix)]
                if !failure {
                    closed_socket_flush_has_a_record(&exe, &dir, route);
                }
                run(&mut Command::new(&exe), &dir, "execute")
            };
            assert_eq!(
                actual.status.success(),
                !failure,
                "{route} ssa={ssa}: {}",
                String::from_utf8_lossy(&actual.stderr)
            );
            let expected = if failure {
                b"before\n".as_slice()
            } else {
                include_bytes!("../examples/io_protocol/explicit_flush.stdout").as_slice()
            };
            assert_eq!(actual.stdout, expected, "{route} ssa={ssa}");
            assert_eq!(
                String::from_utf8(actual.stderr)
                    .unwrap()
                    .replace("\r\n", "\n"),
                if failure {
                    record.clone()
                } else {
                    String::new()
                },
                "{route} ssa={ssa}"
            );
        }
    }
}

#[cfg(unix)]
fn closed_socket_flush_has_a_record(exe: &Path, dir: &Path, route: &str) {
    use std::{
        os::{fd::OwnedFd, unix::net::UnixStream},
        process::Stdio,
        thread,
        time::Instant,
    };
    let (writer, reader) = UnixStream::pair().unwrap();
    drop(reader);
    let stderr = dir.join("closed-output.stderr");
    use std::os::unix::process::CommandExt;
    unsafe extern "C" {
        fn signal(number: i32, handler: usize) -> usize;
    }
    let mut command = Command::new(exe);
    command
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(fs::File::create(&stderr).unwrap());
    // exec直前にLinuxのSIGPIPEを既定へ戻し、Rust親からの無視設定に依存させません。
    // 子プロセス内のlibc呼出しだけを行い、共有状態やメモリ確保には触れません。
    unsafe {
        command.pre_exec(|| {
            if signal(13, 0) == usize::MAX {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut child = command.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{route}: closed-output timeout");
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(1), "{route}: {status}");
    let error = fs::read_to_string(stderr).unwrap();
    assert!(
        error.starts_with(if route == "c" {
            "cerune: io-v1 operation=write requested="
        } else {
            "cerune: io-v1 operation=flush requested=0 accepted=0 node="
        }),
        "{route}: {error}"
    );
    assert_eq!(error.lines().count(), 1, "{route}: {error}");
}

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn output_arrives_before_exit(exe: &Path, route: &str) {
    use std::{io::Read, process::Stdio, sync::mpsc, thread};
    let mut child = ChildGuard(
        Command::new(exe)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut pipe = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
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
            .expect("flush must deliver output before exit")
            .unwrap(),
        *b"ready\n",
        "{route}"
    );
    assert!(running, "{route}");
}
