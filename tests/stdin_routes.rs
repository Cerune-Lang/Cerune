use cerune_lang::{
    codegen::c,
    ir_executor, modules,
    runtime::{
        input::{Failure, Input},
        output::Output,
    },
};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[path = "support/process.rs"]
mod process;

const ROUTES: &[&[&str]] = &[
    &["run"],
    &["run-ir"],
    &["run-vm"],
    &["run-mir"],
    &["run-mir", "--ssa"],
];
fn directory(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("stdin-{name}-{}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
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
        assert!(configured.is_none(), "configured compiler unavailable");
        eprintln!("C stdin checks skipped: compiler unavailable");
        None
    }
}
fn compile_c(dir: &Path, stem: &str, text: &str, opt: &str) -> Option<PathBuf> {
    let cc = compiler()?;
    let source = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&source, text).unwrap();
    let mut cmd = Command::new(cc);
    cmd.arg("-std=c11")
        .arg(opt)
        .arg(&source)
        .arg("-o")
        .arg(&exe);
    if cfg!(unix) {
        cmd.arg("-lm");
    }
    if std::env::var_os("CERUNE_TEST_SANITIZE").is_some() {
        cmd.args(["-fsanitize=address,undefined", "-fno-sanitize-recover=all"]);
    }
    let o =
        process::bounded_output(&mut cmd, dir, "compile input", Duration::from_secs(30)).unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    Some(exe)
}
fn cli(route: &[&str], file: &Path, budget: u64) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg(route[0]).arg(file).args(&route[1..]).args([
        "--diagnostic-format",
        "runtime-v1",
        "--string-heap-limit",
        &budget.to_string(),
    ]);
    cmd
}
fn file_run(cmd: &mut Command, dir: &Path, bytes: &[u8]) -> std::process::Output {
    let input = dir.join("input.bin");
    fs::write(&input, bytes).unwrap();
    process::bounded_output_with_stdin(
        cmd,
        dir,
        "read input",
        Duration::from_secs(10),
        Stdio::from(fs::File::open(input).unwrap()),
    )
    .unwrap()
}
fn expected(hir: &cerune_lang::ir::Program, bytes: &[u8]) -> (Vec<u8>, Option<Failure>) {
    let mut input = Input::new(io::Cursor::new(bytes));
    let mut output = Output::new(Vec::new());
    let result = ir_executor::run_with_io(hir, &mut input, &mut output);
    let failure = input.failure();
    assert_eq!(result.is_err(), failure.is_some());
    (output.into_inner(), failure)
}
fn compare(o: std::process::Output, expected: &(Vec<u8>, Option<Failure>)) {
    assert_eq!(o.stdout, expected.0);
    assert_eq!(
        o.status.success(),
        expected.1.is_none(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    if let Some(f) = expected.1 {
        let stderr = String::from_utf8(o.stderr).unwrap();
        assert_eq!(stderr.trim().trim_start_matches("cerune: "), f.record());
    } else {
        assert!(
            o.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn files_preserve_bytes_and_errors_across_cli_and_optimized_c() {
    let dir = directory("files");
    let original = include_str!("fixtures/line_input.ceru");
    let cases: &[(&[u8], i64, u64)] = &[
        (b"", 16, 64),
        (b"\n\r\n", 16, 64),
        ("日本\0\r\ne\u{301}\nlast".as_bytes(), 16, 64),
        (b"a\x1ab\r\n", 16, 64),
        (b"last", 4, 4),
        (b"abc\n", 4, 4),
        (b"SECRET\n", 4, 64),
        (b"\xff\n", 4, 64),
        (b"\xed\xa0\x80\n", 4, 64),
        (b"\xf4\x90\x80\x80", 4, 64),
        (b"\xe2\x82", 4, 64),
        (b"\xc0\x80\n", 4, 64),
        (b"x", 0, 64),
        (b"x", -1, 64),
        (b"x", 2147483648, 64),
        (b"x", 4, 3),
    ];
    for (i, &(bytes, limit, budget)) in cases.iter().enumerate() {
        let source = original.replace("read_line(16)", &format!("read_line({limit})"));
        let file = dir.join("main.ceru");
        fs::write(&file, &source).unwrap();
        let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
        hir.string_heap_limit = budget;
        let expected = expected(&hir, bytes);
        for route in ROUTES {
            compare(
                file_run(&mut cli(route, &file, budget), &dir, bytes),
                &expected,
            );
        }
        for opt in ["-O0", "-O2"] {
            if let Some(exe) = compile_c(
                &dir,
                &format!("case{i}{opt}"),
                &c::emit_c(&hir).unwrap(),
                opt,
            ) {
                compare(file_run(&mut Command::new(exe), &dir, bytes), &expected);
            }
        }
    }
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn dialogue(cmd: &mut Command, dir: &Path) {
    let err = dir.join("dialogue.stderr");
    let mut child = ChildGuard(
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(fs::File::create(&err).unwrap()))
            .spawn()
            .unwrap(),
    );
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = Vec::new();
        loop {
            let mut b = [0];
            match stdout.read(&mut b) {
                Ok(0) => break,
                Ok(_) => {
                    line.push(b[0]);
                    if b[0] == b'\n' && tx.send(std::mem::take(&mut line)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let receive = || {
        rx.recv_timeout(Duration::from_secs(10))
            .expect("prompt/reply must arrive without EOF")
    };
    assert_eq!(receive(), b"ready\n");
    assert!(child.0.try_wait().unwrap().is_none());
    stdin.write_all("日本\r\n".as_bytes()).unwrap();
    assert_eq!(receive(), "[\"日本\\r\\n\"]\n".as_bytes());
    assert_eq!(receive(), b"ready\n");
    stdin.write_all(b"\n").unwrap();
    assert_eq!(receive(), b"[\"\\n\"]\n");
    assert_eq!(receive(), b"ready\n");
    drop(stdin);
    assert_eq!(receive(), b"end\n");
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.0.try_wait().unwrap() {
            break s;
        }
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    };
    drop(child);
    reader.join().unwrap();
    assert!(status.success());
    assert!(fs::read(err).unwrap().is_empty());
}
#[test]
fn prompts_precede_pipe_input_and_pipe_closure_is_eof() {
    let dir = directory("dialogue");
    let source = include_str!("fixtures/line_input.ceru");
    let file = dir.join("main.ceru");
    fs::write(&file, source).unwrap();
    let hir = modules::from_source(source).unwrap().to_ir().unwrap();
    for route in ROUTES {
        dialogue(&mut cli(route, &file, 64), &dir);
    }
    for opt in ["-O0", "-O2"] {
        if let Some(exe) = compile_c(&dir, opt, &c::emit_c(&hir).unwrap(), opt) {
            dialogue(&mut Command::new(exe), &dir);
        }
    }
}

#[test]
fn copied_lines_short_circuit_and_argument_order_match_c() {
    let dir = directory("ownership");
    for (i,source) in [
        r#"import "std:io" as io;
        fn show()->void {a:io::ReadLine=io::read_line(4);b:io::ReadLine=a;values:[io::ReadLine;2]=[a,b];print(values);}
        for(n:i64 in [1,2,3]) {show();}"#,
        r#"import "std:io" as io;
        fn cap()->i64 {print("cap");return 4;}
        fn get()->io::ReadLine {return io::read_line(cap());}
        fn skip()->bool {x:io::ReadLine=get();return true;}
        fn pair(a:io::ReadLine,b:io::ReadLine)->void {print(a);print(b);}
        if false && skip() {print("bad");} pair(get(),get());"#,
    ].iter().enumerate() {
        let mut hir=modules::from_source(source).unwrap().to_ir().unwrap();hir.string_heap_limit=8;
        let bytes=b"a\nb\nc\n";
        let expected=expected(&hir,bytes);
        let file=dir.join("main.ceru");fs::write(&file,source).unwrap();
        for route in ROUTES {compare(file_run(&mut cli(route,&file,8),&dir,bytes),&expected);}
        if let Some(exe)=compile_c(&dir,&format!("ownership{i}"),&c::emit_c(&hir).unwrap(),"-O2") {compare(file_run(&mut Command::new(exe),&dir,bytes),&expected);}
    }
}

#[test]
fn generated_connections_distinguish_interrupts_failures_and_contract_violations() {
    let dir = directory("injection");
    let source = r#"import "std:io" as io;print("before");flush();value:io::ReadLine=io::read_line(4);print("after");"#;
    let hir = modules::from_source(source).unwrap().to_ir().unwrap();
    let mut c = c::emit_c(&hir).unwrap();
    c=c.replace("static int64_t cerune_input_read_byte(unsigned char *byte) {","static int64_t cerune_input_original_read_byte(unsigned char *byte) {")
        .replace("static const char *cerune_input_connection(void) {","static const char *cerune_input_original_connection(void) {")
        .replace("malloc(sizeof(cerune_string_owner) + (size_t)limit)","(getenv(\"TEST_ALLOC_FAIL\") ? NULL : malloc(sizeof(cerune_string_owner) + (size_t)limit))");
    c.push_str(
        r#"
static const char *cerune_input_connection(void) {
    if (getenv("TEST_UNSUPPORTED")) return "unsupported-connection";
    return cerune_input_original_connection();
}
static int64_t cerune_input_read_byte(unsigned char *byte) {
    static int first=1;
    static size_t consumed;
    if (first) {first=0;errno=EINTR;return -1;}
    const char *mode=getenv("TEST_MODE");
    if (mode && consumed==2) {
        if (*mode=='i') return 2;
        errno=EIO;return -1;
    }
    int64_t n=cerune_input_original_read_byte(byte);
    if(n>0) consumed+=(size_t)n;
    return n;
}
"#,
    );
    for opt in ["-O0", "-O2"] {
        let Some(exe) = compile_c(&dir, opt, &c, opt) else {
            continue;
        };
        for (mode, code, consumed, known) in [
            ("success", None, 0, true),
            ("error", Some("read-failed"), 2, true),
            ("invalid", Some("invalid-response"), 2, false),
            ("allocation", Some("allocation-failed"), 0, true),
            ("unsupported", Some("unsupported-connection"), 0, true),
        ] {
            let mut cmd = Command::new(&exe);
            match mode {
                "error" | "invalid" => {
                    cmd.env("TEST_MODE", mode);
                }
                "allocation" => {
                    cmd.env("TEST_ALLOC_FAIL", "1");
                }
                "unsupported" => {
                    cmd.env("TEST_UNSUPPORTED", "1");
                }
                _ => {}
            }
            let out = file_run(&mut cmd, &dir, b"abc\n");
            if let Some(code) = code {
                assert!(!out.status.success());
                assert_eq!(out.stdout, b"before\n");
                let stderr = String::from_utf8(out.stderr).unwrap();
                let (_, Some(reference)) = expected(&hir, b"SECRET\n") else {
                    panic!()
                };
                let expected = reference
                    .record()
                    .replace("code=limit-exceeded", &format!("code={code}"))
                    .replace("consumed=5", &format!("consumed={consumed}"))
                    .replace(
                        "consumption=known",
                        if known {
                            "consumption=known"
                        } else {
                            "consumption=unknown"
                        },
                    );
                assert_eq!(stderr.trim().trim_start_matches("cerune: "), expected);
            } else {
                compare(out, &expected(&hir, b"abc\n"));
            }
        }
    }
}

#[test]
fn input_stops_at_the_line_or_probe_without_hidden_read_ahead() {
    use std::io::Seek;
    let dir = directory("cursor");
    for (limit, budget, bytes, position) in [
        (4, 64, b"ab\nPRIVATE remaining bytes".as_slice(), 3),
        (2, 64, b"ab\nPRIVATE remaining bytes".as_slice(), 3),
        (4, 3, b"PRIVATE unread".as_slice(), 0),
    ] {
        let source = format!(
            "import \"std:io\" as io;value:io::ReadLine=io::read_line({limit});print(value);"
        );
        let file = dir.join("main.ceru");
        fs::write(&file, &source).unwrap();
        let mut hir = modules::from_source(&source).unwrap().to_ir().unwrap();
        hir.string_heap_limit = budget;
        let expected = expected(&hir, bytes);
        let mut commands: Vec<_> = ROUTES
            .iter()
            .map(|route| cli(route, &file, budget))
            .collect();
        if let Some(exe) = compile_c(
            &dir,
            &format!("cursor-{limit}-{budget}"),
            &c::emit_c(&hir).unwrap(),
            "-O2",
        ) {
            commands.push(Command::new(exe));
        }
        for mut cmd in commands {
            let input = dir.join("input.bin");
            fs::write(&input, bytes).unwrap();
            let mut observed = fs::File::open(input).unwrap();
            let out = process::bounded_output_with_stdin(
                &mut cmd,
                &dir,
                "cursor",
                Duration::from_secs(10),
                Stdio::from(observed.try_clone().unwrap()),
            )
            .unwrap();
            compare(out, &expected);
            assert_eq!(observed.stream_position().unwrap(), position);
        }
    }
}
