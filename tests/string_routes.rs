#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/termination.rs"]
mod termination;
#[path = "support/u64_cases.rs"]
mod u64_cases;

#[path = "support/string_cases.rs"]
mod string_cases;

use cerune_lang::{
    codegen::qbe::Target, compile_to_qbe, compile_to_qbe_with_target, compile_to_wat,
    compile_to_x86_64_win_asm, run_vm,
};
use std::{ffi::OsString, fs, path::PathBuf, process::Command};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        crash_dialogs::suppress();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cerune-string-routes-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        // このテスト自身が新規作成したディレクトリだけを片付けます。
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tool(variable: &str, default: &str, version: &str) -> Option<OsString> {
    let configured = std::env::var_os(variable);
    let name = configured.clone().unwrap_or_else(|| default.into());
    if Command::new(&name)
        .arg(version)
        .output()
        .is_ok_and(|o| o.status.success())
    {
        Some(name)
    } else {
        assert!(configured.is_none(), "{variable} is unavailable: {name:?}");
        eprintln!(
            "execution comparison skipped: {name:?} unavailable; set {variable} to require it"
        );
        None
    }
}

fn run_success(command: &mut Command) -> Vec<u8> {
    let output = command.output().expect("start validation command");
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn cases() -> Vec<(&'static str, String)> {
    let mut cases: Vec<_> = string_cases::CASES
        .iter()
        .map(|&(s, e)| (s, e.to_owned()))
        .collect();
    cases.push((string_cases::UNUSED_DEFAULT, "1\ntrue\n".into()));
    cases.push((
        string_cases::BYTE_LENGTH.0,
        string_cases::BYTE_LENGTH.1.into(),
    ));
    cases.push((
        include_str!("../examples/string_origins.ceru"),
        "日本語\0\ntrue\nfalse\n".into(),
    ));
    for source in [
        include_str!("../examples/string_values.ceru"),
        include_str!("../examples/string_lookup.ceru"),
        include_str!("../examples/native_values.ceru"),
        include_str!("../examples/packet_counter.ceru"),
    ] {
        cases.push((source, run_vm(source).unwrap()));
    }
    cases.push((u64_cases::EXAMPLE.0, u64_cases::EXAMPLE.1.into()));
    cases.push((u64_cases::BOUNDARIES.0, u64_cases::BOUNDARIES.1.into()));
    cases
}

#[test]
fn every_route_generates_deterministic_artifacts() {
    for (source, expected) in cases() {
        assert_eq!(run_vm(source).unwrap(), expected);
        for emit in [compile_to_wat, compile_to_x86_64_win_asm] {
            assert_eq!(emit(source).unwrap(), emit(source).unwrap());
        }
        let qbe = compile_to_qbe_with_target(source, Some(Target::X86_64UnknownLinuxGnu)).unwrap();
        assert_eq!(
            qbe,
            compile_to_qbe_with_target(source, Some(Target::X86_64UnknownLinuxGnu)).unwrap()
        );
        assert!(qbe.starts_with("# target: x86_64-unknown-linux-gnu"));
        assert!(qbe.contains("loadub"));
        let wat = compile_to_wat(source).unwrap();
        assert!(wat.contains("\"write_stdout_v1\"") && wat.contains("i32.load8_u"));
        assert!(
            !wat.contains("(export \"memory\"") && !wat.contains("(import \"cerune\" \"memory\"")
        );
        let asm = compile_to_x86_64_win_asm(source).unwrap();
        assert!(
            asm.contains("callq _setmode") && asm.contains("movzbl"),
            "{source}"
        );
    }
}

#[test]
fn qbe_requires_an_explicit_runtime_target() {
    assert!(compile_to_qbe("x: i64 = 1;").is_ok());
    for source in [
        "print(\"x\");",
        "print(1);",
        "print(true);",
        "fn unused() -> i64 { print(1.5); return 0; }",
        "type Item { text: string, }",
        "fn unused(text: string) -> string { return text; }",
    ] {
        let error = compile_to_qbe(source).unwrap_err();
        assert!(error.message().contains("explicit --target"));
        assert!(error.primary_span().is_some());
        assert!(compile_to_qbe_with_target(source, Some(Target::X86_64UnknownLinuxGnu)).is_ok());
        assert!(compile_to_qbe_with_target(source, Some(Target::X86_64PcWindowsMsvc)).is_ok());
        assert!(compile_to_wat(source).is_ok());
        assert!(compile_to_x86_64_win_asm(source).is_ok());
    }
    let workspace = Workspace::new();
    let input = workspace.0.join("input.ceru");
    let output = workspace.0.join("output.ssa");
    fs::write(&input, "print(\"x\");").unwrap();
    for options in [
        vec![],
        vec!["--target"],
        vec!["--target", "x86_64-pc-windows-gnu"],
        vec!["--target", "unknown"],
    ] {
        fs::write(&output, b"existing artifact").unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-qbe")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .args(options)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(!result.stderr.is_empty());
        assert_eq!(fs::read(&output).unwrap(), b"existing artifact");
    }
    for (target, qbe_name) in [
        ("x86_64-pc-windows-msvc", "amd64_win"),
        ("x86_64-unknown-linux-gnu", "amd64_sysv"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-qbe")
            .arg(&input)
            .args(["--target", target, "-o"])
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            fs::read_to_string(&output)
                .unwrap()
                .starts_with(&format!("# target: {target} (qbe -t {qbe_name})"))
        );
    }
}

#[test]
fn direct_assembly_matches_known_bytes_and_vm_on_windows() {
    if !cfg!(all(
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )) {
        eprintln!("direct assembly execution skipped: requires Windows x64 with MSVC CRT");
        return;
    }
    let Some(clang) = tool("CERUNE_TEST_ASM_CLANG", "clang", "--version") else {
        return;
    };
    let workspace = Workspace::new();
    let assembly = workspace.0.join("program.s");
    let executable = workspace.0.join("program.exe");
    for (source, expected) in cases() {
        fs::write(&assembly, compile_to_x86_64_win_asm(source).unwrap()).unwrap();
        run_success(
            Command::new(&clang)
                .arg("--target=x86_64-pc-windows-msvc")
                .arg(&assembly)
                .arg("-o")
                .arg(&executable),
        );
        assert_eq!(
            run_success(&mut Command::new(&executable)),
            expected.as_bytes(),
            "{source}"
        );
    }
    for source in string_cases::OUT_OF_BOUNDS
        .iter()
        .chain(u64_cases::FAILURES)
    {
        assert!(run_vm(source).is_err());
        fs::write(&assembly, compile_to_x86_64_win_asm(source).unwrap()).unwrap();
        run_success(
            Command::new(&clang)
                .arg("--target=x86_64-pc-windows-msvc")
                .arg(&assembly)
                .arg("-o")
                .arg(&executable),
        );
        let failed = Command::new(&executable)
            .current_dir(&workspace.0)
            .output()
            .unwrap();
        termination::assert_expected(&failed, termination::Expected::NativeTrap, source);
        assert!(failed.stdout.is_empty());
    }
}

#[test]
fn qbe_matches_known_bytes_and_vm_on_linux() {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        eprintln!("QBE execution skipped: requires Linux x86-64");
        return;
    }
    let Some(qbe) = tool("CERUNE_TEST_QBE", "qbe", "-h") else {
        return;
    };
    let Some(cc) = tool("CERUNE_TEST_CC", "cc", "--version") else {
        return;
    };
    let workspace = Workspace::new();
    let input = workspace.0.join("program.ssa");
    let assembly = workspace.0.join("program.s");
    let executable = workspace.0.join("program");
    for (source, expected) in cases() {
        fs::write(
            &input,
            compile_to_qbe_with_target(source, Some(Target::X86_64UnknownLinuxGnu)).unwrap(),
        )
        .unwrap();
        run_success(
            Command::new(&qbe)
                .args(["-t", "amd64_sysv"])
                .arg("-o")
                .arg(&assembly)
                .arg(&input),
        );
        run_success(Command::new(&cc).arg(&assembly).arg("-o").arg(&executable));
        assert_eq!(
            run_success(&mut Command::new(&executable)),
            expected.as_bytes(),
            "{source}"
        );
    }
    for source in string_cases::OUT_OF_BOUNDS
        .iter()
        .chain(u64_cases::FAILURES)
    {
        assert!(run_vm(source).is_err());
        fs::write(
            &input,
            compile_to_qbe_with_target(source, Some(Target::X86_64UnknownLinuxGnu)).unwrap(),
        )
        .unwrap();
        run_success(
            Command::new(&qbe)
                .args(["-t", "amd64_sysv"])
                .arg("-o")
                .arg(&assembly)
                .arg(&input),
        );
        run_success(Command::new(&cc).arg(&assembly).arg("-o").arg(&executable));
        let failed = Command::new(&executable)
            .current_dir(&workspace.0)
            .output()
            .unwrap();
        termination::assert_expected(&failed, termination::Expected::Abort, source);
        assert!(failed.stdout.is_empty());
    }
}

