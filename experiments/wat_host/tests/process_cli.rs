#![cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
use cerune_lang::{self as language, codegen::wat, runtime};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::Duration,
};
#[path = "../../../tests/support/process.rs"]
mod bounded;
const EXAMPLE: &str = include_str!("../../../examples/process_execution/registered.ceru");
fn target() -> &'static str {
    if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "cerune-wat-process-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn put(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let p = self.0.join(name);
        fs::write(&p, bytes).unwrap();
        p
    }
    fn program(&self, source: &str) -> (PathBuf, language::ir::Program) {
        let p = language::compile_to_ir(source).unwrap();
        let file = self.put("program.wat", wat::emit_wat(&p).unwrap());
        (file, p)
    }
    fn run(&self, module: &Path, args: &[&std::ffi::OsStr], stdin: &[u8]) -> Output {
        let input = self.put("input", stdin);
        bounded::bounded_output_with_stdin(
            Command::new(env!("CARGO_BIN_EXE_cerune-wat-host"))
                .arg(module)
                .args(args)
                .env("CERUNE_PARENT_SENTINEL", "not inherited"),
            &self.0,
            "WAT host subprocess",
            Duration::from_secs(40),
            Stdio::from(fs::File::open(input).unwrap()),
        )
        .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn config(t: &Temp) -> serde_json::Value {
    serde_json::json!({"schema":"cerune-process-v1","platform":std::env::consts::OS,
        "linux_supervisor":if cfg!(windows){None}else{Some(std::env::current_exe().unwrap())},
        "limits":{"arguments":32,"argument_bytes":16384,"stdin_bytes":8192,"output_bytes":32768,"timeout_ms":10000,"cleanup_timeout_ms":5000},
        "transfer_bytes":65536,"targets":[{"name":"tool","executable":t.0.join("missing"),"directory":t.0,"environment":[]}]})
}
#[test]
fn target_is_explicit_and_config_errors_precede_program_output() {
    let t = Temp::new();
    let (file, _) = t.program(EXAMPLE);
    let r = t.run(&file, &["--".as_ref()], &[]);
    assert!(!r.status.success() && r.stdout.is_empty());
    assert!(String::from_utf8_lossy(&r.stderr).contains("explicit --process-target"));
    let r = t.run(
        &file,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--".as_ref(),
        ],
        &[],
    );
    assert!(r.status.success(), "{r:?}");
    assert!(String::from_utf8_lossy(&r.stdout).contains("Unconfigured{}"));
    let other = if cfg!(windows) {
        "x86_64-unknown-linux-gnu"
    } else {
        "x86_64-pc-windows-msvc"
    };
    let r = t.run(
        &file,
        &["--process-target".as_ref(), other.as_ref(), "--".as_ref()],
        &[],
    );
    assert!(!r.status.success() && r.stdout.is_empty());
    assert!(String::from_utf8_lossy(&r.stderr).contains("status=18"));
    let mut wrong = config(&t);
    wrong["platform"] = serde_json::json!("other");
    let mut secret = config(&t);
    secret["targets"][0]["environment"] = serde_json::json!([["KEY", "secret-value", "extra"]]);
    for bytes in [
        vec![],
        b"{".to_vec(),
        vec![255],
        vec![b' '; 1_048_577],
        serde_json::to_vec(&wrong).unwrap(),
        serde_json::to_vec(&secret).unwrap(),
    ] {
        let cfg = t.put("config.json", bytes);
        let r = t.run(
            &file,
            &[
                "--process-target".as_ref(),
                target().as_ref(),
                "--process-config".as_ref(),
                cfg.as_os_str(),
                "--".as_ref(),
            ],
            &[],
        );
        assert!(!r.status.success() && r.stdout.is_empty(), "{r:?}");
        assert!(!String::from_utf8_lossy(&r.stderr).contains("secret-value"));
    }
    let missing = t.0.join("absent.json");
    let r = t.run(
        &file,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--process-config".as_ref(),
            missing.as_os_str(),
            "--".as_ref(),
        ],
        &[],
    );
    assert!(!r.status.success() && r.stdout.is_empty());
}
fn executor(
    route: usize,
    p: &language::ir::Program,
    cfg: &[u8],
    root: &Path,
    stdin: &[u8],
    args: &[&[u8]],
) -> Vec<u8> {
    let (mut processes, n) = runtime::process::configuration::decode(cfg).unwrap();
    let mut files = runtime::files::Files::new(runtime::files::Directory::open(root).unwrap());
    let mut input = runtime::input::Input::new(std::io::Cursor::new(stdin));
    let mut output = runtime::output::Output::new(Vec::new());
    let args = runtime::arguments::Arguments::capture(args, Default::default()).unwrap();
    let limit = std::num::NonZeroU32::new(128).unwrap();
    match route {
        0 => language::ir_executor::run_with_io_files_processes_arguments_and_call_limit(
            p,
            &mut input,
            &mut output,
            &mut files,
            &args,
            limit,
            Some((&mut processes, n)),
        )
        .unwrap(),
        1 => language::mir_executor::run_with_io_files_processes_arguments_and_call_limit(
            &language::mir::lower(p).unwrap(),
            &mut input,
            &mut output,
            &mut files,
            &args,
            limit,
            Some((&mut processes, n)),
        )
        .unwrap(),
        2 => language::ssa_executor::run_with_io_files_processes_arguments_and_call_limit(
            &language::mir::ssa::construct(&language::mir::lower(p).unwrap()).unwrap(),
            &mut input,
            &mut output,
            &mut files,
            &args,
            limit,
            Some((&mut processes, n)),
        )
        .unwrap(),
        _ => language::vm::run_with_io_files_processes_arguments_and_call_limit(
            &language::bytecode::lower(p).unwrap(),
            &mut input,
            &mut output,
            &mut files,
            &args,
            limit,
            Some((&mut processes, n)),
        )
        .unwrap(),
    }
    output.into_inner()
}
fn child_config(t: &Temp) -> serde_json::Value {
    let child = t.0.join(if cfg!(windows) { "child.exe" } else { "child" });
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/support/process_child.rs");
    let result = bounded::bounded_output(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&child),
        &t.0,
        "compile child",
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(result.status.success(), "{result:?}");
    t.put("child-directory-marker", b"registered cwd");
    let mut cfg = config(t);
    cfg["targets"][0]["executable"] = serde_json::json!(child);
    cfg["targets"][0]["environment"] = serde_json::json!([["CERUNE_CHILD_VALUE", "日本語\r\n"]]);
    if cfg!(target_os = "linux") {
        cfg["linux_supervisor"] = serde_json::json!(
            std::env::var("CERUNE_TEST_PROCESS_SUPERVISOR").expect("explicit supervisor")
        );
    }
    cfg
}
#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "requires explicit CERUNE_TEST_PROCESS_SUPERVISOR"
)]
fn cli_composes_child_execution_arguments_stdin_files_and_flush() {
    let t = Temp::new();
    let cfg = serde_json::to_vec(&child_config(&t)).unwrap();
    let config = t.put("config.json", &cfg);
    let (example, program) = t.program(EXAMPLE);
    let actual = t.run(
        &example,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--process-config".as_ref(),
            config.as_os_str(),
            "--".as_ref(),
        ],
        &[],
    );
    assert!(actual.status.success(), "{actual:?}");
    assert_eq!(
        actual.stdout,
        include_bytes!("../../../examples/process_execution/registered.stdout")
    );
    let instrumented = t.put(
        "traced.wat",
        wat::emit_wat_for_execution(
            &program,
            Default::default(),
            std::num::NonZeroU32::new(128).unwrap(),
            true,
        )
        .unwrap(),
    );
    let path = t.0.join("child.jsonl");
    let traced = t.run(
        &instrumented,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--process-config".as_ref(),
            config.as_os_str(),
            "--trace".as_ref(),
            path.as_os_str(),
            "--".as_ref(),
        ],
        &[],
    );
    assert_eq!(traced.status, actual.status);
    assert_eq!(traced.stdout, actual.stdout);
    assert_eq!(traced.stderr, actual.stderr);
    let events = fs::read_to_string(path).unwrap();
    assert!(events.contains("\"event\":\"call\"") && events.contains("\"status\":\"complete\""));
    assert!(!events.contains("CERUNE_CHILD_VALUE"));
    let source = format!(
        r#"import "std:env" as env; import "std:io" as io; import "std:fs" as fs;
{EXAMPLE}
print(env::arg_count());print(env::arg(0));print(env::arg(1));print("input-ready");flush();
match io::read_line(32) {{ io::ReadLine::Line {{text:text}} => {{fs::write_new("saved",text,32);print(fs::read_text("saved",32));}}, io::ReadLine::End {{}} => {{print("EOF");}}, }}
print([true,false]);print(1.25);print("done");"#
    );
    let (file, p) = t.program(&source);
    let stdin = "入力\r\n".as_bytes();
    let root = t.0.join("wat");
    fs::create_dir(&root).unwrap();
    let r = t.run(
        &file,
        &[
            "--file-root".as_ref(),
            root.as_os_str(),
            "--process-config".as_ref(),
            config.as_os_str(),
            "--process-target".as_ref(),
            target().as_ref(),
            "--".as_ref(),
            "--process-target".as_ref(),
            "".as_ref(),
        ],
        stdin,
    );
    assert!(r.status.success(), "{r:?}");
    assert!(r.stderr.is_empty());
    assert_eq!(fs::read(root.join("saved")).unwrap(), stdin);
    for route in 0..4 {
        let root = t.0.join(format!("route-{route}"));
        fs::create_dir(&root).unwrap();
        assert_eq!(
            executor(route, &p, &cfg, &root, stdin, &[b"--process-target", b""]),
            r.stdout
        );
        assert_eq!(fs::read(root.join("saved")).unwrap(), stdin);
    }
}
#[test]
#[cfg_attr(
    target_os = "linux",
    ignore = "requires explicit CERUNE_TEST_PROCESS_SUPERVISOR"
)]
fn timeout_returns_cleanup_and_preserves_connection_reuse_decision() {
    let t = Temp::new();
    let cfg = serde_json::to_vec(&child_config(&t)).unwrap();
    let config = t.put("config.json", &cfg);
    let source = r#"import "std:process" as p; import "std:process::run" as run;
request:p::Request=p::Request {target:"tool",arguments:array_copy(["sleep"]),stdin:array_repeat::<u8>(u8(0),0),stdout_limit:32,stderr_limit:32,timeout_ms:100};
print("before");result:p::Outcome=run::run(request);print(result.report.failure);print(result.report.cleanup);print(result.report.progress_known);print("continued");
next:p::Outcome=run::run(p::Request {target:"tool",arguments:array_copy(["empty"]),stdin:array_repeat::<u8>(u8(0),0),stdout_limit:32,stderr_limit:32,timeout_ms:10000});
print(next.report.failure);print(next.report.exit);print(next.stderr);print("done");"#;
    let (file, p) = t.program(source);
    let r = t.run(
        &file,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--process-config".as_ref(),
            config.as_os_str(),
            "--".as_ref(),
        ],
        &[],
    );
    assert!(r.status.success(), "{r:?}");
    let text = String::from_utf8_lossy(&r.stdout);
    timeout_result(&text);
    // OS event timing may select Transfer or Wait as the timeout phase. Compare the cause,
    // cleanup and continuation without pretending cancellation always has known progress.
    for route in 0..4 {
        let actual = String::from_utf8(executor(route, &p, &cfg, &t.0, &[], &[])).unwrap();
        timeout_result(&actual);
    }
}

