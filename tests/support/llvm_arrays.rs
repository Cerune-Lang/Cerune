//! 既存の動的配列ケースを生成LLVMでも実行します。所有の規則は再実装しません。
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
        let configured = std::env::var_os("CERUNE_TEST_LLVM_CLANG");
        let cc = configured
            .clone()
            .unwrap_or_else(|| "clang".into());
        if Command::new(&cc)
            .arg("--version")
            .output()
            .is_ok_and(|r| r.status.success())
        {
            Some(cc)
        } else {
            assert!(configured.is_none(), "CERUNE_TEST_LLVM_CLANG unavailable: {cc:?}");
            eprintln!(
                "dynamic array LLVM execution skipped: {cc:?}; set CERUNE_TEST_LLVM_CLANG to require it"
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
        "cerune-array-llvm-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&w.0).unwrap();
    let input = w.0.join("program.ll");
    let exe = w.0.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    fs::write(&input, text).unwrap();
    let mut command = Command::new(cc);
    command.args([optimization, &format!("--target={}", target().triple())]);
    command.arg(&input).arg("-o").arg(&exe);
    if !cfg!(windows) {
        command.arg("-lm");
    }
    let built = process::bounded_output(
        &mut command,
        &w.0,
        "llvm-array-compile",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        built.status.success(),
        "{}\n{text}",
        String::from_utf8_lossy(&built.stderr)
    );
    Some(
        process::bounded_output(
            Command::new(exe).current_dir(&w.0),
            &w.0,
            "llvm-array-run",
            Duration::from_secs(30),
        )
        .unwrap(),
    )
}
fn checked_failure(actual: &Output, record: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(actual.status.signal(), Some(4), "{actual:?}");
    }
    #[cfg(windows)]
    assert!(
        matches!(actual.status.code().map(|c| c as u32), Some(0xc000001d)),
        "{actual:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: {record}\n")
    );
}

pub fn target() -> codegen::llvm::Target {
    if cfg!(windows) {
        codegen::llvm::Target::X86_64PcWindowsMsvc
    } else {
        codegen::llvm::Target::X86_64UnknownLinuxGnu
    }
}
pub fn compare(program: &ir::Program, direct: &Result<String, ir_executor::ExecutionError>) {
    let text = codegen::llvm::emit_llvm_with_target(program, Some(target())).unwrap();
    // 本来のmainの後で、解放後も使用バイト数が残っていることを別の終了コードとして検出します。
    let text = text.replace("define i32 @main()", "define i32 @cerune.sample.main()");
    let text = format!(
        "{text}\ndefine i32 @main() {{\nentry:\n  %result = call i32 @cerune.sample.main()\n  %live = load i64, ptr @cerune.array.live\n  %leaked = icmp ne i64 %live, 0\n  %status = select i1 %leaked, i32 99, i32 %result\n  ret i32 %status\n}}\n"
    );
    for optimization in ["-O0", "-O2"] {
        let Some(actual) = execute(&text, optimization) else {
            return;
        };
        match direct {
            Ok(expected) => {
                assert!(actual.status.success(), "{actual:?}");
                assert!(actual.stderr.is_empty(), "{actual:?}");
                assert_eq!(&actual.stdout, expected.as_bytes(), "{optimization}");
            }
            Err(error) => {
                assert_eq!(actual.stdout, error.output().as_bytes(), "{optimization}");
                checked_failure(&actual, &error.runtime_failure().unwrap().record());
            }
        }
    }
}

pub fn runtime_boundaries(program: &ir::Program) {
    let text = codegen::llvm::emit_llvm_with_target(program, Some(target())).unwrap();
    let mut limited = program.clone();
    limited.array_heap_limit = 0;
    let expected = ir_executor::run(&limited).unwrap_err();
    let record = expected.runtime_failure().unwrap().record();
    // 巨大な領域を実際に確保せず、論理幅と物理strideの検査を別々に通します。
    let call = text
        .lines()
        .find(|line| line.contains("= call %cerune.array @cerune.array.allocate.elements("))
        .unwrap();
    let failure = call.rsplit_once(", ptr ").unwrap().1;
    for (length, width, stride) in [
        ("9223372036854775807", "8", "8"),
        ("9223372036854775807", "0", "9223372036854775807"),
    ] {
        let start = call.split_once("(i64 ").unwrap().0;
        let replacement = format!("{start}(i64 {length}, i64 {width}, i64 {stride}, ptr {failure}");
        let modified = text.replace(call, &replacement);
        for optimization in ["-O0", "-O2"] {
            let Some(actual) = execute(&modified, optimization) else {
                return;
            };
            assert_eq!(actual.stdout, expected.output().as_bytes());
            checked_failure(
                &actual,
                &record.replace("allocation-limit-exceeded", "allocation-size-overflow"),
            );
        }
    }

    // ソース言語にない幅0も、領域管理プリミティブの境界として直接検証します。
    let zero_width = text.replace("define i32 @main()", "define i32 @cerune.sample.main()");
    let zero_width = format!(
        r#"{zero_width}
define i32 @main() {{
entry:
  %array = call %cerune.array @cerune.array.allocate.elements(i64 3, i64 0, i64 0, ptr null)
  %owner = extractvalue %cerune.array %array, 0
  %count.ptr = getelementptr %cerune.array.owner, ptr %owner, i32 0, i32 3
  store i64 3, ptr %count.ptr
  %last = call i1 @cerune.array.release.owner.last(%cerune.array %array)
  call void @cerune.array.free.elements(%cerune.array %array)
  %length = extractvalue %cerune.array %array, 1
  %valid.length = icmp eq i64 %length, 3
  %live = load i64, ptr @cerune.array.live
  %valid.live = icmp eq i64 %live, 0
  %a = and i1 %valid.length, %valid.live
  %ok = and i1 %a, %last
  %status = select i1 %ok, i32 0, i32 99
  ret i32 %status
}}
"#
    );
    let Some(actual) = execute(&zero_width, "-O2") else {
        return;
    };
    assert!(
        actual.status.success() && actual.stdout.is_empty() && actual.stderr.is_empty(),
        "{actual:?}"
    );
    // 1回目の管理領域と2回目の要素領域のmalloc失敗を、元の式の出自で照合します。
    for nth in [1, 2] {
        let modified = text.replace("call ptr @malloc(", "call ptr @cerune.test.malloc(");
        let allocator = format!(
            r#"
@cerune.test.count = internal global i64 0
define internal ptr @cerune.test.malloc(i64 %size) {{
entry:
  %count = load i64, ptr @cerune.test.count
  %next = add i64 %count, 1
  store i64 %next, ptr @cerune.test.count
  %fail = icmp eq i64 %next, {nth}
  br i1 %fail, label %failed, label %allocate
failed:
  ret ptr null
allocate:
  %result = call ptr @malloc(i64 %size)
  ret ptr %result
}}
"#
        );
        for optimization in ["-O0", "-O2"] {
            let Some(actual) = execute(&format!("{modified}{allocator}"), optimization) else {
                return;
            };
            assert_eq!(actual.stdout, expected.output().as_bytes());
            checked_failure(
                &actual,
                &record.replace("allocation-limit-exceeded", "allocation-failed"),
            );
        }
    }
}
