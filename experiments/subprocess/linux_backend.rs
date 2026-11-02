//! 共通runtimeとLinux実OS Backendの接続検証です。言語exampleではありません。
#[cfg(target_os = "linux")]
#[path = "linux_backend_faults.rs"]
mod faults;
#[cfg(target_os = "linux")]
#[path = "fixture.rs"]
mod fixture;
#[cfg(target_os = "linux")]
#[path = "observation.rs"]
mod observation;
#[cfg(target_os = "linux")]
mod check {
    use cerune_lang::{
        ir::NodeId,
        runtime::process::{
            Backend, CallContext, Channel, Cleanup, Code, Exit, Limits, Origin, Processes, Request,
            linux::{ConfigError, Linux, Target, retained_connections},
            transfers::{Completion, Transfers},
        },
        source::Span,
    };
    use serde_json::json;
    use std::{
        fs::File,
        io::{self, Write},
        num::NonZeroU64,
        path::Path,
        time::Duration,
    };
    fn require(ok: bool, message: &str) -> io::Result<()> {
        if ok {
            Ok(())
        } else {
            Err(io::Error::other(message.to_owned()))
        }
    }
    fn origin() -> Origin {
        Origin {
            node_id: NodeId(17),
            span: Span::new(30, 41),
        }
    }
    fn target(name: &str) -> Target {
        let executable = std::env::current_exe().unwrap();
        Target {
            name: name.into(),
            directory: executable.parent().unwrap().to_owned(),
            executable,
            environment: vec![("CERUNE_PROBE".into(), "日本語\r\n".into())],
        }
    }
    static SUPERVISOR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    fn backend(trace: usize) -> Linux {
        let mut missing = target("missing");
        missing.executable = missing.directory.join("cerune-definitely-missing-239.exe");
        Linux::new(
            SUPERVISOR.get().unwrap().clone(),
            vec![target("fixture"), missing],
            trace,
        )
        .unwrap()
    }
    fn limits() -> Limits {
        Limits {
            arguments: 32,
            argument_bytes: 16384,
            stdin_bytes: 1 << 20,
            output_bytes: 2 << 20,
            timeout: Duration::from_secs(10),
            cleanup_timeout: Duration::from_secs(5),
        }
    }
    fn request<'a>(args: &'a [&'a str], input: &'a [u8], limit: usize, secs: u64) -> Request<'a> {
        Request {
            target: "fixture",
            arguments: args,
            stdin: input,
            stdout_limit: limit,
            stderr_limit: limit,
            timeout: Duration::from_secs(secs),
        }
    }
    fn success(out: &cerune_lang::runtime::process::Outcome) -> io::Result<()> {
        require(
            out.completed()
                && out.report.progress_known
                && matches!(out.report.cleanup, Some(Cleanup::Complete { .. })),
            &format!("unexpected report: {:?}", out.report),
        )
    }
    pub fn run(path: &Path, supervisor: &Path) -> io::Result<()> {
        SUPERVISOR.set(supervisor.to_owned()).unwrap();
        std::fs::create_dir(path)?;
        let mut log = File::create(path.join("observations.jsonl"))?;
        let mut cases = 0;
        let mut record = |name: &str, value: serde_json::Value| -> io::Result<()> {
            cases += 1;
            writeln!(
                log,
                "{}",
                json!({"case":name,"passed":true,"evidence":value})
            )
        };
        let before = retained_connections();
        let mut p = Processes::with_trace_limit(backend(1024), limits(), 4096);
        let args = [
            "--fixture",
            "context",
            "",
            "a b",
            "a\"b",
            "tail\\",
            "日本語\r\n",
            ";&|$()",
        ];
        let out = p.run(request(&args, &[], 16384, 10), 1 << 20, origin());
        success(&out)?;
        let value: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        require(
            value["args"] == json!(&args[2..])
                && value["path_absent"] == true
                && value["value"] == "日本語\r\n",
            "context changed",
        )?;
        require(
            out.report.origin == origin() && p.events().iter().all(|e| e.report.origin == origin()),
            "origin lost",
        )?;
        record(
            "context-and-origin",
            json!({"argv_preserved":true,"environment_explicit":true,"origin_preserved":true}),
        )?;
        let bytes = "日本語\0\r\n".as_bytes();
        let args = ["--fixture", "transport-echo"];
        let out = p.run(request(&args, bytes, 64, 10), 1 << 20, origin());
        success(&out)?;
        require(
            out.stdout == bytes
                && out.stderr == b"err\0\r\n"
                && out.report.exit == Some(Exit::Code(7)),
            "echo changed",
        )?;
        require(
            out.transfers.unwrap().call.get() == 2,
            "call identity reused",
        )?;
        record(
            "echo-nonzero-and-reuse",
            json!({"accepted":out.report.stdin_accepted,"exit":7,"call":2}),
        )?;
        let input = vec![42; 128 * 1024];
        let out = p.run(
            request(&["--fixture", "duplex"], &input, 256 * 1024, 10),
            1 << 20,
            origin(),
        );
        success(&out)?;
        require(
            out.stdout == [vec![255; 128 * 1024], input].concat()
                && out.stderr == vec![128; 128 * 1024],
            "duplex lost bytes",
        )?;
        record(
            "duplex",
            json!({"stdin":out.report.stdin_accepted,"stdout":out.report.stdout_read,"stderr":out.report.stderr_read}),
        )?;
        let out = p.run(
            request(&["--fixture", "exit"], &[], 64, 10),
            1 << 20,
            origin(),
        );
        success(&out)?;
        require(out.report.exit == Some(Exit::Code(255)), "exit width lost")?;
        record("exit-width", json!({"exit":255}))?;
        let out = p.run(
            request(&["--fixture", "exact"], &[], 7, 10),
            1 << 20,
            origin(),
        );
        success(&out)?;
        require(out.stdout == b"1234567", "exact limit failed")?;
        record("exact-limit-eof", json!({"retained":7,"eof":true}))?;
        for (name, target) in [
            ("unregistered", "unknown"),
            ("missing-executable", "missing"),
        ] {
            let mut req = request(&[], &[], 64, 10);
            req.target = target;
            let out = p.run(req, 1 << 20, origin());
            require(
                out.report.failure.unwrap().code
                    == if target == "unknown" {
                        Code::Unregistered
                    } else {
                        Code::NotFound
                    },
                "launch classification",
            )?;
            require(!p.is_disabled(), "not-started disabled connection")?;
            record(name, json!({"started":false,"connection_usable":true}))?;
        }
        let out = p.run(
            request(&["--fixture", "exact"], &[], 7, 10),
            1 << 20,
            origin(),
        );
        success(&out)?;
        record("continue-after-start-failure", json!({"completed":true}))?;
        let inner = p.into_inner().unwrap();
        require(
            !inner.has_unreclaimed_resources() && !inner.events().is_empty(),
            "resource or OS evidence missing",
        )?;
        require(
            inner
                .events()
                .iter()
                .all(|e| e.context.is_some_and(|c| c.origin == origin())),
            "OS origin missing",
        )?;
        require(
            inner.events().iter().filter_map(|e| e.ticket).all(|t| {
                inner
                    .events()
                    .iter()
                    .any(|e| e.ticket == Some(t) && e.context.is_some_and(|c| c.call == t.call()))
            }),
            "OS ticket context mismatch",
        )?;
        drop(inner);
        for (mode, code, limit, input) in [
            ("limit", Code::OutputLimit, 7, Vec::new()),
            ("sleep", Code::Timeout, 64, Vec::new()),
            ("descendant", Code::Timeout, 64, Vec::new()),
            (
                "early-close",
                Code::TransferFailed,
                64,
                vec![42; 128 * 1024],
            ),
        ] {
            let mut p = Processes::with_trace_limit(backend(256), limits(), 1024);
            let out = p.run(
                request(&["--fixture", mode], &input, limit, 2),
                1 << 20,
                origin(),
            );
            require(
                out.report.failure.is_some_and(|f| f.code == code),
                &format!("{mode}: {:?}", out.report),
            )?;
            require(
                matches!(out.report.cleanup, Some(Cleanup::Complete { .. })),
                "failure resources not collected",
            )?;
            if mode == "limit" {
                require(
                    out.stdout == vec![77; 7] && out.report.stdout_read == 8,
                    "overflow prefix",
                )?;
            }
            if mode == "sleep" || mode == "descendant" {
                require(
                    out.stdout == b"ready\n" && out.stderr == b"partial\0\r\n",
                    "timeout prior output",
                )?;
            }
            let disabled = p.is_disabled();
            if !out.report.progress_known {
                require(disabled, "unknown progress reused")?;
            }
            if disabled {
                let next = p.run(
                    request(&["--fixture", "exact"], &[], 7, 10),
                    1 << 20,
                    origin(),
                );
                require(
                    next.report
                        .failure
                        .is_some_and(|f| f.code == Code::ConnectionDisabled),
                    "disabled connection started again",
                )?;
            }
            let inner = p.into_inner().unwrap();
            require(
                !inner.has_unreclaimed_resources(),
                "failure resources retained",
            )?;
            record(
                mode,
                json!({"failure":format!("{code:?}"),"accepted":out.report.stdin_accepted,
                "stdout_retained":out.stdout.len(),"stderr_retained":out.stderr.len(),
                "progress_known":out.report.progress_known,"disabled":disabled,"cleanup_complete":true}),
            )?;
        }
        let mut a = Processes::new(backend(0), limits());
        let mut b = Processes::with_trace_limit(backend(1), limits(), 1);
        let out_a = a.run(request(&args, bytes, 64, 10), 1 << 20, origin());
        let out_b = b.run(request(&args, bytes, 64, 10), 1 << 20, origin());
        success(&out_a)?;
        success(&out_b)?;
        let mut evidence = super::observation::compare_completed(&out_a, &out_b)?;
        require(b.trace_truncated(), "runtime trace not capped")?;
        let b = b.into_inner().unwrap();
        require(b.trace_truncated(), "OS trace not capped")?;
        evidence["bounded_trace"] = json!(true);
        record("observation-does-not-control", evidence)?;
        let mut connection = backend(0);
        let mut transfers =
            Transfers::new(NonZeroU64::new(1).unwrap(), origin(), [0, 64, 64], 16384).unwrap();
        let transfer = transfers.prepare_read(Channel::Stdout).unwrap();
        transfers.submit(transfer.ticket()).unwrap();
        connection.submit(transfer, Duration::ZERO);
        let batch = connection.poll(Duration::ZERO);
        require(
            batch.transfers.into_iter().flatten().next().unwrap().result
                == Completion::NotSubmitted,
            "late transfer started",
        )?;
        record("expired-submit", json!({"not_submitted":true}))?;
        let deadline = connection.now() + Duration::from_secs(10);
        require(
            matches!(
                connection.start(
                    &request(&["--fixture", "sleep"], &[], 64, 10),
                    CallContext {
                        call: NonZeroU64::new(2).unwrap(),
                        origin: origin()
                    },
                    deadline
                ),
                cerune_lang::runtime::process::Launch::Started
            ),
            "reclaim fixture start",
        )?;
        let mut transfers =
            Transfers::new(NonZeroU64::new(2).unwrap(), origin(), [0, 64, 64], 16384).unwrap();
        let transfer = transfers.prepare_read(Channel::Stdout).unwrap();
        transfers.submit(transfer.ticket()).unwrap();
        connection.submit(transfer, deadline);
        require(
            connection.reclaim(Duration::from_secs(5)) && !connection.has_unreclaimed_resources(),
            "explicit reclaim failed",
        )?;
        drop(connection);
        record(
            "explicit-reclaim-pending",
            json!({"resources_reclaimed":true}),
        )?;
        let mut invalid = target("x");
        invalid.directory = "relative".into();
        require(
            matches!(
                Linux::new(SUPERVISOR.get().unwrap().clone(), vec![invalid], 0),
                Err(ConfigError::Invalid)
            ),
            "relative cwd accepted",
        )?;
        let mut duplicate = target("x");
        duplicate.environment = vec![("PATH".into(), "a".into()), ("PATH".into(), "b".into())];
        require(
            matches!(
                Linux::new(SUPERVISOR.get().unwrap().clone(), vec![duplicate], 0),
                Err(ConfigError::Duplicate)
            ),
            "environment duplicate accepted",
        )?;
        require(
            matches!(
                Linux::new(
                    SUPERVISOR.get().unwrap().clone(),
                    vec![target("x"), target("x")],
                    0
                ),
                Err(ConfigError::Duplicate)
            ),
            "target duplicate accepted",
        )?;
        record(
            "configuration-rejection",
            json!({"relative_path":true,"duplicate_environment":true,"duplicate_target":true}),
        )?;
        // 意図した監督異常は成功した通常ケースと分けて検査します。

        for value in super::faults::verify(&path.canonicalize()?, supervisor)? {
            cases += 1;
            writeln!(log, "{value}")?;
        }
        require(
            retained_connections() == before,
            "unexpected Drop retention",
        )?;
        cases += 1;
        writeln!(
            log,
            "{}",
            json!({"case":"all-resources-settled","passed":true,"evidence":{"retained_connections":0}})
        )?;
        println!("Linux subprocess Backend: {cases} cases passed");
        Ok(())
    }
}
fn main() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("--signal-check") {
            return faults::signal_check(std::path::Path::new(
                args.get(1)
                    .ok_or_else(|| std::io::Error::other("supervisor"))?,
            ));
        }
        if let Some(result) = faults::helper(&args) {
            return result;
        }
        if args.first().map(String::as_str) == Some("--fixture-fd") {
            let fd: i32 = args
                .get(1)
                .ok_or_else(|| std::io::Error::other("fd"))?
                .parse()
                .map_err(std::io::Error::other)?;
            if unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1
                || std::io::Error::last_os_error().raw_os_error() != Some(libc::EBADF)
            {
                return Err(std::io::Error::other("host fd inherited"));
            }
            println!("not-inherited");
            return Ok(());
        }
        if args.first().map(String::as_str) == Some("--fixture") && args.len() > 1 {
            return fixture::run(&args[1..]);
        }
        if args.len() != 2 {
            return Err(std::io::Error::other(
                "usage: subprocess_linux_backend <absolute-supervisor> <new-directory>",
            ));
        }
        check::run(
            std::path::Path::new(&args[1]),
            std::path::Path::new(&args[0]),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(std::io::Error::other("Linux Backend probe requires Linux"))
    }
}
