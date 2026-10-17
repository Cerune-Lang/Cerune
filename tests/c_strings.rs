#[path = "support/termination.rs"]
mod termination;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use cerune_lang::{compile_to_c, run_vm};
#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/u64_cases.rs"]
mod u64_cases;

struct NativeC {
    directory: PathBuf,
    compiler: OsString,
}

impl NativeC {
    fn new() -> Option<Self> {
        crash_dialogs::suppress();
        let configured = std::env::var_os("CERUNE_TEST_CC");
        let compiler = configured
            .clone()
            .unwrap_or_else(|| if cfg!(windows) { "clang" } else { "cc" }.into());
        let available = Command::new(&compiler)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            assert!(
                configured.is_none(),
                "CERUNE_TEST_CC compiler is unavailable: {compiler:?}"
            );
            eprintln!(
                "native C execution skipped: {compiler:?} unavailable; set CERUNE_TEST_CC to require it"
            );
            return None;
        }
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "cerune-c-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Some(Self {
            directory,
            compiler,
        })
    }

    fn run(&self, source: &str, optimization: &str) -> Output {
        self.run_c(
            &compile_to_c(source).unwrap(),
            optimization,
            Some("-std=c11"),
            &[],
        )
    }

    fn run_c(
        &self,
        text: &str,
        optimization: &str,
        standard: Option<&str>,
        flags: &[&str],
    ) -> Output {
        let c = self.directory.join("program.c");
        let exe = self.directory.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        fs::write(&c, text).unwrap();
        let mut command = Command::new(&self.compiler);
        command.args(["-pedantic-errors", optimization]);
        command.args(standard).args(flags);
        if std::env::var_os("CERUNE_TEST_SANITIZE").is_some() {
            command.args(["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]);
        }
        command.arg(&c).arg("-o").arg(&exe);
        if !cfg!(windows) {
            command.arg("-lm");
        }
        let built = command.output().expect("start C compiler");
        assert!(
            built.status.success(),
            "C compile failed:\n{}\n{text}",
            String::from_utf8_lossy(&built.stderr)
        );
        Command::new(exe)
            .current_dir(&self.directory)
            .output()
            .expect("run generated C")
    }

    fn matches_vm(&self, source: &str) {
        let expected = run_vm(source).unwrap();
        for optimization in ["-O0", "-O2"] {
            let actual = self.run(source, optimization);
            assert!(
                actual.status.success(),
                "C run failed: {}\n{source}",
                String::from_utf8_lossy(&actual.stderr)
            );
            assert!(
                actual.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&actual.stderr)
            );
            // 改行の正規化をせず、NULを含む出力バイト全体を比較します。
            assert_eq!(
                actual.stdout,
                expected.as_bytes(),
                "{optimization}\n{source}"
            );
        }
    }
}

#[test]
fn u64_boundaries_and_failures_have_no_undefined_c_operations() {
    let Some(c) = NativeC::new() else { return };
    for (source, expected) in [u64_cases::EXAMPLE, u64_cases::BOUNDARIES] {
        assert_eq!(run_vm(source).unwrap(), expected);
        c.matches_vm(source);
    }
    for source in u64_cases::FAILURES {
        for optimization in ["-O0", "-O2"] {
            let failed = c.run(source, optimization);
            termination::assert_expected(&failed, termination::Expected::CheckedCFailure, source);
            assert!(failed.stdout.is_empty(), "{source}");
            let stderr = String::from_utf8_lossy(&failed.stderr);
            assert!(
                !stderr.contains("runtime error:") && !stderr.contains("ERROR: AddressSanitizer"),
                "{source}: {stderr}"
            );
        }
    }
}