fn timeout_result(text: &str) {
    assert!(
        text.starts_with("before\n") && text.contains("Timeout{}") && text.contains("Complete{"),
        "{text}"
    );
    let (first, next) = text.split_once("continued\n").unwrap();
    assert!(next.ends_with("done\n"), "{text}");
    if first.ends_with("false\n") {
        assert!(
            next.contains("ConnectionDisabled{}"),
            "unknown progress must disable reuse: {text}"
        );
    }
    assert!(
        (next.contains("ConnectionDisabled{}") && next.contains("Absent{}"))
            || (next.starts_with("None{}\n") && next.contains("Code{value: 7}")),
        "{text}"
    );
}
#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires explicit CERUNE_TEST_PROCESS_FAULT_SUPERVISOR"]
fn incomplete_cleanup_is_reported_and_disables_the_next_launch() {
    let t = Temp::new();
    let mut cfg = config(&t);
    let fixture = PathBuf::from(
        std::env::var("CERUNE_TEST_PROCESS_FAULT_SUPERVISOR").expect("explicit fault supervisor"),
    );
    let helper = t.0.join("delayed-cleanup");
    std::os::unix::fs::symlink(&fixture, &helper).unwrap();
    cfg["linux_supervisor"] = serde_json::json!(helper);
    cfg["targets"][0]["executable"] = serde_json::json!(fixture);
    cfg["limits"]["cleanup_timeout_ms"] = serde_json::json!(10);
    let cfg = serde_json::to_vec(&cfg).unwrap();
    let config = t.put("config.json", &cfg);
    let source = r#"import "std:process" as p; import "std:process::run" as run;
request:p::Request=p::Request {target:"tool",arguments:array_copy(["unused"]),stdin:array_repeat::<u8>(u8(0),0),stdout_limit:64,stderr_limit:64,timeout_ms:10000};
print("before");result:p::Outcome=run::run(request);print(result.report.failure);print(result.report.cleanup);print(result.report.cleanup_reported);print("continued");
next:p::Outcome=run::run(request);print(next.report.failure);print(next.report.started);print("done");"#;
    let (file, p) = t.program(source);
    let r = t.run(
        &file,
        &[
            "--process-target".as_ref(),
            target().as_ref(),
            "--process-config".as_ref(),
            config.as_os_str(),
            "--".as_ref(),
        ],
        &[],
    );
    assert!(r.status.success(), "{r:?}");
    let text = String::from_utf8_lossy(&r.stdout);
    assert!(
        text.contains("CleanupFailed{}")
            && text.contains("Incomplete{")
            && text.contains("ConnectionDisabled{}")
            && text.ends_with("No{}\ndone\n"),
        "{text}"
    );
    for route in 0..4 {
        assert_eq!(executor(route, &p, &cfg, &t.0, &[], &[]), r.stdout);
    }
}
