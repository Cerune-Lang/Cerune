//! 共通runtimeとWindows実OS Backendの接続検証です。言語exampleではありません。
#[cfg(windows)]
#[path = "fixture.rs"]
mod fixture;
#[cfg(windows)]
#[path = "reservation.rs"]
mod reservation;
#[cfg(windows)]
#[path = "observation.rs"]
mod observation;
#[cfg(windows)]
mod check {
    use cerune_lang::{
        ir::NodeId,
        runtime::process::{
            Backend, CallContext, Channel, Cleanup, Code, Exit, Limits, Origin, Processes, Request,
            transfers::{Completion, Transfers},
            windows::{ConfigError, Target, Windows, retained_connections},
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
    fn backend(trace: usize) -> Windows {
        let mut missing = target("missing");
        missing.executable = missing.directory.join("cerune-definitely-missing-239.exe");
        Windows::new(vec![target("fixture"), missing], trace).unwrap()
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
    pub fn run(path: &Path) -> io::Result<()> {
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
        record(
            "result-reservation-before-launch",
            super::reservation::check(backend(64), limits(), origin())?,
        )?;
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
        require(
            out.report.exit == Some(Exit::Code(u32::MAX)),
            "exit width lost",
        )?;
        record("exit-width", json!({"exit":u32::MAX}))?;
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
                Code::ProgressUnknown,
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
        let mut p = Processes::new(backend(0), limits());
        let out = p.run(
            request(&["--fixture", "transport-zero"], &[], 64, 10),
            1 << 20,
            origin(),
        );
        // OSが0バイトwriteを無視する場合と、0進行として通知する場合を区別します。
        require(
            (out.completed() && out.stdout == b"tail")
                || out
                    .report
                    .failure
                    .is_some_and(|f| f.code == Code::ZeroProgress),
            "zero write mistaken for EOF",
        )?;
        record(
            "zero-write-not-eof",
            json!({"no_premature_eof":true,"zero_progress":out.report.failure.is_some()}),
        )?;
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
            matches!(Windows::new(vec![invalid], 0), Err(ConfigError::Invalid)),
            "relative cwd accepted",
        )?;
        let mut duplicate = target("x");
        duplicate.environment = vec![("Path".into(), "a".into()), ("PATH".into(), "b".into())];
        require(
            matches!(
                Windows::new(vec![duplicate], 0),
                Err(ConfigError::Duplicate)
            ),
            "environment duplicate accepted",
        )?;
        require(
            matches!(
                Windows::new(vec![target("x"), target("x")], 0),
                Err(ConfigError::Duplicate)
            ),
            "target duplicate accepted",
        )?;
        record(
            "configuration-rejection",
            json!({"relative_path":true,"duplicate_environment":true,"duplicate_target":true}),
        )?;
        require(
            retained_connections() == before,
            "unexpected Drop retention",
        )?;
        record("all-resources-settled", json!({"retained_connections":0}))?;
        println!("Windows subprocess Backend: {cases} cases passed");
        Ok(())
    }
}
fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("--fixture") && args.len() > 1 {
            return fixture::run(&args[1..]);
        }
        if args.len() != 1 {
            return Err(std::io::Error::other(
                "usage: subprocess_windows_backend <new-directory>",
            ));
        }
        check::run(std::path::Path::new(&args[0]))
    }
    #[cfg(not(windows))]
    {
        Err(std::io::Error::other(
            "Windows Backend probe requires Windows",
        ))
    }
}
