use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

#[path = "../support/crash_dialogs.rs"]
mod crash_dialogs;

fn execute(command: &mut Command, dir: &Path, name: &str) -> Output {
    super::process::bounded_output(command, dir, name, Duration::from_secs(90)).unwrap()
}

pub(super) fn compile(dir: &Path, src: &Path, flags: &[&str]) -> Option<PathBuf> {
    let cc = std::env::var_os("CERUNE_TEST_CC")?;
    crash_dialogs::suppress();
    fs::create_dir_all(dir).unwrap();
    let c = dir.join("program.c");
    let exe = dir.join(format!("program{}", std::env::consts::EXE_SUFFIX));
    let emit = execute(
        Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-c")
            .arg(src)
            .arg("--with-trace")
            .args(flags)
            .arg("-o")
            .arg(&c),
        dir,
        "trace/emit",
    );
    assert!(
        emit.status.success(),
        "{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let build = execute(
        Command::new(cc)
            .args(["-std=c11", "-O2"])
            .arg(&c)
            .arg("-o")
            .arg(&exe),
        dir,
        "trace/compile",
    );
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    Some(exe)
}

pub(super) fn run(exe: &Path, args: &[&str]) -> Output {
    execute(
        Command::new(exe).args(args),
        exe.parent().unwrap(),
        "trace/run",
    )
}
