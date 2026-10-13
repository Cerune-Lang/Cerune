//! Nativeの出力操作。ASMと自前Objectは同じ命令列を使います。
use std::fmt::Write;

// 内部入口: 値はrax/xmm0、改行はr10、出自はr11。
// フレームの0..48は外部呼出し用、64..96は要求/受理/出自/SIGPIPE、
// 96..160は作業状態、160..416は本文、416..928は診断。呼出し時は16バイト整列。
pub(super) fn support(linux: bool) -> String {
    let mut o = String::from(if linux {
        ".section .rodata\n"
    } else {
        ".section .rdata,\"dr\"\n"
    });
    for (name, value) in [
        (
            "record",
            "cerune: io-v1 operation=write requested=%llu accepted=%llu%s",
        ),
        ("format_error", "cerune: output formatting failed%s"),
        ("i64", "%lld"),
        ("u64", "%llu"),
        ("f32", "%.9g"),
        ("f64", "%.17g"),
        ("true", "true\\n"),
        ("false", "false\\n"),
        ("lf", "\\n"),
        ("quote", "\\\""),
    ] {
        writeln!(o, ".Loutput_{name}:\n  .asciz \"{value}\"").unwrap();
    }
    o.push_str(".text\n");
    // 接続のみ通常ABI。比較テストはこの関数の接続先を置換します。
    o.push_str("cerune_output_write:\n  subq $40, %rsp\n");
    o.push_str(if linux {
        "  movq %rsi, %rdx\n  movq %rdi, %rsi\n  movl $1, %edi\n  callq write\n"
    } else {
        "  movl %edx, %r8d\n  movq %rcx, %rdx\n  movl $1, %ecx\n  callq _write\n  shlq $32, %rax\n  sarq $32, %rax\n"
    });
    o.push_str("  addq $40, %rsp\n  retq\n");
    o.push_str(PART_START);
    o.push_str(if linux {
        "  movq 56(%rsp), %rdi\n  movq 72(%rsp), %rsi\n"
    } else {
        "  movq 56(%rsp), %rcx\n  movq 72(%rsp), %rdx\n"
    });
    o.push_str("  callq cerune_output_write\n  testq %rax, %rax\n  jns .Loutput_part_check\n");
    o.push_str(if linux {
        "  callq __errno_location\n"
    } else {
        "  callq _errno\n"
    });
    o.push_str("  cmpl $4, (%rax)\n  je .Loutput_part_write\n  jmp .Loutput_part_failed\n");
    o.push_str(PART_END);
    o.push_str("cerune_output_failed:\n  subq $584, %rsp\n");
    o.push_str(if linux {
        "  leaq 64(%rsp), %rdi\n  movl $512, %esi\n  leaq .Loutput_record(%rip), %rdx\n  movq (%r10), %rcx\n  movq 8(%r10), %r8\n  movq 16(%r10), %r9\n  xorl %eax, %eax\n"
    } else {
        "  leaq 64(%rsp), %rcx\n  movl $512, %edx\n  leaq .Loutput_record(%rip), %r8\n  movq (%r10), %r9\n  movq 8(%r10), %rax\n  movq %rax, 32(%rsp)\n  movq 16(%r10), %rax\n  movq %rax, 40(%rsp)\n"
    });
    o.push_str(
        "  callq snprintf\n  leaq 64(%rsp), %r10\n  callq cerune_output_diagnostic\n  ud2\n",
    );
    // 診断配送の失敗でも停止します。stdioの終了時flushは行いません。
    o.push_str("cerune_output_diagnostic:\n  subq $40, %rsp\n  testl %eax, %eax\n  jle .Loutput_exit\n  cmpl $512, %eax\n  jb .Loutput_diag_write\n  movl $511, %eax\n.Loutput_diag_write:\n");
    o.push_str(if linux {
        "  movl %eax, %edx\n  movq %r10, %rsi\n  movl $2, %edi\n  callq write\n.Loutput_exit:\n  movl $1, %edi\n"
    } else {
        "  movl %eax, %r8d\n  movq %r10, %rdx\n  movl $2, %ecx\n  callq _write\n.Loutput_exit:\n  movl $1, %ecx\n"
    });
    o.push_str("  callq _Exit\n  ud2\n");
    for kind in ["i64", "u64", "f32", "f64", "bool", "string", "quoted"] {
        writeln!(o, "cerune_output_{kind}:").unwrap();
        if kind == "f32" {
            o.push_str("  cvtss2sd %xmm0, %xmm0\n");
        }
        if matches!(kind, "f32" | "f64") {
            o.push_str("  movq %xmm0, %rax\n");
        }
        o.push_str("  subq $936, %rsp\n  movq %rax, 96(%rsp)\n  movq %r10, 104(%rsp)\n  movq %r11, 80(%rsp)\n");
        let mode = match kind {
            "string" => 1,
            "quoted" => 2,
            _ => 0,
        };
        writeln!(o, "  movq ${mode}, 152(%rsp)").unwrap();
        match kind {
            "bool" => o.push_str(BOOL),
            "string" => o.push_str("  movq (%rax), %rax\n  addq 104(%rsp), %rax\n  movq %rax, 64(%rsp)\n  jmp .Loutput_begin\n"),
            "quoted" => o.push_str(COUNT),
            _ => {
                if linux {
                    writeln!(o, "  leaq 160(%rsp), %rdi\n  movl $128, %esi\n  leaq .Loutput_{kind}(%rip), %rdx").unwrap();
                    o.push_str(if matches!(kind, "f32" | "f64") {
                        "  movl $1, %eax\n"
                    } else { "  movq %rax, %rcx\n  xorl %eax, %eax\n" });
                } else {
                    writeln!(o, "  leaq 160(%rsp), %rcx\n  movl $128, %edx\n  leaq .Loutput_{kind}(%rip), %r8\n  movq %rax, %r9").unwrap();
                    if matches!(kind, "f32" | "f64") { o.push_str("  movapd %xmm0, %xmm3\n"); }
                }
                o.push_str("  callq snprintf\n  cmpl $127, %eax\n  jae .Loutput_format_failed\n  leaq 160(%rsp), %rcx\n  movb $10, (%rcx,%rax)\n  addq 104(%rsp), %rax\n  movq %rax, 64(%rsp)\n  movq %rcx, 96(%rsp)\n  jmp .Loutput_begin\n");
            }
        }
    }
    o.push_str(".Loutput_format_failed:\n");
    o.push_str(if linux {
        "  leaq 416(%rsp), %rdi\n  movl $512, %esi\n  leaq .Loutput_format_error(%rip), %rdx\n  movq 80(%rsp), %rcx\n  xorl %eax, %eax\n"
    } else {
        "  leaq 416(%rsp), %rcx\n  movl $512, %edx\n  leaq .Loutput_format_error(%rip), %r8\n  movq 80(%rsp), %r9\n"
    });
    o.push_str("  callq snprintf\n  leaq 416(%rsp), %r10\n  callq cerune_output_diagnostic\n  ud2\n.Loutput_begin:\n  movq $0, 72(%rsp)\n");
    if linux {
        o.push_str("  movl $13, %edi\n  movl $1, %esi\n  callq signal\n  cmpq $-1, %rax\n  je .Loutput_failed\n  movq %rax, 88(%rsp)\n");
    }
    o.push_str(DISPATCH);
    o.push_str(".Loutput_end:\n");
    if linux {
        o.push_str("  movl $13, %edi\n  movq 88(%rsp), %rsi\n  callq signal\n  cmpq $-1, %rax\n  je .Loutput_failed\n");
    }
    o.push_str("  addq $936, %rsp\n  retq\n.Loutput_failed:\n  leaq 64(%rsp), %r10\n  callq cerune_output_failed\n  ud2\n");
    // バイトの変換のみ。OS接続やプログラム状態へアクセスしません。
    o.push_str("cerune_output_escape:\n");
    let escapes: Vec<_> = (0..=127u8)
        .filter_map(|b| {
            let text = match b {
                0 => "\\0".into(),
                9 => "\\t".into(),
                10 => "\\n".into(),
                13 => "\\r".into(),
                34 => "\\\"".into(),
                92 => "\\\\".into(),
                1..=31 | 127 => format!("\\u{{{b:02x}}}"),
                _ => return None,
            };
            Some((b, text))
        })
        .collect();
    for (b, _) in &escapes {
        writeln!(o, "  cmpl ${b}, %eax\n  je .Loutput_escape_{b}").unwrap();
    }
    o.push_str("  movb %al, (%rcx)\n  movl $1, %eax\n  retq\n");
    for (b, text) in escapes {
        writeln!(o, ".Loutput_escape_{b}:").unwrap();
        for (i, c) in text.bytes().enumerate() {
            writeln!(o, "  movb ${c}, {i}(%rcx)").unwrap();
        }
        writeln!(o, "  movl ${}, %eax\n  retq", text.len()).unwrap();
    }
    o
}
const PART_START: &str = r#"
cerune_output_part:
  subq $88, %rsp
  movq %r10, 48(%rsp)
  movq %rax, 56(%rsp)
  movq %rdx, 64(%rsp)
