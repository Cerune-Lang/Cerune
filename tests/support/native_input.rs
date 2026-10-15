use super::*;
use cerune_lang::{codegen::x86_64 as native, runtime::input::Code};
fn hir(source: &str) -> cerune_lang::ir::Program {
    modules::from_source(source).unwrap().to_ir().unwrap()
}
fn failure(hir: &cerune_lang::ir::Program, code: Code, consumed: usize, known: bool) -> Failure {
    let error = ir_executor::run(hir).unwrap_err();
    let ir_executor::ErrorKind::Input(f) = error.kind() else {
        panic!("{error:?}")
    };
    let mut f = **f;
    f.code = code;
    f.consumed = consumed;
    f.consumption_known = known;
    f
}
#[test]
fn fixture_and_input_without_output_match_all_native_artifacts() {
    let dir = directory("native-fixture");
    let program = hir(include_str!("../fixtures/line_input.ceru"));
    for exe in native_commands(&dir, "fixture", &program) {
        let out = file_run(
            &mut Command::new(exe),
            &dir,
            include_bytes!("../fixtures/line_input.stdin"),
        );
        assert_eq!(out.stdout, include_bytes!("../fixtures/line_input.stdout"));
        compare(
            out,
            &expected(&program, include_bytes!("../fixtures/line_input.stdin")),
        );
    }
    let program = hir(r#"import "std:io" as io;v:io::ReadLine=io::read_line(4);"#);
    for exe in native_commands(&dir, "no-output", &program) {
        for bytes in [b"a\n".as_slice(), b"SECRET\n"] {
            compare(
                file_run(&mut Command::new(&exe), &dir, bytes),
                &expected(&program, bytes),
            );
        }
    }
}
#[test]
fn origins_and_explicit_targets_preserve_native_instructions() {
    let program = hir(r#"import "std:io" as io;v:io::ReadLine=io::read_line(4);"#);
    let strip = |s: &str| {
        s.lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.starts_with("cerune_origin_"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    for target in [
        native::Target::X86_64PcWindowsMsvc,
        native::Target::X86_64UnknownLinuxGnu,
    ] {
        let plain = native::emit_asm(&program, target).unwrap();
        let annotated = native::emit_asm_with_origins(&program, target).unwrap();
        assert_eq!(strip(&plain), strip(&annotated));
        assert_eq!(
            plain.contains("callq ReadFile"),
            target == native::Target::X86_64PcWindowsMsvc
        );
        assert_eq!(
            plain.contains("callq read\n"),
            target == native::Target::X86_64UnknownLinuxGnu
        );
        assert!(plain.contains(".Lcerune_input_origin_"));
    }
}
#[test]
fn injected_failures_preserve_consumption_and_prior_output() {
    let dir = directory("native-injection");
    let program = hir(
        r#"import "std:io" as io;print("before");flush();v:io::ReadLine=io::read_line(4);print("after");"#,
    );
    let base = native::emit_asm(&program, native_target()).unwrap();
    for (mode, code, consumed, known) in [
        ("retry", None, 0, true),
        ("read", Some(Code::ReadFailed), 2, true),
        ("invalid", Some(Code::InvalidResponse), 2, false),
        ("allocate", Some(Code::AllocationFailed), 0, true),
        ("unsupported", Some(Code::UnsupportedConnection), 0, true),
    ] {
        let mut asm = base.clone();
        match mode {
            "allocate" => {
                asm = asm.replace("callq malloc", "xorl %eax, %eax");
            }
            "unsupported" => {
                asm = asm.replace(
                    "callq cerune_input_connection",
                    "leaq .Linput_unsupported_connection(%rip), %rax",
                );
            }
            _ => {
                asm = asm.replace(
                    "cerune_input_read_byte:\n",
                    "cerune_input_original_read_byte:\n",
                );
                asm.push_str(&format!(
                    r#"
.data
.p2align 3
.Ltest_calls:
  .quad 0
.text
cerune_input_read_byte:
  movq .Ltest_calls(%rip), %rax
  addq $1, .Ltest_calls(%rip)
  cmpq $0, %rax
  je .Ltest_retry
  cmpq $2, %rax
  je .Ltest_retry
  cmpq $4, %rax
  jne .Ltest_read
  {failure}
.Ltest_read:
  jmp cerune_input_original_read_byte
.Ltest_retry:
  movq $-2, %rax
  retq
"#,
                    failure = match mode {
                        "read" => "movq $-1, %rax\n  retq",
                        "invalid" => "movq $2, %rax\n  retq",
                        _ => "jmp .Ltest_read",
                    }
                ));
            }
        }
        if let Some(exe) = native_link(&dir, mode, asm.as_bytes(), "s") {
            let out = file_run(&mut Command::new(exe), &dir, b"abc\n");
            if let Some(code) = code {
                compare(
                    out,
                    &(
                        b"before\n".to_vec(),
                        Some(failure(&program, code, consumed, known)),
                    ),
                );
            } else {
                compare(out, &expected(&program, b"abc\n"));
            }
        }
    }
}
#[test]
fn os_responses_preserve_eof_interruptions_and_connection_failures() {
    let dir = directory("native-os");
    let program = hir(include_str!("../fixtures/line_input.ceru"));
    let cases: &[(&str, Option<Code>, usize, bool)] = if cfg!(windows) {
        &[
            ("console", Some(Code::UnsupportedConnection), 0, true),
            ("invalid-handle", Some(Code::ReadFailed), 0, true),
            ("more-data", None, 0, true),
            ("empty-message", None, 0, true),
            ("file-eof", None, 0, true),
            ("read-error", Some(Code::ReadFailed), 1, true),
            ("bad-count", Some(Code::InvalidResponse), 1, false),
        ]
    } else {
        &[
            ("eintr", None, 0, true),
            ("read-error", Some(Code::ReadFailed), 1, true),
            ("bad-count", Some(Code::InvalidResponse), 1, false),
        ]
    };
    for &(mode, code, consumed, known) in cases {
        let mut asm = native::emit_asm(&program, native_target()).unwrap();
        asm.push_str(
            "\n.data\n.p2align 3\n.Ltest_calls:\n  .quad 0\n.Ltest_error:\n  .long 0\n.text\n",
        );
        if cfg!(windows) {
            for name in [
                "GetStdHandle",
                "GetConsoleMode",
                "GetFileType",
                "ReadFile",
                "GetLastError",
            ] {
                asm = asm.replace(&format!("callq {name}\n"), &format!("callq test_{name}\n"));
            }
            asm.push_str(&format!(r#"
test_GetStdHandle:
  movq ${handle}, %rax
  retq
test_GetConsoleMode:
  movl ${console}, %eax
  retq
test_GetFileType:
  movl ${kind}, %eax
  retq
test_GetLastError:
  movl .Ltest_error(%rip), %eax
  retq
test_ReadFile:
  movq .Ltest_calls(%rip), %rax
  addq $1, .Ltest_calls(%rip)
  movl $0, (%r9)
  cmpq $0, %rax
  je .Ltest_first
  cmpq $1, %rax
  je .Ltest_second
  movl ${end}, .Ltest_error(%rip)
  xorl %eax, %eax
  retq
.Ltest_first:
  {first}
.Ltest_second:
  {second}
"#,handle=if mode=="invalid-handle"{-1}else{123},
console=if mode=="console"{1}else{0},kind=if mode=="file-eof"{1}else{3},
end=if mode=="file-eof"{38}else{109},
first=if mode=="empty-message"{"movl $1, %eax\n  retq"}else if mode=="more-data"{"movb $65, (%rdx)\n  movl $1, (%r9)\n  movl $234, .Ltest_error(%rip)\n  xorl %eax, %eax\n  retq"}else{"movb $65, (%rdx)\n  movl $1, (%r9)\n  movl $1, %eax\n  retq"},
second=match mode {
"read-error"=>"movl $5, .Ltest_error(%rip)\n  xorl %eax, %eax\n  retq",
"bad-count"=>"movl $2, (%r9)\n  movl $1, %eax\n  retq",
"empty-message"=>"movb $65, (%rdx)\n  movl $1, (%r9)\n  movl $1, %eax\n  retq",
_=>"movl $109, .Ltest_error(%rip)\n  xorl %eax, %eax\n  retq",
}));
            // file-eofは最後の行の直後にERROR_HANDLE_EOFを返します。
            if mode == "file-eof" {
                asm = asm.replace(
                    "movl $109, .Ltest_error(%rip)",
                    "movl $38, .Ltest_error(%rip)",
                );
            }
        } else {
            asm = asm
                .replace("callq read\n", "callq test_read\n")
                .replace("callq __errno_location\n", "callq test_errno\n");
            asm.push_str(&format!(
                r#"
test_errno:
  leaq .Ltest_error(%rip), %rax
  retq
test_read:
  movq .Ltest_calls(%rip), %rax
  addq $1, .Ltest_calls(%rip)
  cmpq $0, %rax
  je .Ltest_first
  cmpq $1, %rax
  je .Ltest_second
  xorl %eax, %eax
  retq
.Ltest_first:
  {first}
.Ltest_second:
  {second}
"#,
                first = if mode == "eintr" {
                    "movl $4, .Ltest_error(%rip)\n  movq $-1, %rax\n  retq"
                } else {
                    "movb $65, (%rsi)\n  movl $1, %eax\n  retq"
                },
                second = match mode {
                    "read-error" => "movl $5, .Ltest_error(%rip)\n  movq $-1, %rax\n  retq",
                    "bad-count" => "movl $2, %eax\n  retq",
                    _ => "movb $65, (%rsi)\n  movl $1, %eax\n  retq",
                }
            ));
        }
        if let Some(exe) = native_link(&dir, mode, asm.as_bytes(), "s") {
            let out = file_run(&mut Command::new(exe), &dir, b"UNREAD");
            if let Some(code) = code {
                compare(
                    out,
                    &(
                        b"ready\n".to_vec(),
                        Some(failure(&program, code, consumed, known)),
                    ),
                );
            } else {
                compare(out, &expected(&program, b"A"));
            }
        }
    }
}

#[test]
fn simple_stdin_example_matches_all_routes() {
    let dir = directory("stdin-example");
    let source = include_str!("../../examples/stdin/lines.ceru");
    let program = hir(source);
    let bytes = "Cerune\n\n日本語\n".as_bytes();
    let expected = expected(&program, bytes);
    assert_eq!(
        expected.0,
        "line>\n[\"Cerune\\n\"]\n7\nline>\n[\"\\n\"]\n1\nline>\n[\"日本語\\n\"]\n10\nline>\nend\n"
            .as_bytes()
    );
    let file = dir.join("main.ceru");
    fs::write(&file, source).unwrap();
    for route in ROUTES {
        compare(
            file_run(
                &mut cli(route, &file, program.string_heap_limit),
                &dir,
                bytes,
            ),
            &expected,
        );
    }
    for exe in native_commands(&dir, "example", &program) {
        compare(file_run(&mut Command::new(exe), &dir, bytes), &expected);
    }
    if let Some(exe) = compile_c(&dir, "example", &c::emit_c(&program).unwrap(), "-O2") {
        compare(file_run(&mut Command::new(exe), &dir, bytes), &expected);
    }
    if let Some(exe) = compile_llvm(&dir, "example", &generated_llvm(&program, true), "-O2") {
        compare(file_run(&mut Command::new(exe), &dir, bytes), &expected);
    }
    if let Some(exe) = compile_qbe(&dir, "example", &generated_qbe(&program)) {
        compare(file_run(&mut Command::new(exe), &dir, bytes), &expected);
    }
    if let Some(mut cmd) = wat_command(&dir, "example", &generated_wat(&program)) {
        compare(file_run(&mut cmd, &dir, bytes), &expected);
    }
}
#[test]
fn preflight_and_failed_flush_do_not_read() {
    let dir = directory("native-preflight");
    for (name, limit, budget, unsupported, code) in [
        ("limit", 0, 64, true, Code::InvalidLimit),
        ("minimum", i64::MIN, 64, false, Code::InvalidLimit),
        ("connection", 4, 0, true, Code::UnsupportedConnection),
        ("budget", 4, 3, false, Code::BudgetExceeded),
    ] {
        let mut program = hir(&format!(
            "import \"std:io\" as io;v:io::ReadLine=io::read_line({limit});"
        ));
        program.string_heap_limit = budget;
        let mut asm = native::emit_asm(&program, native_target()).unwrap();
        // 誤って読取りへ進んだら正常終了せず、期待するio-v1とも一致しません。
        asm = asm.replace("callq cerune_input_read_byte", "ud2");
        if unsupported {
            asm = asm.replace(
                "callq cerune_input_connection",
                "leaq .Linput_unsupported_connection(%rip), %rax",
            );
        }
        if let Some(exe) = native_link(&dir, name, asm.as_bytes(), "s") {
            compare(
                file_run(&mut Command::new(exe), &dir, b"UNREAD"),
                &(Vec::new(), Some(failure(&program, code, 0, true))),
            );
        }
    }
    let program =
        hir(r#"import "std:io" as io;print("before");flush();v:io::ReadLine=io::read_line(4);"#);
    let asm = native::emit_asm(&program, native_target())
        .unwrap()
        .replace("callq fflush", "movl $-1, %eax")
        .replace("callq cerune_input_read_byte", "ud2");
    if let Some(exe) = native_link(&dir, "flush", asm.as_bytes(), "s") {
        let out = file_run(&mut Command::new(exe), &dir, b"UNREAD");
        assert!(!out.status.success());
        assert_eq!(out.stdout, b"before\n");
        assert!(String::from_utf8_lossy(&out.stderr).contains("io-v1 operation=flush"));
    }
}
