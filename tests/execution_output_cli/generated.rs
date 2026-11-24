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

#[derive(Clone, Copy, Debug)]
pub(super) enum Route {
    C,
    Llvm,
}

impl Route {
    pub(super) fn compile(self, dir: &Path, src: &Path, flags: &[&str]) -> Option<PathBuf> {
        let (var, command, extension) = match self {
            Self::C => ("CERUNE_TEST_CC", "emit-c", "c"),
            Self::Llvm => ("CERUNE_TEST_LLVM_CLANG", "emit-llvm", "ll"),
        };
        let cc = std::env::var_os(var)?;
        let mut emit = Command::new(env!("CARGO_BIN_EXE_cerune"));
        emit.arg(command).arg(src);
        if matches!(self, Self::Llvm) {
            let target = match (std::env::consts::ARCH, std::env::consts::OS) {
                ("x86_64", "windows") => "x86_64-pc-windows-msvc",
                ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
                _ => return None,
            };
            emit.args(["--target", target, "--annotate-origins"]);
        }
        crash_dialogs::suppress();
        fs::create_dir_all(dir).unwrap();
        let artifact = dir.join(format!("program.{extension}"));
        let exe = dir.join(format!("program{}", std::env::consts::EXE_SUFFIX));
        let emit = execute(
            emit.arg("--with-trace")
                .args(flags)
                .arg("-o")
                .arg(&artifact),
            dir,
            "trace/emit",
        );
        assert!(
            emit.status.success(),
            "{}",
            String::from_utf8_lossy(&emit.stderr)
        );
        let mut build = Command::new(cc);
        build.args(["-std=c11", "-O2"]).arg(&artifact);
        if matches!(self, Self::Llvm) {
            let runtime = dir.join("trace-runtime.c");
            let source = execute(
                Command::new(env!("CARGO_BIN_EXE_cerune"))
                    .arg("emit-trace-runtime")
                    .arg("-o")
                    .arg(&runtime),
                dir,
                "trace/runtime",
            );
            assert!(
                source.status.success(),
                "{}",
                String::from_utf8_lossy(&source.stderr)
            );
            build.arg(runtime);
        }
        let build = execute(build.arg("-o").arg(&exe), dir, "trace/compile");
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
        Some(exe)
    }
}

pub(super) fn run(exe: &Path, args: &[&str]) -> Output {
    execute(
        Command::new(exe).args(args),
        exe.parent().unwrap(),
        "trace/run",
    )
}