.Loutput_part_loop:
  cmpq $0, 64(%rsp)
  je .Loutput_part_done
  movq 64(%rsp), %rax
  cmpq $65536, %rax
  jbe .Loutput_part_bound
  movl $65536, %eax
.Loutput_part_bound:
  movq %rax, 72(%rsp)
.Loutput_part_write:
"#;
const PART_END: &str = r#"
.Loutput_part_check:
  testq %rax, %rax
  je .Loutput_part_failed
  cmpq 72(%rsp), %rax
  ja .Loutput_part_failed
  movq 48(%rsp), %r10
  addq %rax, 8(%r10)
  addq %rax, 56(%rsp)
  subq %rax, 64(%rsp)
  jmp .Loutput_part_loop
.Loutput_part_done:
  addq $88, %rsp
  retq
.Loutput_part_failed:
  movq 48(%rsp), %r10
  callq cerune_output_failed
  ud2
"#;
const BOOL: &str = r#"
  testq %rax, %rax
  je .Loutput_bool_false
  leaq .Loutput_true(%rip), %rax
  movq $4, 64(%rsp)
  jmp .Loutput_bool_ready
.Loutput_bool_false:
  leaq .Loutput_false(%rip), %rax
  movq $5, 64(%rsp)
.Loutput_bool_ready:
  movq %rax, 96(%rsp)
  movq 104(%rsp), %rax
  addq %rax, 64(%rsp)
  jmp .Loutput_begin
