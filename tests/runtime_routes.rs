#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/process.rs"]
mod process;
#[path = "support/runtime_cases.rs"]
mod runtime_cases;

use cerune_lang::{RunError, run_vm};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

struct Workspace(PathBuf);
impl Workspace {
    fn new(route: &str) -> Self {
        crash_dialogs::suppress();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cerune-runtime-{route}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn command(&self, command: &mut Command, label: &str, seconds: u64) -> Output {
        process::bounded_output(command, &self.0, label, Duration::from_secs(seconds)).unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tool(variable: &str, default: &str, flag: &str) -> Option<OsString> {
    let configured = std::env::var_os(variable);
    let name = configured.clone().unwrap_or_else(|| default.into());
    if Command::new(&name)
        .arg(flag)
        .output()
        .is_ok_and(|r| r.status.success())
    {
        Some(name)
    } else {
        assert!(configured.is_none(), "{variable} is unavailable: {name:?}");
        eprintln!("runtime comparison skipped: {name:?} unavailable; set {variable} to require it");
        None
    }
}

fn success(output: Output, label: &str) {
    assert!(
        output.status.success(),
        "{label}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: Output, source: &str, expected_code: &str, route: &str) {
    let RunError::Execution(error) = run_vm(source).unwrap_err() else {
        panic!("invalid failure source")
    };
    let failure = error.runtime_failure().unwrap();
    let program = cerune_lang::compile_to_ir(source).unwrap();
    let direct = cerune_lang::ir_executor::run(&program).unwrap_err();
    assert_eq!(direct.runtime_failure(), Some(failure), "{route}");
    assert_eq!(direct.output(), error.vm_error().output(), "{route}");
    assert_eq!(failure.code.name(), expected_code, "{source}");
    match route {
        "wat" => assert_eq!(output.status.code(), Some(1), "{source}"),
        _ => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(
                    output.status.signal(),
                    Some(if route == "llvm" { 4 } else { 6 }),
                    "{route}: {source}"
                );
            }
            #[cfg(windows)]
            {
                let code = output.status.code().map(|c| c as u32);
                if route == "llvm" {
                    assert_eq!(code, Some(0xc000001d), "{source}");
                } else {
                    assert!(
                        matches!(code, Some(3 | 0xc0000409)),
                        "{route}: {source}: {code:?}"
                    );
                }
            }
        }
    }
    // stderrだけCRLF/LFを揃え、stdoutは全経路で実際のバイト列を比較します。
    assert_eq!(
        output.stdout,
        error.vm_error().output().as_bytes(),
        "{route}: {source}"
    );
    assert_eq!(
        String::from_utf8(output.stderr)
            .unwrap()
            .replace("\r\n", "\n"),
        format!("cerune: {}\n", failure.record()),
        "{route}: {source}"
    );
}

fn cases() -> Vec<(String, &'static str)> {
    let mut cases: Vec<_> = runtime_cases::FAILURES.iter().map(|&(body, code)| {
        (format!("// 日本語\r\nprint(\"開始\\0\\r\\n\");\r\nprint(false && (1 / 0 == 0));\r\n{body}"), code)
    }).collect();
    cases.extend([
        (include_str!("../examples/runtime_failures/overflow.ceru").into(), "integer-overflow"),
        (include_str!("../examples/runtime_failures/array_update.ceru").into(), "array-index-out-of-bounds"),
        (include_str!("../examples/runtime_failures/function_division.ceru").into(), "division-by-zero"),
        // 同じ関数を正常・失敗の順に呼んでも、呼び出し元ではなく演算の位置を報告します。
        (include_str!("../examples/runtime_failures/call_sequence.ceru").into(), "division-by-zero"),
        // 左側の失敗で右側を実行せず、別の失敗理由で上書きしません。
        ("fn left() -> i64 { print(1); return 1 / 0; } fn right() -> i64 { print(2); return 1 % 0; } print(left() + right());".into(), "division-by-zero"),
        ("mut a: [[i64; 1]; 1] = [[0]]; a[1][1] = 1 / 0;".into(), "array-index-out-of-bounds"),
        ("print(i8(i16(128)));".into(), "integer-conversion-out-of-range"),
    ]);
    cases
}

fn compare_native(route: &str, compiler: OsString, qbe: Option<OsString>) {
    let workspace = Workspace::new(route);
    let input = workspace.0.join(match route {
        "c" => "program.c",
        "llvm" => "program.ll",
        _ => "program.ssa",
    });
    let assembly = workspace.0.join("program.s");
    let exe = workspace.0.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    for (index, (source, code)) in cases().into_iter().enumerate() {
        let artifact = match route {
            "c" => cerune_lang::compile_to_c(&source).unwrap(),
            "llvm" => cerune_lang::compile_to_llvm_with_target(
                &source,
                Some(if cfg!(windows) {
                    cerune_lang::codegen::llvm::Target::X86_64PcWindowsMsvc
                } else {
                    cerune_lang::codegen::llvm::Target::X86_64UnknownLinuxGnu
                }),
            )
            .unwrap(),
            _ => cerune_lang::compile_to_qbe_with_target(
                &source,
                Some(cerune_lang::codegen::qbe::Target::X86_64UnknownLinuxGnu),
            )
            .unwrap(),
        };
        fs::write(&input, artifact).unwrap();
        if let Some(qbe) = &qbe {
            success(
                workspace.command(
                    Command::new(qbe).arg("-o").arg(&assembly).arg(&input),
                    &format!("{route}-{index}/lower"),
                    30,
                ),
                &source,
            );
        }
        // 最適化によって診断の位置・評価順・先行出力が消えないことを検証します。
        for optimization in ["-O0", "-O2"] {
            let label = format!("{route}-{index}/{code}/{optimization}");
            let mut command = Command::new(&compiler);
            command.arg(optimization);
            if route == "c" {
                command.args(["-std=c11", "-pedantic-errors"]);
            }
            if route == "llvm" {
                command.arg(if cfg!(windows) {
                    "--target=x86_64-pc-windows-msvc"
                } else {
                    "--target=x86_64-unknown-linux-gnu"
                });
            }
            command
                .arg(if qbe.is_some() { &assembly } else { &input })
                .arg("-o")
                .arg(&exe);
            if !cfg!(windows) {
                command.arg("-lm");
            }
            success(
                workspace.command(&mut command, &format!("{label}/link"), 30),
                &source,
            );
            let result = workspace.command(
                Command::new(&exe).current_dir(&workspace.0),
                &format!("{label}/run"),
                10,
            );
            assert_failure(result, &source, code, route);
        }
    }
}

#[test]
fn c_failures_match_vm_with_and_without_optimization() {
    if let Some(cc) = tool("CERUNE_TEST_CC", "clang", "--version") {
        compare_native("c", cc, None);
    }
}

#[test]
fn llvm_failures_match_vm_with_and_without_optimization() {
    if let Some(cc) = tool("CERUNE_TEST_LLVM_CLANG", "clang", "--version") {
        compare_native("llvm", cc, None);
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn qbe_failures_match_vm_with_and_without_optimization() {
    if let (Some(cc), Some(qbe)) = (
        tool("CERUNE_TEST_CC", "cc", "--version"),
        tool("CERUNE_TEST_QBE", "qbe", "-h"),
    ) {
        compare_native("qbe", cc, Some(qbe));
    }
}

#[test]
fn wat_failures_match_vm_and_preserve_output_on_trap() {
    let Some(node) = tool("CERUNE_TEST_NODE", "node", "--version") else {
        return;
    };
    let Some(wabt) = std::env::var_os("CERUNE_TEST_WAT2WASM_JS") else {
        eprintln!("WAT runtime comparison skipped; set CERUNE_TEST_WAT2WASM_JS to require it");
        return;
    };
    assert!(Path::new(&wabt).is_file(), "missing wat2wasm: {wabt:?}");
    let workspace = Workspace::new("wat");
    let input = workspace.0.join("program.wat");
    let wasm = workspace.0.join("program.wasm");
    let host = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/run_wasm.cjs");
    for (index, (source, code)) in cases().into_iter().enumerate() {
        fs::write(&input, cerune_lang::compile_to_wat(&source).unwrap()).unwrap();
        let label = format!("wat-{index}/{code}");
        success(
            workspace.command(
                Command::new(&node)
                    .arg(&wabt)
                    .arg(&input)
                    .arg("-o")
                    .arg(&wasm),
                &format!("{label}/compile"),
                30,
            ),
            &source,
        );
        let result = workspace.command(
            Command::new(&node).arg(&host).arg(&wasm),
            &format!("{label}/run"),
            10,
        );
        assert_failure(result, &source, code, "wat");
    }
}

#[test]
fn wat_host_requires_a_valid_record_and_an_unreachable_trap() {
    let Some(node) = tool("CERUNE_TEST_NODE", "node", "--version") else {
        return;
    };
    let Some(wabt) = std::env::var_os("CERUNE_TEST_WAT2WASM_JS") else {
        eprintln!("WAT host comparison skipped; set CERUNE_TEST_WAT2WASM_JS to require it");
        return;
    };
    let workspace = Workspace::new("wat-host");
    let input = workspace.0.join("program.wat");
    let wasm = workspace.0.join("program.wasm");
    let host = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/run_wasm.cjs");
    let record = "cerune: runtime-v1 code=division-by-zero node=7 bytes=0..1\n";
    for (label, diagnostic, tail, accepted) in [
        ("reported-unreachable", record, "unreachable", true),
        (
            "reported-file",
            "cerune: runtime-v1 code=division-by-zero node=7 file=2 bytes=0..1\n",
            "unreachable",
            true,
        ),
        (
            "zero-file",
            "cerune: runtime-v1 code=division-by-zero node=7 file=0 bytes=0..1\n",
            "unreachable",
            false,
        ),
        (
            "leading-zero-file",
            "cerune: runtime-v1 code=division-by-zero node=7 file=02 bytes=0..1\n",
            "unreachable",
            false,
        ),
        (
            "unsafe-file",
            "cerune: runtime-v1 code=division-by-zero node=7 file=9007199254740992 bytes=0..1\n",
            "unreachable",
            false,
        ),
        (
            "duplicate-file",
            "cerune: runtime-v1 code=division-by-zero node=7 file=2 file=3 bytes=0..1\n",
            "unreachable",
            false,
        ),
        ("reported-return", record, "", false),
        (
            "reported-memory-trap",
            record,
            "i32.const 65536 i32.load drop",
            false,
        ),
        ("unreported-unreachable", "", "unreachable", false),
        (
            "malformed-unreachable",
            "invalid diagnostic\n",
            "unreachable",
            false,
        ),
        (
            "reversed-span",
            "cerune: runtime-v1 code=division-by-zero node=7 bytes=1..0\n",
            "unreachable",
            false,
        ),
        (
            "leading-zero",
            "cerune: runtime-v1 code=division-by-zero node=07 bytes=0..1\n",
            "unreachable",
            false,
        ),
        (
            "unsafe-node",
            "cerune: runtime-v1 code=division-by-zero node=9007199254740992 bytes=0..1\n",
            "unreachable",
            false,
        ),
    ] {
        let report: String = diagnostic
            .bytes()
            .map(|byte| format!("i32.const {byte} call $write_error_byte\n"))
            .collect();
        fs::write(&input, format!("(module\n(import \"cerune\" \"write_error_byte\" (func $write_error_byte (param i32)))\n(import \"cerune\" \"print_i64\" (func $print_i64 (param i64)))\n(memory 1)\n(func (export \"main\") i64.const 7 call $print_i64 {report} {tail}))")).unwrap();
        success(
            workspace.command(
                Command::new(&node)
                    .arg(&wabt)
                    .arg(&input)
                    .arg("-o")
                    .arg(&wasm),
                &format!("{label}/compile"),
                30,
            ),
            label,
        );
        let output = workspace.command(
            Command::new(&node).arg(&host).arg(&wasm),
            &format!("{label}/run"),
            10,
        );
        assert_eq!(output.status.code(), Some(1), "{label}");
        assert_eq!(output.stdout, b"7\n", "{label}");
        assert_eq!(
            output.stderr == diagnostic.as_bytes(),
            accepted,
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
