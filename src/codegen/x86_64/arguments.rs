//! Native argument capture and execution. Internal helpers are not a public ABI.
use super::ir::{Instruction, Module};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| matches!(i, Instruction::ArgumentCount | Instruction::ArgumentGet))
}
fn adapt(text: &str, linux: bool) -> String {
    text.replace("@A@", if linux { "%rdi" } else { "%rcx" })
        .replace("@B@", if linux { "%rsi" } else { "%rdx" })
        .replace("@C@", if linux { "%rdx" } else { "%r8" })
}
pub(super) fn support(module: &Module) -> String {
    let linux = module.target.is_linux();
    let mut out = String::from(if linux {
        ".section .rodata\n"
    } else {
        ".section .rdata,\"dr\"\n"
    });
    for code in [
        "invalid-limit",
        "count-limit-exceeded",
        "byte-limit-exceeded",
        "invalid-utf8",
        "embedded-nul",
        "invalid-unicode",
        "allocation-failed",
        "unconfigured",
        "invalid-index",
        "index-out-of-bounds",
        "budget-exceeded",
    ] {
        writeln!(
            out,
            ".Largs_{}:\n  .asciz \"{code}\"",
            code.replace('-', "_")
        )
        .unwrap();
    }
    for (name, format) in [
        ("capture", "cerune: arguments-v1 phase=capture code=%s\\n"),
        (
            "capture_index",
            "cerune: arguments-v1 phase=capture code=%s index=%lld\\n",
        ),
        (
            "count",
            "cerune: arguments-v1 phase=execute operation=count code=%s%s",
        ),
        (
            "get",
            "cerune: arguments-v1 phase=execute operation=get code=%s index=%lld%s",
        ),
    ] {
        writeln!(out, ".Largs_format_{name}:\n  .asciz \"{format}\"").unwrap();
    }
    writeln!(
        out,
        ".p2align 3\n.Largs_max_count:\n  .quad {}\n.Largs_max_bytes:\n  .quad {}",
        module.argument_limits.max_count, module.argument_limits.max_bytes
    )
    .unwrap();
    out.push_str(&adapt(SNAPSHOT, linux));
    out.push_str(&adapt(GET, linux));
    out.push_str(&adapt(FAILURE, linux));
    out.push_str(if linux {
        "  leaq 80(%rsp), %rdi\n  movl $512, %esi\n  movq 32(%rsp), %rdx\n  movq 40(%rsp), %rcx\n  movq 48(%rsp), %r8\n  movq 56(%rsp), %r9\n  xorl %eax, %eax\n"
    } else {
        "  movq 48(%rsp), %rax\n  movq %rax, 32(%rsp)\n  movq 56(%rsp), %rax\n  movq %rax, 40(%rsp)\n  leaq 80(%rsp), %rcx\n  movl $512, %edx\n  movq 64(%rsp), %r8\n  movq 72(%rsp), %r9\n"
    });
    out.push_str("  callq snprintf\n  testl %eax, %eax\n  jle .Largs_exit\n  cmpl $512, %eax\n  jb .Largs_write\n  movl $511, %eax\n.Largs_write:\n");
    out.push_str(if linux {
        "  movl %eax, %edx\n  leaq 80(%rsp), %rsi\n  movl $2, %edi\n  callq write\n.Largs_exit:\n  movl $1, %edi\n"
    } else {
        "  movl %eax, %r8d\n  leaq 80(%rsp), %rdx\n  movl $2, %ecx\n  callq _write\n.Largs_exit:\n  movl $1, %ecx\n"
    });
    out.push_str("  callq _Exit\n  ud2\n");
    out
}
// Capture: rax=count, rdx=borrowed [pointer,length] pairs. Validate everything
// before allocating a single private block, then publish count/pointer together.
// Frame: 32 count,40 pairs,48 total,56 index,64 block,72 cursor,80 slot,88 length.
const SNAPSHOT: &str = r#"
.data
.p2align 3
.Largs_ready:
  .quad 0
.Largs_count:
  .quad 0
.Largs_values:
  .quad 0
.text
cerune_arguments_capture:
  subq $104, %rsp
  movq %rax, 32(%rsp)
  movq %rdx, 40(%rsp)
  movq $0, 48(%rsp)
  movq $0, 56(%rsp)
  cmpq $0, .Largs_max_count(%rip)
  js .Largs_bad_limit
  testq %rax, %rax
  js .Largs_bad_count
  cmpq .Largs_max_count(%rip), %rax
  ja .Largs_bad_count
