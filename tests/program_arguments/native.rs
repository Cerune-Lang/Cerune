use super::{
    c::{compare, run},
    directory, execute, hir, process, snapshot,
};
use cerune_lang::{
    codegen::x86_64::{self as native, Target},
    ir, mir,
    runtime::arguments::{Arguments, Code, Limits},
};
use std::{
    ffi::OsStr,
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
fn target() -> Target {
    if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    }
}
fn link(bytes: &[u8], extension: &str) -> Option<(PathBuf, PathBuf)> {
    if !cfg!(any(windows, target_os = "linux")) {
        return None;
    }
    let configured = std::env::var_os("CERUNE_TEST_ASM_CLANG");
    let cc = configured.clone().unwrap_or_else(|| "clang".into());
    if !Command::new(&cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(configured.is_none(), "required Native linker unavailable");
        eprintln!("Native arguments skipped: linker unavailable");
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = directory(&format!("native-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let file = dir.join(format!("program.{extension}"));
    let exe = dir.join(format!("program{}", std::env::consts::EXE_SUFFIX));
    fs::write(&file, bytes).unwrap();
    let out = process::bounded_output(
        Command::new(cc).arg(&file).arg("-o").arg(&exe),
        &dir,
        "native-arguments-link",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        file.display()
    );
    Some((dir, exe))
}
fn assembly(p: &ir::Program, limits: Limits) -> String {
    native::emit_asm_from_lir(
        &native::lower_mir_with_argument_limits(&mir::lower(p).unwrap(), target(), limits).unwrap(),
        true,
    )
    .unwrap()
}
fn executables(p: &ir::Program, limits: Limits) -> Vec<(PathBuf, PathBuf)> {
    let m = mir::lower(p).unwrap();
    let s = mir::ssa::lower(&mir::ssa::construct(&m).unwrap())
        .unwrap()
        .program;
    let mut out = Vec::new();
    for m in [&m, &s] {
        let lir = native::lower_mir_with_argument_limits(m, target(), limits).unwrap();
        let asm = native::emit_asm_from_lir(&lir, true).unwrap();
        let obj = native::emit_object_from_lir(&lir, true).unwrap();
        for (bytes, ext) in [
            (asm.as_bytes(), "s"),
            (obj.as_slice(), if cfg!(windows) { "obj" } else { "o" }),
        ] {
            if let Some(pair) = link(bytes, ext) {
                out.push(pair);
            }
        }
    }
    out
}
fn harness(p: &ir::Program, data: &str, body: &str) -> String {
    let entry = if cfg!(windows) { "wmain" } else { "main" };
    let text = assembly(p, Limits::default())
        .replace(&format!(".globl {entry}\n{entry}:"), "unused_os_entry:");
    format!(
        "{text}\n.data\n{data}\n.text\n.globl main\nmain:\n  subq $40, %rsp\n{body}\n  xorl %eax, %eax\n  addq $40, %rsp\n  retq\n"
    )
}
const VALUES: &str = ".Ltest_data:\n  .byte 83,69,67,82,69,84\n.p2align 3\n.Ltest_values:\n  .quad .Ltest_data\n  .quad 6";
const CAPTURE: &str =
    "  movq $1, %rax\n  leaq .Ltest_values(%rip), %rdx\n  callq cerune_arguments_capture\n";
fn capture_failure(actual: &std::process::Output, code: &str) {
    assert_eq!(actual.status.code(), Some(1), "{actual:?}");
    assert!(actual.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: arguments-v1 phase=capture code={code}\n")
    );
}
#[test]
fn native_lir_preserves_arguments_and_origins_for_both_targets() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    let original = p.clone();
    let m = mir::lower(&p).unwrap();
    for t in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
        let mut lir = native::lower_mir(&m, t).unwrap();
        let text = native::lir::emit(&lir).unwrap();
        assert!(text.contains("ArgumentCount"));
        assert!(text.contains("ArgumentGet"));
        assert!(text.contains("argument-count-limit 4096"));
        let asm = native::emit_asm_from_lir(&lir, true).unwrap();
        assert_eq!(
            asm.contains("callq WideCharToMultiByte"),
            !t.triple().contains("linux")
        );
        assert!(asm.contains("cerune_arguments_capture:"));
        let obj = native::emit_object_from_lir(&lir, true).unwrap();
        assert_eq!(
            &obj[..2],
            if t.triple().contains("linux") {
                b"\x7fE"
            } else {
                b"\x64\x86"
            }
        );
        lir.argument_limits.max_count = u64::MAX;
        assert!(
            native::lir::validate(&lir)
                .unwrap_err()
                .message()
                .contains("i64::MAX")
        );
        assert!(native::lower_mir_with_argument_limits(&m, t, lir.argument_limits).is_err());
        let mut lir = native::lower_mir(&m, t).unwrap();
        let f = lir
            .functions
            .iter_mut()
            .find(|f| {
                f.instructions
                    .contains(&native::ir::Instruction::ArgumentCount)
            })
            .unwrap();
        let i = f
            .instructions
            .iter()
            .position(|i| matches!(i, native::ir::Instruction::ArgumentCount))
            .unwrap();
        f.origins[i] = native::ir::Origin::Synthetic;
        f.mir_origins[i] = None;
        assert!(native::lir::validate(&lir).is_err());
    }
    assert_eq!(p, original);
}
#[test]
fn native_example_matches_four_executors_through_mir_ssa_asm_and_objects() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    for (dir, exe) in executables(&p, Limits::default()) {
        for values in [
            vec![],
            vec![""],
            vec![
                "日本語",
                "a\r\nb",
                "e\u{301}",
                "🙂",
                "a\" b\\",
                "--ssa",
                "--",
            ],
        ] {
            compare(
                &p,
                &values,
                &run(
                    &dir,
                    &exe,
                    &values.iter().map(OsStr::new).collect::<Vec<_>>(),
                ),
            );
        }
    }
}
#[test]
fn native_ownership_order_short_circuit_and_failures_match_four_executors() {
    let source = r#"import "std:env" as env;
fn text(i:i64)->string {match env::arg(i) {env::Argument::Value {text:t}=>{return t;},env::Argument::Missing {}=>{return "missing";},}}
fn index()->i64 {print("index");return 0;}
print(false && byte_len(text(-1))==0);
mut a:[string;2]=[text(index()),text(1)]; b:[string;2]=a; a[0]="changed"; print(a);print(b);
for(mut i:i64=0;i<8;i=i+1){print(text(0));}
mut dynamic:[string]=array_copy([text(0),text(1)]); saved:[string]=dynamic; dynamic[0]="changed";print(dynamic);print(saved);"#;
    for (source, limit, values) in [
        (source, 64, vec!["first", "second"]),
        (
            "import \"std:env\" as env; fn main()->void {print(env::arg_count()); print(env::arg(0)); print(env::arg(9223372036854775807));}",
            64,
            vec!["entry"],
        ),
        (
            "import \"std:env\" as env; print(7);print(env::arg(-1));print(99);",
            64,
            vec!["SECRET"],
        ),
        (
            "import \"std:env\" as env; print(7);print(env::arg(0));print(99);",
            4,
            vec!["SECRET"],
        ),
        (
            "import \"std:env\" as env; print(env::arg(0));",
            0,
            vec![""],
        ),
        (
            "import \"std:env\" as env; for(mut i:i64=0;i<8;i=i+1){print(env::arg(0));}",
            6,
            vec!["SECRET"],
        ),
        (
            "import \"std:env\" as env; n:i64=env::arg_count();",
            0,
            vec!["SECRET"],
        ),
    ] {
        let mut p = hir(source);
        p.string_heap_limit = limit;
        for (dir, exe) in executables(&p, Limits::default()) {
            compare(
                &p,
                &values,
                &run(
                    &dir,
                    &exe,
                    &values.iter().map(OsStr::new).collect::<Vec<_>>(),
                ),
            );
        }
        // Verify releases at the process boundary, without making heap state observable to programs.
        let text = assembly(&p,Limits::default()).replace("  callq cerune_arguments_dispose\n", "  cmpq $0, .Lcerune_heap_live(%rip)\n  je .Ltest_clean\n  ud2\n.Ltest_clean:\n  callq cerune_arguments_dispose\n");
        if let Some((dir, exe)) = link(text.as_bytes(), "s") {
            compare(
                &p,
                &values,
                &run(
                    &dir,
                    &exe,
                    &values.iter().map(OsStr::new).collect::<Vec<_>>(),
                ),
            );
        }
    }
}
#[test]
fn native_limits_and_os_unicode_fail_before_program_output() {
    let p = hir("import \"std:env\" as env; print(99);print(env::arg_count());");
    for (dir, exe) in executables(
        &p,
        Limits {
            max_count: 2,
            max_bytes: 3,
        },
    ) {
        compare(
            &p,
            &["あ", ""],
            &run(&dir, &exe, &[OsStr::new("あ"), OsStr::new("")]),
        );
        for (values, code) in [
            (vec!["", "", ""], "count-limit-exceeded"),
            (vec!["SECRET"], "byte-limit-exceeded index=0"),
            (vec!["ab", "cd"], "byte-limit-exceeded index=1"),
        ] {
            capture_failure(
                &run(
                    &dir,
                    &exe,
                    &values.iter().map(OsStr::new).collect::<Vec<_>>(),
                ),
                code,
            );
        }
        #[cfg(windows)]
        let bad = {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[83, 0xd800])
        };
        #[cfg(unix)]
        let bad = {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(vec![83, 255])
        };
        capture_failure(&run(&dir, &exe, &[&bad]), "invalid-unicode index=0");
    }
    for (dir, exe) in executables(
        &p,
        Limits {
            max_count: 0,
            max_bytes: 0,
        },
    ) {
        compare(&p, &[], &run(&dir, &exe, &[]));
    }
}
#[test]
fn native_capture_owns_contents_and_unconfigured_is_not_empty() {
    let p = hir("import \"std:env\" as env; print(env::arg_count());print(env::arg(0));");
    let text = harness(
        &p,
        VALUES,
        &format!(
            "{CAPTURE}  movb $120, .Ltest_data(%rip)\n  callq cerune_program\n  callq cerune_arguments_dispose"
        ),
    );
    if let Some((dir, exe)) = link(text.as_bytes(), "s") {
        compare(&p, &["SECRET"], &run(&dir, &exe, &[]));
    }
    for source in [
        "import \"std:env\" as env; print(7);print(env::arg_count());",
        "import \"std:env\" as env; print(7);print(env::arg(-1));",
    ] {
        let p = hir(source);
        let text = harness(&p, "", "  callq cerune_program");
        if let Some((dir, exe)) = link(text.as_bytes(), "s") {
            let (f, output) = execute(0, &p, &Arguments::unconfigured()).unwrap_err();
            let actual = run(&dir, &exe, &[]);
            assert_eq!(actual.status.code(), Some(1));
            assert_eq!(actual.stdout, output.as_bytes());
            assert_eq!(
                String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                format!("cerune: {}\n", f.record())
            );
        }
    }
}
#[test]
fn native_capture_validates_before_allocation_and_redacts_failures() {
    let p = hir("import \"std:env\" as env; print(7);print(env::arg(0));print(99);");
    for (bytes, limit, expected) in [
        ("255,0", 1, "byte-limit-exceeded index=0"),
        ("255,0", 2, "invalid-utf8 index=0"),
        ("65,0", 2, "embedded-nul index=0"),
        ("237,160,128", 3, "invalid-utf8 index=0"),
        ("240,159", 2, "invalid-utf8 index=0"),
    ] {
        let len = bytes.split(',').count();
        let data = format!(
            ".Ltest_data:\n  .byte {bytes}\n.p2align 3\n.Ltest_values:\n  .quad .Ltest_data\n  .quad {len}"
        );
        let text = harness(&p, &data, CAPTURE).replace(
            ".Largs_max_bytes:\n  .quad 1048576",
            &format!(".Largs_max_bytes:\n  .quad {limit}"),
        );
        if let Some((dir, exe)) = link(text.as_bytes(), "s") {
            capture_failure(&run(&dir, &exe, &[]), expected);
        }
    }
    let text = harness(&p, VALUES, CAPTURE).replace(
        ".Largs_max_count:\n  .quad 4096",
        ".Largs_max_count:\n  .quad -1",
    );
    if let Some((dir, exe)) = link(text.as_bytes(), "s") {
        capture_failure(&run(&dir, &exe, &[]), "invalid-limit");
    }
    for n in 0..2 {
        let mut text = harness(
            &p,
            VALUES,
            &format!("{CAPTURE}  callq cerune_program\n  callq cerune_arguments_dispose"),
        )
        .replace("callq malloc", "callq test_malloc");
        text.push_str(&format!("\n.data\n.Ltest_remaining:\n  .quad {n}\n.text\ntest_malloc:\n  cmpq $0, .Ltest_remaining(%rip)\n  je .Ltest_malloc_fail\n  subq $1, .Ltest_remaining(%rip)\n  jmp malloc\n.Ltest_malloc_fail:\n  xorl %eax, %eax\n  retq\n"));
        if let Some((dir, exe)) = link(text.as_bytes(), "s") {
            let actual = run(&dir, &exe, &[]);
            if n == 0 {
                capture_failure(&actual, "allocation-failed");
            } else {
                let mut baseline = p.clone();
                baseline.string_heap_limit = 0;
                let (mut f, output) = execute(0, &baseline, &snapshot(&["SECRET"])).unwrap_err();
                f.code = Code::AllocationFailed;
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, output.as_bytes());
                assert_eq!(
                    String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                    format!("cerune: {}\n", f.record())
                );
            }
        }
    }
}
#[test]
fn native_arguments_and_input_preserve_order_and_shared_utf8_rules() {
    let p = hir(
        r#"import "std:env" as env; import "std:io" as io; print(env::arg_count());print(io::read_line(8));print(env::arg(0));"#,
    );
    for (dir, exe) in executables(&p, Limits::default()) {
        let file = dir.join("input.txt");
        fs::write(&file, b"line\n").unwrap();
        let out = process::bounded_output_with_stdin(
            Command::new(exe).arg("arg"),
            &dir,
            "native-arguments-input",
            Duration::from_secs(30),
            std::process::Stdio::from(fs::File::open(file).unwrap()),
        )
        .unwrap();
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty());
        assert_eq!(
            out.stdout,
            b"1\nLine{text: \"line\\n\"}\nValue{text: \"arg\"}\n"
        );
    }
}
#[test]
fn native_cli_limits_are_visible_and_emission_does_not_capture_arguments() {
    let dir = directory("native-cli");
    let file = dir.join("source.ceru");
    fs::write(&file, "import \"std:env\" as env;print(env::arg_count());").unwrap();
    for route in ["emit-lir", "emit-asm", "emit-obj"] {
        let artifact = dir.join(format!("{route}.out"));
        let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg(route)
            .arg(&file)
            .args([
                "--target",
                target().triple(),
                "--ssa",
                "--argument-count-limit",
                "2",
                "--argument-byte-limit",
                "3",
                "-o",
            ])
            .arg(&artifact)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        if route != "emit-obj" {
            let text = fs::read_to_string(&artifact).unwrap();
            assert!(text.contains(if route == "emit-lir" {
                "argument-count-limit 2"
            } else {
                ".Largs_max_count:\n  .quad 2"
            }));
            assert!(text.contains(if route == "emit-lir" {
                "argument-byte-limit 3"
            } else {
                ".Largs_max_bytes:\n  .quad 3"
            }));
        }
        for flags in [
            vec!["--", "SECRET"],
            vec!["--argument-count-limit", "2", "--argument-count-limit", "3"],
            vec!["--argument-count-limit", "18446744073709551615"],
            vec!["--argument-byte-limit", "-1"],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg(route)
                .arg(&file)
                .args(["--target", target().triple()])
                .args(flags)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(!String::from_utf8_lossy(&output.stderr).contains("SECRET"));
        }
    }
}

