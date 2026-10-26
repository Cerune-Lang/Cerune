use super::{execute, hir};
use crate::{process, program_arguments::c::build};
use cerune_lang::{
    codegen::c,
    runtime::files::{Directory, Files},
};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::Duration,
};
fn run(dir: &Path, exe: &Path, args: &[&std::ffi::OsStr]) -> Output {
    process::bounded_output(
        Command::new(exe).args(args).current_dir(dir),
        dir,
        "files-c-run",
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
#[test]
fn generated_c_roundtrip_preserves_bytes_and_existing_files() {
    let program = hir(include_str!("../../examples/files/roundtrip.ceru"));
    let c = c::emit_c(&program).unwrap();
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&c, opt, &[]) else {
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
#[test]
fn generated_c_read_failures_limits_and_source_locations_match_executors() {
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
        let c = c::emit_c(&program).unwrap();
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&c, opt, &[]) else {
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
#[test]
fn generated_c_runtime_marker_preserves_arguments_and_validates_settings() {
    let program = hir(r#"import "std:fs" as fs; import "std:env" as env;
        fn unused()->void { fs::write_new("unused","x",1); }
        print(env::arg_count()); mut i:i64=0; while i<env::arg_count() { match env::arg(i) { env::Argument::Value { text: text } => { print(text); }, env::Argument::Missing {} => { print("missing"); } } i=i+1; }"#);
    let c = c::emit_c(&program).unwrap();
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&c, opt, &[]) else {
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
#[test]
fn generated_c_names_budget_and_unconfigured_precede_open() {
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
        let c = c::emit_c(&program).unwrap();
        let Some((dir, exe)) = build(&c, "-O2", &[]) else {
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

#[test]
fn generated_c_injected_transfers_and_close_failures_match_common_runtime() {
    use super::{Memory, Step};
    use cerune_lang::runtime::files::OpenError;
    for writing in [false, true] {
        let source = if writing {
            r#"import "std:fs" as fs; print("before"); fs::write_new("x","abc",3); print("after");"#
        } else {
            r#"import "std:fs" as fs; print("before"); print(fs::read_text("x",3)); print("after");"#
        };
        let program = hir(source);
        let generated = c::emit_c(&program).unwrap();
        let end = generated
            .find("/* Selected by the C compilation target")
            .unwrap();
        let text = format!(
            "#define CERUNE_FILE_TEST_CONNECTION\n{}\n{}",
            &generated[..end],
            TRANSFER_HOST
        );
        for opt in ["-O0", "-O2"] {
            let Some((dir, exe)) = build(&text, opt, &[]) else {
                return;
            };
            for case in 0..7 {
                let actual = run(&dir, &exe, &[case.to_string().as_ref()]);
                for route in 0..4 {
                    let memory = Memory::default();
                    {
                        let mut state = memory.0.borrow_mut();
                        if !writing {
                            state.files.insert("x".into(), b"abc".to_vec());
                        }
                        let steps = if writing {
                            &mut state.writes
                        } else {
                            &mut state.reads
                        };
                        steps.extend([Step::Interrupted, Step::Count(1)]);
                        match case {
                            1 | 5 => steps.push_back(Step::Fail),
                            2 => steps.push_back(Step::Invalid),
                            3 if writing => steps.push_back(Step::Zero),
                            _ => steps.extend([Step::Count(1), Step::Count(1)]),
                        }
                        state.close_error = case == 4 || case == 5;
                        if case == 6 {
                            state.open_error = Some(OpenError::InvalidResponse);
                        }
                    }
                    let mut files = Files::new(memory.clone());
                    match execute(route, &program, &mut files) {
                        Ok(output) => {
                            assert!(actual.status.success(), "{actual:?}");
                            assert_eq!(actual.stdout, output.as_bytes());
                        }
                        Err((failure, output)) => {
                            assert_eq!(actual.status.code(), Some(1));
                            assert_eq!(actual.stdout, output.as_bytes());
                            assert_eq!(
                                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                                format!("cerune: {}\n", failure.record()),
                                "write={writing} case={case}"
                            );
                        }
                    }
                    if case != 6 {
                        assert_eq!(fs::read(dir.join("closes")).unwrap(), b"1");
                        if writing {
                            assert_eq!(
                                fs::read(dir.join("accepted")).unwrap(),
                                memory.0.borrow().files["x"]
                            );
                        }
                    }
                }
            }
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
const TRANSFER_HOST: &str = r#"
static int case_id, step, closes;
static size_t transferred;
static unsigned char accepted[3];
static const char *cerune_file_open(cerune_string name, int create, intptr_t *handle) {
    (void)name; (void)create; *handle=42;
    return case_id==6 ? "invalid-response" : NULL;
}
static int64_t transfer(size_t request, int writing) {
    if (++step==1) {errno=EINTR;return -1;}
    if (step==3) {
        if (case_id==1||case_id==5) {errno=EIO;return -1;}
        if (case_id==2) return (int64_t)request+1;
        if (case_id==3&&writing) return 0;
    }
    return transferred==3 ? 0 : 1;
}
static int64_t cerune_file_read(intptr_t handle,unsigned char *data,size_t length) {
    if (handle!=42) abort();
    int64_t count=transfer(length,0);
    if (count==1) { *data=((const unsigned char *)"abc")[transferred++]; }
    return count;
}
static int64_t cerune_file_write(intptr_t handle,const unsigned char *data,size_t length) {
    if (handle!=42) abort();
    int64_t count=transfer(length,1);
    if (count==1) accepted[transferred++]=*data;
    return count;
}
static int cerune_file_close(intptr_t handle) {
    if (handle!=42||++closes!=1) abort();
    FILE *f=fopen("closes","wb");if(!f)abort();fputs("1",f);fclose(f);
    f=fopen("accepted","wb");if(!f)abort();fwrite(accepted,1,transferred,f);fclose(f);
    return case_id==4||case_id==5 ? -1 : 0;
}
int main(int argc,char **argv) {
    case_id=argc>1?atoi(argv[1]):0; cerune_file_configured=1; return cerune_program();
}
"#;

#[test]
fn generated_c_ownership_recursion_short_circuit_and_order_match_executors() {
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
    let c = c::emit_c_with_call_limit(&program, std::num::NonZeroU32::new(128).unwrap()).unwrap();
    for opt in ["-O0", "-O2"] {
        let Some((dir, exe)) = build(&c, opt, &[]) else {
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
#[test]
fn generated_c_rejects_links_special_files_and_invalid_roots() {
    let program = hir(r#"import "std:fs" as fs; print(7); print(fs::read_text("x",3)); print(8);"#);
    let c = c::emit_c(&program).unwrap();
    let Some((dir, exe)) = build(&c, "-O2", &[]) else {
        return;
    };
    let root = std::env::temp_dir().join(format!(
        "cerune-files-c-{}",
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

#[test]
fn generated_c_configuration_is_excluded_from_argument_snapshot_limits() {
    let program = hir(r#"import "std:fs" as fs; import "std:env" as env;
        fn unused()->void { fs::write_new("x","",0); } print(env::arg_count());"#);
    let c = c::emit_c(&program).unwrap();
    let Some((dir, exe)) = build(
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
