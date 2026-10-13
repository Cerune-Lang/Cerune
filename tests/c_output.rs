use cerune_lang::{
    ir,
    runtime::output::{Failure, Output},
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
#[path = "support/process.rs"]
mod process;

struct Writer {
    bytes: Vec<u8>,
    limit: usize,
    chunk: usize,
    mode: &'static str,
    interrupted: bool,
    interrupted_after: bool,
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if !self.interrupted_after && !self.bytes.is_empty() {
            self.interrupted_after = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.bytes.len() == self.limit {
            return match self.mode {
                "zero" => Ok(0),
                "invalid" => Ok(bytes.len() + 1),
                _ => Err(io::ErrorKind::BrokenPipe.into()),
            };
        }
        let n = bytes
            .len()
            .min(self.chunk)
            .min(self.limit - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn reference(
    program: &ir::Program,
    route: usize,
    limit: usize,
    chunk: usize,
    mode: &'static str,
    trace: usize,
) -> (Vec<u8>, Option<Failure>) {
    let mut output = Output::with_trace_limit(
        Writer {
            bytes: vec![],
            limit,
            chunk,
            mode,
            interrupted: false,
            interrupted_after: false,
        },
        trace,
    );
    let ok = match route {
        0 => cerune_lang::ir_executor::run_with_output(program, &mut output).is_ok(),
        1 => cerune_lang::mir_executor::run_with_output(
            &cerune_lang::mir::lower(program).unwrap(),
            &mut output,
        )
        .is_ok(),
        2 => cerune_lang::ssa_executor::run_with_output(
            &cerune_lang::mir::ssa::construct(&cerune_lang::mir::lower(program).unwrap()).unwrap(),
            &mut output,
        )
        .is_ok(),
        _ => cerune_lang::run_bytecode_with_output(
            &cerune_lang::bytecode::lower(program).unwrap(),
            &mut output,
        )
        .is_ok(),
    };
    let failure = output.failure();
    assert_eq!(ok, failure.is_none());
    (output.into_inner().bytes, failure)
}
fn run(command: &mut Command, dir: &Path, label: &str) -> std::process::Output {
    process::bounded_output(command, dir, label, Duration::from_secs(30)).unwrap()
}
fn compiler() -> Option<std::ffi::OsString> {
    let configured = std::env::var_os("CERUNE_TEST_CC");
    let cc = configured.clone().unwrap_or_else(|| "clang".into());
    if Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        Some(cc)
    } else {
        assert!(configured.is_none(), "configured C compiler unavailable");
        eprintln!("C output execution skipped");
        None
    }
}

// 試験用の実行接続に置換します。観測APIから外部応答を差し替える機構は追加しません。
const CONNECTION: &str = r#"
static int64_t cerune_output_write(const unsigned char *data, size_t length) {
    static size_t accepted = 0;
    static int interrupted = 0;
    static int interrupted_after = 0;
    size_t limit = (size_t)strtoull(getenv("TEST_OUTPUT_LIMIT"), NULL, 10);
    size_t chunk = (size_t)strtoull(getenv("TEST_OUTPUT_CHUNK"), NULL, 10);
    const char *mode = getenv("TEST_OUTPUT_MODE");
    if (!interrupted) { interrupted = 1; errno = EINTR; return -1; }
    if (!interrupted_after && accepted != 0) { interrupted_after = 1; errno = EINTR; return -1; }
    if (accepted == limit) {
        if (mode[0] == 'z') return 0;
        if (mode[0] == 'i') return (int64_t)length + 1;
        errno = EPIPE;
        return -1;
    }
    if (length > chunk) length = chunk;
    if (length > limit - accepted) length = limit - accepted;
    int64_t n = cerune_output_original_write(data, length);
    if (n > 0) accepted += (size_t)n;
    return n;
}
"#;
fn controlled(c: String) -> String {
    let c = c.replace(
        "static int64_t cerune_output_write(",
        "static int64_t cerune_output_original_write(",
    );
    let marker = "static cerune_output_operation cerune_output_begin";
    assert!(c.contains(marker));
    c.replacen(marker, &format!("{CONNECTION}\n{marker}"), 1)
}

#[test]
fn generated_c_write_failures_match_four_executors() {
    let Some(cc) = compiler() else { return };
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("c-output-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let example = include_str!("../examples/io_protocol/output_values.ceru");
    let long = format!("print([\"{}\"]); print(99);", "日本\\0".repeat(100));
    let source_cases = [
        "print(\"日本\\0\\r\\n\"); print(\"after\");",
        "value: u64 = 18446744073709551615; print(value); print(-0.0); print(true);",
        "print([\"日本\\0\\r\\n\\u{7f}\", \"e\\u{301}\"]); print(\"after\");",
        example,
        "v: f32 = 0.1; print(v); print(0.0000001); print(false);",
        long.as_str(),
        "fn next() -> i64 { print(7); return 42; } answer: infer = i64(next()); print(answer);",
        "fn next() -> i64 { print(7); return 42; } answer: infer = convert<i64>(next()); print(answer);",
    ];
    for (index, source) in source_cases.into_iter().enumerate() {
        let hir = cerune_lang::compile_to_ir(source).unwrap();
        let text = cerune_lang::compile_to_c(source).unwrap();
        assert!(text.is_ascii());
        let c = dir.join("program.c");
        fs::write(&c, controlled(text)).unwrap();
        let exe = dir.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        let compiled = run(
            Command::new(&cc)
                .arg(&c)
                .args(["-std=c11", "-O2"])
                .arg("-o")
                .arg(&exe),
            &dir,
            "compile",
        );
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        for (limit, chunk, mode) in [
            (0, 2, "error"),
            (1, 2, "error"),
            (2, 1, "error"),
            (3, 1, "error"),
            (7, 3, "zero"),
            (12, 2, "invalid"),
            (300, 17, "error"),
            (usize::MAX, 3, "error"),
        ] {
            let expected = reference(&hir, 0, limit, chunk, mode, 0);
            for route in 0..4 {
                for trace in [0, 1, 256] {
                    assert_eq!(
                        reference(&hir, route, limit, chunk, mode, trace),
                        expected,
                        "case {index} route {route}"
                    );
                }
            }
            let actual = run(
                Command::new(&exe)
                    .env("TEST_OUTPUT_LIMIT", limit.to_string())
                    .env("TEST_OUTPUT_CHUNK", chunk.to_string())
                    .env("TEST_OUTPUT_MODE", mode),
                &dir,
                "execute",
            );
            assert_eq!(
                actual.status.success(),
                expected.1.is_none(),
                "case {index}: {}",
                String::from_utf8_lossy(&actual.stderr)
            );
            assert_eq!(
                actual.stdout, expected.0,
                "case {index}, limit {limit}, {mode}"
            );
            let record = expected
                .1
                .map(|f| format!("cerune: {}\n", f.record()))
                .unwrap_or_default();
            assert_eq!(
                String::from_utf8(actual.stderr)
                    .unwrap()
                    .replace("\r\n", "\n"),
                record,
                "case {index}, limit {limit}, {mode}"
            );
            if index == 3 && expected.1.is_none() {
                assert_eq!(
                    actual.stdout,
                    include_bytes!("../examples/io_protocol/output_values.stdout")
                );
            }
        }
    }
}

#[test]
fn generated_c_output_example_and_module_failure_keep_origins() {
    let Some(cc) = compiler() else { return };
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("c-output-modules-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("entry.ceru");
    fs::write(
        dir.join("lib.ceru"),
        "pub fn send() -> void { print(\"日本\"); flush(); print(999); }",
    )
    .unwrap();
    fs::write(&entry, "import \"lib.ceru\" as lib; lib::send();").unwrap();
    let hir = cerune_lang::modules::load(&entry).unwrap().to_ir().unwrap();
    let generated = cerune_lang::codegen::c::emit_c(&hir).unwrap();
    let c = dir.join("program.c");
    let exe = dir.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&c, controlled(generated)).unwrap();
    let result = run(
        Command::new(&cc).arg(&c).arg("-o").arg(&exe),
        &dir,
        "compile",
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected = reference(&hir, 0, 2, 1, "error", 32);
    let f = expected.1.unwrap();
    assert!(!f.origin.unwrap().span.source_id().record_field().is_empty());
    let actual = run(
        Command::new(&exe)
            .env("TEST_OUTPUT_LIMIT", "2")
            .env("TEST_OUTPUT_CHUNK", "1")
            .env("TEST_OUTPUT_MODE", "error"),
        &dir,
        "module",
    );
    assert_eq!(actual.status.code(), Some(1));
    assert_eq!(actual.stdout, expected.0);
    assert_eq!(
        String::from_utf8(actual.stderr)
            .unwrap()
            .replace("\r\n", "\n"),
        format!("cerune: {}\n", f.record())
    );
}

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn real_closed_stdout_stops_c_before_later_operations() {
    use std::{process::Stdio, thread, time::Instant};
    let Some(cc) = compiler() else { return };
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("c-output-closed-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let c = dir.join("closed.c");
    fs::write(
        &c,
        cerune_lang::compile_to_c("while true { print(\"日本\"); }").unwrap(),
    )
    .unwrap();
    let exe = dir.join(if cfg!(windows) {
        "closed.exe"
    } else {
        "closed"
    });
    let result = run(
        Command::new(&cc).arg(&c).arg("-o").arg(&exe),
        &dir,
        "compile",
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = dir.join("closed.stderr");
    let mut command = Command::new(exe);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(fs::File::create(&stderr).unwrap());
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe extern "C" {
            fn signal(number: i32, handler: usize) -> usize;
        }
        // 親のSIGPIPE無視設定に依存せず、子のexec直前に既定へ戻します。
        unsafe {
            command.pre_exec(|| {
                if signal(13, 0) == usize::MAX {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
    }
    let mut child = ChildGuard(command.spawn().unwrap());
    drop(child.0.stdout.take());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "closed stdout must stop"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(1));
    let text = fs::read_to_string(stderr).unwrap();
    assert!(
        text.starts_with("cerune: io-v1 operation=write requested=7 accepted="),
        "{text}"
    );
    assert!(
        text.contains(" node=") && text.contains(" bytes="),
        "{text}"
    );
    assert_eq!(text.lines().count(), 1);
}