"#;
const COUNT: &str = r#"
  movq $0, 112(%rsp)
  movq 104(%rsp), %rax
  addq $2, %rax
  movq %rax, 64(%rsp)
.Loutput_count:
  movq 96(%rsp), %rdx
  movq 112(%rsp), %rax
  cmpq (%rdx), %rax
  je .Loutput_begin
  movzbl 8(%rdx,%rax), %eax
  leaq 160(%rsp), %rcx
  callq cerune_output_escape
  addq %rax, 64(%rsp)
  incq 112(%rsp)
  jmp .Loutput_count
"#;
const DISPATCH: &str = r#"
  cmpq $2, 152(%rsp)
  je .Loutput_quoted
  cmpq $1, 152(%rsp)
  je .Loutput_raw
  movq 96(%rsp), %rax
  movq 64(%rsp), %rdx
  leaq 64(%rsp), %r10
  callq cerune_output_part
  jmp .Loutput_end
.Loutput_raw:
  movq 96(%rsp), %rax
  movq (%rax), %rdx
  addq $8, %rax
  leaq 64(%rsp), %r10
  callq cerune_output_part
  jmp .Loutput_newline
.Loutput_quoted:
  leaq .Loutput_quote(%rip), %rax
  movl $1, %edx
  leaq 64(%rsp), %r10
  callq cerune_output_part
  movq $0, 112(%rsp)
  movq $0, 120(%rsp)
.Loutput_quote_loop:
  movq 96(%rsp), %rdx
  movq 112(%rsp), %rax
  cmpq (%rdx), %rax
  je .Loutput_quote_tail
  cmpq $250, 120(%rsp)
  jbe .Loutput_quote_byte
  leaq 160(%rsp), %rax
  movq 120(%rsp), %rdx
  leaq 64(%rsp), %r10
  callq cerune_output_part
  movq $0, 120(%rsp)
.Loutput_quote_byte:
  movq 96(%rsp), %rdx
  movq 112(%rsp), %rax
  movzbl 8(%rdx,%rax), %eax
  leaq 160(%rsp), %rcx
  addq 120(%rsp), %rcx
  callq cerune_output_escape
  addq %rax, 120(%rsp)
  incq 112(%rsp)
  jmp .Loutput_quote_loop
.Loutput_quote_tail:
  leaq 160(%rsp), %rax
  movq 120(%rsp), %rdx
  leaq 64(%rsp), %r10
  callq cerune_output_part
  leaq .Loutput_quote(%rip), %rax
  movl $1, %edx
  leaq 64(%rsp), %r10
  callq cerune_output_part
.Loutput_newline:
  leaq .Loutput_lf(%rip), %rax
  movq 104(%rsp), %rdx
  leaq 64(%rsp), %r10
  callq cerune_output_part
  jmp .Loutput_end
"#;

#[cfg(test)]
mod native_object_output {
    use super::super::test_process as process;
    use crate as cerune_lang;
    const NATIVE: bool = true;
    const QBE: bool = false;
    const LLVM: bool = false;
    const ENCODER: Option<fn(&str) -> Vec<u8>> = Some(encode);
    fn encode(assembly: &str) -> Vec<u8> {
        super::super::object::assemble(
            assembly,
            if cfg!(windows) {
                super::super::Target::X86_64PcWindowsMsvc
            } else {
                super::super::Target::X86_64UnknownLinuxGnu
            },
        )
        .unwrap()
    }
    include!("../../../tests/support/generated_output.rs");
}