impl Drop for NativeC {
    fn drop(&mut self) {
        // このテスト自身がcreate_dirで作成した専用ディレクトリだけを削除します。
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn literals_emit_ascii_c_and_explicit_utf8_lengths() {
    let source = r#"print("日本語\n\0\u{1f600}\"\\??/");"#;
    let c = compile_to_c(source).unwrap();
    assert!(c.is_ascii());
    assert!(c.contains("const unsigned char *data;"));
    assert!(c.contains("size_t length;"));
    assert!(c.contains("\\346\\227\\245"));
    assert!(c.contains("cerune_output_part(&op, value.data, value.length)"));
    assert!(c.contains("memcmp(left.data, right.data, left.length)"));
    assert!(
        !c.contains("malloc(")
            && !c.contains("free(")
            && !c.contains("strlen(")
            && !c.contains("strcmp(")
    );
    assert_eq!(c, compile_to_c(source).unwrap());
}

#[test]
fn strings_and_examples_match_vm_as_exact_bytes() {
    let Some(c) = NativeC::new() else { return };
    for source in [
        include_str!("../examples/string_values.ceru"),
        include_str!("../examples/string_lookup.ceru"),
        r#"print(""); print("a\0b\n\r\t\"\\??/9\u{1f600}");
            print("\u{e9}" == "e\u{301}"); print("a\0x" != "a\0y");
            print("a\0" == "a"); print("" == ""); print("日本語" == "\u{65e5}本語");
            if "a" != "b" { print("different"); }
            while "same" == "same" { print("same"); break; }"#,
    ] {
        c.matches_vm(source);
    }
}

#[test]
fn returned_strings_survive_calls_loops_and_aggregate_copies() {
    let Some(c) = NativeC::new() else { return };
    c.matches_vm(
        r#"
        type Label { id: i64, text: string = "既定", }
        fn make() -> [[Label; 1]; 2] {
            mut values: [[Label; 1]; 2] = [[Label { id: 0, }], [Label { id: 1, text: "保存", }]];
            saved: infer = values;
            values[1][0] = Label { id: 2, text: "変更", };
            print(values[1][0].text);
            return saved;
        }
        fn main() -> void {
            original: infer = make();
            mut copy: infer = original;
            for (mut i: i64 = 0; i < 3; i = i + 1) {
                words: [string; 2] = ["loop", "\0end"];
                copy[1][0] = Label { id: 3, text: words[1], };
            }
            print(original[0][0].text);
            print(original[1][0].text);
            print(copy[1][0].text);
        }
    "#,
    );
}

#[test]
fn c_preserves_call_comparison_construction_and_index_order() {
    let Some(c) = NativeC::new() else { return };
    c.matches_vm(
        r#"
        fn mark(text: string) -> string { print(text); return text; }
        fn pair(left: string, right: string) -> string { print(left); return right; }
        fn show(left: string, right: string) -> void { print(left); print(right); }
        fn array() -> [string; 2] { print("base"); return ["zero", "one"]; }
        fn index() -> i64 { print("index"); return 1; }
        type Pair { first: string = mark("default"), second: string, }
        print(mark("left") == mark("right"));
        print(pair(mark("arg1"), mark("arg2")));
        show(mark("void1"), mark("void2"));
        value: Pair = Pair { second: mark("explicit"), };
        print(value.first);
        words: [string; 2] = [mark("item1"), mark("item2")];
        print(words[1]);
        print(array()[index()]);
        print(false && mark("skipped1") == mark("skipped2"));
        print(true || mark("skipped3") != mark("skipped4"));
        for (mut flag: bool = true; flag; flag = mark("update1") == mark("update2")) {
            if mark("condition1") != mark("condition2") { continue; }
        }
    "#,
    );
}

#[test]
fn sequencing_also_preserves_numeric_effects_without_strings() {
    let Some(c) = NativeC::new() else { return };
    let source = r#"
        fn mark(value: i64) -> i64 { print(value); return value; }
        fn pair(left: i64, right: i64) -> i64 { return left + right; }
        print(mark(1) + mark(2));
        print(pair(mark(3), mark(4)));
        print(mark(5) < mark(6));
    "#;
    for optimization in ["-O0", "-O2"] {
        let actual = c.run(source, optimization);
        assert!(actual.status.success());
        assert!(actual.stderr.is_empty());
        // 数値のみの既存C出力ではWindowsのテキストモードを維持します。
        assert_eq!(
            String::from_utf8(actual.stdout)
                .unwrap()
                .replace("\r\n", "\n"),
            run_vm(source).unwrap()
        );
    }
    let source = "fn divide() -> i64 { return 1 / 0; } fn index() -> i64 { return [1][2]; } print(divide() + index());";
    assert!(run_vm(source).is_err());
    for optimization in ["-O0", "-O2"] {
        let failed = c.run(source, optimization);
        termination::assert_expected(&failed, termination::Expected::CheckedCFailure, source);
    }
}

#[test]
fn string_bindings_do_not_collide_with_outer_names_or_runtime_helpers() {
    let Some(c) = NativeC::new() else { return };
    c.matches_vm(
        r#"
        text: string = "outer";
        if true {
            text: infer = text;
            print(text);
        }
        for (mut text: string = text; text == "outer"; text = "done") {
            print(text);
        }
        string_equal: string = "helper name";
        print_string: string = "print helper name";
        print(string_equal == "helper name");
        print(print_string);
        print(text);
    "#,
    );
}

#[test]
fn all_current_examples_run_as_generated_c() {
    let Some(c) = NativeC::new() else { return };
    let mut paths: Vec<_> =
        fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "ceru")
            })
            .collect();
    paths.sort();
    assert!(!paths.is_empty());
    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let expected = run_vm(&source).unwrap();
        let uses_strings = compile_to_c(&source)
            .unwrap()
            .contains("typedef struct cerune_string {");
        for optimization in ["-O0", "-O2"] {
            let actual = c.run(&source, optimization);
            assert!(
                actual.status.success(),
                "{}: {}",
                path.display(),
                String::from_utf8_lossy(&actual.stderr)
            );
            assert!(
                actual.stderr.is_empty(),
                "{}: {}",
                path.display(),
                String::from_utf8_lossy(&actual.stderr)
            );
            let mut output = String::from_utf8(actual.stdout).unwrap();
            // ファイル名ではなく生成物で判定し、文字列中のCR/LFを保存します。
            if cfg!(windows) && !uses_strings {
                output = output.replace("\r\n", "\n");
            }
            assert_eq!(output, expected, "{} {optimization}", path.display());
        }
    }
}

