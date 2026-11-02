//! 既存のJob起動と非同期pipeを接続し、終了とI/O終端を別々に確認します。
use super::*;
#[path = "../os_types.rs"]
#[allow(dead_code)]
mod os_types;
use os_types::{Exit, Pipes};
// 既存試作と起動・引用符・Job管理を共有します。匿名pipe用入口はここでは未使用です。
#[path = "../windows.rs"]
#[allow(dead_code)]
mod scope;
struct Parents {
    input: Arc<OwnedHandle>,
    output: Arc<OwnedHandle>,
    error: Arc<OwnedHandle>,
}
fn launch(mode: &str) -> io::Result<(scope::Scope, Parents)> {
    let a = pair_with_client(false)?;
    let b = pair_with_client(false)?;
    let c = pair_with_client(false)?;
    let mut children = Vec::new();
    for client in [a.client, b.client, c.client] {
        let client =
            Arc::try_unwrap(client).map_err(|_| io::Error::other("shared child handle"))?;
        require(
            unsafe { SetHandleInformation(client.as_raw_handle(), 1, 1) } != 0,
            "child handle inheritance",
        )?;
        children.push(client);
    }
    let exe = std::env::current_exe()?;
    let cwd = exe
        .parent()
        .ok_or_else(|| io::Error::other("fixture directory"))?;
    let stdio = children
        .try_into()
        .map_err(|_| io::Error::other("stdio count"))?;
    let managed =
        scope::Scope::spawn_with_stdio(&exe, &["--fixture".into(), mode.into()], cwd, &[], stdio)?;
    Ok((
        managed,
        Parents {
            input: a.server,
            output: b.server,
            error: c.server,
        },
    ))
}
fn exit(managed: &mut scope::Scope) -> io::Result<Exit> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = managed.peek()? {
            return Ok(value);
        }
        require(Instant::now() < deadline, "child exit deadline")?;
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn cleanup(managed: &mut scope::Scope) -> io::Result<()> {
    managed.terminate()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !managed.collect()? {
        require(Instant::now() < deadline, "job cleanup deadline")?;
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
fn prefix(handle: Arc<OwnedHandle>, expected: &[u8]) -> io::Result<()> {
    let mut op = Operation::start(handle, None, 64)?;
    let result = op.wait()?;
    require(
        result.accepted == Some(expected.len() as u32)
            && &op.resources.as_ref().unwrap().buffer[..expected.len()] == expected,
        "fixture prefix changed",
    )
}
fn roundtrip() -> io::Result<serde_json::Value> {
    let sentinel = pair_with_client(false)?;
    require(
        unsafe { SetHandleInformation(sentinel.client.as_raw_handle(), 1, 1) } != 0,
        "sentinel inheritable",
    )?;
    let (mut managed, pipes) = launch("transport-echo")?;
    drop(sentinel.client);
    let mut excluded = Operation::start(sentinel.server, None, 64)?;
    require(
        excluded.wait()?.error == Some(109),
        "unlisted host handle inherited",
    )?;
    let bytes = "日本語\0\r\n".as_bytes();
    let mut output = Operation::start(pipes.output.clone(), None, 64)?;
    let mut error = Operation::start(pipes.error.clone(), None, 64)?;
    let mut input = Operation::start(pipes.input.clone(), Some(bytes), bytes.len())?;
    require(
        input.wait()?.accepted == Some(bytes.len() as u32),
        "stdin accepted",
    )?;
    drop(input);
    drop(pipes.input);
    require(
        output.wait()?.accepted == Some(bytes.len() as u32),
        "stdout count",
    )?;
    require(
        &output.resources.as_ref().unwrap().buffer[..bytes.len()] == bytes,
        "stdout content",
    )?;
    require(
        error.wait()?.accepted == Some(6)
            && &error.resources.as_ref().unwrap().buffer[..6] == b"err\0\r\n",
        "stderr content",
    )?;
    require(exit(&mut managed)? == Exit::Code(7), "exit changed")?;
    cleanup(&mut managed)?;
    for handle in [pipes.output, pipes.error] {
        let mut tail = Operation::start(handle, None, 64)?;
        require(tail.wait()?.error == Some(109), "stdio not closed")?;
    }
    Ok(
        json!({"accepted":12,"stdout":12,"stderr":6,"exit":7,"job_empty":true,"stdio_closed":true,"unlisted_host_handle_excluded":true}),
    )
}
fn termination(descendant: bool) -> io::Result<serde_json::Value> {
    let (mut managed, pipes) = launch(if descendant { "descendant" } else { "sleep" })?;
    prefix(pipes.output.clone(), b"ready\n")?;
    prefix(pipes.error.clone(), b"partial\0\r\n")?;
    if descendant {
        require(exit(&mut managed)? == Exit::Code(0), "leader exit lost")?;
    }
    let mut output = Operation::start(pipes.output.clone(), None, 64)?;
    let mut error = Operation::start(pipes.error.clone(), None, 64)?;
    require(
        output.poll().is_none() && error.poll().is_none(),
        "read not pending",
    )?;
    let mut input = if descendant {
        None
    } else {
        let mut op = Operation::start(pipes.input.clone(), Some(&[42; 65536]), 65536)?;
        require(op.poll().is_none(), "write not pending")?;
        Some(op)
    };
    cleanup(&mut managed)?;
    let stdout = output.wait()?;
    let stderr = error.wait()?;
    for result in [stdout, stderr] {
        require(
            matches!(result.error, Some(109 | ABORTED)),
            "read termination",
        )?;
        require(result.accepted.is_none(), "unknown read progress invented")?;
    }
    let stdin_error = if let Some(input) = &mut input {
        let result = input.wait()?;
        require(
            result.error.is_some() && result.accepted.is_none(),
            "write termination",
        )?;
        require(input.poll().is_none(), "write terminal twice")?;
        result.error
    } else {
        None
    };
    require(
        output.poll().is_none() && error.poll().is_none(),
        "read terminal twice",
    )?;
    Ok(
        json!({"descendant":descendant,"job_empty":true,"stdout_prefix":6,"stderr_prefix":10,
        "stdout_terminal_error":stdout.error,"stderr_terminal_error":stderr.error,
        "stdin_terminal_error":stdin_error,"terminal_counts_unknown":true,"counted_once":true}),
    )
}
fn owner_drop() -> io::Result<serde_json::Value> {
    let (managed, pipes) = launch("sleep")?;
    prefix(pipes.output.clone(), b"ready\n")?;
    prefix(pipes.error.clone(), b"partial\0\r\n")?;
    let process = managed.duplicate_process_handle()?;
    let mut read = Operation::start(pipes.output.clone(), None, 64)?;
    require(read.poll().is_none(), "owner-drop read not pending")?;
    drop(pipes);
    // 操作だけがpipeを所有した状態で、Jobの最終所有者を破棄します。
    drop(managed);
    let done = read.wait()?;
    require(done.error == Some(109), "owner-drop I/O terminal")?;
    require(
        unsafe { WaitForSingleObject(process.as_raw_handle(), 5000) } == 0,
        "owner-drop child wait",
    )?;
    require(read.poll().is_none(), "owner-drop duplicate terminal")?;
    Ok(
        json!({"job_owner_dropped":true,"direct_waited":true,"io_terminal":true,"accepted":done.accepted}),
    )
}
pub fn run(log: &mut File) -> io::Result<()> {
    case(log, "job-stdio-roundtrip", roundtrip)?;
    case(log, "job-terminate-pending", || termination(false))?;
    case(log, "job-descendant-pending", || termination(true))?;
    case(log, "job-owner-drop", owner_drop)
}
