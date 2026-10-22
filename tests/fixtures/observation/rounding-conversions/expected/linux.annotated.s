# cerune-asm-origins v1: UTF-8 byte ranges, end exclusive
# cerune-origin: synthetic
# target: x86_64-unknown-linux-gnu
.extern write
.extern __errno_location
.extern snprintf
.extern _Exit
.extern signal
.globl main
.extern fflush
.section .rodata
.p2align 4
.Lcerune_sign_f32:
.long 0x80000000
.long 0x00000000
.long 0x00000000
.long 0x00000000
.p2align 4
.Lcerune_sign_f64:
.quad 0x8000000000000000
.quad 0x0000000000000000
.p2align 3
.Lcerune_f64_0:
.quad 0x4004000000000000
.p2align 3
.Lcerune_f64_1:
.quad 0x4004000000000000
.p2align 3
.Lcerune_f64_2:
.quad 0x4004000000000000
.p2align 3
.Lcerune_f64_3:
.quad 0x4004000000000000
.p2align 3
.Lcerune_f64_4:
.quad 0x4072C00000000000
.p2align 3
.Lcerune_f64_5:
.quad 0x3FB999999999999A
.p2align 3
.Lcerune_f64_6:
.quad 0x406FE33333333333
.p2align 3
.Lcerune_f64_7:
.quad 0x406FF00000000000
.p2align 3
.Lcerune_f64_8:
.quad 0x406FD00000000000
.text
.section .rodata
.Loutput_record:
.asciz "cerune: io-v1 operation=write requested=%llu accepted=%llu%s"
.Loutput_format_error:
.asciz "cerune: output formatting failed%s"
.Loutput_i64:
.asciz "%lld"
.Loutput_u64:
.asciz "%llu"
.Loutput_f32:
.asciz "%.9g"
.Loutput_f64:
.asciz "%.17g"
.Loutput_true:
.asciz "true\n"
.Loutput_false:
.asciz "false\n"
.Loutput_lf:
.asciz "\n"
.Loutput_quote:
.asciz "\""
.text
cerune_output_write:
  subq $40, %rsp
  movq %rsi, %rdx
  movq %rdi, %rsi
  movl $1, %edi
  callq write@PLT
  addq $40, %rsp
  retq
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
  movq 56(%rsp), %rdi
  movq 72(%rsp), %rsi
  callq cerune_output_write
  testq %rax, %rax
  jns .Loutput_part_check
  callq __errno_location@PLT
  cmpl $4, 0(%rax)
  je .Loutput_part_write
  jmp .Loutput_part_failed
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
cerune_output_failed:
  subq $584, %rsp
  leaq 64(%rsp), %rdi
  movl $512, %esi
  leaq .Loutput_record(%rip), %rdx
  movq 0(%r10), %rcx
  movq 8(%r10), %r8
  movq 16(%r10), %r9
  xorl %eax, %eax
  callq snprintf@PLT
  leaq 64(%rsp), %r10
  callq cerune_output_diagnostic
  ud2
cerune_output_diagnostic:
  subq $40, %rsp
  testl %eax, %eax
  jle .Loutput_exit
  cmpl $512, %eax
  jb .Loutput_diag_write
  movl $511, %eax
.Loutput_diag_write:
  movl %eax, %edx
  movq %r10, %rsi
  movl $2, %edi
  callq write@PLT
.Loutput_exit:
  movl $1, %edi
  callq _Exit@PLT
  ud2
cerune_output_i64:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
  leaq 160(%rsp), %rdi
  movl $128, %esi
  leaq .Loutput_i64(%rip), %rdx
  movq %rax, %rcx
  xorl %eax, %eax
  callq snprintf@PLT
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, 0(%rcx,%rax,1)
  addq 104(%rsp), %rax
  movq %rax, 64(%rsp)
  movq %rcx, 96(%rsp)
  jmp .Loutput_begin
cerune_output_u64:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
  leaq 160(%rsp), %rdi
  movl $128, %esi
  leaq .Loutput_u64(%rip), %rdx
  movq %rax, %rcx
  xorl %eax, %eax
  callq snprintf@PLT
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, 0(%rcx,%rax,1)
  addq 104(%rsp), %rax
  movq %rax, 64(%rsp)
  movq %rcx, 96(%rsp)
  jmp .Loutput_begin
cerune_output_f32:
  cvtss2sd %xmm0, %xmm0
  movq %xmm0, %rax
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
  leaq 160(%rsp), %rdi
  movl $128, %esi
  leaq .Loutput_f32(%rip), %rdx
  movl $1, %eax
  callq snprintf@PLT
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, 0(%rcx,%rax,1)
  addq 104(%rsp), %rax
  movq %rax, 64(%rsp)
  movq %rcx, 96(%rsp)
  jmp .Loutput_begin
cerune_output_f64:
  movq %xmm0, %rax
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
  leaq 160(%rsp), %rdi
  movl $128, %esi
  leaq .Loutput_f64(%rip), %rdx
  movl $1, %eax
  callq snprintf@PLT
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, 0(%rcx,%rax,1)
  addq 104(%rsp), %rax
  movq %rax, 64(%rsp)
  movq %rcx, 96(%rsp)
  jmp .Loutput_begin
cerune_output_bool:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
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
cerune_output_string:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $1, 152(%rsp)
  movq 0(%rax), %rax
  addq 104(%rsp), %rax
  movq %rax, 64(%rsp)
  jmp .Loutput_begin
cerune_output_quoted:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $2, 152(%rsp)
  movq $0, 112(%rsp)
  movq 104(%rsp), %rax
  addq $2, %rax
  movq %rax, 64(%rsp)
.Loutput_count:
  movq 96(%rsp), %rdx
  movq 112(%rsp), %rax
  cmpq 0(%rdx), %rax
  je .Loutput_begin
  movzbl 8(%rdx,%rax,1), %eax
  leaq 160(%rsp), %rcx
  callq cerune_output_escape
  addq %rax, 64(%rsp)
  incq 112(%rsp)
  jmp .Loutput_count
.Loutput_format_failed:
  leaq 416(%rsp), %rdi
  movl $512, %esi
  leaq .Loutput_format_error(%rip), %rdx
  movq 80(%rsp), %rcx
  xorl %eax, %eax
  callq snprintf@PLT
  leaq 416(%rsp), %r10
  callq cerune_output_diagnostic
  ud2
.Loutput_begin:
  movq $0, 72(%rsp)
  movl $13, %edi
  movl $1, %esi
  callq signal@PLT
  cmpq $-1, %rax
  je .Loutput_failed
  movq %rax, 88(%rsp)
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
  movq 0(%rax), %rdx
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
  cmpq 0(%rdx), %rax
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
  movzbl 8(%rdx,%rax,1), %eax
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
.Loutput_end:
  movl $13, %edi
  movq 88(%rsp), %rsi
  callq signal@PLT
  cmpq $-1, %rax
  je .Loutput_failed
  addq $936, %rsp
  retq
