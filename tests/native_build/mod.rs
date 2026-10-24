use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
#[path = "../support/process.rs"]
mod process;
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "cerune-native-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn target() -> &'static str {
    if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}
fn driver() -> Option<PathBuf> {
    if !cfg!(any(windows, target_os = "linux")) {
        return None;
    }
    let configured = std::env::var_os("CERUNE_TEST_ASM_CLANG");
    let name = PathBuf::from(configured.clone().unwrap_or_else(|| "clang".into()));
    let path = if name.is_file() {
        Some(name)
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| {
                dir.join(&name)
                    .with_extension(if cfg!(windows) { "exe" } else { "" })
            })
            .find(|p| p.is_file())
    };
    let path = path.and_then(|p| fs::canonicalize(p).ok());
    assert!(
        path.is_some() || configured.is_none(),
        "required native build driver unavailable"
    );
    if path.is_none() {
        eprintln!("native build execution skipped: Clang unavailable");
    }
    path
}
fn output(command: &mut Command, dir: &Path) -> Output {
    process::bounded_output(command, dir, "native-build", Duration::from_secs(45)).unwrap()
}
fn build(input: &Path, dir: &Path, clang: &Path, ssa: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
    cmd.arg("build")
        .arg("native")
        .arg(input)
        .args(["--target", target(), "--linker"])
        .arg(clang)
        .arg("-o")
        .arg(dir);
    if ssa {
        cmd.arg("--ssa");
    }
    cmd
}
fn success(out: &Output) {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "{out:?}");
}
fn text(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap()
}
fn exe(dir: &Path) -> PathBuf {
    dir.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    })
}

#[test]
fn builds_native_examples_and_preserves_the_exact_generation_stages() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for ssa in [false, true] {
        let dir = scratch.0.join(format!("built with spaces {ssa}"));
        let source = root.join("examples/build_native/main.ceru");
        let mut cmd = build(&source, &dir, &clang, ssa);
        cmd.args([
            "--include-sources",
            "--string-heap-limit",
            "4096",
            "--array-heap-limit",
            "4096",
            "--argument-count-limit",
            "8",
            "--argument-byte-limit",
            "128",
        ]);
        success(&output(&mut cmd, &scratch.0));
        let manifest = text(&dir, "manifest.json");
        assert!(manifest.contains("\"status\":\"success\""));
        assert!(manifest.contains("\"executed\":false"));
        assert!(manifest.contains("\"optimization_passes\":[]"));
        assert!(manifest.contains("\"argument_count_limit\":8"));
        assert!(text(&dir, "link-plan.json").contains(&format!("--target={}", target())));
        assert!(text(&dir, "link.stderr.log").contains("program.o"));
        assert!(text(&dir, "program.origins.json").contains("\"source\":{\"node\":"));
        assert!(text(&dir, "program.object-map.txt").contains("object-section-offsets"));
        assert!(text(&dir, "sources.json").contains("std:env"));
        for (command, name) in [
            ("emit-ir", "program.ceir"),
            ("emit-mir", "program.mir.txt"),
            ("emit-lir", "program.lir.txt"),
            ("emit-asm", "program.origins.s"),
            ("emit-obj", "program.o"),
        ] {
            let comparison = scratch.0.join(format!("{ssa}-{command}"));
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
            cmd.arg(command)
                .arg(&source)
                .arg("-o")
                .arg(&comparison)
                .args(["--string-heap-limit", "4096", "--array-heap-limit", "4096"]);
            if matches!(command, "emit-lir" | "emit-asm" | "emit-obj") {
                cmd.args([
                    "--target",
                    target(),
                    "--argument-count-limit",
                    "8",
                    "--argument-byte-limit",
                    "128",
                ]);
                if ssa {
                    cmd.arg("--ssa");
                }
                if command != "emit-lir" {
                    cmd.arg("--annotate-origins");
                }
            }
            success(&output(&mut cmd, &scratch.0));
            assert_eq!(
                fs::read(&comparison).unwrap(),
                fs::read(dir.join(name)).unwrap(),
                "{command} ssa={ssa}"
            );
        }
        assert_eq!(dir.join("program.ssa.txt").is_file(), ssa);
        let result = output(Command::new(exe(&dir)).arg("3"), &scratch.0);
        success(&result);
        assert_eq!(result.stdout, b"[9, 4]\n[2, 4]\n6\n");
        for route in ["run", "run-vm", "run-mir"] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
            cmd.arg(route).arg(&source);
            if ssa && route == "run-mir" {
                cmd.arg("--ssa");
            }
            cmd.args(["--", "3"]);
            let interpreted = output(&mut cmd, &scratch.0);
            success(&interpreted);
            assert_eq!(interpreted.stdout, result.stdout);
        }
        // Capture/limits remain the generated route's contract.
        let too_many = output(Command::new(exe(&dir)).args(["x"; 9]), &scratch.0);
        assert!(!too_many.status.success());
        assert!(
            String::from_utf8_lossy(&too_many.stderr)
                .contains("arguments-v1 phase=capture code=count-limit-exceeded")
        );
        // Existing completed output is never replaced.
        let retry = output(&mut build(&source, &dir, &clang, ssa), &scratch.0);
        assert!(!retry.status.success());
        assert_eq!(text(&dir, "manifest.json"), manifest);
    }
}