#[test]
fn native_startup_allocation_failures_stop_before_execution_in_asm_and_object() {
    let p = hir("import \"std:env\" as env; print(7);print(env::arg(0));print(99);");
    let allocations_before_get = if cfg!(windows) { 3 } else { 2 };
    // Reserve Windows shadow space or SysV alignment before the external call.
    let call_stack = if cfg!(windows) { 40 } else { 8 };
    for remaining in 0..=allocations_before_get {
        let mut text = assembly(&p, Limits::default()).replace("callq malloc", "callq test_malloc");
        text.push_str(&format!("\n.data\n.Ltest_remaining:\n  .quad {remaining}\n.text\ntest_malloc:\n  cmpq $0, .Ltest_remaining(%rip)\n  je .Ltest_malloc_fail\n  subq $1, .Ltest_remaining(%rip)\n  subq ${call_stack}, %rsp\n  callq malloc\n  addq ${call_stack}, %rsp\n  retq\n.Ltest_malloc_fail:\n  xorl %eax, %eax\n  retq\n"));
        let selected = match target() {
            Target::X86_64PcWindowsMsvc => sashimi_assembler::Target::X86_64PcWindowsMsvc,
            Target::X86_64UnknownLinuxGnu => sashimi_assembler::Target::X86_64UnknownLinuxGnu,
        };
        let object = sashimi_assembler::assemble(&text, selected).unwrap();
        for (bytes, ext) in [
            (text.as_bytes(), "s"),
            (object.as_slice(), if cfg!(windows) { "obj" } else { "o" }),
        ] {
            let Some((dir, exe)) = link(bytes, ext) else {
                continue;
            };
            let actual = run(&dir, &exe, &[OsStr::new("SECRET")]);
            if remaining < allocations_before_get {
                capture_failure(
                    &actual,
                    if cfg!(windows) && remaining == 1 {
                        "allocation-failed index=0"
                    } else {
                        "allocation-failed"
                    },
                );
            } else {
                let mut baseline = p.clone();
                baseline.string_heap_limit = 0;
                let (mut f, output) = execute(0, &baseline, &snapshot(&["SECRET"])).unwrap_err();
                f.code = Code::AllocationFailed;
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, output.as_bytes());
                assert_eq!(
                    String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                    format!("cerune: {}\n", f.record())
                );
            }
        }
    }
}