.Loutput_failed:
  leaq 64(%rsp), %r10
  callq cerune_output_failed
  ud2
cerune_output_escape:
  cmpl $0, %eax
  je .Loutput_escape_0
  cmpl $1, %eax
  je .Loutput_escape_1
  cmpl $2, %eax
  je .Loutput_escape_2
  cmpl $3, %eax
  je .Loutput_escape_3
  cmpl $4, %eax
  je .Loutput_escape_4
  cmpl $5, %eax
  je .Loutput_escape_5
  cmpl $6, %eax
  je .Loutput_escape_6
  cmpl $7, %eax
  je .Loutput_escape_7
  cmpl $8, %eax
  je .Loutput_escape_8
  cmpl $9, %eax
  je .Loutput_escape_9
  cmpl $10, %eax
  je .Loutput_escape_10
  cmpl $11, %eax
  je .Loutput_escape_11
  cmpl $12, %eax
  je .Loutput_escape_12
  cmpl $13, %eax
  je .Loutput_escape_13
  cmpl $14, %eax
  je .Loutput_escape_14
  cmpl $15, %eax
  je .Loutput_escape_15
  cmpl $16, %eax
  je .Loutput_escape_16
  cmpl $17, %eax
  je .Loutput_escape_17
  cmpl $18, %eax
  je .Loutput_escape_18
  cmpl $19, %eax
  je .Loutput_escape_19
  cmpl $20, %eax
  je .Loutput_escape_20
  cmpl $21, %eax
  je .Loutput_escape_21
  cmpl $22, %eax
  je .Loutput_escape_22
  cmpl $23, %eax
  je .Loutput_escape_23
  cmpl $24, %eax
  je .Loutput_escape_24
  cmpl $25, %eax
  je .Loutput_escape_25
  cmpl $26, %eax
  je .Loutput_escape_26
  cmpl $27, %eax
  je .Loutput_escape_27
  cmpl $28, %eax
  je .Loutput_escape_28
  cmpl $29, %eax
  je .Loutput_escape_29
  cmpl $30, %eax
  je .Loutput_escape_30
  cmpl $31, %eax
  je .Loutput_escape_31
  cmpl $34, %eax
  je .Loutput_escape_34
  cmpl $92, %eax
  je .Loutput_escape_92
  cmpl $127, %eax
  je .Loutput_escape_127
  movb %al, 0(%rcx)
  movl $1, %eax
  retq
.Loutput_escape_0:
  movb $92, 0(%rcx)
  movb $48, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_1:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $49, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_2:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $50, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_3:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $51, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_4:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $52, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_5:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $53, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_6:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $54, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_7:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $55, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_8:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $56, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_9:
  movb $92, 0(%rcx)
  movb $116, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_10:
  movb $92, 0(%rcx)
  movb $110, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_11:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $98, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_12:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $99, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_13:
  movb $92, 0(%rcx)
  movb $114, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_14:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $101, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_15:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $48, 3(%rcx)
  movb $102, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_16:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $48, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_17:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $49, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_18:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $50, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_19:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $51, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_20:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $52, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_21:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $53, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_22:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $54, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_23:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $55, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_24:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $56, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_25:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $57, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_26:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $97, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_27:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $98, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_28:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $99, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_29:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $100, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_30:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $101, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_31:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $49, 3(%rcx)
  movb $102, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
.Loutput_escape_34:
  movb $92, 0(%rcx)
  movb $34, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_92:
  movb $92, 0(%rcx)
  movb $92, 1(%rcx)
  movl $2, %eax
  retq
.Loutput_escape_127:
  movb $92, 0(%rcx)
  movb $117, 1(%rcx)
  movb $123, 2(%rcx)
  movb $55, 3(%rcx)
  movb $102, 4(%rcx)
  movb $125, 5(%rcx)
  movl $6, %eax
  retq
# cerune-origin: synthetic
.p2align 4
cerune_fn_value_0:
  pushq %rbp
  movq %rsp, %rbp
  subq $16, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  movsd %xmm0, -8(%rbp)
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_fn_0_block_0
# cerune-origin: synthetic
# cerune-mir: v1 fn_0 bb0 block -> lir 2 (function-entry)
cerune_origin_mir_fn_0_bb0_block_lir2:
.Lcerune_fn_0_block_0:
# mir_block
# cerune-origin: #5 bytes 68..69
cerune_origin_n5_fn_0_3:
# cerune-mir: v1 fn_0 bb0 i0 -> lir 3 (source)
cerune_origin_mir_fn_0_bb0_i0_lir3:
  movsd -8(%rbp), %xmm0
# cerune-origin: #5 bytes 68..69
cerune_origin_n5_fn_0_4:
# cerune-mir: v1 fn_0 bb0 i0 -> lir 4 (source)
cerune_origin_mir_fn_0_bb0_i0_lir4:
  movsd %xmm0, -16(%rbp)
# cerune-origin: #4 bytes 61..70
cerune_origin_n4_fn_0_5:
# cerune-mir: v1 fn_0 bb0 i1 -> lir 5 (source)
cerune_origin_mir_fn_0_bb0_i1_lir5:
  movsd -16(%rbp), %xmm0
# cerune-origin: #4 bytes 61..70
cerune_origin_n4_fn_0_6:
# cerune-mir: v1 fn_0 bb0 i1 -> lir 6 (source)
cerune_origin_mir_fn_0_bb0_i1_lir6:
  addq $16, %rsp
  popq %rbp
  retq
# cerune-origin: #4 bytes 61..70
cerune_origin_n4_fn_0_7:
# cerune-mir: v1 fn_0 bb1 block -> lir 7 (after-return)
cerune_origin_mir_fn_0_bb1_block_lir7:
.Lcerune_fn_0_block_1:
# mir_block
# cerune-origin: synthetic
# cerune-mir: v1 fn_0 bb1 i2 -> lir 8 (function-end)
cerune_origin_mir_fn_0_bb1_i2_lir8:
  ud2
# cerune-origin: synthetic
.p2align 4
main:
  pushq %rbp
  movq %rsp, %rbp
  subq $256, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_block_0
# cerune-origin: synthetic
# cerune-mir: v1 main bb0 block -> lir 1 (function-entry)
cerune_origin_mir_main_bb0_block_lir1:
.Lcerune_block_0:
# mir_block
# cerune-origin: #10 bytes 97..100
cerune_origin_n10_main_2:
# cerune-mir: v1 main bb0 i0 -> lir 2 (source)
cerune_origin_mir_main_bb0_i0_lir2:
  movsd .Lcerune_f64_0(%rip), %xmm0
