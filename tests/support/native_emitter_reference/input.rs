//! MIR由来の行入力。ASMと自前Objectへ同じ処理を出力します。
use super::ir::{Instruction, Module};
use std::fmt::Write;

pub(super) fn used(m: &Module) -> bool {
    m.instructions
        .iter()
        .chain(m.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i, Instruction::ReadLine))
}
pub(super) fn support(linux: bool) -> String {
    let mut out = String::from(if linux {
        ".section .rodata\n"
    } else {
        ".section .rdata,\"dr\"\n"
    });
    for code in [
        "invalid-limit",
        "unsupported-connection",
        "budget-exceeded",
        "allocation-failed",
        "read-failed",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
        "known",
        "unknown",
    ] {
        let suffix = if matches!(code, "invalid-limit" | "read-failed" | "invalid-response") {
            "_text"
        } else {
            ""
        };
        let label = code.replace('-', "_");
        writeln!(out, ".Linput_{label}{suffix}:\n  .asciz \"{code}\"").unwrap();
    }
    out.push_str(".Linput_format:\n  .asciz \"cerune: io-v1 operation=read-line code=%s limit=%lld consumed=%llu consumption=%s%s\"\n");
    out.push_str(COMMON);
    out.push_str(if linux { LINUX } else { WINDOWS });
    // 呼出し時のrspは16byte整列。Windowsのshadow spaceとstack引数は0..64です。
    out.push_str(".Linput_format_failure:\n");
    out.push_str(if linux {
        "  leaq 160(%rsp), %rdi\n  movl $512, %esi\n  leaq .Linput_format(%rip), %rdx\n  movq 112(%rsp), %rcx\n  movq 64(%rsp), %r8\n  movq 96(%rsp), %r9\n  movq 104(%rsp), %rax\n  movq %rax, (%rsp)\n  movq 72(%rsp), %rax\n  movq %rax, 8(%rsp)\n  xorl %eax, %eax\n"
    } else {
        "  leaq 160(%rsp), %rcx\n  movl $512, %edx\n  leaq .Linput_format(%rip), %r8\n  movq 112(%rsp), %r9\n  movq 64(%rsp), %rax\n  movq %rax, 32(%rsp)\n  movq 96(%rsp), %rax\n  movq %rax, 40(%rsp)\n  movq 104(%rsp), %rax\n  movq %rax, 48(%rsp)\n  movq 72(%rsp), %rax\n  movq %rax, 56(%rsp)\n"
    });
    out.push_str("  callq snprintf\n  testl %eax, %eax\n  jle .Linput_exit\n  cmpl $512, %eax\n  jb .Linput_diagnostic\n  movl $511, %eax\n.Linput_diagnostic:\n");
    out.push_str(if linux {
        "  movl %eax, %edx\n  leaq 160(%rsp), %rsi\n  movl $2, %edi\n  callq write\n.Linput_exit:\n  movl $1, %edi\n"
    } else {
        "  movl %eax, %r8d\n  leaq 160(%rsp), %rdx\n  movl $2, %ecx\n  callq _write\n.Linput_exit:\n  movl $1, %ecx\n"
    });
    out.push_str("  callq _Exit\n  ud2\n");
    out.replace("@ARG@", if linux { "%rdi" } else { "%rcx" })
        .replace("@ARG2@", if linux { "%rsi" } else { "%rdx" })
}
// 入口rax=limit、r11=不変の出自。戻り値rax=既存string表現。
// frame: 64 limit,72 origin,80 owner,88 length,96 consumed,104 known,112 code,
// 120 byte,128 UTF8残数,136下限,144上限,160..672診断。所有権は成功時だけ公開します。
const COMMON: &str = r#"
.data
.p2align 3
.Linput_eof:
  .quad 0
