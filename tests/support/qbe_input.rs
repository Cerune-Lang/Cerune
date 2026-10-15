use super::*;
use cerune_lang::runtime::input::Code;

fn replace_body(text: &str, signature: &str, body: &str) -> String {
    let start = text.find(signature).expect("QBE helper");
    let end = start + text[start..].find("\n}\n").unwrap() + 3;
    format!(
        "{}{}\n@start\n{}\n}}\n{}",
        &text[..start],
        signature,
        body,
        &text[end..]
    )
}
fn failure(hir: &cerune_lang::ir::Program, code: Code, consumed: usize, known: bool) -> Failure {
    let (_, Some(mut f)) = expected(hir, &[b'x'; 17]) else {
        panic!()
    };
    f.code = code;
    f.consumed = consumed;
    f.consumption_known = known;
    f
}
#[test]
fn explicit_targets_keep_input_and_origins_visible() {
    let hir = modules::from_source(include_str!("../fixtures/line_input.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    assert!(
        qbe::emit_qbe(&hir)
            .unwrap_err()
            .message()
            .contains("line input requires an explicit --target")
    );
    for target in [
        qbe::Target::X86_64PcWindowsMsvc,
        qbe::Target::X86_64UnknownLinuxGnu,
    ] {
        let text = qbe::emit_qbe_with_target(&hir, Some(target)).unwrap();
        assert_eq!(text, qbe::emit_qbe_with_target(&hir, Some(target)).unwrap());
        assert!(text.contains(&format!("qbe -t {}", target.qbe_name())));
        assert_eq!(
            text.contains("call $ReadFile("),
            target == qbe::Target::X86_64PcWindowsMsvc
        );
        assert_eq!(
            text.contains("call $read("),
            target == qbe::Target::X86_64UnknownLinuxGnu
        );
        assert!(text.contains("$cerune_input_origin_"));
        assert!(text.contains("call $cerune_input_read_line(l "));
        assert!(text.contains("function w $cerune_input_utf8("));
    }
}
#[test]
fn input_without_output_retains_failure_diagnostics() {
    let dir = directory("qbe-no-output");
    let hir = modules::from_source(r#"import "std:io" as io;v:io::ReadLine=io::read_line(4);"#)
        .unwrap()
        .to_ir()
        .unwrap();
    if let Some(exe) = compile_qbe(&dir, "input", &generated_qbe(&hir)) {
        for bytes in [b"a\n".as_slice(), b"SECRET\n".as_slice()] {
            compare(
                file_run(&mut Command::new(&exe), &dir, bytes),
                &expected(&hir, bytes),
            );
        }
    }
}
#[test]
fn injected_failures_preserve_consumption_origin_and_prior_output() {
    let dir = directory("qbe-injection");
    let hir=modules::from_source(r#"import "std:io" as io;print("before");flush();v:io::ReadLine=io::read_line(4);print("after");"#).unwrap().to_ir().unwrap();
    let base = generated_qbe(&hir);
    for (mode, code, consumed, known) in [
        ("retry", None, 0, true),
        ("read", Some(Code::ReadFailed), 2, true),
        ("invalid", Some(Code::InvalidResponse), 2, false),
        ("allocate", Some(Code::AllocationFailed), 0, true),
        ("unsupported", Some(Code::UnsupportedConnection), 0, true),
    ] {
        let mut text = base.clone();
        match mode {
            "allocate" => {
                text = text.replace("%owner =l call $malloc(l %size)", "%owner =l copy 0");
            }
            "unsupported" => {
                text = replace_body(
                    &text,
                    "function l $cerune_input_connection() {",
                    "  ret $cerune_input_unsupported_connection",
                );
            }
            _ => {
                text = text.replace(
                    "function l $cerune_input_read_byte(",
                    "function l $cerune_input_original_read_byte(",
                );
                text.push_str(&format!(
                    r#"
data $input_test_calls = align 8 {{ l 0 }}
function l $cerune_input_read_byte(l %byte) {{
@start
  %call =l loadl $input_test_calls
  %next =l add %call, 1
  storel %next, $input_test_calls
  %first =w ceql %call, 0
  %after =w ceql %call, 2
  %interrupt =w or %first, %after
  jnz %interrupt, @retry, @check
@retry
  ret -2
@check
  %fail =w ceql %call, 4
  jnz %fail, @failure, @read
@failure
  {failure}
@read
  %count =l call $cerune_input_original_read_byte(l %byte)
  ret %count
}}
"#,
                    failure = match mode {
                        "read" => "ret -1",
                        "invalid" => "ret 2",
                        _ => "jmp @read",
                    }
                ));
            }
        }
        if let Some(exe) = compile_qbe(&dir, mode, &text) {
            let out = file_run(&mut Command::new(exe), &dir, b"abc\n");
            if let Some(code) = code {
                compare(
                    out,
                    &(
                        b"before\n".to_vec(),
                        Some(failure(&hir, code, consumed, known)),
                    ),
                );
            } else {
                compare(out, &expected(&hir, b"abc\n"));
            }
        }
    }
}
#[test]
fn os_responses_preserve_eof_interruptions_and_connection_failures() {
    let dir = directory("qbe-os");
    let hir = modules::from_source(include_str!("../fixtures/line_input.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
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
        let mut text = generated_qbe(&hir);
        text.push_str(
            "\ndata $os_test_calls = align 8 { l 0 }\ndata $os_test_error = align 4 { w 0 }\n",
        );
        if cfg!(windows) {
            text.push_str(&format!(
                r#"
function l $GetStdHandle(w %which) {{
@start
  ret {handle}
}}
function w $GetConsoleMode(l %handle, l %mode) {{
@start
  ret {console}
}}
function w $GetFileType(l %handle) {{
@start
  ret {kind}
}}
function w $GetLastError() {{
@start
  %code =w loadw $os_test_error
  ret %code
}}
function w $ReadFile(l %handle, l %byte, w %requested, l %count, l %overlapped) {{
@start
  %call =l loadl $os_test_calls
  %next =l add %call, 1
  storel %next, $os_test_calls
  %empty =w ceql %call, {empty}
  jnz %empty, @empty_message, @check
@empty_message
  storew 0, %count
  ret 1
@check
  %first =w ceql %call, {first}
  jnz %first, @data, @end
@data
  storeb 97, %byte
  storew 1, %count
  storew 234, $os_test_error
  ret {ok}
@end
  storew {count}, %count
  storew {error}, $os_test_error
  ret {endok}
}}
"#,
                handle = if mode == "invalid-handle" { 0 } else { 42 },
                console = if mode == "console" { 1 } else { 0 },
                kind = if mode == "file-eof" { 1 } else { 3 },
                empty = if mode == "empty-message" { 0 } else { -1 },
                first = if mode == "empty-message" { 1 } else { 0 },
                ok = if mode == "more-data" { 0 } else { 1 },
                count = if mode == "bad-count" { 2 } else { 0 },
                error = match mode {
                    "file-eof" => 38,
                    "read-error" => 995,
                    _ => 109,
                },
                endok = if mode == "bad-count" { 1 } else { 0 }
            ));
        } else {
            text.push_str(&format!(
                r#"
function l $read(w %fd, l %byte, l %requested) {{
@start
  %call =l loadl $os_test_calls
  %next =l add %call, 1
  storel %next, $os_test_calls
  %interrupt =w cultl %call, {interruptions}
  jnz %interrupt, @retry, @check
@retry
  %ep =l call $__errno_location()
  storew 4, %ep
  ret -1
@check
  %first =w ceql %call, {interruptions}
  jnz %first, @data, @end
@data
  storeb 97, %byte
  ret 1
@end
  %err =l call $__errno_location()
  storew 5, %err
  ret {result}
}}
"#,
                interruptions = if mode == "eintr" { 2 } else { 0 },
                result = match mode {
                    "eintr" => 0,
                    "bad-count" => 2,
                    _ => -1,
                }
            ));
        }
        // OSライブラリの実シンボルを上書きせず、生成接続の呼出しだけを置換します。
        for name in [
            "GetStdHandle",
            "GetConsoleMode",
            "GetFileType",
            "ReadFile",
            "GetLastError",
            "read",
        ] {
            text = text.replace(&format!("${name}("), &format!("$os_test_{name}("));
        }
        if let Some(exe) = compile_qbe(&dir, mode, &text) {
            let out = file_run(&mut Command::new(exe), &dir, b"UNREAD");
            if let Some(code) = code {
                compare(
                    out,
                    &(
                        b"ready\n".to_vec(),
                        Some(failure(&hir, code, consumed, known)),
                    ),
                );
            } else {
                compare(out, &expected(&hir, b"a"));
            }
        }
    }
}
