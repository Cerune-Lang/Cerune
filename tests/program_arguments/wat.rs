use super::{c::compare, directory, execute, hir, process, snapshot};
use cerune_lang::{
    codegen::wat,
    runtime::arguments::{Arguments, Code, Limits},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
fn build(text: &str) -> Option<(PathBuf, Command)> {
    let configured = std::env::var_os("CERUNE_TEST_NODE");
    let node = configured.clone().unwrap_or_else(|| "node".into());
    let wabt_config = std::env::var_os("CERUNE_TEST_WAT2WASM_JS");
    let wabt = wabt_config.clone().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/wasm-tools/node_modules/wabt/bin/wat2wasm")
    });
    if !Command::new(&node)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(configured.is_none(), "required Node unavailable");
        eprintln!("WAT argument tests skipped: Node unavailable");
        return None;
    }
    if !wabt.is_file() {
        assert!(wabt_config.is_none(), "required WABT unavailable");
        eprintln!("WAT argument tests skipped; set CERUNE_TEST_WAT2WASM_JS to require WABT");
        return None;
    }
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = directory(&format!("wat-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let source = dir.join("program.wat");
    let binary = dir.join("program.wasm");
    fs::write(&source, text).unwrap();
    let out = process::bounded_output(
        Command::new(&node)
            .arg(wabt)
            .arg(&source)
            .arg("-o")
            .arg(&binary),
        &dir,
        "arguments-wat-build",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        source.display()
    );
    let mut cmd = Command::new(node);
    cmd.arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/run_wasm_input.cjs"))
        .arg(binary);
    Some((dir, cmd))
}
fn run(dir: &std::path::Path, cmd: &mut Command, config: Option<&str>, input: &[u8]) -> Output {
    cmd.env_remove("CERUNE_TEST_WAT_ARGUMENTS");
    if let Some(config) = config {
        let file = dir.join("arguments.json");
        fs::write(&file, config).unwrap();
        cmd.env("CERUNE_TEST_WAT_ARGUMENTS_FILE", file);
    } else {
        cmd.env_remove("CERUNE_TEST_WAT_ARGUMENTS_FILE");
    }
    let input_file = dir.join("input.bin");
    fs::write(&input_file, input).unwrap();
    process::bounded_output_with_stdin(
        cmd,
        dir,
        "arguments-wat-run",
        Duration::from_secs(30),
        Stdio::from(fs::File::open(input_file).unwrap()),
    )
    .unwrap()
}
fn config(values: &[&str]) -> String {
    format!(
        "{{\"values\":{:?}}}",
        values.iter().map(|s| s.as_bytes()).collect::<Vec<_>>()
    )
}
fn capture_failure(actual: Output, code: &str) {
    assert_eq!(actual.status.code(), Some(1));
    assert!(actual.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr),
        format!("cerune: arguments-v1 phase=capture code={code}\n")
    );
}
#[test]
fn wat_example_matches_four_executors_without_public_memory_or_runtime_host_reads() {
    let p = hir(include_str!("../../examples/arguments/list.ceru"));
    let original = p.clone();
    let text = wat::emit_wat(&p).unwrap();
    assert_eq!(p, original);
    assert!(!text.contains("(export \"memory\""));
    assert!(!text.contains("(import \"cerune\" \"memory\""));
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    cmd.env("CERUNE_TEST_WAT_INPUT", r#"{"retry_after_success":true}"#);
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
            &run(&dir, &mut cmd, Some(&config(&values)), b""),
        );
    }
}
#[test]
fn wat_ownership_short_circuit_and_failures_match_existing_executors() {
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
            "import \"std:env\" as env; fn main()->void {print(env::arg_count());print(env::arg(0));print(env::arg(9223372036854775807));}",
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
    ] {
        let mut p = hir(source);
        p.string_heap_limit = limit;
        let text=wat::emit_wat(&p).unwrap().replace("  )\n  (export \"main\"", "    (if (i64.ne (global.get $cerune_heap_live) (i64.const 0)) (then unreachable))\n  )\n  (export \"main\"");
        let Some((dir, mut cmd)) = build(&text) else {
            return;
        };
        cmd.env("CERUNE_TEST_WAT_INPUT", r#"{"retry_after_failure":true}"#);
        compare(
            &p,
            &values,
            &run(&dir, &mut cmd, Some(&config(&values)), b""),
        );
    }
}
#[test]
fn wat_capture_limits_validation_and_allocation_failure_are_redacted() {
    let p = hir("import \"std:env\" as env; print(99); print(env::arg_count());");
    let text = wat::emit_wat_with_argument_limits(
        &p,
        Limits {
            max_count: 2,
            max_bytes: 3,
        },
    )
    .unwrap();
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    compare(
        &p,
        &["あ", ""],
        &run(&dir, &mut cmd, Some(&config(&["あ", ""])), b""),
    );
    for (cfg, expected) in [
        (r#"{"values":["","",""]}"#, "count-limit-exceeded"),
        (r#"{"values":["SECRET"]}"#, "byte-limit-exceeded index=0"),
        (r#"{"values":["ab","cd"]}"#, "byte-limit-exceeded index=1"),
        (r#"{"values":[[255,0,0,0]]}"#, "byte-limit-exceeded index=0"),
        (r#"{"values":[[255,0]]}"#, "invalid-utf8 index=0"),
        (r#"{"values":[[65,0]]}"#, "embedded-nul index=0"),
        (
            r#"{"values":[[240,128,128,128]]}"#,
            "byte-limit-exceeded index=0",
        ),
        (r#"{"values":[[237,160,128]]}"#, "invalid-utf8 index=0"),
        (r#"{"values":[[194]]}"#, "invalid-utf8 index=0"),
    ] {
        capture_failure(run(&dir, &mut cmd, Some(cfg), b""), expected);
    }
    let text = text.replace(
        "(call $cerune_arguments_reserve (i64.add (local.get $table_size) (local.get $bytes)))",
        "(i32.const 0)",
    );
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    capture_failure(
        run(&dir, &mut cmd, Some(&config(&["a"])), b""),
        "allocation-failed",
    );
}
#[test]
fn wat_argument_host_protocol_failures_and_unconfigured_keep_their_phase() {
    let p = hir("import \"std:env\" as env; print(7);print(env::arg_count());print(99);");
    let Some((dir, mut cmd)) = build(&wat::emit_wat(&p).unwrap()) else {
        return;
    };
    for (cfg, expected) in [
        (r#"{"count":-1}"#, "invalid-response"),
        (r#"{"count_status":99}"#, "invalid-response"),
        (r#"{"count_status":2}"#, "connection-failed"),
        (r#"{"count":1,"count_status":1}"#, "invalid-response"),
        (
            r#"{"values":["a"],"length":-1}"#,
            "invalid-response index=0",
        ),
        (
            r#"{"values":["a"],"length_status":2}"#,
            "connection-failed index=0",
        ),
        (
            r#"{"values":["a"],"length_status":99}"#,
            "invalid-response index=0",
        ),
        (r#"{"values":["a"],"byte":256}"#, "invalid-response index=0"),
        (r#"{"values":["a"],"byte":-1}"#, "invalid-response index=0"),
        (
            r#"{"values":["a"],"byte_status":2}"#,
            "connection-failed index=0",
        ),
        (
            r#"{"values":["a"],"byte_status":99}"#,
            "invalid-response index=0",
        ),
    ] {
        capture_failure(run(&dir, &mut cmd, Some(cfg), b""), expected);
    }
    let (failure, output) = execute(0, &p, &Arguments::unconfigured()).unwrap_err();
    let actual = run(&dir, &mut cmd, Some(r#"{"configured":false}"#), b"");
    assert_eq!(actual.status.code(), Some(1));
    assert_eq!(actual.stdout, output.as_bytes());
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr),
        format!("cerune: {}\n", failure.record())
    );
}
#[test]
fn wat_old_hosts_fail_instantiation_but_argument_free_programs_stay_compatible() {
    let p = hir("import \"std:env\" as env; print(env::arg_count());");
    let Some((dir, mut cmd)) = build(&wat::emit_wat(&p).unwrap()) else {
        return;
    };
    let old = run(&dir, &mut cmd, None, b"");
    assert!(!old.status.success());
    assert!(old.stdout.is_empty());
    assert!(String::from_utf8_lossy(&old.stderr).contains("arguments_count_v1"));
    for name in [
        "arguments_count_v1",
        "argument_length_v1",
        "argument_byte_v1",
        "flush_stdout_v1",
    ] {
        cmd.env(
            "CERUNE_TEST_WAT_INPUT",
            format!("{{\"omit\":[\"{name}\"]}}"),
        );
        let out = run(&dir, &mut cmd, Some(&config(&[])), b"");
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains(name));
    }
    let p = hir("print(123);");
    let text = wat::emit_wat(&p).unwrap();
    assert!(!text.contains("arguments_count_v1"));
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    let out = run(&dir, &mut cmd, None, b"");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"123\n");
}
#[test]
fn wat_arguments_and_stdin_keep_order_and_returned_allocation_failures_keep_origin() {
    let p = hir(
        r#"import "std:env" as env; import "std:io" as io; print(env::arg_count());print(io::read_line(8));print(env::arg(0));"#,
    );
    let Some((dir, mut cmd)) = build(&wat::emit_wat(&p).unwrap()) else {
        return;
    };
    let out = run(&dir, &mut cmd, Some(&config(&["arg"])), b"line\n");
    assert!(out.status.success(), "{out:?}");
    assert!(out.stderr.is_empty());
    assert_eq!(
        out.stdout,
        b"1\nLine{text: \"line\\n\"}\nValue{text: \"arg\"}\n"
    );
    let mut p = hir("import \"std:env\" as env;print(7);print(env::arg(0));print(99);");
    let text = wat::emit_wat(&p).unwrap().replace(
        "(call $cerune_string_allocate (local.get $length)) local.set $code local.set $value",
        "i32.const 0 i32.const 3 local.set $code local.set $value",
    );
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    let out = run(&dir, &mut cmd, Some(&config(&["SECRET"])), b"");
    p.string_heap_limit = 5;
    let (mut failure, output) = execute(0, &p, &snapshot(&["SECRET"])).unwrap_err();
    failure.code = Code::AllocationFailed;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, output.as_bytes());
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        format!("cerune: {}\n", failure.record())
    );
}
#[test]
fn wat_cli_limits_are_explicit_and_do_not_capture_host_arguments() {
    let dir = directory("wat-cli");
    let file = dir.join("source.ceru");
    fs::write(&file, "import \"std:env\" as env; print(env::arg_count());").unwrap();
    let out = process::bounded_output(
        Command::new(env!("CARGO_BIN_EXE_cerune"))
            .arg("emit-wat")
            .arg(&file)
            .args(["--argument-count-limit", "2", "--argument-byte-limit", "3"]),
        &dir,
        "wat-cli",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("$cerune_arguments_max_count i64 (i64.const 2)"));
    assert!(text.contains("$cerune_arguments_max_bytes i64 (i64.const 3)"));
    for flags in [
        vec!["--", "SECRET"],
        vec!["--argument-count-limit", "18446744073709551615"],
        vec!["--argument-byte-limit", "-1"],
        vec!["--argument-count-limit", "1", "--argument-count-limit", "2"],
    ] {
        let out = process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("emit-wat")
                .arg(&file)
                .args(flags),
            &dir,
            "wat-cli-invalid",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("SECRET"));
    }
    let p = hir("import \"std:env\" as env;");
    assert!(
        wat::emit_wat_with_argument_limits(
            &p,
            Limits {
                max_count: u64::MAX,
                max_bytes: 0
            }
        )
        .is_err()
    );
}
#[test]
fn wat_test_host_freezes_input_and_rejects_unpaired_surrogates() {
    let Some((dir, cmd)) = build(&wat::emit_wat(&hir("print(1);")).unwrap()) else {
        return;
    };
    let script = dir.join("host-check.cjs");
    fs::write(&script, r#"
const assert=require('node:assert/strict');
const {createArguments}=require(process.argv[2]);
const source={values:[[65,66],''],count:2};
const host=createArguments(source);
source.values[0][0]=90;source.values.push('later');source.count=9;
assert.deepEqual(host.imports.arguments_count_v1(),[2n,0]);
assert.deepEqual(host.imports.argument_byte_v1(0n,0n),[65,0]);
assert.deepEqual(host.imports.argument_length_v1(1n),[0n,0]);
for(const value of ['\ud800','\udc00','SECRET\ud800']) {
  assert.throws(()=>createArguments({values:[value]}),error=>error.message==='arguments-v1 phase=capture code=invalid-unicode index=0');
}
"#).unwrap();
    let output = process::bounded_output(
        Command::new(cmd.get_program())
            .arg(script)
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/wat_arguments.cjs")),
        &dir,
        "wat-argument-host",
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(output.status.success(), "{output:?}");
}
#[test]
fn wat_snapshot_growth_is_separate_from_returned_string_limits() {
    let mut p = hir("import \"std:env\" as env;print(env::arg_count());print(env::arg(1));");
    p.string_heap_limit = 5;
    let text = wat::emit_wat(&p).unwrap();
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    let large = "a".repeat(70000);
    let values = [large.as_str(), "small"];
    compare(
        &p,
        &values,
        &run(&dir, &mut cmd, Some(&config(&values)), b""),
    );
    let memory = text
        .lines()
        .find(|l| l.trim_start().starts_with("(memory "))
        .unwrap();
    let pages = memory
        .trim()
        .trim_start_matches("(memory ")
        .trim_end_matches(')');
    let limited = text.replace(memory, &format!("  (memory {pages} {pages})"));
    let Some((dir, mut cmd)) = build(&limited) else {
        return;
    };
    capture_failure(
        run(&dir, &mut cmd, Some(&config(&values)), b""),
        "allocation-failed",
    );
    let text = wat::emit_wat_with_argument_limits(
        &p,
        Limits {
            max_count: 0,
            max_bytes: 0,
        },
    )
    .unwrap();
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    compare(&p, &[], &run(&dir, &mut cmd, Some(&config(&[])), b""));
}

#[test]
fn integer_parsing_wat_matches_independent_expectations() {
    for group in super::integer_parsing::compiled_cases() {
        let p = &group.program;
        let text = wat::emit_wat(p).unwrap();
        assert!(!text.contains("(export \"memory\""));
        let Some((dir, mut cmd)) = build(&text) else {
            return;
        };
        for (args, expected) in &group.runs {
            super::integer_parsing::check(&run(&dir, &mut cmd, Some(&config(args)), &[]), expected);
        }
    }
}

#[test]
fn optional_let_formatting_wat_preserves_execution() {
    for (p, expected) in super::formatting::cases() {
        let Some((dir, mut cmd)) = build(&wat::emit_wat(&p).unwrap()) else {
            return;
        };
        super::formatting::check(&run(&dir, &mut cmd, Some(&config(&[])), &[]), expected);
    }
}