#[test]
fn native_build_does_not_execute_sources_and_retains_failure_origins() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let source = scratch.0.join("failure.ceru");
    fs::write(&source, "print(7); x: = 0; print(1/x); print(99);").unwrap();
    let dir = scratch.0.join("failure");
    let built = output(&mut build(&source, &dir, &clang, false), &scratch.0);
    success(&built);
    assert!(built.stdout.is_empty());
    assert!(!dir.join("sources.json").exists());
    let result = output(&mut Command::new(exe(&dir)), &scratch.0);
    assert!(!result.status.success());
    assert_eq!(result.stdout, b"7\n");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("runtime-v1 code=division-by-zero"),
        "{stderr}"
    );
    assert!(stderr.contains("node="));
    assert!(stderr.contains("bytes="));
    let bad = scratch.0.join("invalid.ceru");
    fs::write(&bad, "missing();").unwrap();
    let no_output = scratch.0.join("invalid");
    assert!(
        !output(&mut build(&bad, &no_output, &clang, false), &scratch.0)
            .status
            .success()
    );
    assert!(!no_output.exists());
}

#[test]
fn build_requires_explicit_route_target_linker_and_new_output() {
    let scratch = Scratch::new();
    let source = scratch.0.join("input.ceru");
    fs::write(&source, "print(1);").unwrap();
    for args in [
        vec![],
        vec!["native"],
        vec!["ir"],
        vec!["native", source.to_str().unwrap()],
        vec![
            "native",
            source.to_str().unwrap(),
            "--target",
            "x86_64-apple-darwin",
        ],
    ] {
        let result = output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("build")
                .args(args),
            &scratch.0,
        );
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn link_failures_timeouts_and_invalid_outputs_are_recorded_without_success() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let stub = scratch
        .0
        .join(if cfg!(windows) { "stub.exe" } else { "stub" });
    let c = scratch.0.join("stub.c");
    fs::write(
        &c,
        r#"
#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif
int main(int argc,char **argv){
 if(argc>1 && strcmp(argv[1],"--version")==0){puts("clang version test-stub");return 0;}
 const char *mode=getenv("CERUNE_BUILD_TEST_MODE");
 if(mode && strcmp(mode,"timeout")==0){
#ifdef _WIN32
 Sleep(30000);
#else
 sleep(30);
#endif
 }
 if(mode && strcmp(mode,"malformed")==0){
  for(int i=1;i+1<argc;i++) if(strcmp(argv[i],"-o")==0){
   FILE *out=fopen(argv[i+1],"wb"); if(!out) return 18;
   for(int j=0;j<64;j++) fputc(0,out); fclose(out);
  }
 }
 if(mode && strcmp(mode,"fail")==0){fputs("intentional linker failure",stderr);return 17;}
 return 0;
}
"#,
    )
    .unwrap();
    let compile = output(
        Command::new(&clang).arg(&c).arg("-o").arg(&stub),
        &scratch.0,
    );
    success(&compile);
    let source = scratch.0.join("input.ceru");
    fs::write(&source, "print(1);").unwrap();
    for (mode, reason) in [
        ("fail", "link-failed"),
        ("empty", "invalid-link-output"),
        ("malformed", "invalid-link-output"),
        ("timeout", "linker-timeout"),
    ] {
        let dir = scratch.0.join(mode);
        let mut cmd = build(&source, &dir, &stub, false);
        cmd.args(["--link-timeout-seconds", "1"])
            .env("CERUNE_BUILD_TEST_MODE", mode);
        let result = output(&mut cmd, &scratch.0);
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(reason),
            "{result:?}"
        );
        assert!(text(&dir, "manifest.json").contains("\"status\":\"failed\""));
        assert!(text(&dir, "link-result.json").contains(reason));
        assert!(dir.join("program.o").is_file());
        if mode == "fail" {
            assert!(text(&dir, "link.result.json").contains("\"exit_code\":17"));
        }
    }
}

#[test]
fn linked_stdin_example_matches_ir_and_preserves_newline_bytes() {
    let Some(clang) = driver() else {
        return;
    };
    let scratch = Scratch::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/stdin/lines.ceru");
    let dir = scratch.0.join("stdin");
    success(&output(
        &mut build(&source, &dir, &clang, false),
        &scratch.0,
    ));
    let input = scratch.0.join("input");
    fs::write(&input, b"a\r\n\nlast").unwrap();
    let mut native = Command::new(exe(&dir));
    let mut ir = Command::new(env!("CARGO_BIN_EXE_cerune"));
    ir.arg("run").arg(&source);
    let mut results = Vec::new();
    for cmd in [&mut native, &mut ir] {
        let out = process::bounded_output_with_stdin(
            cmd,
            &scratch.0,
            "input",
            Duration::from_secs(20),
            fs::File::open(&input).unwrap().into(),
        )
        .unwrap();
        success(&out);
        results.push(out.stdout);
    }
    assert_eq!(results[0], results[1]);
    assert!(String::from_utf8_lossy(&results[0]).contains("end"));
}

#[test]
fn native_build_rejects_missing_or_non_clang_drivers() {
    let scratch = Scratch::new();
    let source = scratch.0.join("input.ceru");
    fs::write(&source, "print(1);").unwrap();
    let missing = scratch.0.join("missing");
    let dir = scratch.0.join("unavailable");
    let out = output(&mut build(&source, &dir, &missing, false), &scratch.0);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("linker-unavailable"));
    assert!(!dir.exists());
    let dir = scratch.0.join("unsupported");
    let out = output(
        &mut build(
            &source,
            &dir,
            Path::new(env!("CARGO_BIN_EXE_cerune")),
            false,
        ),
        &scratch.0,
    );
    assert!(!out.status.success());
    assert!(text(&dir, "manifest.json").contains("failed"));
    assert!(text(&dir, "link-result.json").contains("unsupported-linker"));
    assert!(!dir.join("link.stdout.log").exists());
}

mod release;
