use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
#[path = "fixture.rs"]
mod fixture;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "isolation.rs"]
mod isolation;

#[path = "os_types.rs"]
mod os_types;
use os_types::{Exit, Pipes};
#[derive(Default)]
struct Transfer {
    data: Vec<u8>,
    count: usize,
    eof: bool,
    error: Option<&'static str>,
}
fn receive(mut reader: impl Read, limit: usize) -> Transfer {
    let mut result = Transfer::default();
    let mut buffer = [0; 4096];
    loop {
        let size = buffer.len().min(limit - result.data.len() + 1);
        match reader.read(&mut buffer[..size]) {
            Ok(0) => {
                result.eof = true;
                return result;
            }
            Ok(n) => {
                result.count += n;
                let keep = n.min(limit - result.data.len());
                result.data.extend_from_slice(&buffer[..keep]);
                if keep != n {
                    result.error = Some("output-limit");
                    return result;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                result.error = Some("read-error");
                return result;
            }
        }
    }
}
fn send(mut writer: impl Write, bytes: Vec<u8>) -> Transfer {
    let mut result = Transfer::default();
    while result.count < bytes.len() {
        match writer.write(&bytes[result.count..]) {
            Ok(0) => {
                result.error = Some("zero-progress");
                return result;
            }
            Ok(n) => result.count += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                result.error = Some("write-error");
                return result;
            }
        }
    }
    result.eof = true;
    result
}
struct Case {
    name: &'static str,
    args: Vec<String>,
    input: Vec<u8>,
    limit: usize,
    timeout: Duration,
}
fn case(name: &'static str) -> Case {
    Case {
        name,
        args: vec!["--fixture".into(), name.into()],
        input: Vec::new(),
        limit: 1024 * 1024,
        timeout: Duration::from_secs(5),
    }
}
struct Observed {
    transfers: [Transfer; 3],
    exit: Option<Exit>,
    reason: Option<&'static str>,
    cleanup_complete: bool,
    cleanup_error: Option<&'static str>,
    scope: Value,
    elapsed_ms: u128,
    cleanup_ms: u128,
}
fn execute(exe: &Path, cwd: &Path, env: &[(String, String)], case: &Case) -> io::Result<Observed> {
    eprintln!("[subprocess-probe] start {}", case.name);
    let start = Instant::now();
    let deadline = start + case.timeout;
    let (mut scope, pipes) = platform::Scope::spawn(exe, &case.args, cwd, env)?;
    let (tx, rx) = mpsc::sync_channel(3);
    let input = case.input.clone();
    let limit = case.limit;
    let mut threads = Vec::new();
    let sender = tx.clone();
    threads.push(thread::spawn(move || {
        let _ = sender.send((0, send(pipes.input, input)));
    }));
    let sender = tx.clone();
    threads.push(thread::spawn(move || {
        let _ = sender.send((1, receive(pipes.output, limit)));
    }));
    threads.push(thread::spawn(move || {
        let _ = tx.send((2, receive(pipes.error, limit)));
    }));
    let mut results: [Option<Transfer>; 3] = [None, None, None];
    let mut exit = None;
    let reason = loop {
        for (index, transfer) in rx.try_iter() {
            results[index] = Some(transfer);
        }
        if exit.is_none() {
            match scope.peek() {
                Ok(status) => exit = status,
                Err(_) => break Some("wait-error"),
            }
        }
        if let Some(error) = results.iter().flatten().find_map(|r| r.error) {
            break Some(error);
        }
        if Instant::now() >= deadline {
            break Some("timeout");
        }
        if exit.is_some() && results.iter().all(Option::is_some) {
            break None;
        }
        thread::sleep(Duration::from_millis(1));
    };
    let elapsed_ms = start.elapsed().as_millis();
    let cleanup_start = Instant::now();
    let cleanup_deadline = cleanup_start + Duration::from_secs(3);
    let mut cleanup_error = scope.terminate().err().map(|_| "terminate-error");
    let mut collected = false;
    loop {
        for (index, transfer) in rx.try_iter() {
            results[index] = Some(transfer);
        }
        if !collected && cleanup_error.is_none() {
            match scope.collect() {
                Ok(done) => collected = done,
                Err(_) => cleanup_error = Some("collection-error"),
            }
        }
        if collected && threads.iter().all(thread::JoinHandle::is_finished) {
            break;
        }
        if Instant::now() >= cleanup_deadline {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let mut joined = true;
    for handle in threads {
        if handle.is_finished() {
            joined &= handle.join().is_ok();
        } else {
            joined = false;
        } // 無期限のjoinをしません。
    }
    for (index, transfer) in rx.try_iter() {
        results[index] = Some(transfer);
    }
    let cleanup_complete =
        collected && joined && results.iter().all(Option::is_some) && cleanup_error.is_none();
    if !cleanup_complete && cleanup_error.is_none() {
        cleanup_error = Some("cleanup-timeout-or-worker-failure");
    }
    let transfers = results.map(Option::unwrap_or_default);
    eprintln!(
        "[subprocess-probe] end {}: {:?}, cleanup={cleanup_complete}",
        case.name, reason
    );
    Ok(Observed {
        transfers,
        exit,
        reason,
        cleanup_complete,
        cleanup_error,
        scope: scope.evidence(),
        elapsed_ms,
        cleanup_ms: cleanup_start.elapsed().as_millis(),
    })
}
fn require(condition: bool, message: &str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}
fn check(case: &Case, out: &Observed, cwd: &Path) -> io::Result<()> {
    require(out.cleanup_complete, "cleanup not confirmed")?;
    match case.name {
        "context" => {
            require(
                out.reason.is_none() && out.exit == Some(Exit::Code(0)),
                "context exit",
            )?;
            let data: Value = serde_json::from_slice(&out.transfers[1].data)?;
            require(data["args"] == json!(&case.args[2..]), "argv mismatch")?;
            require(data["cwd"] == json!(cwd), "cwd mismatch")?;
            require(data["value"] == "日本語\r\n", "environment mismatch")?;
            require(
                data["path_absent"] == true,
                "implicit environment inheritance",
            )?;
            require(out.transfers[2].data == b"err\0\r\n", "stderr bytes")?;
        }
        "duplex" => {
            require(
                out.reason.is_none() && out.exit == Some(Exit::Code(0)),
                "duplex exit",
            )?;
            let expected: Vec<_> = std::iter::repeat_n(255, fixture::BLOCK)
                .chain(std::iter::repeat_n(42, fixture::BLOCK))
                .collect();
            require(out.transfers[1].data == expected, "duplex stdout")?;
            require(
                out.transfers[2].data == vec![128; fixture::BLOCK],
                "duplex stderr",
            )?;
            require(out.transfers[0].count == fixture::BLOCK, "stdin count")?;
        }
        "exit" => {
            let code = if cfg!(windows) { u32::MAX } else { 255 };
            require(
                out.reason.is_none() && out.exit == Some(Exit::Code(code)),
                "nonzero exit lost",
            )?;
        }
        "early-close" => {
            require(
                out.reason == Some("write-error"),
                "early close not reported",
            )?;
            require(
                out.transfers[0].count < case.input.len(),
                "stdin falsely fully accepted",
            )?;
        }
        "exact" => {
            require(
                out.reason.is_none() && out.transfers[1].eof,
                "exact-limit EOF",
            )?;
            require(
                out.transfers[1].data == b"1234567" && out.transfers[1].count == 7,
                "exact-limit bytes",
            )?;
        }
        "limit" => {
            require(
                out.reason == Some("output-limit"),
                "output limit not reported",
            )?;
            require(
                out.transfers[1].count == 8 && out.transfers[1].data == vec![77; 7],
                "overflow probe count",
            )?;
        }
        "sleep" | "descendant" => {
            require(out.reason == Some("timeout"), "deadline not reported")?;
            require(out.transfers[1].data == b"ready\n", "timeout stdout lost")?;
            require(
                out.transfers[2].data == b"partial\0\r\n",
                "timeout stderr lost",
            )?;
            if case.name == "descendant" {
                require(
                    out.exit == Some(Exit::Code(0)),
                    "direct exit not observed before pipe EOF",
                )?;
                #[cfg(target_os = "linux")]
                require(
                    out.scope["adopted_descendants_reaped"].as_u64().unwrap() >= 1,
                    "descendant not reaped",
                )?;
            }
        }
        _ => return Err(io::Error::other("unknown case")),
    }
    Ok(())
}
fn snapshot(case: &Case, out: &Observed) -> Value {
    json!({"case":case.name, "passed":true, "reason":out.reason,
        "exit_before_cleanup":out.exit.map(Exit::value), "requested_stdin":case.input.len(),
        "stdin_accepted":out.transfers[0].count,
        "stdin_error":out.transfers[0].error, "stdout_error":out.transfers[1].error,
        "stderr_error":out.transfers[2].error,
        "stdout_read":out.transfers[1].count, "stdout_retained":out.transfers[1].data.len(),
        "stderr_read":out.transfers[2].count, "stderr_retained":out.transfers[2].data.len(),
        "stdout_eof":out.transfers[1].eof, "stderr_eof":out.transfers[2].eof,
        "cleanup_complete":out.cleanup_complete, "cleanup_error":out.cleanup_error, "scope":out.scope,
        "elapsed_ms":out.elapsed_ms, "cleanup_ms":out.cleanup_ms})
}
pub fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--fixture") {
        if args.len() < 2 {
            return Err(io::Error::other("missing fixture"));
        }
        return fixture::run(&args[1..]);
    }
    #[cfg(target_os = "linux")]
    if args.first().map(String::as_str) == Some("--scope-probe-worker-v1") {
        if args.len() != 2 {
            return Err(io::Error::other("missing worker output directory"));
        }
        return run_cases(&args[1]);
    }
    if args.len() != 1 {
        return Err(io::Error::other(
            "usage: subprocess_probe <new-output-directory>",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        isolation::run(&args[0])
    }
    #[cfg(windows)]
    {
        run_cases(&args[0])
    }
}
fn run_cases(directory: &str) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    let supervisor_before = platform::subreaper()?;
    platform::init()?;
    #[cfg(target_os = "linux")]
    let supervisor_after = platform::subreaper()?;
    let root = std::path::absolute(PathBuf::from(directory))?;
    fs::create_dir(&root)?;
    let cwd = root.join("work 空間");
    fs::create_dir(&cwd)?;
    let exe = root.join(if cfg!(windows) {
        "child 空間.exe"
    } else {
        "child 空間"
    });
    fs::copy(std::env::current_exe()?, &exe)?;
    let env = {
        let mut entries = vec![("CERUNE_PROBE".to_owned(), "日本語\r\n".to_owned())];
        if cfg!(windows) {
            entries.push((
                "SystemRoot".into(),
                std::env::var("SystemRoot")
                    .map_err(|_| io::Error::other("SystemRoot required by probe configuration"))?,
            ));
        }
        entries
    };
    let mut log = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("observations.jsonl"))?;
    let mut context = case("context");
    context.args.extend(
        [
            "",
            "a b",
            "a\"b",
            "\\",
            "tail\\",
            "日本語",
            "CR\rLF\n",
            "&|$()<>",
        ]
        .map(str::to_owned),
    );
    let mut duplex = case("duplex");
    duplex.input = vec![42; fixture::BLOCK];
    let mut early = case("early-close");
    early.input = vec![42; 1024 * 1024];
    let mut limit = case("limit");
    limit.limit = 7;
    let mut exact = case("exact");
    exact.limit = 7;
    let mut sleep = case("sleep");
    sleep.timeout = Duration::from_secs(2);
    let mut descendant = case("descendant");
    descendant.timeout = Duration::from_secs(2);
    for spec in [
        context,
        duplex,
        case("exit"),
        early,
        limit,
        exact,
        sleep,
        descendant,
    ] {
        let observed = execute(&exe, &cwd, &env, &spec)?;
        if let Err(error) = check(&spec, &observed, &cwd) {
            let mut record = snapshot(&spec, &observed);
            record["passed"] = json!(false);
            writeln!(log, "{record}")?;
            return Err(error);
        }
        writeln!(log, "{}", snapshot(&spec, &observed))?;
    }
    let missing = root.join("missing-executable");
    let result = platform::Scope::spawn(&missing, &[], &cwd, &env);
    require(result.is_err(), "missing executable unexpectedly launched")?;
    let missing_record = json!({"case":"missing-executable", "passed":true});
    #[cfg(target_os = "linux")]
    let missing_record = {
        let mut record = missing_record;
        record["supervisor"] = json!({"before":supervisor_before, "after":supervisor_after});
        record
    };
    writeln!(log, "{missing_record}")?;
    println!("subprocess OS probe: 9 cases passed");
    Ok(())
}