#[test]
fn wat_matches_known_bytes_and_vm_without_exposing_memory() {
    let Some(node) = tool("CERUNE_TEST_NODE", "node", "--version") else {
        return;
    };
    // WABTのJS版をNodeで呼び、Windowsでも同じ変換ツールを使います。
    let wabt = std::env::var_os("CERUNE_TEST_WAT2WASM_JS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/wasm-tools/node_modules/wabt/bin/wat2wasm")
        });
    if !wabt.is_file() {
        assert!(
            std::env::var_os("CERUNE_TEST_WAT2WASM_JS").is_none(),
            "configured WABT is unavailable"
        );
        eprintln!("WAT execution skipped: set CERUNE_TEST_WAT2WASM_JS to WABT's bin/wat2wasm");
        return;
    }
    let workspace = Workspace::new();
    let wat = workspace.0.join("program.wat");
    let wasm = workspace.0.join("program.wasm");
    let host = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/run_wasm.cjs");
    for (source, expected) in cases() {
        fs::write(&wat, compile_to_wat(source).unwrap()).unwrap();
        run_success(
            Command::new(&node)
                .arg(&wabt)
                .arg(&wat)
                .arg("-o")
                .arg(&wasm),
        );
        assert_eq!(
            run_success(Command::new(&node).arg(&host).arg(&wasm)),
            expected.as_bytes(),
            "{source}"
        );
    }
    for source in string_cases::OUT_OF_BOUNDS
        .iter()
        .chain(u64_cases::FAILURES)
    {
        assert!(run_vm(source).is_err());
        fs::write(&wat, compile_to_wat(source).unwrap()).unwrap();
        run_success(
            Command::new(&node)
                .arg(&wabt)
                .arg(&wat)
                .arg("-o")
                .arg(&wasm),
        );
        let failed = Command::new(&node).arg(&host).arg(&wasm).output().unwrap();
        let cerune_lang::RunError::Execution(expected) = run_vm(source).unwrap_err() else {
            panic!("expected a runtime failure");
        };
        assert!(!failed.status.success());
        assert!(failed.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&failed.stderr),
            format!("cerune: {}\n", expected.runtime_failure().unwrap().record())
        );
    }
}
