use super::{Workspace, process};
use cerune_lang::{
    bytecode, ir_executor, mir, mir_executor, modules, source::SourceKind, ssa_executor,
};
use std::{fs, process::Command, time::Duration};
const SOURCE: &str = include_str!("../../examples/process_results/outcome.ceru");
const EXPECTED: &str = include_str!("../../examples/process_results/outcome.stdout");
fn compare(source: &str, expected: &str) {
    let p = cerune_lang::compile_to_ir(source).unwrap();
    let m = mir::lower(&p).unwrap();
    assert_eq!(ir_executor::run(&p).unwrap(), expected);
    assert_eq!(mir_executor::run(&m).unwrap(), expected);
    assert_eq!(
        ssa_executor::run(&mir::ssa::construct(&m).unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        cerune_lang::run_bytecode(&bytecode::lower(&p).unwrap()).unwrap(),
        expected
    );
}
#[test]
fn outcome_values_keep_partial_bytes_failures_and_independent_copies_in_four_executors() {
    compare(SOURCE, EXPECTED);
}
#[test]
fn virtual_source_and_nominal_identity_are_shared_across_imports() {
    let w = Workspace::new();
    w.put("left.ceru",r#"import "std:process" as p; pub fn value()->p::Exit {return p::Exit::Code{value:4294967295};}"#);
    let entry = w.put(
        "main.ceru",
        r#"import "left.ceru" as l; import "std:process" as p; import "std:process" as again;
        a:p::Exit=l::value(); b:again::Exit=a; print(a==b);"#,
    );
    let c = modules::load(&entry).unwrap();
    let standard: Vec<_> = c
        .sources
        .files()
        .filter(|s| {
            matches!(
                s.kind(),
                SourceKind::Standard {
                    identifier: "std:process",
                    version: 1
                }
            )
        })
        .collect();
    assert_eq!(standard.len(), 1);
    assert_eq!(
        standard[0].text(),
        include_str!("../../src/standard/process.ceru")
    );
    let p = c.to_ir().unwrap();
    let definitions: Vec<_> = p
        .type_definitions
        .iter()
        .filter(|t| t.span.source_id() == standard[0].id())
        .collect();
    assert_eq!(definitions.len(), 12);
    assert_eq!(ir_executor::run(&p).unwrap(), "true\n");
    let manifest: serde_json::Value = serde_json::from_str(&c.source_manifest()).unwrap();
    assert_eq!(
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["module"] == "std:process")
            .count(),
        1
    );
    // メモリ内ソースだけでも解決でき、相対ファイル探索は追加しません。
    let direct =
        modules::from_source(r#"import "std:process" as p; value:p::Exit=p::Exit::Absent{};"#)
            .unwrap();
    assert_eq!(direct.sources.files().count(), 2);
}
#[test]
fn run_and_private_intrinsics_are_not_pretended_to_exist() {
    for call in [
        "p::run()",
        "p::__cerune_process_run()",
        "__cerune_process_run()",
    ] {
        let source = format!("import \"std:process\" as p; {call};");
        let error = cerune_lang::compile_to_ir(&source).unwrap_err();
        assert!(error.message().contains("unknown function"), "{error:?}");
    }
    for module in ["std:process.ceru", "std:Process", "std:process/../process"] {
        let source = format!("import \"{module}\" as p;");
        assert!(
            modules::from_source(&source)
                .unwrap_err()
                .render()
                .contains("unknown standard module")
        );
    }
}
#[test]
fn request_copies_preserve_argument_order_empty_arguments_and_binary_input() {
    compare(
        r#"import "std:process" as p;
        req:p::Request=p::Request {
            target:"tool", arguments:array_copy(["", "a b", "引用\"\\", "日本語"]),
            stdin:array_copy([u8(255),u8(0),u8(13),u8(10)]),
            stdout_limit:0, stderr_limit:18446744073709551615, timeout_ms:1,
        };
        mut args:[string]=req.arguments; args[0]="changed";
        mut bytes:[u8]=req.stdin; bytes[0]=u8(65);
        print(byte_len(req.arguments[0])); print(req.arguments[1]); print(req.arguments[3]);
        print(req.stdin); print(req.stdout_limit); print(req.stderr_limit); print(req.timeout_ms);
        print(args[0]); print(bytes[0]);"#,
        "0\na b\n日本語\n[255, 0, 13, 10]\n0\n18446744073709551615\n1\nchanged\n65\n",
    );
}
#[test]
fn every_runtime_failure_code_has_a_public_variant() {
    use cerune_lang::runtime::process::Code;
    fn name(c: Code) -> &'static str {
        match c {
            Code::InvalidRequest => "InvalidRequest",
            Code::RequestLimit => "RequestLimit",
            Code::Unconfigured => "Unconfigured",
            Code::Unregistered => "Unregistered",
            Code::ConnectionDisabled => "ConnectionDisabled",
            Code::OutputBudget => "OutputBudget",
            Code::AllocationFailed => "AllocationFailed",
            Code::NotFound => "NotFound",
            Code::Denied => "Denied",
            Code::Resources => "Resources",
            Code::Unsupported => "Unsupported",
            Code::StartFailed => "StartFailed",
            Code::TransferFailed => "TransferFailed",
            Code::ProgressUnknown => "ProgressUnknown",
            Code::CloseFailed => "CloseFailed",
            Code::WaitFailed => "WaitFailed",
            Code::ZeroProgress => "ZeroProgress",
            Code::InvalidResponse => "InvalidResponse",
            Code::OutputLimit => "OutputLimit",
            Code::Timeout => "Timeout",
            Code::CleanupFailed => "CleanupFailed",
        }
    }
    let codes = [
        Code::InvalidRequest,
        Code::RequestLimit,
        Code::Unconfigured,
        Code::Unregistered,
        Code::ConnectionDisabled,
        Code::OutputBudget,
        Code::AllocationFailed,
        Code::NotFound,
        Code::Denied,
        Code::Resources,
        Code::Unsupported,
        Code::StartFailed,
        Code::TransferFailed,
        Code::ProgressUnknown,
        Code::CloseFailed,
        Code::WaitFailed,
        Code::ZeroProgress,
        Code::InvalidResponse,
        Code::OutputLimit,
        Code::Timeout,
        Code::CleanupFailed,
    ];
    let c = modules::from_source(r#"import "std:process" as p;"#).unwrap();
    let definition = c
        .program
        .items
        .iter()
        .find_map(|i| match i {
            cerune_lang::ast::Item::TypeDefinition(t) if t.name.ends_with("FailureCode") => Some(t),
            _ => None,
        })
        .unwrap();
    let mut actual: Vec<_> = definition
        .variants
        .as_ref()
        .unwrap()
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    let mut expected: Vec<_> = codes.into_iter().map(name).collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
#[test]
fn result_values_keep_visible_lowering_and_execute_in_c_and_llvm() {
    let w = Workspace::new();
    let entry = w.put("outcome.ceru", SOURCE);
    for command in ["emit-sources", "emit-ir", "emit-mir", "emit-bytecode"] {
        let out = w.cli(command, &entry, &[]);
        assert!(
            out.status.success(),
            "{}: {}",
            command,
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.stdout.is_empty());
        if command == "emit-sources" {
            assert!(String::from_utf8_lossy(&out.stdout).contains("std:process"));
        }
    }
    execute_generated(&w, &entry, EXPECTED);
}
fn execute_generated(w: &Workspace, entry: &std::path::Path, expected: &str) {
    let llvm_target = if cfg!(all(windows, target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else {
        None
    };
    for (emitter, extension, variable) in [
        ("emit-c", "c", "CERUNE_TEST_CC"),
        ("emit-llvm", "ll", "CERUNE_TEST_LLVM_CLANG"),
    ] {
        // Cはhostの処理系で実行できます。LLVMの実OS接続は対応targetだけで実行します。
        if emitter == "emit-llvm" && llvm_target.is_none() {
            eprintln!("skipped LLVM execution: unsupported host OS/architecture");
            continue;
        }
        let configured = std::env::var_os(variable);
        let compiler = configured.clone().unwrap_or_else(|| "clang".into());
        if !Command::new(&compiler)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            assert!(
                configured.is_none(),
                "required compiler missing: {compiler:?}"
            );
            eprintln!("skipped {emitter}: compiler unavailable");
            continue;
        }
        let artifact = w.0.join(format!("program.{extension}"));
        let mut args = vec!["-o", artifact.to_str().unwrap()];
        if emitter == "emit-llvm" {
            args.extend(["--target", llvm_target.unwrap()]);
        }
        let out = w.cli(emitter, entry, &args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let exe = w.0.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        let mut cmd = Command::new(&compiler);
        cmd.arg(&artifact).arg("-o").arg(&exe);
        if emitter == "emit-llvm" {
            cmd.arg("-target").arg(llvm_target.unwrap());
        }
        let built =
            process::bounded_output(&mut cmd, &w.0, "build", Duration::from_secs(30)).unwrap();
        assert!(
            built.status.success(),
            "{emitter}: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        let run = process::bounded_output(
            &mut Command::new(&exe),
            &w.0,
            "run",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(run.stdout, expected.as_bytes());
        fs::remove_file(&exe).unwrap();
    }
}

#[test]
fn c_fields_do_not_collide_with_header_macros_keywords_or_generated_names() {
    let source = r#"
        type Fields { stdin:i64, stdout:i64, stderr:i64, switch:i64, NULL:i64,
            errno:i64, cerune_field_stdin:i64, cerune_internal_field__dtag:i64 }
        enum E { Value { stdin:i64, stdout:i64, stderr:i64 } }
        v:Fields=Fields{stdin:1,stdout:2,stderr:3,switch:4,NULL:5,errno:6,
            cerune_field_stdin:7,cerune_internal_field__dtag:8};
        fn replacement()->i64 {print("update");return 9;}
        updated:Fields=Fields{..v,stdin:replacement()};
        print(v.stdin);print(v.stdout);print(v.stderr);print(v.switch);
        print(v.NULL);print(v.errno);print(v.cerune_field_stdin);
        print(v.cerune_internal_field__dtag);print(updated.stdin);print(updated.stderr);
        e:E=E::Value{stdin:10,stdout:11,stderr:12};
        match e { E::Value{stdin:a,stdout:b,stderr:c}=>{print(a);print(b);print(c);} }
    "#;
    let expected = "update\n1\n2\n3\n4\n5\n6\n7\n8\n9\n3\n10\n11\n12\n";
    compare(source, expected);
    let w = Workspace::new();
    let entry = w.put("fields.ceru", source);
    execute_generated(&w, &entry, expected);
}

#[test]
fn ordinary_result_types_lower_to_remaining_generated_routes() {
    let w = Workspace::new();
    let entry = w.put("outcome.ceru", SOURCE);
    for command in ["emit-qbe", "emit-wat", "emit-asm", "emit-obj"] {
        for target in ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"] {
            if command == "emit-wat" && target.ends_with("msvc") {
                continue;
            }
            let artifact = w.0.join(format!("{command}-{target}"));
            let mut args = vec!["-o", artifact.to_str().unwrap()];
            if command != "emit-wat" {
                args.extend(["--target", target]);
            }
            let out = w.cli(command, &entry, &args);
            assert!(
                out.status.success(),
                "{command}/{target}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(fs::metadata(&artifact).unwrap().len() > 0);
        }
    }
}

const EXECUTION: &str = include_str!("../../examples/process_execution/unconfigured.ceru");
#[test]
fn explicit_run_module_preserves_type_identity_and_virtual_origins() {
    let source = format!("import \"std:process/run\" as again; {EXECUTION}");
    let c = modules::from_source(&source).unwrap();
    let p = c.to_ir().unwrap();
    let operation = p.first_process_span().unwrap();
    let module = c.sources.get(operation.source_id()).unwrap();
    assert!(matches!(
        module.kind(),
        SourceKind::Standard {
            identifier: "std:process/run",
            version: 1
        }
    ));
    assert_eq!(
        module.text(),
        include_str!("../../src/standard/process_run.ceru")
    );
    assert_eq!(c.sources.files().count(), 3);
    assert!(module.text()[operation.start()..operation.end()].contains("__cerune_process_run"));
    assert_eq!(p.type_definitions.len(), 12);
    compare(
        EXECUTION,
        include_str!("../../examples/process_execution/unconfigured.stdout"),
    );
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    assert!(cerune_lang::ir::text::emit(&p).contains("process.run"));
    assert!(mir::text::emit(&m).contains("process.run"));
    assert!(mir::ssa::text::emit(&s).unwrap().contains("process.run"));
    let bytecode = bytecode::format_program(&bytecode::lower(&p).unwrap());
    assert!(bytecode.contains("process.run"));
    assert!(bytecode.contains(".variants "));
    assert!(bytecode.contains("\"Unconfigured\""));
}
#[test]
fn execution_module_rejects_unsupported_emitters_without_affecting_data_module() {
    use cerune_lang::codegen::{c, llvm, qbe, wat, x86_64};
    let p = cerune_lang::compile_to_ir(EXECUTION).unwrap();
    let errors = [
        c::emit_c(&p).unwrap_err(),
        llvm::emit_llvm(&p).unwrap_err(),
        qbe::emit_qbe(&p).unwrap_err(),
        wat::emit_wat(&p).unwrap_err(),
        x86_64::emit_x86_64_win_asm(&p).unwrap_err(),
    ];
    for error in errors {
        assert!(
            error
                .message()
                .contains("subprocess execution is not supported yet"),
            "{error:?}"
        );
        assert_eq!(error.primary_span(), p.first_process_span());
    }
}
#[test]
fn run_intrinsic_remains_private_and_arguments_are_checked() {
    for source in [
        r#"import "std:process/run" as p; p::__cerune_process_run();"#,
        r#"import "std:process/run" as p; __cerune_process_run();"#,
    ] {
        assert!(
            cerune_lang::compile_to_ir(source)
                .unwrap_err()
                .message()
                .contains("unknown function")
        );
    }
    for call in ["p::run()", "p::run(3)"] {
        let source = format!("import \"std:process/run\" as p; {call};");
        assert!(cerune_lang::compile_to_ir(&source).is_err());
    }
}
