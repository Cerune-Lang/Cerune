#![cfg(any(windows, target_os = "linux"))]
use cerune_lang as language;
use language::codegen::wat;
use language::runtime::files::{Backend, Directory, Files as RuntimeFiles};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
#[path = "../../../tests/support/process.rs"]
mod process;
#[allow(dead_code)]
#[path = "../../../tests/file_io/support.rs"]
mod support;
#[path = "../src/files.rs"]
mod transport;
use support::{Memory, Step, execute, hir};
use transport::Files;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "cerune-wat-os-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn root(&self, name: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::create_dir(&p).unwrap();
        p
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn module(t: &Temp, p: &language::ir::Program) -> PathBuf {
    let text = wat::emit_wat(p).unwrap();
    assert!(!text.contains("(export \"memory\""));
    let path = t.0.join("program.wat");
    fs::write(&path, text).unwrap();
    path
}
fn run(t: &Temp, path: &Path, root: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_cerune-wat-host"));
    c.arg(path);
    if let Some(root) = root {
        c.arg("--file-root").arg(root);
    }
    c.arg("--");
    process::bounded_output(&mut c, &t.0, "wat-os", Duration::from_secs(30)).unwrap()
}
fn compare(source: &str, initial: &[(&str, &[u8])], configured: bool) {
    compare_program(hir(source), initial, configured);
}
fn compare_program(p: language::ir::Program, initial: &[(&str, &[u8])], configured: bool) {
    let t = Temp::new();
    let before = p.clone();
    let path = module(&t, &p);
    let root = t.root("host");
    for (name, bytes) in initial {
        fs::write(root.join(name), bytes).unwrap();
    }
    let actual = run(&t, &path, configured.then_some(root.as_path()));
    for route in 0..4 {
        let memory = Memory::default();
        for (name, bytes) in initial {
            memory
                .0
                .borrow_mut()
                .files
                .insert((*name).into(), bytes.to_vec());
        }
        let mut files = if configured {
            RuntimeFiles::new(memory.clone())
        } else {
            RuntimeFiles::unconfigured()
        };
        match execute(route, &p, &mut files) {
            Ok(out) => {
                assert!(actual.status.success(), "{actual:?}");
                assert_eq!(actual.stdout, out.as_bytes());
                assert!(actual.stderr.is_empty());
            }
            Err((failure, out)) => {
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, out.as_bytes());
                assert_eq!(
                    String::from_utf8_lossy(&actual.stderr),
                    format!("cerune: {}\n", failure.record())
                );
            }
        }
        let s = memory.0.borrow();
        for (name, bytes) in &s.files {
            assert_eq!(fs::read(root.join(name)).unwrap(), *bytes);
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), s.files.len());
    }
    assert_eq!(p, before);
}
#[test]
fn real_roundtrip_empty_unicode_and_exclusive_create_match_executors() {
    compare(
        include_str!("../../../examples/files/roundtrip.ceru"),
        &[],
        true,
    );
    compare(
        include_str!("../../../examples/files/roundtrip.ceru"),
        &[("sample.txt", b"keep")],
        true,
    );
    compare(
        r#"import "std:fs" as fs; fs::write_new("空.txt","",0); print(fs::read_text("空.txt",0));"#,
        &[],
        true,
    );
    compare(
        r#"import "std:fs" as fs; print(fs::read_text("日本語.txt",16));"#,
        &[("日本語.txt", "日本語\0\r\ne\u{301}\n".as_bytes())],
        true,
    );
}
#[test]
fn real_read_errors_limits_budget_and_unconfigured_match_executors() {
    for (bytes, limit) in [
        (b"abc".as_slice(), 2),
        (b"abc", 3),
        (b"", 0),
        (b"x", 0),
        (b"\xff", 1),
    ] {
        compare(
            &format!(
                "import \"std:fs\" as fs; print(7); print(fs::read_text(\"x\",{limit})); print(99);"
            ),
            &[("x", bytes)],
            true,
        );
    }
    compare(
        r#"import "std:fs" as fs; print(fs::read_text("missing",3));"#,
        &[],
        true,
    );
    compare(
        r#"import "std:fs" as fs; print(fs::read_text("../outside",1));"#,
        &[],
        true,
    );
    compare(
        r#"import "std:fs" as fs; print(fs::read_text("x",-1));"#,
        &[],
        true,
    );
    compare(
        include_str!("../../../examples/files/roundtrip.ceru"),
        &[],
        false,
    );
    let mut p = hir(r#"import "std:fs" as fs; print(7); print(fs::read_text("x",8)); print(99);"#);
    p.string_heap_limit = 7;
    compare_program(p, &[("x", b"abc")], true);
    let long = vec![b'x'; 5000];
    compare(
        r#"import "std:fs" as fs; x:string=fs::read_text("long",5000); print(byte_len(x)); fs::write_new("copy",x,5000);"#,
        &[("long", &long)],
        true,
    );
}
#[test]
fn output_values_arguments_and_stdin_compose_with_file_operations() {
    let t = Temp::new();
    // Shared float formatting and aggregate/boolean output remain available.
    let p = hir(include_str!(
        "../../../examples/io_protocol/output_values.ceru"
    ));
    let actual = run(&t, &module(&t, &p), None);
    assert!(actual.status.success(), "{actual:?}");
    assert_eq!(
        actual.stdout,
        language::ir_executor::run(&p).unwrap().as_bytes()
    );
    let source = r#"import "std:fs" as fs; import "std:env" as env; import "std:io" as io;
print(env::arg_count()); print(env::arg(1)); print("ready"); flush();
match io::read_line(32) { io::ReadLine::Line { text: text } => {fs::write_new("input",text,32); print(fs::read_text("input",32));}, io::ReadLine::End {} => {print("eof");}, }"#;
    let p = hir(source);
    let path = module(&t, &p);
    let root = t.root("data");
    let input = t.0.join("stdin");
    fs::write(&input, "日本語\r\n").unwrap();
    let actual = process::bounded_output_with_stdin(
        Command::new(env!("CARGO_BIN_EXE_cerune-wat-host"))
            .arg(path)
            .arg("--file-root")
            .arg(&root)
            .args(["--", "--file-root", "--cerune-runtime-v1", "日本語"]),
        &t.0,
        "compose",
        Duration::from_secs(30),
        fs::File::open(input).unwrap().into(),
    )
    .unwrap();
    assert!(actual.status.success(), "{actual:?}");
    assert_eq!(
        fs::read(root.join("input")).unwrap(),
        "日本語\r\n".as_bytes()
    );
    assert!(String::from_utf8_lossy(&actual.stdout).contains("--cerune-runtime-v1"));
    assert!(String::from_utf8_lossy(&actual.stdout).contains("ready\n"));
}
fn name<B: Backend>(f: &mut Files<B>, name: &str) {
    assert_eq!(f.begin(name.len() as i32).unwrap(), 0);
    for chunk in name.as_bytes().chunks(8) {
        let mut bytes = [0; 8];
        bytes[..chunk.len()].copy_from_slice(chunk);
        assert_eq!(
            f.chunk(i64::from_le_bytes(bytes), chunk.len() as i32)
                .unwrap(),
            0
        );
    }
}
#[test]
fn transport_preserves_all_bytes_partial_transfers_and_close_consumption() {
    let memory = Memory::default();
    memory
        .0
        .borrow_mut()
        .files
        .insert("all".into(), (0..=255).collect());
    let mut f = Files::new(Some(memory.clone()));
    name(&mut f, "all");
    let (h, s) = f.open(0).unwrap();
    assert_eq!(s, 0);
    for chunk in (0..=255u8).collect::<Vec<_>>().chunks(8) {
        let (bits, n) = f.read(h, 8).unwrap();
        assert_eq!(n, 8);
        assert_eq!(&bits.to_le_bytes(), chunk);
    }
    assert_eq!(f.read(h, 1).unwrap(), (0, 0));
    assert_eq!(f.close(h).unwrap(), 0);
    assert!(f.close(h).is_err());
    name(&mut f, "write");
    let (h, s) = f.open(1).unwrap();
    assert_eq!(s, 0);
    memory.0.borrow_mut().writes.extend([
        Step::Interrupted,
        Step::Count(2),
        Step::Fail,
        Step::Invalid,
        Step::Zero,
    ]);
    for expected in [-2, 2, -1, 9, 0] {
        assert_eq!(f.write(h, -1, 8).unwrap(), expected);
    }
    memory.0.borrow_mut().close_error = true;
    assert_eq!(f.close(h).unwrap(), -1);
    assert!(f.close(h).is_err());
    assert_eq!(memory.0.borrow().files["write"], [255, 255]);
    let calls = memory.0.borrow().calls.clone();
    f.shutdown();
    assert_eq!(memory.0.borrow().calls, calls);
    assert!(f.status().is_err());
}
#[test]
fn transport_rejects_protocol_errors_and_reclaims_live_handles_once() {
    let memory = Memory::default();
    let mut f = Files::new(Some(memory.clone()));
    assert!(f.open(1).is_err());
    assert!(f.begin(256).is_err());
    assert!(f.chunk(0, 0).is_err());
    f.begin(2).unwrap();
    f.chunk(65, 1).unwrap();
    assert!(f.open(1).is_err());
    assert!(memory.0.borrow().files.is_empty());
    name(&mut f, "a");
    assert!(f.open(2).is_err());
    name(&mut f, "a");
    let (h, s) = f.open(1).unwrap();
    assert_eq!(s, 0);
    assert!(f.read(h, 1).is_err());
    assert!(f.write(h, 0, 9).is_err());
    assert!(f.close(h + 1).is_err());
    drop(f);
    assert_eq!(
        memory
            .0
            .borrow()
            .calls
            .iter()
            .filter(|c| *c == "close")
            .count(),
        1
    );
}
#[test]
fn retained_root_and_special_file_rejection_use_the_shared_os_connection() {
    let t = Temp::new();
    let root = t.root("root");
    let outside = t.root("outside");
    fs::write(outside.join("secret"), b"outside").unwrap();
    let mut f = Files::new(Some(Directory::open(&root).unwrap()));
    let moved = t.0.join("moved");
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    name(&mut f, "saved");
    let (h, s) = f.open(1).unwrap();
    assert_eq!(s, 0);
    assert_eq!(f.write(h, 65, 1).unwrap(), 1);
    assert_eq!(f.close(h).unwrap(), 0);
    assert_eq!(fs::read(moved.join("saved")).unwrap(), b"A");
    assert!(!root.join("saved").exists());
    name(&mut f, "../outside/secret");
    assert_eq!(f.open(0).unwrap().1, 4);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.join("secret"), moved.join("link")).unwrap();
        name(&mut f, "link");
        assert_eq!(f.open(0).unwrap().1, 3);
    }
    #[cfg(windows)]
    {
        let result = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(moved.join("junction"))
            .arg(&outside)
            .output()
            .unwrap();
        assert!(result.status.success());
        name(&mut f, "junction");
        assert_eq!(f.open(0).unwrap().1, 3);
        fs::remove_dir(moved.join("junction")).unwrap();
    }
    fs::create_dir(moved.join("directory")).unwrap();
    name(&mut f, "directory");
    assert_eq!(f.open(0).unwrap().1, 3);
    assert_eq!(fs::read(outside.join("secret")).unwrap(), b"outside");
}
#[test]
fn host_rejects_bad_settings_and_public_memory_without_success_fallback() {
    let t = Temp::new();
    let path = t.0.join("public.wat");
    fs::write(
        &path,
        r#"(module (memory (export "memory") 1) (func (export "main")))"#,
    )
    .unwrap();
    let actual = run(&t, &path, None);
    assert!(!actual.status.success());
    assert!(String::from_utf8_lossy(&actual.stderr).contains("public memory"));
    for args in [
        vec![],
        vec!["--file-root"],
        vec!["--unknown", "--"],
        vec!["--file-root", "x", "--file-root", "y", "--"],
    ] {
        let actual = process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune-wat-host"))
                .arg(&path)
                .args(args),
            &t.0,
            "invalid",
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(!actual.status.success());
    }
}

