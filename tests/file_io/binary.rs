use super::*;

const COPY: &str = r#"import "std:fs::bytes" as fs;
print(7);
bytes: [u8] = fs::read_bytes("input", 4);
fs::write_bytes_new("out", bytes, 4);
print(array_len(bytes));"#;

#[test]
fn executors_agree_on_payloads_progress_and_stopping_site() {
    for (input, read_step, write_step, limit, expected_code) in [
        (&b""[..], None, None, 1024, None),
        (&[0, 255, 13, 10], None, None, 1024, None),
        (
            &[0, 255, 13, 10, 99],
            None,
            None,
            1024,
            Some(Code::LimitExceeded),
        ),
        (
            &[0xff, 3],
            Some(Step::Fail),
            None,
            1024,
            Some(Code::ReadFailed),
        ),
        (
            &[0xff, 3],
            None,
            Some(Step::Zero),
            1024,
            Some(Code::ZeroProgress),
        ),
        (&[0xff, 3], None, None, 31, Some(Code::BudgetExceeded)),
    ] {
        let mut p = hir(COPY);
        p.array_heap_limit = limit;
        let mut baseline = None;
        for route in 0..4 {
            let memory = Memory::default();
            {
                let mut s = memory.0.borrow_mut();
                s.files.insert("input".into(), input.to_vec());
                s.reads.extend([Step::Interrupted, Step::Count(1)]);
                s.reads.extend(read_step);
                s.writes.push_back(Step::Count(1));
                s.writes.extend(write_step);
            }
            let mut files = Files::with_trace_limit(memory.clone(), 2);
            let result = execute(route, &p, &mut files);
            assert_eq!(result.as_ref().err().map(|(e, _)| e.code), expected_code);
            let state = memory.0.borrow();
            if let Err((failure, output)) = &result {
                assert_eq!(output, "7\n");
                match failure.code {
                    Code::BudgetExceeded => assert!(state.calls.is_empty()),
                    Code::LimitExceeded => assert_eq!(failure.details.transferred, 5),
                    Code::ReadFailed => assert_eq!(failure.details.transferred, 1),
                    Code::ZeroProgress => assert_eq!(state.files["out"], input[..1]),
                    _ => unreachable!(),
                }
            } else {
                assert_eq!(state.files["out"], input);
                assert_eq!(result.as_ref().unwrap(), &format!("7\n{}\n", input.len()));
            }
            let observed = (result, files.events().to_vec());
            if let Some(previous) = &baseline {
                assert_eq!(&observed, previous);
            } else {
                baseline = Some(observed);
            }
        }
    }
}

#[test]
fn elf_example_reads_fields_and_copies_the_actual_object_on_each_route() {
    let object = language::compile_to_native_object(
        include_str!("../../examples/recursion/factorial.ceru"),
        language::codegen::x86_64::Target::X86_64UnknownLinuxGnu,
        false,
    )
    .unwrap();
    let p = hir(include_str!("../../examples/files/elf_header.ceru"));
    for route in 0..4 {
        let memory = Memory::default();
        memory
            .0
            .borrow_mut()
            .files
            .insert("input.o".into(), object.clone());
        let mut files = Files::new(memory.clone());
        assert_eq!(execute(route, &p, &mut files).unwrap(), "1\n1\n0\n");
        assert_eq!(memory.0.borrow().files["copy.o"], object);
        let (collision, output) = execute(route, &p, &mut files).unwrap_err();
        assert_eq!(
            (collision.code, output.as_str()),
            (Code::AlreadyExists, "1\n1\n0\n")
        );
        assert_eq!(memory.0.borrow().files["copy.o"], object);
        let short = Memory::default();
        short
            .0
            .borrow_mut()
            .files
            .insert("input.o".into(), object[..63].to_vec());
        assert_eq!(
            execute(route, &p, &mut Files::new(short.clone())).unwrap(),
            "expected a little-endian ELF64 header\n"
        );
        assert!(!short.0.borrow().files.contains_key("copy.o"));
    }
}

#[test]
fn binary_module_is_observable_and_compiled_routes_report_the_missing_operation() {
    use std::process::Command;
    let binary = env!("CARGO_BIN_EXE_cerune");
    let source = "examples/files/elf_header.ceru";
    for args in [
        vec!["emit-ir", source],
        vec!["emit-mir", source],
        vec!["emit-mir", source, "--ssa"],
        vec!["emit-bytecode", source],
    ] {
        let out = Command::new(binary).args(args).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let ir = String::from_utf8(out.stdout).unwrap();
        assert!(ir.contains("file.read_bytes") && ir.contains("file.write_bytes_new"));
    }
    for command in ["emit-c", "emit-llvm", "emit-qbe", "emit-wat"] {
        let mut cmd = Command::new(binary);
        cmd.args([command, source]);
        if matches!(command, "emit-llvm" | "emit-qbe") {
            cmd.args(["--target", "x86_64-unknown-linux-gnu"]);
        }
        let out = cmd.output().unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success() && out.stdout.is_empty(), "{command}");
        assert!(
            err.contains("binary file I/O") && err.contains("std:fs::bytes"),
            "{command}: {err}"
        );
    }
}

#[test]
fn repeated_reads_release_owned_storage_and_copies_can_be_edited() {
    let mut p = hir(r#"import "std:fs::bytes" as fs;
        mut i: i64 = 0;
        while i < 8 {
            data: [u8] = fs::read_bytes("input", 4);
            mut copy: [u8] = data;
            copy[0] = 55;
            print(data[0]);
            i = i + 1;
        }"#);
    p.array_heap_limit = 64;
    for route in 0..4 {
        let memory = Memory::default();
        memory
            .0
            .borrow_mut()
            .files
            .insert("input".into(), vec![0xff]);
        assert_eq!(
            execute(route, &p, &mut Files::new(memory)).unwrap(),
            "255\n".repeat(8)
        );
    }
}