.Largs_validate:
  movq 56(%rsp), %rax
  cmpq 32(%rsp), %rax
  je .Largs_allocate
  shlq $4, %rax
  addq 40(%rsp), %rax
  movq %rax, 80(%rsp)
  movq 8(%rax), %rcx
  movq %rcx, 88(%rsp)
  addq 48(%rsp), %rcx
  jc .Largs_bad_bytes
  cmpq .Largs_max_bytes(%rip), %rcx
  ja .Largs_bad_bytes
  movq %rcx, 48(%rsp)
  movq 8(%rax), %rcx
  movq (%rax), %rdx
  callq cerune_utf8
  testl %eax, %eax
  je .Largs_bad_utf8
  movq 80(%rsp), %rax
  movq (%rax), %rdx
  xorl %ecx, %ecx
.Largs_nul_loop:
  cmpq 88(%rsp), %rcx
  je .Largs_valid
  cmpb $0, (%rdx,%rcx)
  je .Largs_bad_nul
  addq $1, %rcx
  jmp .Largs_nul_loop
.Largs_valid:
  addq $1, 56(%rsp)
  jmp .Largs_validate
.Largs_allocate:
  movq $-1, 56(%rsp)
  movq 32(%rsp), %rax
  testq %rax, %rax
  je .Largs_empty_snapshot
  movabsq $576460752303423487, %rcx
  cmpq %rcx, %rax
  ja .Largs_snapshot_allocation
  shlq $4, %rax
  movq %rax, 72(%rsp)
  addq 48(%rsp), %rax
  jc .Largs_snapshot_allocation
  testq %rax, %rax
  js .Largs_snapshot_allocation
  movq %rax, @A@
  callq malloc
  testq %rax, %rax
  je .Largs_snapshot_allocation
  movq %rax, 64(%rsp)
  addq %rax, 72(%rsp)
  movq $0, 56(%rsp)
.Largs_copy:
  movq 56(%rsp), %rax
  cmpq 32(%rsp), %rax
  je .Largs_publish
  shlq $4, %rax
  movq %rax, %r10
  addq 40(%rsp), %rax
  addq 64(%rsp), %r10
  movq 8(%rax), %r11
  movq %r11, 88(%rsp)
  movq %r11, 8(%r10)
  movq 72(%rsp), %r11
  movq %r11, (%r10)
  cmpq $0, 88(%rsp)
  je .Largs_copy_next
  movq 72(%rsp), @A@
  movq (%rax), @B@
  movq 88(%rsp), @C@
  callq memcpy
.Largs_copy_next:
  movq 88(%rsp), %rax
  addq %rax, 72(%rsp)
  addq $1, 56(%rsp)
  jmp .Largs_copy
.Largs_empty_snapshot:
  movq $0, 64(%rsp)
.Largs_publish:
  movq 64(%rsp), %rax
  movq %rax, .Largs_values(%rip)
  movq 32(%rsp), %rax
  movq %rax, .Largs_count(%rip)
  movq $1, .Largs_ready(%rip)
  addq $104, %rsp
  retq
.Largs_bad_limit:
  leaq .Largs_invalid_limit(%rip), %rax
  movq $-1, 56(%rsp)
  jmp .Largs_capture_error
.Largs_bad_count:
  leaq .Largs_count_limit_exceeded(%rip), %rax
  movq $-1, 56(%rsp)
  jmp .Largs_capture_error
.Largs_bad_bytes:
  leaq .Largs_byte_limit_exceeded(%rip), %rax
  jmp .Largs_capture_error
.Largs_bad_utf8:
  leaq .Largs_invalid_utf8(%rip), %rax
  jmp .Largs_capture_error
.Largs_bad_nul:
  leaq .Largs_embedded_nul(%rip), %rax
  jmp .Largs_capture_error
.Largs_snapshot_allocation:
  leaq .Largs_allocation_failed(%rip), %rax
.Largs_capture_error:
  movq 56(%rsp), %rcx
  callq cerune_arguments_capture_failed
  ud2
cerune_arguments_dispose:
  subq $40, %rsp
  movq .Largs_values(%rip), @A@
  callq free
  movq $0, .Largs_ready(%rip)
  movq $0, .Largs_values(%rip)
  movq $0, .Largs_count(%rip)
  addq $40, %rsp
  retq
"#;
// Execution: r11=immutable origin, rax=get index. Return the existing string
// representation [length,bytes], owned by the ordinary string heap.
const GET: &str = r#"
cerune_arguments_count:
  cmpq $0, .Largs_ready(%rip)
  je .Largs_count_unconfigured
  movq .Largs_count(%rip), %rax
  retq
.Largs_count_unconfigured:
  leaq .Largs_format_count(%rip), %rax
  leaq .Largs_unconfigured(%rip), %rdx
  movq %r11, %rcx
  jmp cerune_arguments_failed
