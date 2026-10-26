use crate::process;
use cerune_lang::runtime::files::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "file-os-{}-{} 日本語",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        Self(base)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(temp: &Temp, source: &str, route: &[&str], root: Option<&Path>) -> std::process::Output {
    let source_path = temp.0.join("program.ceru");
    fs::write(&source_path, source).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg(route[0]).arg(&source_path).args(&route[1..]);
    cmd.args(["--diagnostic-format", "runtime-v1"]);
    if let Some(root) = root {
        cmd.arg("--file-root").arg(root);
    }
    process::bounded_output(&mut cmd, &temp.0, "file-os", Duration::from_secs(20)).unwrap()
}
const ROUTES: &[&[&str]] = &[
    &["run"],
    &["run-ir"],
    &["run-mir"],
    &["run-mir", "--ssa"],
    &["run-vm"],
];
#[test]
fn cli_roundtrip_example_and_exclusive_create_match_all_routes() {
    for route in ROUTES {
        let temp = Temp::new();
        let source = include_str!("../../examples/files/roundtrip.ceru");
        let out = run(&temp, source, route, Some(&temp.0));
        assert!(
            out.status.success(),
            "{route:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, b"true\n16\n");
        assert_eq!(
            fs::read(temp.0.join("sample.txt")).unwrap(),
            "日本語\0\r\né\n".as_bytes()
        );
        let out = run(&temp, source, route, Some(&temp.0));
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(
            err.contains("code=already-exists") && err.contains("created=no"),
            "{err}"
        );
        assert_eq!(
            fs::read(temp.0.join("sample.txt")).unwrap(),
            "日本語\0\r\né\n".as_bytes()
        );
    }
}
#[test]
fn read_boundaries_failures_origins_and_prior_output_match_all_routes() {
    for (bytes, limit, code) in [
        (b"abc".as_slice(), 3, None),
        (b"".as_slice(), 0, None),
        (b"a".as_slice(), 0, Some("limit-exceeded")),
        (b"abcd".as_slice(), 3, Some("limit-exceeded")),
        (b"\xff".as_slice(), 1, Some("invalid-utf8")),
    ] {
        let mut expected = None;
        for route in ROUTES {
            let temp = Temp::new();
            fs::write(temp.0.join("input.txt"), bytes).unwrap();
            let src = format!(
                "import \"std:fs\" as fs; print(7); text: string = fs::read_text(\"input.txt\", {limit}); print(byte_len(text)); print(8);"
            );
            let out = run(&temp, &src, route, Some(&temp.0));
            assert_eq!(
                out.status.success(),
                code.is_none(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            if let Some(code) = code {
                assert_eq!(out.stdout, b"7\n");
                let err = String::from_utf8_lossy(&out.stderr);
                assert!(
                    err.contains(&format!("code={code}"))
                        && err.contains("node=")
                        && err.contains("file=")
                        && err.contains("bytes="),
                    "{err}"
                );
                assert!(!err.contains(temp.0.to_str().unwrap()));
            } else {
                assert_eq!(out.stdout, format!("7\n{}\n8\n", bytes.len()).as_bytes());
            }
            let result = (out.stdout, out.stderr);
            if let Some(expected) = &expected {
                assert_eq!(expected, &result);
            } else {
                expected = Some(result);
            }
        }
    }
}
#[test]
fn startup_failure_unconfigured_and_invalid_flags_have_no_file_effects() {
    let temp = Temp::new();
    let source = "import \"std:fs\" as fs; print(7); fs::write_new(\"new.txt\", \"x\", 1);";
    for route in ROUTES {
        let out = run(&temp, source, route, Some(&temp.0.join("missing")));
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("origin=host"));
        let out = run(&temp, source, route, None);
        assert_eq!(out.stdout, b"7\n");
        assert!(String::from_utf8_lossy(&out.stderr).contains("code=unconfigured"));
    }
    for extra in [
        vec!["--file-root"],
        vec!["--file-root", "", "--file-root", "x"],
        vec!["--file-root", ".", "--file-root", "."],
    ] {
        let out = run(&temp, source, &[&["run"], extra.as_slice()].concat(), None);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    assert!(!temp.0.join("new.txt").exists());
}
#[test]
fn directory_handle_survives_rename_and_replacement() {
    let temp = Temp::new();
    let root = temp.0.join("root");
    let moved = temp.0.join("moved");
    fs::create_dir(&root).unwrap();
    let mut directory = Directory::open(&root).unwrap();
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    let mut file = directory.open("anchored.txt", Operation::WriteNew).unwrap();
    std::io::Write::write_all(&mut file, b"anchored").unwrap();
    file.close().unwrap();
    assert_eq!(fs::read(moved.join("anchored.txt")).unwrap(), b"anchored");
    assert!(!root.join("anchored.txt").exists());
    for name in ["../escape", "/absolute", "CON", "x:y", "a\\b"] {
        assert!(directory.open(name, Operation::WriteNew).is_err());
    }
}
#[test]
fn competing_new_writes_have_one_winner() {
    let temp = Temp::new();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let path = temp.0.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut directory = Directory::open(&path).unwrap();
                barrier.wait();
                match directory.open("winner", Operation::WriteNew) {
                    Ok(mut file) => {
                        std::io::Write::write_all(&mut file, &[i]).unwrap();
                        file.close().unwrap();
                        Some(i)
                    }
                    Err(OpenError::AlreadyExists) => None,
                    Err(error) => panic!("{error:?}"),
                }
            })
        })
        .collect();
    let winners: Vec<_> = threads
        .into_iter()
        .filter_map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(winners.len(), 1);
    assert_eq!(fs::read(temp.0.join("winner")).unwrap(), winners);
}
fn rejected_target(temp: &Temp, name: &str) {
    let source = format!("import \"std:fs\" as fs; print(fs::read_text(\"{name}\", 16));");
    for route in ROUTES {
        let out = run(temp, &source, route, Some(&temp.0));
        assert!(!out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("code=unsupported-target"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let mut directory = Directory::open(&temp.0).unwrap();
    assert!(matches!(
        directory.open(name, Operation::WriteNew),
        Err(OpenError::AlreadyExists)
    ));
}
#[test]
fn directories_and_links_are_not_followed() {
    let temp = Temp::new();
    let outside = Temp::new();
    fs::create_dir(temp.0.join("directory")).unwrap();
    rejected_target(&temp, "directory");
    fs::write(outside.0.join("secret"), b"unchanged").unwrap();
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(outside.0.join("secret"), temp.0.join("link")).unwrap();
        rejected_target(&temp, "link");
        std::os::unix::fs::symlink(outside.0.join("missing"), temp.0.join("dangling")).unwrap();
        rejected_target(&temp, "dangling");
        std::os::unix::fs::symlink(&outside.0, temp.0.join("junction")).unwrap();
    }
    #[cfg(windows)]
    {
        let status = Command::new("cmd")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(temp.0.join("junction"))
            .arg(&outside.0)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    rejected_target(&temp, "junction");
    assert!(matches!(
        Directory::open(&temp.0.join("junction")),
        Err(ConnectError::UnsupportedTarget)
    ));
    assert_eq!(fs::read(outside.0.join("secret")).unwrap(), b"unchanged");
}
#[cfg(target_os = "linux")]
#[test]
fn fifo_is_rejected_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt;
    let temp = Temp::new();
    let name = std::ffi::CString::new(temp.0.join("fifo").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    rejected_target(&temp, "fifo");
    let out = run(&temp, "print(1);", &["run"], Some(&temp.0.join("fifo")));
    assert!(String::from_utf8_lossy(&out.stderr).contains("code=unsupported-target"));
}

#[test]
fn accepted_names_empty_creation_and_missing_file_preserve_bytes() {
    use std::io::{Read, Write};
    let temp = Temp::new();
    let mut directory = Directory::open(&temp.0).unwrap();
    for name in [
        "日本語",
        "é",
        "é",
        "🌱",
        ".hidden",
        "has space",
        &"x".repeat(255),
    ] {
        let mut file = directory.open(name, Operation::WriteNew).unwrap();
        file.write_all(b"\0\r\n").unwrap();
        file.close().unwrap();
        let mut file = directory.open(name, Operation::ReadText).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        file.close().unwrap();
        assert_eq!(bytes, b"\0\r\n");
    }
    directory
        .open("empty", Operation::WriteNew)
        .unwrap()
        .close()
        .unwrap();
    assert!(fs::read(temp.0.join("empty")).unwrap().is_empty());
    assert!(matches!(
        directory.open("missing", Operation::ReadText),
        Err(OpenError::NotFound)
    ));
    assert!(matches!(
        Directory::open(&temp.0.join("empty")),
        Err(ConnectError::UnsupportedTarget)
    ));
    assert!(matches!(
        Directory::open(Path::new("")),
        Err(ConnectError::InvalidRoot)
    ));
}
#[test]
fn file_root_is_only_an_execution_option_before_argument_delimiter() {
    let temp = Temp::new();
    let source = temp.0.join("program.ceru");
    fs::write(&source, "print(9);").unwrap();
    for route in ROUTES {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
        cmd.arg(route[0])
            .arg(&source)
            .args(&route[1..])
            .args(["--", "--file-root", "missing"]);
        let out = process::bounded_output(
            &mut cmd,
            &temp.0,
            "argument-delimiter",
            Duration::from_secs(20),
        )
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, b"9\n");
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg("emit-ir")
        .arg(&source)
        .arg("--file-root")
        .arg(&temp.0);
    let out = process::bounded_output(
        &mut cmd,
        &temp.0,
        "reject-emit-option",
        Duration::from_secs(20),
    )
    .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}