# cerune-origin: #10 bytes 97..100
cerune_origin_n10_main_3:
# cerune-mir: v1 main bb0 i0 -> lir 3 (source)
cerune_origin_mir_main_bb0_i0_lir3:
  movsd %xmm0, -8(%rbp)
# cerune-origin: #9 bytes 96..100
cerune_origin_n9_main_4:
# cerune-mir: v1 main bb0 i1 -> lir 4 (source)
cerune_origin_mir_main_bb0_i1_lir4:
  movsd -8(%rbp), %xmm0
# cerune-origin: #9 bytes 96..100
cerune_origin_n9_main_5:
# cerune-mir: v1 main bb0 i1 -> lir 5 (source)
cerune_origin_mir_main_bb0_i1_lir5:
  xorpd .Lcerune_sign_f64(%rip), %xmm0
# cerune-origin: #9 bytes 96..100
cerune_origin_n9_main_6:
# cerune-mir: v1 main bb0 i1 -> lir 6 (source)
cerune_origin_mir_main_bb0_i1_lir6:
  movsd %xmm0, -16(%rbp)
# cerune-origin: #8 bytes 90..101
cerune_origin_n8_main_7:
# cerune-mir: v1 main bb0 i2 -> lir 7 (source)
cerune_origin_mir_main_bb0_i2_lir7:
  movsd -16(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #8 bytes 90..101
cerune_origin_n8_main_8:
# cerune-mir: v1 main bb0 i2 -> lir 8 (source)
cerune_origin_mir_main_bb0_i2_lir8:
  movsd %xmm0, -24(%rbp)
# cerune-origin: #7 bytes 79..102
cerune_origin_n7_main_9:
# cerune-mir: v1 main bb0 i3 -> lir 9 (source)
cerune_origin_mir_main_bb0_i3_lir9:
  movsd -24(%rbp), %xmm0
# cerune-origin: #7 bytes 79..102
cerune_origin_n7_main_10:
# cerune-mir: v1 main bb0 i3 -> lir 10 (source)
cerune_origin_mir_main_bb0_i3_lir10:
# policy: floor
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_floor_2_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_floor_2_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_floor_2_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_floor_2_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_floor_2_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_floor_2_down
  jmp .Lcerune_main_floor_2_rounded
.Lcerune_main_floor_2_up:
  addq $1, %rax
  jmp .Lcerune_main_floor_2_rounded
.Lcerune_main_floor_2_down:
  subq $1, %rax
.Lcerune_main_floor_2_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_floor_2_bounds:
# range policy after rounding, then convert
  movabsq $-4332462841530417152, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_floor_2_minimum
  movabsq $4890909195324358656, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_floor_2_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_floor_2_done
.Lcerune_main_floor_2_nonfinite:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_0(%rip), %rsi
  movl $67, %edx
  callq write@PLT
  ud2
.Lcerune_main_floor_2_minimum:
.Lcerune_main_floor_2_maximum:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_1(%rip), %rsi
  movl $69, %edx
  callq write@PLT
  ud2
.Lcerune_main_floor_2_done:
# cerune-origin: #7 bytes 79..102
cerune_origin_n7_main_11:
# cerune-mir: v1 main bb0 i3 -> lir 11 (source)
cerune_origin_mir_main_bb0_i3_lir11:
  movq %rax, -32(%rbp)
# cerune-origin: #6 bytes 73..104
cerune_origin_n6_main_12:
# cerune-mir: v1 main bb0 i4 -> lir 12 (source)
cerune_origin_mir_main_bb0_i4_lir12:
  movq -32(%rbp), %rax
# cerune-origin: #6 bytes 73..104
cerune_origin_n6_main_13:
# cerune-mir: v1 main bb0 i4 -> lir 13 (source)
cerune_origin_mir_main_bb0_i4_lir13:
  movl $1, %r10d
  leaq .Lcerune_output_origin_2(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #15 bytes 128..131
cerune_origin_n15_main_14:
# cerune-mir: v1 main bb0 i5 -> lir 14 (source)
cerune_origin_mir_main_bb0_i5_lir14:
  movsd .Lcerune_f64_1(%rip), %xmm0
# cerune-origin: #15 bytes 128..131
cerune_origin_n15_main_15:
# cerune-mir: v1 main bb0 i5 -> lir 15 (source)
cerune_origin_mir_main_bb0_i5_lir15:
  movsd %xmm0, -40(%rbp)
# cerune-origin: #14 bytes 127..131
cerune_origin_n14_main_16:
# cerune-mir: v1 main bb0 i6 -> lir 16 (source)
cerune_origin_mir_main_bb0_i6_lir16:
  movsd -40(%rbp), %xmm0
# cerune-origin: #14 bytes 127..131
cerune_origin_n14_main_17:
# cerune-mir: v1 main bb0 i6 -> lir 17 (source)
cerune_origin_mir_main_bb0_i6_lir17:
  xorpd .Lcerune_sign_f64(%rip), %xmm0
# cerune-origin: #14 bytes 127..131
cerune_origin_n14_main_18:
# cerune-mir: v1 main bb0 i6 -> lir 18 (source)
cerune_origin_mir_main_bb0_i6_lir18:
  movsd %xmm0, -48(%rbp)
# cerune-origin: #13 bytes 121..132
cerune_origin_n13_main_19:
# cerune-mir: v1 main bb0 i7 -> lir 19 (source)
cerune_origin_mir_main_bb0_i7_lir19:
  movsd -48(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #13 bytes 121..132
cerune_origin_n13_main_20:
# cerune-mir: v1 main bb0 i7 -> lir 20 (source)
cerune_origin_mir_main_bb0_i7_lir20:
  movsd %xmm0, -56(%rbp)
# cerune-origin: #12 bytes 111..133
cerune_origin_n12_main_21:
# cerune-mir: v1 main bb0 i8 -> lir 21 (source)
cerune_origin_mir_main_bb0_i8_lir21:
  movsd -56(%rbp), %xmm0
# cerune-origin: #12 bytes 111..133
cerune_origin_n12_main_22:
# cerune-mir: v1 main bb0 i8 -> lir 22 (source)
cerune_origin_mir_main_bb0_i8_lir22:
# policy: ceil
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_ceil_3_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_ceil_3_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_ceil_3_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_ceil_3_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_ceil_3_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_ceil_3_up
  jmp .Lcerune_main_ceil_3_rounded
.Lcerune_main_ceil_3_up:
  addq $1, %rax
  jmp .Lcerune_main_ceil_3_rounded
.Lcerune_main_ceil_3_down:
  subq $1, %rax
.Lcerune_main_ceil_3_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_ceil_3_bounds:
# range policy after rounding, then convert
  movabsq $-4332462841530417152, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_ceil_3_minimum
  movabsq $4890909195324358656, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_ceil_3_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_ceil_3_done
.Lcerune_main_ceil_3_nonfinite:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_3(%rip), %rsi
  movl $69, %edx
  callq write@PLT
  ud2
.Lcerune_main_ceil_3_minimum:
.Lcerune_main_ceil_3_maximum:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_4(%rip), %rsi
  movl $71, %edx
  callq write@PLT
  ud2
.Lcerune_main_ceil_3_done:
# cerune-origin: #12 bytes 111..133
cerune_origin_n12_main_23:
# cerune-mir: v1 main bb0 i8 -> lir 23 (source)
cerune_origin_mir_main_bb0_i8_lir23:
  movq %rax, -64(%rbp)
# cerune-origin: #11 bytes 105..135
cerune_origin_n11_main_24:
# cerune-mir: v1 main bb0 i9 -> lir 24 (source)
cerune_origin_mir_main_bb0_i9_lir24:
  movq -64(%rbp), %rax
# cerune-origin: #11 bytes 105..135
cerune_origin_n11_main_25:
# cerune-mir: v1 main bb0 i9 -> lir 25 (source)
cerune_origin_mir_main_bb0_i9_lir25:
  movl $1, %r10d
  leaq .Lcerune_output_origin_5(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #19 bytes 159..162
cerune_origin_n19_main_26:
# cerune-mir: v1 main bb0 i10 -> lir 26 (source)
cerune_origin_mir_main_bb0_i10_lir26:
  movsd .Lcerune_f64_2(%rip), %xmm0
# cerune-origin: #19 bytes 159..162
cerune_origin_n19_main_27:
# cerune-mir: v1 main bb0 i10 -> lir 27 (source)
cerune_origin_mir_main_bb0_i10_lir27:
  movsd %xmm0, -72(%rbp)
# cerune-origin: #18 bytes 153..163
cerune_origin_n18_main_28:
# cerune-mir: v1 main bb0 i11 -> lir 28 (source)
cerune_origin_mir_main_bb0_i11_lir28:
  movsd -72(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #18 bytes 153..163
cerune_origin_n18_main_29:
# cerune-mir: v1 main bb0 i11 -> lir 29 (source)
cerune_origin_mir_main_bb0_i11_lir29:
  movsd %xmm0, -80(%rbp)
# cerune-origin: #17 bytes 142..164
cerune_origin_n17_main_30:
# cerune-mir: v1 main bb0 i12 -> lir 30 (source)
cerune_origin_mir_main_bb0_i12_lir30:
  movsd -80(%rbp), %xmm0
# cerune-origin: #17 bytes 142..164
cerune_origin_n17_main_31:
# cerune-mir: v1 main bb0 i12 -> lir 31 (source)
cerune_origin_mir_main_bb0_i12_lir31:
# policy: round
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_round_4_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_round_4_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_round_4_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_round_4_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_round_4_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $4602678819172646912, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_round_4_up
  movabsq $-4620693217682128896, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_round_4_down
  jmp .Lcerune_main_round_4_rounded
.Lcerune_main_round_4_up:
  addq $1, %rax
  jmp .Lcerune_main_round_4_rounded
.Lcerune_main_round_4_down:
  subq $1, %rax
.Lcerune_main_round_4_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_round_4_bounds:
# range policy after rounding, then convert
  movabsq $-4332462841530417152, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_round_4_minimum
  movabsq $4890909195324358656, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_round_4_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_round_4_done
.Lcerune_main_round_4_nonfinite:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_6(%rip), %rsi
  movl $69, %edx
  callq write@PLT
  ud2
.Lcerune_main_round_4_minimum:
.Lcerune_main_round_4_maximum:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_7(%rip), %rsi
  movl $71, %edx
  callq write@PLT
  ud2
.Lcerune_main_round_4_done:
# cerune-origin: #17 bytes 142..164
cerune_origin_n17_main_32:
# cerune-mir: v1 main bb0 i12 -> lir 32 (source)
cerune_origin_mir_main_bb0_i12_lir32:
  movq %rax, -88(%rbp)
# cerune-origin: #16 bytes 136..166
cerune_origin_n16_main_33:
# cerune-mir: v1 main bb0 i13 -> lir 33 (source)
cerune_origin_mir_main_bb0_i13_lir33:
  movq -88(%rbp), %rax
# cerune-origin: #16 bytes 136..166
cerune_origin_n16_main_34:
# cerune-mir: v1 main bb0 i13 -> lir 34 (source)
cerune_origin_mir_main_bb0_i13_lir34:
  movl $1, %r10d
  leaq .Lcerune_output_origin_8(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #23 bytes 200..203
cerune_origin_n23_main_35:
# cerune-mir: v1 main bb0 i14 -> lir 35 (source)
cerune_origin_mir_main_bb0_i14_lir35:
  movsd .Lcerune_f64_3(%rip), %xmm0
# cerune-origin: #23 bytes 200..203
cerune_origin_n23_main_36:
# cerune-mir: v1 main bb0 i14 -> lir 36 (source)
cerune_origin_mir_main_bb0_i14_lir36:
  movsd %xmm0, -96(%rbp)
# cerune-origin: #22 bytes 194..204
cerune_origin_n22_main_37:
# cerune-mir: v1 main bb0 i15 -> lir 37 (source)
cerune_origin_mir_main_bb0_i15_lir37:
  movsd -96(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #22 bytes 194..204
cerune_origin_n22_main_38:
# cerune-mir: v1 main bb0 i15 -> lir 38 (source)
cerune_origin_mir_main_bb0_i15_lir38:
  movsd %xmm0, -104(%rbp)
# cerune-origin: #21 bytes 173..205
cerune_origin_n21_main_39:
# cerune-mir: v1 main bb0 i16 -> lir 39 (source)
cerune_origin_mir_main_bb0_i16_lir39:
  movsd -104(%rbp), %xmm0
# cerune-origin: #21 bytes 173..205
cerune_origin_n21_main_40:
# cerune-mir: v1 main bb0 i16 -> lir 40 (source)
cerune_origin_mir_main_bb0_i16_lir40:
# policy: round_ties_even
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_round_ties_even_5_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_round_ties_even_5_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_round_ties_even_5_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_round_ties_even_5_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_round_ties_even_5_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $4602678819172646912, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_round_ties_even_5_up
  je .Lcerune_main_round_ties_even_5_tie
  movabsq $-4620693217682128896, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_round_ties_even_5_down
  jne .Lcerune_main_round_ties_even_5_rounded
.Lcerune_main_round_ties_even_5_tie:
  testq $1, %rax
  je .Lcerune_main_round_ties_even_5_rounded
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_round_ties_even_5_up
  jmp .Lcerune_main_round_ties_even_5_down
  jmp .Lcerune_main_round_ties_even_5_rounded
.Lcerune_main_round_ties_even_5_up:
  addq $1, %rax
  jmp .Lcerune_main_round_ties_even_5_rounded
.Lcerune_main_round_ties_even_5_down:
  subq $1, %rax
.Lcerune_main_round_ties_even_5_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_round_ties_even_5_bounds:
# range policy after rounding, then convert
  movabsq $-4332462841530417152, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_round_ties_even_5_minimum
  movabsq $4890909195324358656, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_round_ties_even_5_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_round_ties_even_5_done
.Lcerune_main_round_ties_even_5_nonfinite:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_9(%rip), %rsi
  movl $69, %edx
  callq write@PLT
  ud2
.Lcerune_main_round_ties_even_5_minimum:
.Lcerune_main_round_ties_even_5_maximum:
  xorl %edi, %edi
  callq fflush@PLT
  movl $2, %edi
  leaq .Lcerune_failure_10(%rip), %rsi
  movl $71, %edx
  callq write@PLT
  ud2
.Lcerune_main_round_ties_even_5_done:
# cerune-origin: #21 bytes 173..205
cerune_origin_n21_main_41:
# cerune-mir: v1 main bb0 i16 -> lir 41 (source)
cerune_origin_mir_main_bb0_i16_lir41:
  movq %rax, -112(%rbp)
# cerune-origin: #20 bytes 167..207
cerune_origin_n20_main_42:
# cerune-mir: v1 main bb0 i17 -> lir 42 (source)
cerune_origin_mir_main_bb0_i17_lir42:
  movq -112(%rbp), %rax
# cerune-origin: #20 bytes 167..207
cerune_origin_n20_main_43:
# cerune-mir: v1 main bb0 i17 -> lir 43 (source)
cerune_origin_mir_main_bb0_i17_lir43:
  movl $1, %r10d
  leaq .Lcerune_output_origin_11(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #27 bytes 241..246
cerune_origin_n27_main_44:
# cerune-mir: v1 main bb0 i18 -> lir 44 (source)
cerune_origin_mir_main_bb0_i18_lir44:
  movsd .Lcerune_f64_4(%rip), %xmm0
# cerune-origin: #27 bytes 241..246
cerune_origin_n27_main_45:
# cerune-mir: v1 main bb0 i18 -> lir 45 (source)
cerune_origin_mir_main_bb0_i18_lir45:
  movsd %xmm0, -120(%rbp)
# cerune-origin: #26 bytes 235..247
cerune_origin_n26_main_46:
# cerune-mir: v1 main bb0 i19 -> lir 46 (source)
cerune_origin_mir_main_bb0_i19_lir46:
  movsd -120(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #26 bytes 235..247
cerune_origin_n26_main_47:
# cerune-mir: v1 main bb0 i19 -> lir 47 (source)
cerune_origin_mir_main_bb0_i19_lir47:
  movsd %xmm0, -128(%rbp)
# cerune-origin: #25 bytes 214..248
cerune_origin_n25_main_48:
# cerune-mir: v1 main bb0 i20 -> lir 48 (source)
cerune_origin_mir_main_bb0_i20_lir48:
  movsd -128(%rbp), %xmm0
# cerune-origin: #25 bytes 214..248
cerune_origin_n25_main_49:
# cerune-mir: v1 main bb0 i20 -> lir 49 (source)
cerune_origin_mir_main_bb0_i20_lir49:
# policy: saturating_trunc
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_trunc_6_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_trunc_6_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_trunc_6_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_trunc_6_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_trunc_6_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  jmp .Lcerune_main_saturating_trunc_6_rounded
.Lcerune_main_saturating_trunc_6_up:
  addq $1, %rax
  jmp .Lcerune_main_saturating_trunc_6_rounded
.Lcerune_main_saturating_trunc_6_down:
  subq $1, %rax
.Lcerune_main_saturating_trunc_6_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_saturating_trunc_6_bounds:
# range policy after rounding, then convert
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_trunc_6_minimum
  movabsq $4643211215818981376, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_trunc_6_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_saturating_trunc_6_done
.Lcerune_main_saturating_trunc_6_nonfinite:
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_trunc_6_zero
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_trunc_6_maximum
  jmp .Lcerune_main_saturating_trunc_6_minimum
.Lcerune_main_saturating_trunc_6_zero:
  xorq %rax, %rax
  jmp .Lcerune_main_saturating_trunc_6_done
.Lcerune_main_saturating_trunc_6_minimum:
  movabsq $0, %rax
  jmp .Lcerune_main_saturating_trunc_6_done
.Lcerune_main_saturating_trunc_6_maximum:
  movabsq $255, %rax
.Lcerune_main_saturating_trunc_6_done:
# cerune-origin: #25 bytes 214..248
cerune_origin_n25_main_50:
# cerune-mir: v1 main bb0 i20 -> lir 50 (source)
cerune_origin_mir_main_bb0_i20_lir50:
  movq %rax, -136(%rbp)
# cerune-origin: #24 bytes 208..250
cerune_origin_n24_main_51:
# cerune-mir: v1 main bb0 i21 -> lir 51 (source)
cerune_origin_mir_main_bb0_i21_lir51:
  movq -136(%rbp), %rax
# cerune-origin: #24 bytes 208..250
cerune_origin_n24_main_52:
# cerune-mir: v1 main bb0 i21 -> lir 52 (source)
cerune_origin_mir_main_bb0_i21_lir52:
  movl $1, %r10d
  leaq .Lcerune_output_origin_12(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #32 bytes 285..288
cerune_origin_n32_main_53:
# cerune-mir: v1 main bb0 i22 -> lir 53 (source)
cerune_origin_mir_main_bb0_i22_lir53:
  movsd .Lcerune_f64_5(%rip), %xmm0
# cerune-origin: #32 bytes 285..288
cerune_origin_n32_main_54:
# cerune-mir: v1 main bb0 i22 -> lir 54 (source)
cerune_origin_mir_main_bb0_i22_lir54:
  movsd %xmm0, -144(%rbp)
# cerune-origin: #31 bytes 284..288
cerune_origin_n31_main_55:
# cerune-mir: v1 main bb0 i23 -> lir 55 (source)
cerune_origin_mir_main_bb0_i23_lir55:
  movsd -144(%rbp), %xmm0
# cerune-origin: #31 bytes 284..288
cerune_origin_n31_main_56:
# cerune-mir: v1 main bb0 i23 -> lir 56 (source)
cerune_origin_mir_main_bb0_i23_lir56:
  xorpd .Lcerune_sign_f64(%rip), %xmm0
# cerune-origin: #31 bytes 284..288
cerune_origin_n31_main_57:
# cerune-mir: v1 main bb0 i23 -> lir 57 (source)
cerune_origin_mir_main_bb0_i23_lir57:
  movsd %xmm0, -152(%rbp)
# cerune-origin: #30 bytes 278..289
cerune_origin_n30_main_58:
# cerune-mir: v1 main bb0 i24 -> lir 58 (source)
cerune_origin_mir_main_bb0_i24_lir58:
  movsd -152(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #30 bytes 278..289
cerune_origin_n30_main_59:
# cerune-mir: v1 main bb0 i24 -> lir 59 (source)
cerune_origin_mir_main_bb0_i24_lir59:
  movsd %xmm0, -160(%rbp)
# cerune-origin: #29 bytes 257..290
cerune_origin_n29_main_60:
# cerune-mir: v1 main bb0 i25 -> lir 60 (source)
cerune_origin_mir_main_bb0_i25_lir60:
  movsd -160(%rbp), %xmm0
# cerune-origin: #29 bytes 257..290
cerune_origin_n29_main_61:
# cerune-mir: v1 main bb0 i25 -> lir 61 (source)
cerune_origin_mir_main_bb0_i25_lir61:
# policy: saturating_floor
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_floor_7_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_floor_7_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_floor_7_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_floor_7_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_floor_7_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_floor_7_down
  jmp .Lcerune_main_saturating_floor_7_rounded
.Lcerune_main_saturating_floor_7_up:
  addq $1, %rax
  jmp .Lcerune_main_saturating_floor_7_rounded
.Lcerune_main_saturating_floor_7_down:
  subq $1, %rax
.Lcerune_main_saturating_floor_7_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_saturating_floor_7_bounds:
# range policy after rounding, then convert
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_floor_7_minimum
  movabsq $4643211215818981376, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_floor_7_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_saturating_floor_7_done
.Lcerune_main_saturating_floor_7_nonfinite:
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_floor_7_zero
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_floor_7_maximum
  jmp .Lcerune_main_saturating_floor_7_minimum
.Lcerune_main_saturating_floor_7_zero:
  xorq %rax, %rax
  jmp .Lcerune_main_saturating_floor_7_done
.Lcerune_main_saturating_floor_7_minimum:
  movabsq $0, %rax
  jmp .Lcerune_main_saturating_floor_7_done
.Lcerune_main_saturating_floor_7_maximum:
  movabsq $255, %rax
.Lcerune_main_saturating_floor_7_done:
# cerune-origin: #29 bytes 257..290
cerune_origin_n29_main_62:
# cerune-mir: v1 main bb0 i25 -> lir 62 (source)
cerune_origin_mir_main_bb0_i25_lir62:
  movq %rax, -168(%rbp)
# cerune-origin: #28 bytes 251..292
cerune_origin_n28_main_63:
# cerune-mir: v1 main bb0 i26 -> lir 63 (source)
cerune_origin_mir_main_bb0_i26_lir63:
  movq -168(%rbp), %rax
# cerune-origin: #28 bytes 251..292
cerune_origin_n28_main_64:
# cerune-mir: v1 main bb0 i26 -> lir 64 (source)
cerune_origin_mir_main_bb0_i26_lir64:
  movl $1, %r10d
  leaq .Lcerune_output_origin_13(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #36 bytes 325..330
cerune_origin_n36_main_65:
# cerune-mir: v1 main bb0 i27 -> lir 65 (source)
cerune_origin_mir_main_bb0_i27_lir65:
  movsd .Lcerune_f64_6(%rip), %xmm0
# cerune-origin: #36 bytes 325..330
cerune_origin_n36_main_66:
# cerune-mir: v1 main bb0 i27 -> lir 66 (source)
cerune_origin_mir_main_bb0_i27_lir66:
  movsd %xmm0, -176(%rbp)
# cerune-origin: #35 bytes 319..331
cerune_origin_n35_main_67:
# cerune-mir: v1 main bb0 i28 -> lir 67 (source)
cerune_origin_mir_main_bb0_i28_lir67:
  movsd -176(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #35 bytes 319..331
cerune_origin_n35_main_68:
# cerune-mir: v1 main bb0 i28 -> lir 68 (source)
cerune_origin_mir_main_bb0_i28_lir68:
  movsd %xmm0, -184(%rbp)
# cerune-origin: #34 bytes 299..332
cerune_origin_n34_main_69:
# cerune-mir: v1 main bb0 i29 -> lir 69 (source)
cerune_origin_mir_main_bb0_i29_lir69:
  movsd -184(%rbp), %xmm0
# cerune-origin: #34 bytes 299..332
cerune_origin_n34_main_70:
# cerune-mir: v1 main bb0 i29 -> lir 70 (source)
cerune_origin_mir_main_bb0_i29_lir70:
# policy: saturating_ceil
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_ceil_8_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_ceil_8_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_ceil_8_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_ceil_8_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_ceil_8_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_ceil_8_up
  jmp .Lcerune_main_saturating_ceil_8_rounded
.Lcerune_main_saturating_ceil_8_up:
  addq $1, %rax
  jmp .Lcerune_main_saturating_ceil_8_rounded
.Lcerune_main_saturating_ceil_8_down:
  subq $1, %rax
.Lcerune_main_saturating_ceil_8_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_saturating_ceil_8_bounds:
# range policy after rounding, then convert
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_ceil_8_minimum
  movabsq $4643211215818981376, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_ceil_8_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_saturating_ceil_8_done
.Lcerune_main_saturating_ceil_8_nonfinite:
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_ceil_8_zero
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_ceil_8_maximum
  jmp .Lcerune_main_saturating_ceil_8_minimum
.Lcerune_main_saturating_ceil_8_zero:
  xorq %rax, %rax
  jmp .Lcerune_main_saturating_ceil_8_done
.Lcerune_main_saturating_ceil_8_minimum:
  movabsq $0, %rax
  jmp .Lcerune_main_saturating_ceil_8_done
.Lcerune_main_saturating_ceil_8_maximum:
  movabsq $255, %rax
.Lcerune_main_saturating_ceil_8_done:
# cerune-origin: #34 bytes 299..332
cerune_origin_n34_main_71:
# cerune-mir: v1 main bb0 i29 -> lir 71 (source)
cerune_origin_mir_main_bb0_i29_lir71:
  movq %rax, -192(%rbp)
# cerune-origin: #33 bytes 293..334
cerune_origin_n33_main_72:
# cerune-mir: v1 main bb0 i30 -> lir 72 (source)
cerune_origin_mir_main_bb0_i30_lir72:
  movq -192(%rbp), %rax
# cerune-origin: #33 bytes 293..334
cerune_origin_n33_main_73:
# cerune-mir: v1 main bb0 i30 -> lir 73 (source)
cerune_origin_mir_main_bb0_i30_lir73:
  movl $1, %r10d
  leaq .Lcerune_output_origin_14(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #40 bytes 368..373
cerune_origin_n40_main_74:
# cerune-mir: v1 main bb0 i31 -> lir 74 (source)
cerune_origin_mir_main_bb0_i31_lir74:
  movsd .Lcerune_f64_7(%rip), %xmm0
# cerune-origin: #40 bytes 368..373
cerune_origin_n40_main_75:
# cerune-mir: v1 main bb0 i31 -> lir 75 (source)
cerune_origin_mir_main_bb0_i31_lir75:
  movsd %xmm0, -200(%rbp)
# cerune-origin: #39 bytes 362..374
cerune_origin_n39_main_76:
# cerune-mir: v1 main bb0 i32 -> lir 76 (source)
cerune_origin_mir_main_bb0_i32_lir76:
  movsd -200(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #39 bytes 362..374
cerune_origin_n39_main_77:
# cerune-mir: v1 main bb0 i32 -> lir 77 (source)
cerune_origin_mir_main_bb0_i32_lir77:
  movsd %xmm0, -208(%rbp)
# cerune-origin: #38 bytes 341..375
cerune_origin_n38_main_78:
# cerune-mir: v1 main bb0 i33 -> lir 78 (source)
cerune_origin_mir_main_bb0_i33_lir78:
  movsd -208(%rbp), %xmm0
# cerune-origin: #38 bytes 341..375
cerune_origin_n38_main_79:
# cerune-mir: v1 main bb0 i33 -> lir 79 (source)
cerune_origin_mir_main_bb0_i33_lir79:
# policy: saturating_round
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_round_9_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_round_9_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_round_9_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_round_9_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_round_9_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $4602678819172646912, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_round_9_up
  movabsq $-4620693217682128896, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_round_9_down
  jmp .Lcerune_main_saturating_round_9_rounded
.Lcerune_main_saturating_round_9_up:
  addq $1, %rax
  jmp .Lcerune_main_saturating_round_9_rounded
.Lcerune_main_saturating_round_9_down:
  subq $1, %rax
.Lcerune_main_saturating_round_9_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_saturating_round_9_bounds:
# range policy after rounding, then convert
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_round_9_minimum
  movabsq $4643211215818981376, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_round_9_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_saturating_round_9_done
.Lcerune_main_saturating_round_9_nonfinite:
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_round_9_zero
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_round_9_maximum
  jmp .Lcerune_main_saturating_round_9_minimum
.Lcerune_main_saturating_round_9_zero:
  xorq %rax, %rax
  jmp .Lcerune_main_saturating_round_9_done
.Lcerune_main_saturating_round_9_minimum:
  movabsq $0, %rax
  jmp .Lcerune_main_saturating_round_9_done
.Lcerune_main_saturating_round_9_maximum:
  movabsq $255, %rax
.Lcerune_main_saturating_round_9_done:
# cerune-origin: #38 bytes 341..375
cerune_origin_n38_main_80:
# cerune-mir: v1 main bb0 i33 -> lir 80 (source)
cerune_origin_mir_main_bb0_i33_lir80:
  movq %rax, -216(%rbp)
# cerune-origin: #37 bytes 335..377
cerune_origin_n37_main_81:
# cerune-mir: v1 main bb0 i34 -> lir 81 (source)
cerune_origin_mir_main_bb0_i34_lir81:
  movq -216(%rbp), %rax
# cerune-origin: #37 bytes 335..377
cerune_origin_n37_main_82:
# cerune-mir: v1 main bb0 i34 -> lir 82 (source)
cerune_origin_mir_main_bb0_i34_lir82:
  movl $1, %r10d
  leaq .Lcerune_output_origin_15(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #44 bytes 421..426
cerune_origin_n44_main_83:
# cerune-mir: v1 main bb0 i35 -> lir 83 (source)
cerune_origin_mir_main_bb0_i35_lir83:
  movsd .Lcerune_f64_8(%rip), %xmm0
# cerune-origin: #44 bytes 421..426
cerune_origin_n44_main_84:
# cerune-mir: v1 main bb0 i35 -> lir 84 (source)
cerune_origin_mir_main_bb0_i35_lir84:
  movsd %xmm0, -224(%rbp)
# cerune-origin: #43 bytes 415..427
cerune_origin_n43_main_85:
# cerune-mir: v1 main bb0 i36 -> lir 85 (source)
cerune_origin_mir_main_bb0_i36_lir85:
  movsd -224(%rbp), %xmm0
  callq cerune_fn_value_0
# cerune-origin: #43 bytes 415..427
cerune_origin_n43_main_86:
# cerune-mir: v1 main bb0 i36 -> lir 86 (source)
cerune_origin_mir_main_bb0_i36_lir86:
  movsd %xmm0, -232(%rbp)
# cerune-origin: #42 bytes 384..428
cerune_origin_n42_main_87:
# cerune-mir: v1 main bb0 i37 -> lir 87 (source)
cerune_origin_mir_main_bb0_i37_lir87:
  movsd -232(%rbp), %xmm0
# cerune-origin: #42 bytes 384..428
cerune_origin_n42_main_88:
# cerune-mir: v1 main bb0 i37 -> lir 88 (source)
cerune_origin_mir_main_bb0_i37_lir88:
# policy: saturating_round_ties_even
  movapd %xmm0, %xmm2
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_round_ties_even_10_nonfinite
  movabsq $9218868437227405312, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_round_ties_even_10_nonfinite
  movabsq $-4503599627370496, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  je .Lcerune_main_saturating_round_ties_even_10_nonfinite
# round finite input; large values are already integral
  movabsq $4841369599423283200, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_round_ties_even_10_bounds
  movabsq $-4382002437431492608, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jbe .Lcerune_main_saturating_round_ties_even_10_bounds
  cvttsd2siq %xmm2, %rax
  cvtsi2sdq %rax, %xmm3
  subsd %xmm3, %xmm2
  movabsq $4602678819172646912, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_round_ties_even_10_up
  je .Lcerune_main_saturating_round_ties_even_10_tie
  movabsq $-4620693217682128896, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_round_ties_even_10_down
  jne .Lcerune_main_saturating_round_ties_even_10_rounded
.Lcerune_main_saturating_round_ties_even_10_tie:
  testq $1, %rax
  je .Lcerune_main_saturating_round_ties_even_10_rounded
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_round_ties_even_10_up
  jmp .Lcerune_main_saturating_round_ties_even_10_down
  jmp .Lcerune_main_saturating_round_ties_even_10_rounded
.Lcerune_main_saturating_round_ties_even_10_up:
  addq $1, %rax
  jmp .Lcerune_main_saturating_round_ties_even_10_rounded
.Lcerune_main_saturating_round_ties_even_10_down:
  subq $1, %rax
.Lcerune_main_saturating_round_ties_even_10_rounded:
  cvtsi2sdq %rax, %xmm2
.Lcerune_main_saturating_round_ties_even_10_bounds:
# range policy after rounding, then convert
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jb .Lcerune_main_saturating_round_ties_even_10_minimum
  movabsq $4643211215818981376, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  jae .Lcerune_main_saturating_round_ties_even_10_maximum
  cvttsd2siq %xmm2, %rax
  jmp .Lcerune_main_saturating_round_ties_even_10_done
.Lcerune_main_saturating_round_ties_even_10_nonfinite:
  ucomisd %xmm2, %xmm2
  jp .Lcerune_main_saturating_round_ties_even_10_zero
  movabsq $0, %r11
  movq %r11, %xmm1
  ucomisd %xmm1, %xmm2
  ja .Lcerune_main_saturating_round_ties_even_10_maximum
  jmp .Lcerune_main_saturating_round_ties_even_10_minimum
.Lcerune_main_saturating_round_ties_even_10_zero:
  xorq %rax, %rax
  jmp .Lcerune_main_saturating_round_ties_even_10_done
.Lcerune_main_saturating_round_ties_even_10_minimum:
  movabsq $0, %rax
  jmp .Lcerune_main_saturating_round_ties_even_10_done
.Lcerune_main_saturating_round_ties_even_10_maximum:
  movabsq $255, %rax
.Lcerune_main_saturating_round_ties_even_10_done:
# cerune-origin: #42 bytes 384..428
cerune_origin_n42_main_89:
# cerune-mir: v1 main bb0 i37 -> lir 89 (source)
cerune_origin_mir_main_bb0_i37_lir89:
  movq %rax, -240(%rbp)
# cerune-origin: #41 bytes 378..430
cerune_origin_n41_main_90:
# cerune-mir: v1 main bb0 i38 -> lir 90 (source)
cerune_origin_mir_main_bb0_i38_lir90:
  movq -240(%rbp), %rax
# cerune-origin: #41 bytes 378..430
cerune_origin_n41_main_91:
# cerune-mir: v1 main bb0 i38 -> lir 91 (source)
cerune_origin_mir_main_bb0_i38_lir91:
  movl $1, %r10d
  leaq .Lcerune_output_origin_16(%rip), %r11
  callq cerune_output_i64
# cerune-origin: #47 bytes 437..442
cerune_origin_n47_main_92:
# cerune-mir: v1 main bb0 i39 -> lir 92 (source)
cerune_origin_mir_main_bb0_i39_lir92:
  movabsq $3, %rax
# cerune-origin: #47 bytes 437..442
cerune_origin_n47_main_93:
# cerune-mir: v1 main bb0 i39 -> lir 93 (source)
cerune_origin_mir_main_bb0_i39_lir93:
  movq %rax, -248(%rbp)
# cerune-origin: #46 bytes 437..442
cerune_origin_n46_main_94:
# cerune-mir: v1 main bb0 i40 -> lir 94 (source)
cerune_origin_mir_main_bb0_i40_lir94:
  movq -248(%rbp), %rax
# cerune-origin: #46 bytes 437..442
cerune_origin_n46_main_95:
# cerune-mir: v1 main bb0 i40 -> lir 95 (source)
cerune_origin_mir_main_bb0_i40_lir95:
  movq %rax, -256(%rbp)
# cerune-origin: #45 bytes 431..444
cerune_origin_n45_main_96:
# cerune-mir: v1 main bb0 i41 -> lir 96 (source)
cerune_origin_mir_main_bb0_i41_lir96:
  movq -256(%rbp), %rax
# cerune-origin: #45 bytes 431..444
cerune_origin_n45_main_97:
# cerune-mir: v1 main bb0 i41 -> lir 97 (source)
cerune_origin_mir_main_bb0_i41_lir97:
  movl $1, %r10d
  leaq .Lcerune_output_origin_17(%rip), %r11
  callq cerune_output_i64
# cerune-origin: synthetic
# cerune-mir: v1 main bb0 i42 -> lir 98 (function-end)
cerune_origin_mir_main_bb0_i42_lir98:
  jmp .Lcerune_block_1
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
.Lcerune_block_1:
# main_exit
# cerune-origin: synthetic
  xorl %eax, %eax
  addq $256, %rsp
  popq %rbp
  retq
.section .rodata
.Lcerune_failure_0:
.asciz "cerune: runtime-v1 code=conversion-not-finite node=7 bytes=79..102\n"
.Lcerune_failure_1:
.asciz "cerune: runtime-v1 code=conversion-out-of-range node=7 bytes=79..102\n"
.Lcerune_output_origin_2:
.asciz " node=6 bytes=73..104\n"
.Lcerune_failure_3:
.asciz "cerune: runtime-v1 code=conversion-not-finite node=12 bytes=111..133\n"
.Lcerune_failure_4:
.asciz "cerune: runtime-v1 code=conversion-out-of-range node=12 bytes=111..133\n"
.Lcerune_output_origin_5:
.asciz " node=11 bytes=105..135\n"
.Lcerune_failure_6:
.asciz "cerune: runtime-v1 code=conversion-not-finite node=17 bytes=142..164\n"
.Lcerune_failure_7:
.asciz "cerune: runtime-v1 code=conversion-out-of-range node=17 bytes=142..164\n"
.Lcerune_output_origin_8:
.asciz " node=16 bytes=136..166\n"
.Lcerune_failure_9:
.asciz "cerune: runtime-v1 code=conversion-not-finite node=21 bytes=173..205\n"
.Lcerune_failure_10:
.asciz "cerune: runtime-v1 code=conversion-out-of-range node=21 bytes=173..205\n"
.Lcerune_output_origin_11:
.asciz " node=20 bytes=167..207\n"
.Lcerune_output_origin_12:
.asciz " node=24 bytes=208..250\n"
.Lcerune_output_origin_13:
.asciz " node=28 bytes=251..292\n"
.Lcerune_output_origin_14:
.asciz " node=33 bytes=293..334\n"
.Lcerune_output_origin_15:
.asciz " node=37 bytes=335..377\n"
.Lcerune_output_origin_16:
.asciz " node=41 bytes=378..430\n"
.Lcerune_output_origin_17:
.asciz " node=45 bytes=431..444\n"
.section .note.GNU-stack,"",@progbits
