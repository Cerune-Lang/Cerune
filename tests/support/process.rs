use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

// pipeのEOFを待たず、子プロセスの終了と取得済み出力を別々に観測します。
// 異常停止を大量に実行するCIでも、どのケースを待っているか残します。
pub fn bounded_output(
    command: &mut Command,
    directory: &Path,
    label: &str,
    limit: Duration,
) -> Result<Output, String> {
    bounded_output_with_stdin(command, directory, label, limit, Stdio::null())
}

pub fn bounded_output_with_stdin(
    command: &mut Command,
    directory: &Path,
    label: &str,
    limit: Duration,
    stdin: Stdio,
) -> Result<Output, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let stdout = directory.join(format!("process-{id}.stdout"));
    let stderr = directory.join(format!("process-{id}.stderr"));
    command.stdin(stdin);
    command.stdout(Stdio::from(fs::File::create(&stdout).unwrap()));
    command.stderr(Stdio::from(fs::File::create(&stderr).unwrap()));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let trace = std::env::var_os("CERUNE_TEST_TRACE").is_some();
    if trace {
        use std::io::Write;
        let _ = writeln!(std::io::stderr().lock(), "[test-process] start {label}");
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{label}: {error}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("{label}: {error}"))?
        {
            break status;
        }
        if start.elapsed() >= limit {
            #[cfg(unix)]
            {
                // この呼び出し専用のprocess groupだけを終了します。
                unsafe extern "C" {
                    fn kill(pid: i32, signal: i32) -> i32;
                }
                unsafe {
                    kill(-(child.id() as i32), 9);
                }
            }
            #[cfg(windows)]
            {
                // テストが起動した子とその子孫だけが対象です。
                let _ = Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{label}: timed out after {limit:?}; stdout={:?}; stderr={:?}",
                String::from_utf8_lossy(&fs::read(&stdout).unwrap()),
                String::from_utf8_lossy(&fs::read(&stderr).unwrap())
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if trace {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr().lock(),
            "[test-process] end {label}: {status} in {:?}",
            start.elapsed()
        );
    }
    Ok(Output {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    })
}