.text
cerune_input_read_line:
  subq $696, %rsp
  movq %rax, 64(%rsp)
  movq %r11, 72(%rsp)
  movq $0, 80(%rsp)
  movq $0, 88(%rsp)
  movq $0, 96(%rsp)
  leaq .Linput_known(%rip), %rcx
  movq %rcx, 104(%rsp)
  subq $1, %rax
  cmpq $2147483647, %rax
  jae .Linput_invalid_limit
  callq cerune_input_connection
  testq %rax, %rax
  jne .Linput_failed
  cmpq $0, .Linput_eof(%rip)
  jne .Linput_empty
  movq .Lcerune_heap_limit(%rip), %rax
  subq .Lcerune_heap_live(%rip), %rax
  cmpq 64(%rsp), %rax
  jb .Linput_budget
  movq 64(%rsp), @ARG@
  addq $24, @ARG@
  callq malloc
  testq %rax, %rax
  je .Linput_allocation
  movq %rax, 80(%rsp)
  movq 64(%rsp), %rcx
  addq %rcx, .Lcerune_heap_live(%rip)
.Linput_read:
  leaq 120(%rsp), @ARG@
  callq cerune_input_read_byte
  cmpq $-2, %rax
  je .Linput_read
  testq %rax, %rax
  js .Linput_read_failed
  cmpq $1, %rax
  ja .Linput_invalid_response
  testq %rax, %rax
  je .Linput_eof_seen
  addq $1, 96(%rsp)
  movq 88(%rsp), %rcx
  cmpq 64(%rsp), %rcx
  je .Linput_limit
  movq 80(%rsp), %rdx
  addq $24, %rdx
  movzbl 120(%rsp), %eax
  movb %al, (%rdx,%rcx)
  addq $1, 88(%rsp)
  cmpl $10, %eax
  jne .Linput_read
  jmp .Linput_validate
.Linput_eof_seen:
  movq $1, .Linput_eof(%rip)
.Linput_validate:
  movq 88(%rsp), %rcx
  movq 80(%rsp), %rdx
  addq $24, %rdx
  callq cerune_utf8
  testl %eax, %eax
  je .Linput_utf8_failed
  movq 64(%rsp), %rax
  subq %rax, .Lcerune_heap_live(%rip)
  cmpq $0, 88(%rsp)
  je .Linput_free_empty
  movq 64(%rsp), %rax
  shrq $1, %rax
  cmpq 88(%rsp), %rax
  jb .Linput_publish
  movq 64(%rsp), %rax
  subq 88(%rsp), %rax
  cmpq $4096, %rax
  jb .Linput_publish
  movq 80(%rsp), @ARG@
  movq 88(%rsp), @ARG2@
  addq $24, @ARG2@
  callq realloc
  testq %rax, %rax
  je .Linput_publish
  movq %rax, 80(%rsp)
.Linput_publish:
  movq 80(%rsp), %rax
  movq .Lcerune_heap_head(%rip), %rcx
  movq %rcx, (%rax)
  movq $1, 8(%rax)
  movq 88(%rsp), %rcx
  movq %rcx, 16(%rax)
  movq %rax, .Lcerune_heap_head(%rip)
  addq %rcx, .Lcerune_heap_live(%rip)
  addq $16, %rax
  addq $696, %rsp
  retq
.Linput_free_empty:
  movq 80(%rsp), @ARG@
  callq free
.Linput_empty:
  leaq .Lcerune_heap_empty(%rip), %rax
  addq $696, %rsp
  retq
.Linput_invalid_limit:
  leaq .Linput_invalid_limit_text(%rip), %rax
  jmp .Linput_failed
.Linput_budget:
  leaq .Linput_budget_exceeded(%rip), %rax
  jmp .Linput_failed
.Linput_allocation:
  leaq .Linput_allocation_failed(%rip), %rax
  jmp .Linput_failed
.Linput_read_failed:
  leaq .Linput_read_failed_text(%rip), %rax
  jmp .Linput_failed
.Linput_invalid_response:
  leaq .Linput_unknown(%rip), %rax
  movq %rax, 104(%rsp)
  leaq .Linput_invalid_response_text(%rip), %rax
  jmp .Linput_failed
