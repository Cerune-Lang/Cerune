use super::{execute, hir};
use crate::process;
use cerune_lang::{
    codegen::{c, llvm},
    runtime::files::{Directory, Files},
};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::Duration,
};
pub(super) fn run(dir: &Path, exe: &Path, args: &[&std::ffi::OsStr]) -> Output {
    process::bounded_output(
        Command::new(exe).args(args).current_dir(dir),
        dir,
        "files-generated-run",
        Duration::from_secs(20),
    )
    .unwrap()
}
fn connected(dir: &Path, exe: &Path, root: &Path) -> Output {
    run(
        dir,
        exe,
        &[
            "--cerune-runtime-v1".as_ref(),
            "--file-root".as_ref(),
            root.as_os_str(),
            "--".as_ref(),
        ],
    )
}
fn compare(program: &cerune_lang::ir::Program, actual: &Output, root: &Path) {
    for route in 0..4 {
        let mut files = Files::new(Directory::open(root).unwrap());
        match execute(route, program, &mut files) {
            Ok(output) => {
                assert!(
                    actual.status.success(),
                    "{}",
                    String::from_utf8_lossy(&actual.stderr)
                );
                assert_eq!(actual.stdout, output.as_bytes());
            }
            Err((failure, output)) => {
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, output.as_bytes());
                assert_eq!(
                    String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                    format!("cerune: {}\n", failure.record())
                );
            }
        }
    }
}
fn roundtrip_preserves_bytes_and_existing_files(route: Route) {
    let program = hir(include_str!("../../examples/files/roundtrip.ceru"));
    let c = route.emit(&program, Default::default());
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = route.build(&c, opt, &[]) else {
            return;
        };
        let root = dir.join("data 日本語");
        fs::create_dir(&root).unwrap();
        let actual = connected(&dir, &exe, &root);
        assert!(
            actual.status.success(),
            "{}",
            String::from_utf8_lossy(&actual.stderr)
        );
        assert_eq!(actual.stdout, b"true\n16\n");
        assert_eq!(
            fs::read(root.join("sample.txt")).unwrap(),
            "日本語\0\r\né\n".as_bytes()
        );
        compare(&program, &connected(&dir, &exe, &root), &root);
        assert_eq!(
            fs::read(root.join("sample.txt")).unwrap(),
            "日本語\0\r\né\n".as_bytes()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
fn read_failures_limits_and_source_locations_match_executors(route: Route) {
    for (bytes, limit) in [
        (b"abc".as_slice(), 3),
        (b"".as_slice(), 0),
        (b"a".as_slice(), 0),
        (b"abcd".as_slice(), 3),
        (b"\xff".as_slice(), 1),
    ] {
        let program = hir(&format!(
            r#"import "std:fs" as fs; print(7); text:string=fs::read_text("x",{limit}); print(byte_len(text)); print(8);"#
        ));
        let c = route.emit(&program, Default::default());
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = route.build(&c, opt, &[]) else {
                return;
            };
            let root = dir.join("data");
            fs::create_dir(&root).unwrap();
            fs::write(root.join("x"), bytes).unwrap();
            compare(&program, &connected(&dir, &exe, &root), &root);
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
fn runtime_marker_preserves_arguments_and_validates_settings(route: Route) {
    let program = hir(r#"import "std:fs" as fs; import "std:env" as env;
        fn unused()->void { fs::write_new("unused","x",1); }
        print(env::arg_count()); mut i:i64=0; while i<env::arg_count() { match env::arg(i) { env::Argument::Value { text: text } => { print(text); }, env::Argument::Missing {} => { print("missing"); } } i=i+1; }"#);
    let c = route.emit(&program, Default::default());
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = route.build(&c, opt, &[]) else {
            return;
        };
        for (args, expected) in [
            (vec!["--file-root", "missing"], "2\n--file-root\nmissing\n"),
            (
                vec!["x", "--cerune-runtime-v1", "--"],
                "3\nx\n--cerune-runtime-v1\n--\n",
            ),
            (
                vec![
                    "--cerune-runtime-v1",
                    "--",
                    "--cerune-runtime-v1",
                    "",
                    "--file-root",
                ],
                "3\n--cerune-runtime-v1\n\n--file-root\n",
            ),
            (vec!["--cerune-runtime-v1", "--"], "0\n"),
        ] {
            let args: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
            let actual = run(&dir, &exe, &args);
            assert!(actual.status.success(), "{actual:?}");
            assert_eq!(actual.stdout, expected.as_bytes());
        }
        for (args, code) in [
            (vec!["--cerune-runtime-v1"], "missing-delimiter"),
            (vec!["--cerune-runtime-v1", "--file-root"], "invalid-root"),
            (
                vec!["--cerune-runtime-v1", "--file-root", "", "--"],
                "invalid-root",
            ),
            (
                vec!["--cerune-runtime-v1", "--file-root", "--"],
                "invalid-root",
            ),
            (
                vec!["--cerune-runtime-v1", "--unknown", "--"],
                "unknown-setting",
            ),
            (
                vec![
                    "--cerune-runtime-v1",
                    "--file-root",
                    "missing",
                    "--file-root",
                    "x",
                    "--",
                ],
                "duplicate-setting",
            ),
            (
                vec!["--cerune-runtime-v1", "--file-root", "missing"],
                "missing-delimiter",
            ),
        ] {
            let args: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
            let actual = run(&dir, &exe, &args);
            assert_eq!(actual.status.code(), Some(1));
            assert!(actual.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&actual.stderr)
                    .contains(&format!("phase=configure code={code} origin=host")),
                "{actual:?}"
            );
        }
        let root = dir.join("data");
        fs::create_dir(&root).unwrap();
        let actual = run(
            &dir,
            &exe,
            &[
                "--cerune-runtime-v1".as_ref(),
                "--file-root".as_ref(),
                root.as_os_str(),
                "--".as_ref(),
                "日本語".as_ref(),
            ],
        );
        assert!(actual.status.success());
        assert_eq!(actual.stdout, "1\n日本語\n".as_bytes());
        assert!(!root.join("unused").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
fn names_budget_and_unconfigured_precede_open(route: Route) {
    for (name, limit, heap) in [
        ("../outside", 1, 64),
        ("CON.txt", 1, 64),
        ("COM¹", 1, 64),
        ("x", -1, 64),
        ("x", 65, 64),
        ("missing", 1, 64),
    ] {
        let mut program = hir(&format!(
            r#"import "std:fs" as fs; print(7); print(fs::read_text("{name}",{limit})); print(8);"#
        ));
        program.string_heap_limit = heap;
        let c = route.emit(&program, Default::default());
        let Some((dir, exe)) = route.build(&c, "-O2", &[]) else {
            return;
        };
        let root = dir.join("data");
        fs::create_dir(&root).unwrap();
        compare(&program, &connected(&dir, &exe, &root), &root);
        let actual = run(&dir, &exe, &[]);
        let mut files = Files::<Directory>::unconfigured();
        let (failure, output) = execute(0, &program, &mut files).unwrap_err();
        assert_eq!(actual.stdout, output.as_bytes());
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: {}\n", failure.record())
        );
        fs::remove_dir_all(dir).unwrap();
    }
}

fn ownership_recursion_short_circuit_and_order_match_executors(route: Route) {
    use super::Memory;
    let program = hir(r#"import "std:fs" as fs;
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
        print(values); fs::write_new("copy",original,3);"#);
    let c = route.emit(&program, Default::default());
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = route.build(&c, opt, &[]) else {
            return;
        };
        let root = dir.join("data");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("x"), b"abc").unwrap();
        let actual = connected(&dir, &exe, &root);
        assert!(actual.status.success(), "{actual:?}");
        for route in 0..4 {
            let memory = Memory::default();
            memory
                .0
                .borrow_mut()
                .files
                .insert("x".into(), b"abc".to_vec());
            let mut files = Files::new(memory.clone());
            assert_eq!(
                actual.stdout,
                execute(route, &program, &mut files).unwrap().as_bytes()
            );
            assert_eq!(
                fs::read(root.join("copy")).unwrap(),
                memory.0.borrow().files["copy"]
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
fn rejects_links_special_files_and_invalid_roots(route: Route) {
    let program = hir(r#"import "std:fs" as fs; print(7); print(fs::read_text("x",3)); print(8);"#);
    let c = route.emit(&program, Default::default());
    let Some((dir, exe)) = route.build(&c, "-O2", &[]) else {
        return;
    };
    let root = std::env::temp_dir().join(format!(
        "cerune-files-generated-{}",
        dir.file_name().unwrap().to_string_lossy()
    ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("x")).unwrap();
    compare(&program, &connected(&dir, &exe, &root), &root);
    fs::remove_dir(root.join("x")).unwrap();
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        std::os::unix::fs::symlink(root.join("missing"), root.join("x")).unwrap();
        compare(&program, &connected(&dir, &exe, &root), &root);
        fs::remove_file(root.join("x")).unwrap();
        let name = std::ffi::CString::new(root.join("x").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        compare(&program, &connected(&dir, &exe, &root), &root);
        fs::remove_file(root.join("x")).unwrap();
        std::os::unix::fs::symlink(&root, dir.join("link")).unwrap();
    }
    #[cfg(windows)]
    {
        let out = Command::new("cmd")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(root.join("x"))
            .arg(&dir)
            .output()
            .unwrap();
        assert!(out.status.success());
        compare(&program, &connected(&dir, &exe, &root), &root);
        fs::remove_dir(root.join("x")).unwrap();
        let out = Command::new("cmd")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(dir.join("link"))
            .arg(&root)
            .output()
            .unwrap();
        assert!(out.status.success());
    }
    for (bad_root, code) in [
        (dir.join("missing"), "not-found"),
        (dir.join("link"), "unsupported-target"),
    ] {
        let actual = connected(&dir, &exe, &bad_root);
        assert_eq!(actual.status.code(), Some(1));
        assert!(actual.stdout.is_empty());
        let err = String::from_utf8_lossy(&actual.stderr);
        assert!(
            err.contains(&format!("phase=open-root code={code} origin=host")),
            "{err}"
        );
    }
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn configuration_is_excluded_from_argument_snapshot_limits(route: Route) {
    let program = hir(r#"import "std:fs" as fs; import "std:env" as env;
        fn unused()->void { fs::write_new("x","",0); } print(env::arg_count());"#);
    let c = route.emit(
        &program,
        cerune_lang::runtime::arguments::Limits {
            max_count: 1,
            max_bytes: 1,
        },
    );
    let Some((dir, exe)) = route.build(
        &c,
        "-O2",
        &[
            "-DCERUNE_ARGUMENT_COUNT_LIMIT=1",
            "-DCERUNE_ARGUMENT_BYTE_LIMIT=1",
        ],
    ) else {
        return;
    };
    let actual = run(
        &dir,
        &exe,
        &[
            "--cerune-runtime-v1".as_ref(),
            "--file-root".as_ref(),
            dir.as_os_str(),
            "--".as_ref(),
            "x".as_ref(),
        ],
    );
    assert!(actual.status.success(), "{actual:?}");
    assert_eq!(actual.stdout, b"1\n");
    let actual = run(
        &dir,
        &exe,
        &[
            "--cerune-runtime-v1".as_ref(),
            "--file-root".as_ref(),
            dir.as_os_str(),
            "--".as_ref(),
            "xx".as_ref(),
        ],
    );
    assert_eq!(actual.status.code(), Some(1));
    assert!(actual.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&actual.stderr)
            .contains("arguments-v1 phase=capture code=byte-limit-exceeded")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[derive(Clone, Copy)]
enum Route {
    C,
    Llvm,
}
impl Route {
    fn emit(
        self,
        p: &cerune_lang::ir::Program,
        limits: cerune_lang::runtime::arguments::Limits,
    ) -> String {
        match self {
            Self::C => {
                c::emit_c_with_call_limit(p, std::num::NonZeroU32::new(128).unwrap()).unwrap()
            }
            Self::Llvm => llvm::emit_llvm_with_runtime_limits(
                p,
                llvm::Options {
                    target: Some(llvm_target()),
                    annotate_origins: true,
                },
                limits,
                std::num::NonZeroU32::new(128).unwrap(),
            )
            .unwrap(),
        }
    }
    fn build(
        self,
        text: &str,
        opt: &str,
        flags: &[&str],
    ) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        match self {
            Self::C => crate::program_arguments::c::build(text, opt, flags),
            Self::Llvm => crate::program_arguments::llvm::build(text, opt, None),
        }
    }
}
fn llvm_target() -> llvm::Target {
    if cfg!(windows) {
        llvm::Target::X86_64PcWindowsMsvc
    } else {
        llvm::Target::X86_64UnknownLinuxGnu
    }
}
macro_rules! cases {
    ($route:expr) => {
        #[test]
        fn root_handle_survives_move_and_composes_with_stdin_arguments() {
            super::root_handle_survives_move_and_composes_with_stdin_arguments($route);
        }
        #[test]
        fn argument_errors_precede_root_connection() {
            super::argument_errors_precede_root_connection($route);
        }
        #[test]
        fn unicode_names_and_empty_creation_use_exclusive_open() {
            super::unicode_names_and_empty_creation_use_exclusive_open($route);
        }
        #[test]
        fn roundtrip_preserves_bytes_and_existing_files() {
            super::roundtrip_preserves_bytes_and_existing_files($route);
        }
        #[test]
        fn read_failures_limits_and_source_locations_match_executors() {
            super::read_failures_limits_and_source_locations_match_executors($route);
        }
        #[test]
        fn runtime_marker_preserves_arguments_and_validates_settings() {
            super::runtime_marker_preserves_arguments_and_validates_settings($route);
        }
        #[test]
        fn names_budget_and_unconfigured_precede_open() {
            super::names_budget_and_unconfigured_precede_open($route);
        }
        #[test]
        fn ownership_recursion_short_circuit_and_order_match_executors() {
            super::ownership_recursion_short_circuit_and_order_match_executors($route);
        }
        #[test]
        fn rejects_links_special_files_and_invalid_roots() {
            super::rejects_links_special_files_and_invalid_roots($route);
        }
        #[test]
        fn configuration_is_excluded_from_argument_snapshot_limits() {
            super::configuration_is_excluded_from_argument_snapshot_limits($route);
        }
    };
}
mod c_tests {
    cases!(super::Route::C);
}
mod llvm_tests {
    cases!(super::Route::Llvm);
}

fn root_handle_survives_move_and_composes_with_stdin_arguments(route: Route) {
    use std::{io::Write, process::Stdio, thread, time::Instant};
    let program = hir(
        r#"import "std:fs" as fs; import "std:io" as io; import "std:env" as env;
        print("ready"); flush();
        line:infer=io::read_line(1);
        text:string=fs::read_text("x",3); fs::write_new("saved",text,3);
        print(text); print(env::arg_count());"#,
    );
    let text = route.emit(&program, Default::default());
    let Some((dir, exe)) = route.build(&text, "-O2", &[]) else {
        return;
    };
    let root = dir.join("root");
    let moved = dir.join("moved");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("x"), b"old").unwrap();
    let stdout = dir.join("stdout");
    let stderr = dir.join("stderr");
    let child = Command::new(exe)
        .current_dir(&dir)
        .arg("--cerune-runtime-v1")
        .arg("--file-root")
        .arg(&root)
        .arg("--")
        .arg("user-argument")
        .stdin(Stdio::piped())
        .stdout(fs::File::create(&stdout).unwrap())
        .stderr(fs::File::create(&stderr).unwrap())
        .spawn()
        .unwrap();
    // 検証失敗時も、この試験が起動した子だけを終了・回収します。
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(child);
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
    assert_eq!(fs::read(&stdout).unwrap(), b"ready\nold\n1\n");
    assert!(fs::read(&stderr).unwrap().is_empty());
    assert_eq!(fs::read(moved.join("saved")).unwrap(), b"old");
    assert!(!root.join("saved").exists());
    drop(child);
    fs::remove_dir_all(dir).unwrap();
}
fn argument_errors_precede_root_connection(route: Route) {
    let program =
        hir(r#"import "std:fs" as fs; fn unused()->void {fs::write_new("x","",0);} print(99);"#);
    let text = route.emit(
        &program,
        cerune_lang::runtime::arguments::Limits {
            max_count: 1,
            max_bytes: 1,
        },
    );
    let Some((dir, exe)) = route.build(
        &text,
        "-O2",
        &[
            "-DCERUNE_ARGUMENT_COUNT_LIMIT=1",
            "-DCERUNE_ARGUMENT_BYTE_LIMIT=1",
        ],
    ) else {
        return;
    };
    let root = dir.join("missing");
    let out = run(
        &dir,
        &exe,
        &[
            "--cerune-runtime-v1".as_ref(),
            "--file-root".as_ref(),
            root.as_os_str(),
            "--".as_ref(),
            "x".as_ref(),
            "y".as_ref(),
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&out.stderr).trim(),
        "cerune: arguments-v1 phase=capture code=count-limit-exceeded"
    );
    #[cfg(any(windows, target_os = "linux"))]
    {
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[0xd800])
        };
        #[cfg(target_os = "linux")]
        let invalid = {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(vec![0xff])
        };
        let out = run(
            &dir,
            &exe,
            &[
                "--cerune-runtime-v1".as_ref(),
                "--file-root".as_ref(),
                root.as_os_str(),
                "--".as_ref(),
                invalid.as_os_str(),
            ],
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&out.stderr)
                .contains("arguments-v1 phase=capture code=invalid-unicode"),
            "{out:?}"
        );
    }
    assert!(!root.exists());
    fs::remove_dir_all(dir).unwrap();
}
fn unicode_names_and_empty_creation_use_exclusive_open(route: Route) {
    let program = hir(r#"import "std:fs" as fs;
        fs::write_new("日本語-é.txt","",0);
        print(byte_len(fs::read_text("日本語-é.txt",0)));
        fs::write_new("日本語-é.txt","replacement",11);
        print("unreachable");"#);
    let text = route.emit(&program, Default::default());
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = route.build(&text, opt, &[]) else {
            return;
        };
        let root = dir.join("root");
        fs::create_dir(&root).unwrap();
        let out = connected(&dir, &exe, &root);
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(out.stdout, b"0\n");
        assert!(String::from_utf8_lossy(&out.stderr).contains("phase=open code=already-exists"));
        assert_eq!(fs::read(root.join("日本語-é.txt")).unwrap(), b"");
        fs::remove_dir_all(dir).unwrap();
    }
}