cerune_arguments_get:
  subq $104, %rsp
  movq %rax, 32(%rsp)
  movq %r11, 40(%rsp)
  cmpq $0, .Largs_ready(%rip)
  je .Largs_get_unconfigured
  testq %rax, %rax
  js .Largs_get_negative
  cmpq .Largs_count(%rip), %rax
  jae .Largs_get_bounds
  shlq $4, %rax
  addq .Largs_values(%rip), %rax
  movq (%rax), %rcx
  movq %rcx, 48(%rsp)
  movq 8(%rax), %rax
  movq %rax, 56(%rsp)
  testq %rax, %rax
  je .Largs_get_empty
  movq .Lcerune_heap_limit(%rip), %rcx
  subq .Lcerune_heap_live(%rip), %rcx
  cmpq %rcx, %rax
  ja .Largs_get_budget
  cmpq .Lcerune_heap_max(%rip), %rax
  ja .Largs_get_allocation
  leaq 24(%rax), @A@
  callq malloc
  testq %rax, %rax
  je .Largs_get_allocation
  movq %rax, 64(%rsp)
  movq .Lcerune_heap_head(%rip), %rcx
  movq %rcx, (%rax)
  movq $1, 8(%rax)
  movq 56(%rsp), %rcx
  movq %rcx, 16(%rax)
  leaq 24(%rax), @A@
  movq 48(%rsp), @B@
  movq 56(%rsp), @C@
  callq memcpy
  movq 64(%rsp), %rax
  movq %rax, .Lcerune_heap_head(%rip)
  movq 56(%rsp), %rcx
  addq %rcx, .Lcerune_heap_live(%rip)
  addq $16, %rax
  addq $104, %rsp
  retq
.Largs_get_empty:
  leaq .Lcerune_heap_empty(%rip), %rax
  addq $104, %rsp
  retq
.Largs_get_unconfigured:
  leaq .Largs_unconfigured(%rip), %rdx
  jmp .Largs_get_failed
.Largs_get_negative:
  leaq .Largs_invalid_index(%rip), %rdx
  jmp .Largs_get_failed
.Largs_get_bounds:
  leaq .Largs_index_out_of_bounds(%rip), %rdx
  jmp .Largs_get_failed
.Largs_get_budget:
  leaq .Largs_budget_exceeded(%rip), %rdx
  jmp .Largs_get_failed
.Largs_get_allocation:
  leaq .Largs_allocation_failed(%rip), %rdx
.Largs_get_failed:
  leaq .Largs_format_get(%rip), %rax
  movq 32(%rsp), %rcx
  movq 40(%rsp), %r11
  callq cerune_arguments_failed
  ud2
"#;
// Format/code/argument/origin in rax/rdx/rcx/r11; no input contents.
// The optional origin distinguishes execute failures from capture failures.
const FAILURE: &str = r#"
cerune_arguments_capture_failed:
  movq %rax, %rdx
  leaq .Largs_format_capture(%rip), %rax
  testq %rcx, %rcx
  js .Largs_capture_format_ready
  leaq .Largs_format_capture_index(%rip), %rax
.Largs_capture_format_ready:
  xorl %r11d, %r11d
  jmp cerune_arguments_failed
cerune_arguments_failed:
  subq $600, %rsp
  movq %rax, 32(%rsp)
  movq %rdx, 40(%rsp)
  movq %rcx, 48(%rsp)
  movq %r11, 56(%rsp)
  movq %rax, 64(%rsp)
  movq %rdx, 72(%rsp)
  testq %r11, %r11
  je .Largs_failure_format
  movq $0, @A@
  callq fflush
.Largs_failure_format:
"#;

pub(super) fn entry(linux: bool) -> String {
    let mut out = adapt(ENTRY, linux)
        .replace("@ENTRY@", if linux { "main" } else { "wmain" })
        .replace("@ARGC@", if linux { "%edi" } else { "%ecx" })
        .replace("@ARGV@", if linux { "%rsi" } else { "%rdx" })
        .replace("@FREE_VALUES@", if linux { "" } else { FREE_VALUES });
    out.push_str(&adapt(
        if linux { LINUX_VALUE } else { WINDOWS_VALUE },
        linux,
    ));
    // FREE_VALUES has one target argument placeholder.
    adapt(&out, linux)
}
// OS conversion precedes common capture, as in C/LLVM/QBE. argc excludes argv[0].
// Frame: 64 count,72 argv,80 temporary pairs,88 index,96 result.
const ENTRY: &str = r#"
.text
.globl @ENTRY@
@ENTRY@:
  subq $120, %rsp
  movq @ARGV@, 72(%rsp)
  movl @ARGC@, %eax
  testl %eax, %eax
  jle .Largs_start_empty
  subl $1, %eax
  jmp .Largs_start_size
.Largs_start_empty:
  xorl %eax, %eax