.Linput_limit:
  leaq .Linput_limit_exceeded(%rip), %rax
  jmp .Linput_failed
.Linput_utf8_failed:
  leaq .Linput_invalid_utf8(%rip), %rax
.Linput_failed:
  movq %rax, 112(%rsp)
  cmpq $0, 80(%rsp)
  je .Linput_format_failure
  movq 64(%rsp), %rax
  subq %rax, .Lcerune_heap_live(%rip)
  movq 80(%rsp), @ARG@
  callq free
  jmp .Linput_format_failure
"#;
const LINUX: &str = r#"
cerune_input_connection:
  xorl %eax, %eax
  retq
cerune_input_read_byte:
  subq $40, %rsp
  movq %rdi, %rsi
  xorl %edi, %edi
  movl $1, %edx
  callq read
  testq %rax, %rax
  jns .Linput_linux_done
  callq __errno_location
  cmpl $4, (%rax)
  jne .Linput_linux_error
  movq $-2, %rax
  jmp .Linput_linux_done
.Linput_linux_error:
  movq $-1, %rax
.Linput_linux_done:
  addq $40, %rsp
  retq
"#;
const WINDOWS: &str = r#"
.data
.p2align 3
.Linput_initialized:
  .quad 0
.Linput_handle:
  .quad 0
.Linput_pipe:
  .quad 0
.Linput_connection_error:
  .quad 0
.text
cerune_input_connection:
  subq $56, %rsp
  cmpq $0, .Linput_initialized(%rip)
  jne .Linput_connection_done
  movq $1, .Linput_initialized(%rip)
  movl $-10, %ecx
  callq GetStdHandle
  movq %rax, .Linput_handle(%rip)
  testq %rax, %rax
  je .Linput_connection_failed
  cmpq $-1, %rax
  je .Linput_connection_failed
  movq %rax, %rcx
  leaq 40(%rsp), %rdx
  callq GetConsoleMode
  testl %eax, %eax
  jne .Linput_console
  movq .Linput_handle(%rip), %rcx
  callq GetFileType
  cmpl $3, %eax
  jne .Linput_connection_done
  movq $1, .Linput_pipe(%rip)
  jmp .Linput_connection_done
.Linput_console:
  leaq .Linput_unsupported_connection(%rip), %rax
  jmp .Linput_connection_store
.Linput_connection_failed:
  leaq .Linput_read_failed_text(%rip), %rax
.Linput_connection_store:
  movq %rax, .Linput_connection_error(%rip)
.Linput_connection_done:
  movq .Linput_connection_error(%rip), %rax
  addq $56, %rsp
  retq
cerune_input_read_byte:
  subq $72, %rsp
  movq %rcx, 48(%rsp)
.Linput_windows_read:
  movl $0, 56(%rsp)
  movq .Linput_handle(%rip), %rcx
  movq 48(%rsp), %rdx
  movl $1, %r8d
  leaq 56(%rsp), %r9
  movq $0, 32(%rsp)
  callq ReadFile
  testl %eax, %eax
  je .Linput_windows_error
  movl 56(%rsp), %eax
  testl %eax, %eax
  jne .Linput_windows_done
  cmpq $0, .Linput_pipe(%rip)
  jne .Linput_windows_read
  jmp .Linput_windows_done
.Linput_windows_error:
  callq GetLastError
  cmpl $234, %eax
  jne .Linput_windows_eof_check
  cmpl $0, 56(%rsp)
  je .Linput_windows_eof_check
  movl 56(%rsp), %eax
  jmp .Linput_windows_done
.Linput_windows_eof_check:
  cmpl $109, %eax
  je .Linput_windows_eof
  cmpl $38, %eax
  je .Linput_windows_eof
  movq $-1, %rax
  jmp .Linput_windows_done
.Linput_windows_eof:
  xorl %eax, %eax
.Linput_windows_done:
  addq $72, %rsp
  retq
"#;
