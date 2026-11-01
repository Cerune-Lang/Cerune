use super::{Memory, Step, execute, hir};
use crate::{process, program_arguments::wat::build};
use cerune_lang::{
    codegen::wat,
    ir,
    runtime::files::{Files, OpenError},
};
use serde_json::{Value, json};
use std::{fs, process::Command, time::Duration};

fn run(dir: &std::path::Path, cmd: &mut Command, config: &Value) -> (std::process::Output, Value) {
    let settings = dir.join("files.json");
    let trace = dir.join("files-trace.json");
    fs::write(&settings, serde_json::to_vec(config).unwrap()).unwrap();
    cmd.env("CERUNE_TEST_WAT_FILES_FILE", settings)
        .env("CERUNE_TEST_WAT_FILES_TRACE", &trace)
        .env(
            "CERUNE_TEST_WAT_INPUT",
            r#"{"retry_after_failure":true,"retry_after_success":true}"#,
        );
    let output =
        process::bounded_output(cmd, dir, "wat-files-run", Duration::from_secs(30)).unwrap();
    let trace: Value = serde_json::from_slice(&fs::read(trace).unwrap()).unwrap();
    (output, trace)
}
fn memory(config: &Value) -> Memory {
    let memory = Memory::default();
    let mut s = memory.0.borrow_mut();
    if let Some(files) = config["files"].as_array() {
        for pair in files {
            s.files.insert(
                pair[0].as_str().unwrap().into(),
                pair[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|b| b.as_u64().unwrap() as u8)
                    .collect(),
            );
        }
    }
    for key in ["reads", "writes"] {
        let steps = if key == "reads" {
            &mut s.reads
        } else {
            &mut s.writes
        };
        if let Some(values) = config[key].as_array() {
            for v in values {
                steps.push_back(if v == "invalid" {
                    Step::Invalid
                } else {
                    match v.as_i64().unwrap() {
                        -2 => Step::Interrupted,
                        -1 => Step::Fail,
                        0 => Step::Zero,
                        n if n > 0 => Step::Count(n as usize),
                        _ => Step::Invalid,
                    }
                });
            }
        }
    }
    s.close_error = config["closeError"] == true;
    if let Some(response) = config["openResponse"].as_array() {
        s.open_error = Some(match response[1].as_i64().unwrap() {
            1 => OpenError::NotFound,
            2 => OpenError::AlreadyExists,
            3 => OpenError::UnsupportedTarget,
            4 => OpenError::Failed,
            _ => OpenError::InvalidResponse,
        });
    }
    drop(s);
    memory
}
fn generated(program: &ir::Program) -> String {
    let before = program.clone();
    let mut text =
        wat::emit_wat_with_call_limit(program, std::num::NonZeroU32::new(128).unwrap()).unwrap();
    assert_eq!(&before, program);
    assert!(!text.contains("(export \"memory\""));
    assert!(text.contains("file_open_v1"));
    // Test-only inspection of ownership after the whole source program has returned.
    let end = text.rfind("\n  )\n").unwrap();
    text.insert_str(
        end,
        "\n    (if (i64.ne (global.get $cerune_heap_live) (i64.const 0)) (then unreachable))",
    );
    text
}
fn compare(program: &ir::Program, config: Value) {
    let Some((dir, mut cmd)) = build(&generated(program)) else {
        return;
    };
    let (actual, trace) = run(&dir, &mut cmd, &config);
    for route in 0..4 {
        let memory = memory(&config);
        let mut files = if config["configured"] == false {
            Files::unconfigured()
        } else {
            Files::new(memory.clone())
        };
        match execute(route, program, &mut files) {
            Ok(output) => {
                assert!(actual.status.success(), "{actual:?}\n{config}");
                assert_eq!(actual.stdout, output.as_bytes());
                assert!(actual.stderr.is_empty());
            }
            Err((failure, output)) => {
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, output.as_bytes());
                assert_eq!(
                    String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
                    format!("cerune: {}\n", failure.record()),
                    "{config}"
                );
            }
        }
        for (name, bytes) in trace["contents"].as_object().unwrap() {
            assert_eq!(
                *bytes,
                memory
                    .0
                    .borrow()
                    .files
                    .get(name)
                    .map_or(Value::Null, |b| json!(b))
            );
        }
        if config["traceLimit"] != 0 {
            let closes = trace["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["operation"] == "close")
                .count();
            assert_eq!(
                closes,
                memory
                    .0
                    .borrow()
                    .calls
                    .iter()
                    .filter(|c| *c == "close")
                    .count()
            );
        }
        assert_eq!(trace["handles"], 0);
    }
}
#[test]
fn wat_example_transfers_and_failures_match_four_executors() {
    for trace in [0, 1, 1000] {
        // Keep full close records for differential comparison; the ABI test covers truncation.
        let p = hir(include_str!("../../examples/files/roundtrip.ceru"));
        let Some((dir, mut cmd)) = build(&generated(&p)) else {
            return;
        };
        let (out, state) = run(
            &dir,
            &mut cmd,
            &json!({"traceLimit":trace,"inspect":["sample.txt"],"writes":[-2,1,2],"reads":[-2,1,2]}),
        );
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"true\n16\n");
        assert_eq!(
            state["contents"]["sample.txt"],
            json!("日本語\0\r\ne\u{301}\n".as_bytes())
        );
        assert_eq!(state["handles"], 0);
    }
    for writing in [false, true] {
        let p = hir(if writing {
            r#"import "std:fs" as fs; print("before"); fs::write_new("x","abc",3); print("after");"#
        } else {
            r#"import "std:fs" as fs; print("before"); print(fs::read_text("x",3)); print("after");"#
        });
        for step in [
            json!([-2, 1, 1, 1]),
            json!([-2, 1, -1]),
            json!([-2, 1, "invalid"]),
            json!([-2, 1, 0]),
            json!([-2, 1, -3]),
        ] {
            for close in [false, true] {
                let mut config = json!({"inspect":["x"],"closeError":close});
                if !writing {
                    config["files"] = json!([["x", b"abc"]]);
                }
                config[if writing { "writes" } else { "reads" }] = step.clone();
                compare(&p, config);
            }
        }
        for status in 1..=5 {
            compare(&p, json!({"openResponse":[0,status],"inspect":["x"]}));
        }
    }
}
#[test]
fn wat_names_limits_empty_eof_and_utf8_match_four_executors() {
    for name in [
        "",
        "a/b",
        "a\\b",
        "x:",
        "x ",
        "x.",
        ".",
        "..",
        "CON",
        "com1.txt",
        "LPT².ext",
        "aux .txt",
        "ConIn$.x",
        "x\u{7f}",
        "x\u{1f}",
        "x?",
        "x*",
    ] {
        let p = hir(&format!(
            "import \"std:fs\" as fs; fs::write_new({name:?},\"\",0);"
        ));
        compare(&p, json!({"inspect":[]}));
    }
    for name in [
        "a",
        "COM0",
        "com10.txt",
        ".hidden",
        "日本語",
        "e\u{301}",
        "é",
        "内部 空白",
    ] {
        let p = hir(&format!(
            "import \"std:fs\" as fs; fs::write_new({name:?},\"\",0); print(fs::read_text({name:?},0));"
        ));
        compare(&p, json!({"inspect":[name]}));
    }
    for n in [255, 256] {
        let name = "x".repeat(n);
        let p = hir(&format!(
            "import \"std:fs\" as fs; fs::write_new(\"{name}\",\"\",0);"
        ));
        compare(&p, json!({"inspect":[name]}));
    }
    for bytes in [
        vec![],
        b"a".to_vec(),
        b"abc".to_vec(),
        vec![255],
        vec![0xe3, 0x81],
        b"a\r\n".to_vec(),
    ] {
        for limit in [0, 1, 2, 3, 4] {
            let p = hir(&format!(
                "import \"std:fs\" as fs; print(\"before\"); print(fs::read_text(\"x\",{limit})); print(\"after\");"
            ));
            compare(&p, json!({"files":[["x",bytes]],"inspect":["x"]}));
        }
    }
    for limit in ["-1", "2147483648", "-9223372036854775808"] {
        compare(
            &hir(&format!(
                "import \"std:fs\" as fs; print(fs::read_text(\"x\",{limit}));"
            )),
            json!({"inspect":[]}),
        );
    }
    compare(
        &hir(r#"import "std:fs" as fs; fs::write_new("x","abc",2);"#),
        json!({"inspect":["x"]}),
    );
    compare(
        &hir(r#"import "std:fs" as fs; fs::write_new("x","abc",3);"#),
        json!({"inspect":["x"],"configured":false}),
    );
    compare(
        &hir(r#"import "std:fs" as fs; fs::write_new("x","abc",3);"#),
        json!({"files":[["x",b"old"]],"inspect":["x"]}),
    );
}
#[test]
fn wat_large_reads_budget_and_ownership_match_four_executors() {
    let text = "日本語\0\r\ne\u{301}\n".repeat(300);
    let bytes = text.as_bytes();
    for limit in [0, 4095, 4096, bytes.len(), bytes.len() + 1] {
        let p = hir(&format!(
            "import \"std:fs\" as fs; print(byte_len(fs::read_text(\"x\",{limit})));"
        ));
        compare(&p, json!({"files":[["x",bytes]],"inspect":["x"]}));
    }
    let mut p = hir(r#"import "std:fs" as fs; print(fs::read_text("x",8));"#);
    p.string_heap_limit = 7;
    compare(&p, json!({"files":[["x",b"x"]],"inspect":["x"]}));
    let p = hir(r#"import "std:fs" as fs;
      fn read(n:i64)->string { if n==0 {return fs::read_text("x",16);} return read(n-1); }
      mut saved:string=read(4); copy:string=saved; saved="changed";
      print(copy); print(saved);
      print(false && fs::read_text("absent",1)=="x");
      for(mut i:i64=0;i<20;i=i+1){print(byte_len(read(1)));}
      fs::write_new("out",copy,16);"#);
    compare(&p, json!({"files":[["x",b"hello"]],"inspect":["out"]}));
}

#[test]
fn wat_host_protocol_and_physical_allocation_failures_keep_origins() {
    use cerune_lang::runtime::files::{Code, Created, Phase};
    let p = hir(
        r#"import "std:fs" as fs; print("before"); print(fs::read_text("x",8)); print("after");"#,
    );
    let (template, _) = execute(0, &p, &mut Files::<Memory>::unconfigured()).unwrap_err();
    let text = generated(&p);
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    for (config, code, phase) in [
        (json!({"status":99}), Code::InvalidResponse, Phase::Validate),
        (json!({"beginStatus":1}), Code::OpenFailed, Phase::Open),
        (
            json!({"beginStatus":99}),
            Code::InvalidResponse,
            Phase::Open,
        ),
        (json!({"chunkStatus":1}), Code::OpenFailed, Phase::Open),
        (
            json!({"chunkStatus":99}),
            Code::InvalidResponse,
            Phase::Open,
        ),
        (
            json!({"openResponse":[0,0]}),
            Code::InvalidResponse,
            Phase::Open,
        ),
        (
            json!({"openResponse":[-1,0]}),
            Code::InvalidResponse,
            Phase::Open,
        ),
        (
            json!({"openResponse":[1,4]}),
            Code::InvalidResponse,
            Phase::Open,
        ),
    ] {
        let (actual, state) = run(&dir, &mut cmd, &config);
        let mut expected = template;
        expected.code = code;
        expected.details.phase = phase;
        assert!(!actual.status.success());
        assert_eq!(actual.stdout, b"before\n");
        assert_eq!(
            String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
            format!("cerune: {}\n", expected.record())
        );
        assert_eq!(state["handles"], 0);
        assert!(
            state["events"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["operation"] == "open")
        );
    }
    let p = hir(r#"import "std:fs" as fs; fs::write_new("x","a",1);"#);
    let (mut expected, _) = execute(0, &p, &mut Files::<Memory>::unconfigured()).unwrap_err();
    expected.code = Code::InvalidResponse;
    expected.details.phase = Phase::Open;
    expected.details.created = Created::Unknown;
    let Some((dir, mut cmd)) = build(&generated(&p)) else {
        return;
    };
    let (actual, _) = run(&dir, &mut cmd, &json!({"openResponse":[1,4]}));
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: {}\n", expected.record())
    );

    let mut p = hir(r#"import "std:fs" as fs; print(fs::read_text("x",2000000));"#);
    p.string_heap_limit = 4000000;
    let text = generated(&p).replace("(memory 1)", "(memory 1 1)");
    assert!(text.contains("(memory 1 1)"));
    let (mut expected, _) = execute(0, &p, &mut Files::<Memory>::unconfigured()).unwrap_err();
    expected.code = Code::AllocationFailed;
    expected.details.phase = Phase::Reserve;
    let Some((dir, mut cmd)) = build(&text) else {
        return;
    };
    let (actual, state) = run(&dir, &mut cmd, &json!({"inspect":[]}));
    assert_eq!(
        String::from_utf8_lossy(&actual.stderr).replace("\r\n", "\n"),
        format!("cerune: {}\n", expected.record())
    );
    assert_eq!(state["events"], json!([]));
}
#[test]
fn wat_argument_order_large_writes_and_required_imports_are_explicit() {
    let p = hir(r#"import "std:fs" as fs;
      fn name()->string {print("name");return "x";}
      fn text()->string {print("text");return "abc";}
      fn limit()->i64 {print("limit");return 3;}
      fs::write_new(name(),text(),limit());
      print(fs::read_text(name(),limit()));"#);
    compare(&p, json!({"inspect":["x"]}));
    let bytes = "日本語\0\r\ne\u{301}\n".repeat(300).into_bytes();
    let p = hir(
        r#"import "std:fs" as fs; text:string=fs::read_text("x",5000); fs::write_new("out",text,5000); print(byte_len(text));"#,
    );
    compare(
        &p,
        json!({"files":[["x",bytes]],"inspect":["out"],"traceLimit":20000}),
    );
    let p = hir(
        r#"import "std:fs" as fs; fn unused()->string {return fs::read_text("x",0);} print(1);"#,
    );
    let Some((dir, mut cmd)) = build(&generated(&p)) else {
        return;
    };
    let path = dir.join("files.json");
    fs::write(&path, b"{}").unwrap();
    cmd.env("CERUNE_TEST_WAT_FILES_FILE", path);
    for import in [
        "files_status_v1",
        "file_name_begin_v1",
        "file_name_chunk_v1",
        "file_open_v1",
        "file_read_v1",
        "file_write_v1",
        "file_close_v1",
    ] {
        cmd.env(
            "CERUNE_TEST_WAT_INPUT",
            json!({"omit":[import]}).to_string(),
        );
        let actual = process::bounded_output(
            &mut cmd,
            &dir,
            "missing-file-import",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(!actual.status.success());
        assert!(actual.stdout.is_empty());
        assert!(String::from_utf8_lossy(&actual.stderr).contains(import));
    }
}

#[test]
fn wat_files_compose_with_input_arguments_and_flush() {
    use cerune_lang::runtime::{
        arguments::{Arguments, Limits},
        input::Input,
        output::Output,
    };
    use cerune_lang::{bytecode, ir_executor, mir, mir_executor, ssa_executor, vm};
    let p = hir(
        r#"import "std:fs" as fs; import "std:env" as env; import "std:io" as io;
      print(env::arg_count()); flush();
      match io::read_line(8) { io::ReadLine::Line{text:text}=>{fs::write_new("x",text,8);},io::ReadLine::End{}=>{} }
      print(fs::read_text("x",8));"#,
    );
    let Some((dir, mut cmd)) = build(&generated(&p)) else {
        return;
    };
    let config = dir.join("files.json");
    fs::write(&config, br#"{"inspect":["x"]}"#).unwrap();
    let input = dir.join("stdin");
    fs::write(&input, b"abc\n").unwrap();
    let trace = dir.join("trace.json");
    cmd.env("CERUNE_TEST_WAT_FILES_FILE", config)
        .env("CERUNE_TEST_WAT_FILES_TRACE", &trace)
        .env("CERUNE_TEST_WAT_ARGUMENTS", r#"{"values":["one"]}"#);
    let actual = process::bounded_output_with_stdin(
        &mut cmd,
        &dir,
        "combined-files",
        Duration::from_secs(30),
        std::process::Stdio::from(fs::File::open(input).unwrap()),
    )
    .unwrap();
    assert!(actual.status.success(), "{actual:?}");
    for route in 0..4 {
        let memory = Memory::default();
        let mut files = Files::new(memory.clone());
        let mut input = Input::new(std::io::Cursor::new(b"abc\n"));
        let mut bytes = Vec::new();
        let mut output = Output::new(&mut bytes);
        let args = Arguments::capture(&[b"one"], Limits::default()).unwrap();
        let depth = std::num::NonZeroU32::new(128).unwrap();
        match route {
            0 => ir_executor::run_with_io_files_arguments_and_call_limit(
                &p,
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            1 => mir_executor::run_with_io_files_arguments_and_call_limit(
                &mir::lower(&p).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            2 => ssa_executor::run_with_io_files_arguments_and_call_limit(
                &mir::ssa::construct(&mir::lower(&p).unwrap()).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
            _ => vm::run_with_io_files_arguments_and_call_limit(
                &bytecode::lower(&p).unwrap(),
                &mut input,
                &mut output,
                &mut files,
                &args,
                depth,
            )
            .unwrap(),
        }
        assert_eq!(actual.stdout, bytes);
        assert_eq!(memory.0.borrow().files["x"], b"abc\n");
    }
    let state: Value = serde_json::from_slice(&fs::read(trace).unwrap()).unwrap();
    assert_eq!(state["contents"]["x"], json!(b"abc\n"));
}