#[test]
fn evaluation_short_circuit_recursion_and_owned_copies_match_executors() {
    compare(
        r#"
        import "std:fs" as fs;
        fn name()->string {print(1);return "x";}
        fn limit()->i64 {print(2);return 3;}
        fn read(n:i64)->string {
            text:string=fs::read_text(name(),limit());
            if n==0 {return text;}
            return read(n-1);
        }
        print(false && fs::read_text("missing",1)=="x");
        mut text:string=read(2); original:string=text;
        text=concat(text,"d"); values:[string;2]=[original,text];
        print(values); fs::write_new("copy",original,3);
    "#,
        &[("x", b"abc")],
        true,
    );
}
#[test]
fn wasm_keeps_root_across_rename_while_waiting_for_input() {
    use std::{io::Write, process::Stdio, thread, time::Instant};
    let t = Temp::new();
    let p = hir(r#"import "std:fs" as fs; import "std:io" as io;
        print("ready"); flush(); line:infer=io::read_line(1);
        text:string=fs::read_text("x",3); fs::write_new("saved",text,3); print(text);"#);
    let path = module(&t, &p);
    let root = t.root("root");
    let moved = t.0.join("moved");
    fs::write(root.join("x"), b"old").unwrap();
    let stdout = t.0.join("stdout");
    let stderr = t.0.join("stderr");
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(env!("CARGO_BIN_EXE_cerune-wat-host"))
            .arg(path)
            .arg("--file-root")
            .arg(&root)
            .arg("--")
            .stdin(Stdio::piped())
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    while fs::read(&stdout).unwrap() != b"ready\n" {
        assert!(
            Instant::now() < deadline,
            "waiting for ready: {:?}",
            fs::read(&stderr)
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "{:?}",
            fs::read(&stderr)
        );
        thread::sleep(Duration::from_millis(10));
    }
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("x"), b"new").unwrap();
    child.0.stdin.take().unwrap().write_all(b"\n").unwrap();
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "{:?}", fs::read(&stderr));
            break;
        }
        assert!(Instant::now() < deadline, "waiting for read");
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(&stdout).unwrap(), b"ready\nold\n");
    assert!(fs::read(&stderr).unwrap().is_empty());
    assert_eq!(fs::read(moved.join("saved")).unwrap(), b"old");
    assert!(!root.join("saved").exists());
}
#[test]
fn missing_imports_and_engine_failures_remain_explicit() {
    let t = Temp::new();
    let path = t.0.join("invalid.wat");
    for (source, expected) in [
        (
            r#"(module (import "cerune" "unknown_v1" (func)) (func (export "main")))"#,
            "instantiation failed",
        ),
        (
            r#"(module (func (export "main") unreachable))"#,
            "Wasm execution failed",
        ),
        (
            r#"(module (import "cerune" "file_close_v1" (func $close (param i32) (result i32))) (func (export "main") i32.const 1 call $close drop))"#,
            "invalid file host protocol",
        ),
    ] {
        fs::write(&path, source).unwrap();
        let actual = run(&t, &path, None);
        assert_eq!(actual.status.code(), Some(1));
        assert!(actual.stdout.is_empty());
        let error = String::from_utf8_lossy(&actual.stderr);
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("file-v1"), "{error}");
    }
}