#[path = "support/sum_cases.rs"]
mod sum_cases;
#[test]
fn enum_storage_is_initialized_and_copies_preserve_payloads() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in sum_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[allow(dead_code)]
#[path = "support/array_cases.rs"]
mod array_cases;
#[test]
fn array_length_evaluates_operands_even_when_the_result_is_known() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in array_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/iteration_cases.rs"]
mod iteration_cases;
#[test]
fn array_iteration_preserves_snapshots_and_loop_control() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in iteration_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/rounding_cases.rs"]
mod rounding_cases;
#[test]
fn rounding_matches_vm_in_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in rounding_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/truncation_cases.rs"]
mod truncation_cases;
#[test]
fn truncation_matches_vm_in_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in truncation_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/generic_cases.rs"]
mod generic_cases;
#[test]
fn generic_functions_preserve_values_across_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in generic_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/length_constant_cases.rs"]
mod length_constant_cases;
#[test]
fn constant_array_lengths_preserve_values_across_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in length_constant_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/aggregate_cases.rs"]
mod aggregate_cases;
#[test]
fn aggregate_values_match_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in aggregate_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[path = "support/concat_cases.rs"]
mod concat_cases;
#[test]
fn dynamic_strings_match_optimized_c() {
    let Some(native) = NativeC::new() else { return };
    for &(source, expected) in concat_cases::CASES {
        assert_eq!(run_vm(source).unwrap(), expected);
        native.matches_vm(source);
    }
}

#[test]
fn floating_point_contraction_preserves_generated_function_result_bits() {
    let Some(c) = NativeC::new() else { return };
    let source = "fn step32(a:f32,b:f32,c:f32)->f32{return a*b+c;}
                  fn step64(a:f64,b:f64,c:f64)->f64{return a*b+c;}";
    let generated = compile_to_c(source)
        .unwrap()
        .replace("int main(void)", "int cerune_sample_main(void)");
    // Volatile inputs prevent folding the calls; memcpy observes object bits
    // without aliasing UB or relying on decimal output formatting.
    let driver = r#"
#include <string.h>
int main(void) {
    volatile float a=0x1.000002p0f, b=0x1.fffffcp-1f, c=-1.0f;
    volatile double x=0x1.0000000000001p0, y=0x1.ffffffffffffep-1, z=-1.0;
    float f=cerune_fn_step32_0(a,b,c);
    double d=cerune_fn_step64_1(x,y,z);
    uint32_t u; uint64_t v;
    memcpy(&u,&f,sizeof u); memcpy(&v,&d,sizeof v);
    printf("%08x %016llx\n",(unsigned)u,(unsigned long long)v);
    return 0;
}
"#;
    let flags: &[&str] = &[];
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let flags: &[&str] = if std::is_x86_feature_detected!("fma") {
        &["-mfma"]
    } else {
        flags
    };
    for optimization in ["-O0", "-O2"] {
        // Exercise both strict C11 and the compiler's default language mode:
        // GCC's default GNU mode can contract even without fast-math.
        for standard in [Some("-std=c11"), None] {
            let actual = c.run_c(
                &format!("{generated}{driver}"),
                optimization,
                standard,
                flags,
            );
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty(), "{actual:?}");
            assert_eq!(
                String::from_utf8(actual.stdout).unwrap().trim(),
                "00000000 0000000000000000",
                "{optimization} {standard:?}"
            );
        }
    }
}

#[test]
fn rounding_example_matches_known_values_and_vm() {
    let source = include_str!("../examples/float_rounding.ceru");
    assert_eq!(run_vm(source).unwrap(), "0\n0\n0\n0\n-0\n-0\ntrue\ntrue\n");
    if let Some(c) = NativeC::new() {
        c.matches_vm(source);
    }
}
