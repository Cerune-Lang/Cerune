//! 既存の動的配列ケースを生成QBEでも実行します。所有の規則は再実装しません。
use super::{crash_dialogs, process};
use cerune_lang::{codegen, ir, ir_executor};
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn compiler() -> Option<&'static OsString> {
    static CC: OnceLock<Option<OsString>> = OnceLock::new();
    CC.get_or_init(|| {
        crash_dialogs::suppress();
        let configured = std::env::var_os("CERUNE_TEST_QBE");
        let cc = configured.clone().unwrap_or_else(|| "qbe".into());
        if Command::new(&cc)
            .arg("-h")
            .output()
            .is_ok_and(|r| r.status.success())
        {
            Some(cc)
        } else {
            assert!(configured.is_none(), "CERUNE_TEST_QBE unavailable: {cc:?}");
            eprintln!(
                "dynamic array QBE execution skipped: {cc:?}; set CERUNE_TEST_QBE to require it"
            );
            None
        }
    })
    .as_ref()
}
fn execute(text: &str, optimization: &str) -> Option<Output> {
    let cc = compiler()?;
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let w = Workspace(std::env::temp_dir().join(format!(
        "cerune-array-qbe-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&w.0).unwrap();
    let input = w.0.join("program.ssa");
    let exe = w.0.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&input, text).unwrap();
    let assembly = w.0.join("program.s");
    let built = process::bounded_output(
        Command::new(cc)
            .args(["-t", target().qbe_name(), "-o"])
            .arg(&assembly)
            .arg(&input),
        &w.0,
        "qbe-array-generate",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        built.status.success(),
        "{}\n{text}",
        String::from_utf8_lossy(&built.stderr)
    );
    let linker = std::env::var_os("CERUNE_TEST_QBE_CLANG")
        .unwrap_or_else(|| if cfg!(windows) { "clang" } else { "cc" }.into());
    let mut command = Command::new(linker);
    if cfg!(windows) {
        command.arg(format!("--target={}", target().triple()));
    }
    command.arg(optimization).arg(&assembly).arg("-o").arg(&exe);
    if !cfg!(windows) {
        command.arg("-lm");
    }
    let linked = process::bounded_output(
        &mut command,
        &w.0,
        "qbe-array-link",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    Some(
        process::bounded_output(
            Command::new(exe).current_dir(&w.0),
            &w.0,
            "qbe-array-run",
            Duration::from_secs(30),
        )
        .unwrap(),
    )
}
fn checked_failure(actual: &Output, record: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(actual.status.signal(), Some(6), "{actual:?}");
    }
    #[cfg(windows)]
    assert!(
        matches!(actual.status.code().map(|c| c as u32), Some(3 | 0xc0000409)),
        "{actual:?}"
    );
    assert_eq!(actual.stderr, format!("cerune: {record}\n").as_bytes());
}
pub fn target() -> codegen::qbe::Target {
    if cfg!(windows) {
        codegen::qbe::Target::X86_64PcWindowsMsvc
    } else {
        codegen::qbe::Target::X86_64UnknownLinuxGnu
    }
}
pub fn compare(program: &ir::Program, direct: &Result<String, ir_executor::ExecutionError>) {
    let text = codegen::qbe::emit_qbe_with_target(program, Some(target()))
        .unwrap()
        .replace(
            "export function w $main()",
            "function w $cerune_sample_main()",
        );
    let text = format!(
        r#"{text}
export function w $main() {{
@start
  %result =w call $cerune_sample_main()
  %live =l loadl $cerune_array_live
  %leaked =w cnel %live, 0
  jnz %leaked, @fail, @done
@fail
  ret 99
@done
  ret %result
}}
"#
    );
    // QBEは独自の最適化設定を持たないため、生成したAssemblyを一度リンクします。
    let Some(actual) = execute(&text, "-O2") else {
        return;
    };
    match direct {
        Ok(expected) => {
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty(), "{actual:?}");
            assert_eq!(&actual.stdout, expected.as_bytes());
        }
        Err(error) => {
            assert_eq!(actual.stdout, error.output().as_bytes());
            checked_failure(&actual, &error.runtime_failure().unwrap().record());
        }
    }
}

pub fn runtime_boundaries(program: &ir::Program) {
    let text = codegen::qbe::emit_qbe_with_target(program, Some(target())).unwrap();
    let mut limited = program.clone();
    limited.array_heap_limit = 0;
    let expected = ir_executor::run(&limited).unwrap_err();
    let record = expected.runtime_failure().unwrap().record();
    let call = text
        .lines()
        .find(|line| line.contains("=l call $cerune_array_allocate("))
        .unwrap();
    let origin = call.split_once(", l $cerune_origin_").unwrap().1;
    let start = call.split_once("(l ").unwrap().0;
    for (length, width, stride) in [
        ("-1", "8", "8"),
        ("9223372036854775807", "8", "8"),
        ("9223372036854775807", "0", "9223372036854775807"),
    ] {
        let replacement =
            format!("{start}(l {length}, l {width}, l {stride}, l $cerune_origin_{origin}");
        let Some(actual) = execute(&text.replace(call, &replacement), "-O2") else {
            return;
        };
        assert_eq!(actual.stdout, expected.output().as_bytes());
        checked_failure(
            &actual,
            &record.replace(
                "allocation-limit-exceeded",
                if length == "-1" {
                    "array-length-out-of-range"
                } else {
                    "allocation-size-overflow"
                },
            ),
        );
    }
    for nth in [1, 2] {
        let modified = text.replace("call $malloc(", "call $cerune_test_malloc(");
        let allocator = format!(
            r#"
data $cerune_test_count = align 8 {{ l 0 }}
function l $cerune_test_malloc(l %size) {{
@start
  %count =l loadl $cerune_test_count
  %next =l add %count, 1
  storel %next, $cerune_test_count
  %fail =w ceql %next, {nth}
  jnz %fail, @failed, @allocate
@failed
  ret 0
@allocate
  %result =l call $malloc(l %size)
  ret %result
}}
"#
        );
        let Some(actual) = execute(&format!("{modified}{allocator}"), "-O2") else {
            return;
        };
        assert_eq!(actual.stdout, expected.output().as_bytes());
        checked_failure(
            &actual,
            &record.replace("allocation-limit-exceeded", "allocation-failed"),
        );
    }
    // 言語で表現できない幅0も、サイズ検査と所有解放の境界として直接確認します。
    let modified = text.replace(
        "export function w $main()",
        "function w $cerune_sample_main()",
    );
    let modified = format!(
        r#"{modified}
export function w $main() {{
@start
  %p =l call $cerune_array_allocate(l 3, l 0, l 0, l 0, l 0)
  %count =l add %p, 32
  storel 3, %count
  %length =l call $cerune_array_length(l %p)
  %last =w call $cerune_array_release_owner_last(l %p)
  call $cerune_array_free_elements(l %p)
  %live =l loadl $cerune_array_live
  %length_ok =w ceql %length, 3
  %live_ok =w ceql %live, 0
  %both =w and %length_ok, %live_ok
  %ok =w and %both, %last
  jnz %ok, @done, @failed
@done
  ret 0
@failed
  ret 99
}}
"#
    );
    let Some(actual) = execute(&modified, "-O2") else {
        return;
    };
    assert!(
        actual.status.success() && actual.stdout.is_empty() && actual.stderr.is_empty(),
        "{actual:?}"
    );
}