.Largs_start_size:
  movq %rax, 64(%rsp)
  movq $0, 80(%rsp)
  movq $0, 88(%rsp)
  testq %rax, %rax
  je .Largs_start_capture
  shlq $4, %rax
  movq %rax, @A@
  callq malloc
  testq %rax, %rax
  je .Largs_start_allocation
  movq %rax, 80(%rsp)
.Largs_start_convert:
  movq 88(%rsp), %rcx
  cmpq 64(%rsp), %rcx
  je .Largs_start_capture
  movq 72(%rsp), %rdx
  movq 8(%rdx,%rcx,8), %rax
  callq cerune_arguments_os_value
  movq 88(%rsp), %rdx
  shlq $4, %rdx
  addq 80(%rsp), %rdx
  movq %rax, (%rdx)
  movq %rcx, 8(%rdx)
  addq $1, 88(%rsp)
  jmp .Largs_start_convert
.Largs_start_capture:
  movq 64(%rsp), %rax
  movq 80(%rsp), %rdx
  callq cerune_arguments_capture
@FREE_VALUES@
  movq 80(%rsp), @A@
  callq free
  callq cerune_program
  movq %rax, 96(%rsp)
  callq cerune_arguments_dispose
  movq 96(%rsp), %rax
  addq $120, %rsp
  retq
.Largs_start_allocation:
  leaq .Largs_allocation_failed(%rip), %rax
  movq $-1, %rcx
  callq cerune_arguments_capture_failed
  ud2
"#;
const FREE_VALUES: &str = r#"
  movq $0, 88(%rsp)
.Largs_free_os:
  movq 88(%rsp), %rax
  cmpq 64(%rsp), %rax
  je .Largs_free_os_done
  shlq $4, %rax
  addq 80(%rsp), %rax
  cmpq $0, 8(%rax)
  je .Largs_free_os_next
  movq (%rax), @A@
  callq free
.Largs_free_os_next:
  addq $1, 88(%rsp)
  jmp .Largs_free_os
.Largs_free_os_done:
"#;
// Internal OS adapter: rax=text, rcx=index; returns rax=UTF8 bytes, rcx=length.
const LINUX_VALUE: &str = r#"
cerune_arguments_os_value:
  subq $72, %rsp
  movq %rax, 32(%rsp)
  movq %rcx, 40(%rsp)
  movq %rax, %rdi
  callq strlen
  movq %rax, 48(%rsp)
  movq %rax, %rcx
  movq 32(%rsp), %rdx
  callq cerune_utf8
  testl %eax, %eax
  je .Largs_os_unicode
  movq 32(%rsp), %rax
  movq 48(%rsp), %rcx
  addq $72, %rsp
  retq
.Largs_os_unicode:
  leaq .Largs_invalid_unicode(%rip), %rax
  movq 40(%rsp), %rcx
  callq cerune_arguments_capture_failed
  ud2
"#;
const WINDOWS_VALUE: &str = r#"
cerune_arguments_os_value:
  subq $104, %rsp
  movq %rax, 64(%rsp)
  movq %rcx, 72(%rsp)
  movl $65001, %ecx
  movl $128, %edx
  movq %rax, %r8
  movl $-1, %r9d
  movq $0, 32(%rsp)
  movq $0, 40(%rsp)
  movq $0, 48(%rsp)
  movq $0, 56(%rsp)
  callq WideCharToMultiByte
  testl %eax, %eax
  je .Largs_os_unicode
  movq %rax, 80(%rsp)
  cmpl $1, %eax
  je .Largs_os_empty
  movq %rax, %rcx
  callq malloc
  testq %rax, %rax
  je .Largs_os_allocation
  movq %rax, 88(%rsp)
  movl $65001, %ecx
  movl $128, %edx
  movq 64(%rsp), %r8
  movl $-1, %r9d
  movq %rax, 32(%rsp)
  movq 80(%rsp), %rax
  movq %rax, 40(%rsp)
  movq $0, 48(%rsp)
  movq $0, 56(%rsp)
  callq WideCharToMultiByte
  cmpq 80(%rsp), %rax
  jne .Largs_os_unicode
  movq 88(%rsp), %rax
  movq 80(%rsp), %rcx
  subq $1, %rcx
  addq $104, %rsp
  retq
.Largs_os_empty:
  leaq .Lcerune_heap_empty(%rip), %rax
  xorl %ecx, %ecx
  addq $104, %rsp
  retq
.Largs_os_allocation:
  leaq .Largs_allocation_failed(%rip), %rax
  jmp .Largs_os_failure
.Largs_os_unicode:
  leaq .Largs_invalid_unicode(%rip), %rax
.Largs_os_failure:
  movq 72(%rsp), %rcx
  callq cerune_arguments_capture_failed
  ud2
"#;
