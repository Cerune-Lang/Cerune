//! 専用監督プロセスと呼出し元の状態が分離されることを調べるLinux実験です。
use super::{platform, require};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(PartialEq, Eq)]
struct HostState {
    subreaper: i32,
    group: libc::pid_t,
    handler: usize,
    flags: i32,
    mask: Vec<bool>,
}
impl HostState {
    fn read() -> io::Result<Self> {
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        if unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            subreaper: platform::subreaper()?,
            group: unsafe { libc::getpgrp() },
            handler: action.sa_sigaction,
            flags: action.sa_flags,
            mask: (1..=64)
                .map(|signal| unsafe { libc::sigismember(&action.sa_mask, signal) == 1 })
                .collect(),
        })
    }
}
struct Guard(Child);
impl Guard {
    fn wait_until(&mut self, deadline: Instant) -> io::Result<ExitStatus> {
        loop {
            if let Some(status) = self.0.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "probe child deadline",
                ));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(None)) {
            return;
        }
        let _ = self.0.kill();
        // 試作の異常経路だけの後始末です。管理対象の子孫回収成功とは見なしません。
        let _ = self.wait_until(Instant::now() + Duration::from_secs(2));
    }
}

pub fn run(directory: &str) -> io::Result<()> {
    let root = std::path::absolute(Path::new(directory))?;
    let exe = std::env::current_exe()?;
    let before = HostState::read()?;
    // 監督対象に含めない別の子です。その終了状態を呼出し元が回収できることを確認します。
    let mut unrelated = Guard(
        Command::new(&exe)
            .args(["--fixture", "exit"])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut supervisor = Guard(
        Command::new(&exe)
            .arg("--scope-probe-worker-v1")
            .arg(&root)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    let supervisor_status = supervisor.wait_until(Instant::now() + Duration::from_secs(60))?;
    let unrelated_status = unrelated.wait_until(Instant::now() + Duration::from_secs(2))?;
    let after = HostState::read()?;
    require(supervisor_status.success(), "isolated supervisor failed")?;
    require(
        unrelated_status.code() == Some(255),
        "unrelated child status lost",
    )?;
    require(before == after, "caller process state changed")?;

    let path = root.join("observations.jsonl");
    let mut bytes = Vec::new();
    File::open(&path)?.take(65537).read_to_end(&mut bytes)?;
    require(bytes.len() <= 65536, "probe observation limit exceeded")?;
    let text = std::str::from_utf8(&bytes).map_err(io::Error::other)?;
    let records: Vec<Value> = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(io::Error::other)?;
    require(
        records.len() == 9 && records.iter().all(|v| v["passed"] == true),
        "incomplete supervised cases",
    )?;
    let proof = records
        .iter()
        .find(|v| v["case"] == "missing-executable")
        .ok_or_else(|| io::Error::other("supervisor evidence missing"))?;
    require(
        proof["supervisor"]["before"] == 0 && proof["supervisor"]["after"] == 1,
        "subreaper not isolated",
    )?;
    let descendant = records
        .iter()
        .find(|v| v["case"] == "descendant")
        .ok_or_else(|| io::Error::other("descendant evidence missing"))?;
    require(
        descendant["scope"]["adopted_descendants_reaped"]
            .as_u64()
            .is_some_and(|n| n >= 1),
        "supervisor did not reap adopted descendant",
    )?;
    let observation = json!({
        "case":"supervisor-isolation", "passed":true,
        "caller_subreaper_before":before.subreaper, "caller_subreaper_after":after.subreaper,
        "caller_process_group_unchanged":before.group == after.group,
        "caller_sigchld_unchanged":before.handler == after.handler && before.flags == after.flags && before.mask == after.mask,
        "unrelated_child_exit":unrelated_status.code(),
        "supervisor_reaped":true, "supervisor_subreaper_before":0, "supervisor_subreaper_after":1,
        "scope_descendant_reaped":true
    });
    writeln!(OpenOptions::new().append(true).open(path)?, "{observation}")?;
    println!("subprocess supervisor isolation: passed (10 Linux cases total)");
    Ok(())
}
