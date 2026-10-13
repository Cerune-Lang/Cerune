# cerune-asm-origins v1: UTF-8 byte ranges, end exclusive
# cerune-origin: synthetic
# target: x86_64-unknown-linux-gnu
.section .rodata
.p2align 4
.Lcerune_sign_f32:
  .long 0x80000000
  .long 0
  .long 0
  .long 0
.p2align 4
.Lcerune_sign_f64:
  .quad 0x8000000000000000
  .quad 0
.p2align 3
.Lcerune_string_0:
  .quad 3
  .byte 230
  .byte 151
  .byte 165
.p2align 3
.Lcerune_string_1:
  .quad 3
  .byte 230
  .byte 156
  .byte 172
.p2align 3
.Lcerune_string_2:
  .quad 1
  .byte 33

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
  callq write
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
  callq __errno_location
  cmpl $4, (%rax)
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
  movq (%r10), %rcx
  movq 8(%r10), %r8
  movq 16(%r10), %r9
  xorl %eax, %eax
  callq snprintf
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
  callq write
.Loutput_exit:
  movl $1, %edi
  callq _Exit
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
  callq snprintf
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, (%rcx,%rax)
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
  callq snprintf
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, (%rcx,%rax)
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
  callq snprintf
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, (%rcx,%rax)
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
  callq snprintf
  cmpl $127, %eax
  jae .Loutput_format_failed
  leaq 160(%rsp), %rcx
  movb $10, (%rcx,%rax)
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
  movq (%rax), %rax
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
  cmpq (%rdx), %rax
  je .Loutput_begin
  movzbl 8(%rdx,%rax), %eax
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
  callq snprintf
  leaq 416(%rsp), %r10
  callq cerune_output_diagnostic
  ud2
.Loutput_begin:
  movq $0, 72(%rsp)
  movl $13, %edi
  movl $1, %esi
  callq signal
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
.Loutput_end:
  movl $13, %edi
  movq 88(%rsp), %rsi
  callq signal
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
  movb %al, (%rcx)
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

.p2align 4
cerune_string_equal:
  movq (%rdi), %rdx
  cmpq (%rsi), %rdx
  jne .Lstring_different
  xorq %r8, %r8
.Lstring_compare:
  cmpq %rdx, %r8
  je .Lstring_equal
  movzbl 8(%rdi,%r8), %eax
  cmpb 8(%rsi,%r8), %al
  jne .Lstring_different
  incq %r8
  jmp .Lstring_compare
.Lstring_equal:
  movl $1, %eax
  retq
.Lstring_different:
  xorl %eax, %eax
  retq


.data
.p2align 3
.Lcerune_heap_head:
  .quad 0
.Lcerune_heap_live:
  .quad 0
.Lcerune_heap_limit:
  .quad 67108864
.Lcerune_heap_max:
  .quad 9223372036854775783
.Lcerune_heap_empty:
  .quad 0
.text
cerune_string_retain:
  movq %rdi, %r10
  movq .Lcerune_heap_head(%rip), %rax
.Lcerune_heap_retain_search:
  testq %rax, %rax
  je .Lcerune_heap_retain_done
  leaq 16(%rax), %r11
  cmpq %r10, %r11
  je .Lcerune_heap_retain_found
  movq (%rax), %rax
  jmp .Lcerune_heap_retain_search
.Lcerune_heap_retain_found:
  addq $1, 8(%rax)
.Lcerune_heap_retain_done:
  retq
cerune_string_release:
  pushq %rbp
  movq %rsp, %rbp
  subq $32, %rsp
  movq %rdi, %r10
  leaq .Lcerune_heap_head(%rip), %r11
.Lcerune_heap_release_search:
  movq (%r11), %rax
  testq %rax, %rax
  je .Lcerune_heap_release_done
  leaq 16(%rax), %rcx
  cmpq %r10, %rcx
  je .Lcerune_heap_release_found
  movq %rax, %r11
  jmp .Lcerune_heap_release_search
.Lcerune_heap_release_found:
  subq $1, 8(%rax)
  jne .Lcerune_heap_release_done
  movq (%rax), %rcx
  movq %rcx, (%r11)
  movq 16(%rax), %rcx
  subq %rcx, .Lcerune_heap_live(%rip)
  movq %rax, %rdi
  callq free
.Lcerune_heap_release_done:
  addq $32, %rsp
  popq %rbp
  retq
cerune_string_concat:
  pushq %rbp
  movq %rsp, %rbp
  subq $80, %rsp
  movq %rdi, -8(%rbp)
  movq %rsi, -16(%rbp)
  movq (%rdi), %rax
  movq %rax, -32(%rbp)
  addq (%rsi), %rax
  jc .Lcerune_heap_size_fail
  cmpq .Lcerune_heap_max(%rip), %rax
  ja .Lcerune_heap_size_fail
  testq %rax, %rax
  je .Lcerune_heap_concat_empty
  movq %rax, -24(%rbp)
  movq .Lcerune_heap_limit(%rip), %rcx
  subq .Lcerune_heap_live(%rip), %rcx
  cmpq %rcx, %rax
  ja .Lcerune_heap_limit_fail
  leaq 24(%rax), %rdi
  callq malloc
  testq %rax, %rax
  je .Lcerune_heap_allocation_fail
  movq %rax, -40(%rbp)
  movq .Lcerune_heap_head(%rip), %rcx
  movq %rcx, (%rax)
  movq $1, 8(%rax)
  movq -24(%rbp), %rcx
  movq %rcx, 16(%rax)
  leaq 24(%rax), %rdi
  movq -8(%rbp), %rsi
  addq $8, %rsi
  movq -32(%rbp), %rdx
  callq memcpy
  movq -40(%rbp), %rdi
  addq $24, %rdi
  addq -32(%rbp), %rdi
  movq -16(%rbp), %rsi
  movq (%rsi), %rdx
  addq $8, %rsi
  callq memcpy
  movq -40(%rbp), %rax
  movq %rax, .Lcerune_heap_head(%rip)
  movq -24(%rbp), %rcx
  addq %rcx, .Lcerune_heap_live(%rip)
  addq $16, %rax
  xorl %edx, %edx
  jmp .Lcerune_heap_concat_done
.Lcerune_heap_concat_empty:
  leaq .Lcerune_heap_empty(%rip), %rax
  xorl %edx, %edx
  jmp .Lcerune_heap_concat_done
.Lcerune_heap_size_fail:
  movl $1, %edx
  jmp .Lcerune_heap_concat_done
.Lcerune_heap_limit_fail:
  movl $2, %edx
  jmp .Lcerune_heap_concat_done
.Lcerune_heap_allocation_fail:
  movl $3, %edx
.Lcerune_heap_concat_done:
  addq $80, %rsp
  popq %rbp
  retq
# cerune-origin: synthetic
.p2align 4
cerune_fn__ownership0_0:
  pushq %rbp
  movq %rsp, %rbp
  subq $80, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_fn_0_block_0
# cerune-origin: synthetic
# cerune-mir: v1 fn_0 bb0 block -> lir 1 (function-entry)
cerune_origin_mir_fn_0_bb0_block_lir1:
.Lcerune_fn_0_block_0: # mir_block
# cerune-origin: #2 bytes 26..31
cerune_origin_n2_fn_0_2:
# cerune-mir: v1 fn_0 bb0 i0 -> lir 2 (source)
cerune_origin_mir_fn_0_bb0_i0_lir2:
  leaq .Lcerune_string_0(%rip), %rax
# cerune-origin: #2 bytes 26..31
cerune_origin_n2_fn_0_3:
# cerune-mir: v1 fn_0 bb0 i0 -> lir 3 (source)
cerune_origin_mir_fn_0_bb0_i0_lir3:
  movq %rax, -8(%rbp)
# cerune-origin: #14 bytes 26..31
cerune_origin_n14_fn_0_4:
# cerune-mir: v1 fn_0 bb0 i1 -> lir 4 (source)
cerune_origin_mir_fn_0_bb0_i1_lir4:
  movq -8(%rbp), %rax
# cerune-origin: #14 bytes 26..31
cerune_origin_n14_fn_0_5:
# cerune-mir: v1 fn_0 bb0 i1 -> lir 5 (source)
cerune_origin_mir_fn_0_bb0_i1_lir5:
  movq %rax, -16(%rbp)
# cerune-origin: #3 bytes 33..38
cerune_origin_n3_fn_0_6:
# cerune-mir: v1 fn_0 bb0 i2 -> lir 6 (source)
cerune_origin_mir_fn_0_bb0_i2_lir6:
  leaq .Lcerune_string_1(%rip), %rax
# cerune-origin: #3 bytes 33..38
cerune_origin_n3_fn_0_7:
# cerune-mir: v1 fn_0 bb0 i2 -> lir 7 (source)
cerune_origin_mir_fn_0_bb0_i2_lir7:
  movq %rax, -24(%rbp)
# cerune-origin: #16 bytes 33..38
cerune_origin_n16_fn_0_8:
# cerune-mir: v1 fn_0 bb0 i3 -> lir 8 (source)
cerune_origin_mir_fn_0_bb0_i3_lir8:
  movq -24(%rbp), %rax
# cerune-origin: #16 bytes 33..38
cerune_origin_n16_fn_0_9:
# cerune-mir: v1 fn_0 bb0 i3 -> lir 9 (source)
cerune_origin_mir_fn_0_bb0_i3_lir9:
  movq %rax, -32(%rbp)
# cerune-origin: #15 bytes 26..31
cerune_origin_n15_fn_0_10:
# cerune-mir: v1 fn_0 bb0 i4 -> lir 10 (source)
cerune_origin_mir_fn_0_bb0_i4_lir10:
  movq -16(%rbp), %rax
# cerune-origin: #15 bytes 26..31
cerune_origin_n15_fn_0_11:
# cerune-mir: v1 fn_0 bb0 i4 -> lir 11 (source)
cerune_origin_mir_fn_0_bb0_i4_lir11:
  movq %rax, -40(%rbp)
# cerune-origin: #17 bytes 33..38
cerune_origin_n17_fn_0_12:
# cerune-mir: v1 fn_0 bb0 i5 -> lir 12 (source)
cerune_origin_mir_fn_0_bb0_i5_lir12:
  movq -32(%rbp), %rax
# cerune-origin: #17 bytes 33..38
cerune_origin_n17_fn_0_13:
# cerune-mir: v1 fn_0 bb0 i5 -> lir 13 (source)
cerune_origin_mir_fn_0_bb0_i5_lir13:
  movq %rax, -48(%rbp)
# cerune-origin: #1 bytes 19..39
cerune_origin_n1_fn_0_14:
# cerune-mir: v1 fn_0 bb0 i6 -> lir 14 (source)
cerune_origin_mir_fn_0_bb0_i6_lir14:
  movq -48(%rbp), %rax
# cerune-origin: #1 bytes 19..39
cerune_origin_n1_fn_0_15:
# cerune-mir: v1 fn_0 bb0 i6 -> lir 15 (source)
cerune_origin_mir_fn_0_bb0_i6_lir15:
  movq %rax, %rsi
  movq -40(%rbp), %rdi
  callq cerune_string_concat
  testq %rdx, %rdx
  je .Lfn_0_concat_ok_3
  cmpq $1, %rdx
  jne .Lfn_0_concat_limit_3
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_0(%rip), %rsi
  movl $69, %edx
  callq write
  ud2
.Lfn_0_concat_limit_3:
  cmpq $2, %rdx
  jne .Lfn_0_concat_failed_3
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_1(%rip), %rsi
  movl $70, %edx
  callq write
  ud2
.Lfn_0_concat_failed_3:
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_2(%rip), %rsi
  movl $62, %edx
  callq write
  ud2
.Lfn_0_concat_ok_3:
# cerune-origin: #1 bytes 19..39
cerune_origin_n1_fn_0_16:
# cerune-mir: v1 fn_0 bb0 i6 -> lir 16 (source)
cerune_origin_mir_fn_0_bb0_i6_lir16:
  movq %rax, -56(%rbp)
# cerune-origin: #18 bytes 19..39
cerune_origin_n18_fn_0_17:
# cerune-mir: v1 fn_0 bb0 i7 -> lir 17 (source)
cerune_origin_mir_fn_0_bb0_i7_lir17:
  movq -56(%rbp), %rax
# cerune-origin: #18 bytes 19..39
cerune_origin_n18_fn_0_18:
# cerune-mir: v1 fn_0 bb0 i7 -> lir 18 (source)
cerune_origin_mir_fn_0_bb0_i7_lir18:
  movq %rax, -64(%rbp)
# cerune-origin: #19 bytes 19..39
cerune_origin_n19_fn_0_19:
# cerune-mir: v1 fn_0 bb0 i8 -> lir 19 (source)
cerune_origin_mir_fn_0_bb0_i8_lir19:
  movq -64(%rbp), %rax
# cerune-origin: #19 bytes 19..39
cerune_origin_n19_fn_0_20:
# cerune-mir: v1 fn_0 bb0 i8 -> lir 20 (source)
cerune_origin_mir_fn_0_bb0_i8_lir20:
  movq %rax, -72(%rbp)
# cerune-origin: #20 bytes 19..39
cerune_origin_n20_fn_0_21:
# cerune-mir: v1 fn_0 bb0 i9 -> lir 21 (source)
cerune_origin_mir_fn_0_bb0_i9_lir21:
  movq -72(%rbp), %rax
# cerune-origin: #20 bytes 19..39
cerune_origin_n20_fn_0_22:
# cerune-mir: v1 fn_0 bb0 i9 -> lir 22 (source)
cerune_origin_mir_fn_0_bb0_i9_lir22:
  addq $80, %rsp
  popq %rbp
  retq
# cerune-origin: #20 bytes 19..39
cerune_origin_n20_fn_0_23:
# cerune-mir: v1 fn_0 bb1 block -> lir 23 (after-return)
cerune_origin_mir_fn_0_bb1_block_lir23:
.Lcerune_fn_0_block_1: # mir_block
# cerune-origin: synthetic
# cerune-mir: v1 fn_0 bb1 i10 -> lir 24 (function-end)
cerune_origin_mir_fn_0_bb1_i10_lir24:
  ud2

# cerune-origin: synthetic
.p2align 4
cerune_fn__ownership1_1:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  movq %rdi, -8(%rbp)
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_fn_1_block_0
# cerune-origin: synthetic
# cerune-mir: v1 fn_1 bb0 block -> lir 2 (function-entry)
cerune_origin_mir_fn_1_bb0_block_lir2:
.Lcerune_fn_1_block_0: # mir_block
# cerune-origin: #5 bytes 57..61
cerune_origin_n5_fn_1_3:
# cerune-mir: v1 fn_1 bb0 i0 -> lir 3 (source)
cerune_origin_mir_fn_1_bb0_i0_lir3:
  movq -8(%rbp), %rax
# cerune-origin: #5 bytes 57..61
cerune_origin_n5_fn_1_4:
# cerune-mir: v1 fn_1 bb0 i0 -> lir 4 (source)
cerune_origin_mir_fn_1_bb0_i0_lir4:
  movq %rax, -16(%rbp)
# cerune-origin: #22 bytes 57..61
cerune_origin_n22_fn_1_5:
# cerune-mir: v1 fn_1 bb0 i1 -> lir 5 (source)
cerune_origin_mir_fn_1_bb0_i1_lir5:
  movq -16(%rbp), %rax
# cerune-origin: #22 bytes 57..61
cerune_origin_n22_fn_1_6:
# cerune-mir: v1 fn_1 bb0 i1 -> lir 6 (source)
cerune_origin_mir_fn_1_bb0_i1_lir6:
  movq %rax, -24(%rbp)
# cerune-origin: #23 bytes 57..61
cerune_origin_n23_fn_1_7:
# cerune-mir: v1 fn_1 bb0 i2 -> lir 7 (source)
cerune_origin_mir_fn_1_bb0_i2_lir7:
  movq -24(%rbp), %rax
# cerune-origin: #23 bytes 57..61
cerune_origin_n23_fn_1_8:
# cerune-mir: v1 fn_1 bb0 i2 -> lir 8 (source)
cerune_origin_mir_fn_1_bb0_i2_lir8:
  movq %rax, -32(%rbp)
# cerune-origin: #24 bytes 57..61
cerune_origin_n24_fn_1_9:
# cerune-mir: v1 fn_1 bb0 i3 -> lir 9 (source)
cerune_origin_mir_fn_1_bb0_i3_lir9:
  movq -32(%rbp), %rax
# cerune-origin: #24 bytes 57..61
cerune_origin_n24_fn_1_10:
# cerune-mir: v1 fn_1 bb0 i3 -> lir 10 (source)
cerune_origin_mir_fn_1_bb0_i3_lir10:
  movq %rax, -40(%rbp)
# cerune-origin: #25 bytes 57..61
cerune_origin_n25_fn_1_11:
# cerune-mir: v1 fn_1 bb0 i4 -> lir 11 (source)
cerune_origin_mir_fn_1_bb0_i4_lir11:
  movq -40(%rbp), %rax
# cerune-origin: #25 bytes 57..61
cerune_origin_n25_fn_1_12:
# cerune-mir: v1 fn_1 bb0 i4 -> lir 12 (source)
cerune_origin_mir_fn_1_bb0_i4_lir12:
  movq %rax, -48(%rbp)
# cerune-origin: #26 bytes 57..61
cerune_origin_n26_fn_1_13:
# cerune-mir: v1 fn_1 bb0 i5 -> lir 13 (source)
cerune_origin_mir_fn_1_bb0_i5_lir13:
  movq -48(%rbp), %rax
# cerune-origin: #26 bytes 57..61
cerune_origin_n26_fn_1_14:
# cerune-mir: v1 fn_1 bb0 i5 -> lir 14 (source)
cerune_origin_mir_fn_1_bb0_i5_lir14:
  movq %rax, %rdi
  callq cerune_string_retain
# cerune-origin: #27 bytes 57..61
cerune_origin_n27_fn_1_15:
# cerune-mir: v1 fn_1 bb0 i6 -> lir 15 (source)
cerune_origin_mir_fn_1_bb0_i6_lir15:
  movq -40(%rbp), %rax
# cerune-origin: #27 bytes 57..61
cerune_origin_n27_fn_1_16:
# cerune-mir: v1 fn_1 bb0 i6 -> lir 16 (source)
cerune_origin_mir_fn_1_bb0_i6_lir16:
  movq %rax, -56(%rbp)
# cerune-origin: #28 bytes 57..61
cerune_origin_n28_fn_1_17:
# cerune-mir: v1 fn_1 bb0 i7 -> lir 17 (source)
cerune_origin_mir_fn_1_bb0_i7_lir17:
  movq -56(%rbp), %rax
# cerune-origin: #28 bytes 57..61
cerune_origin_n28_fn_1_18:
# cerune-mir: v1 fn_1 bb0 i7 -> lir 18 (source)
cerune_origin_mir_fn_1_bb0_i7_lir18:
  addq $64, %rsp
  popq %rbp
  retq
# cerune-origin: #28 bytes 57..61
cerune_origin_n28_fn_1_19:
# cerune-mir: v1 fn_1 bb1 block -> lir 19 (after-return)
cerune_origin_mir_fn_1_bb1_block_lir19:
.Lcerune_fn_1_block_1: # mir_block
# cerune-origin: synthetic
# cerune-mir: v1 fn_1 bb1 i8 -> lir 20 (function-end)
cerune_origin_mir_fn_1_bb1_i8_lir20:
  ud2

# cerune-origin: synthetic
.p2align 4
cerune_fn__ownership2_2:
  pushq %rbp
  movq %rsp, %rbp
  subq $80, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  movq %rdi, -8(%rbp)
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_fn_2_block_0
# cerune-origin: synthetic
# cerune-mir: v1 fn_2 bb0 block -> lir 2 (function-entry)
cerune_origin_mir_fn_2_bb0_block_lir2:
.Lcerune_fn_2_block_0: # mir_block
# cerune-origin: #8 bytes 77..81
cerune_origin_n8_fn_2_3:
# cerune-mir: v1 fn_2 bb0 i0 -> lir 3 (source)
cerune_origin_mir_fn_2_bb0_i0_lir3:
  movq -8(%rbp), %rax
# cerune-origin: #8 bytes 77..81
cerune_origin_n8_fn_2_4:
# cerune-mir: v1 fn_2 bb0 i0 -> lir 4 (source)
cerune_origin_mir_fn_2_bb0_i0_lir4:
  movq %rax, -16(%rbp)
# cerune-origin: #32 bytes 77..81
cerune_origin_n32_fn_2_5:
# cerune-mir: v1 fn_2 bb0 i1 -> lir 5 (source)
cerune_origin_mir_fn_2_bb0_i1_lir5:
  movq -16(%rbp), %rax
# cerune-origin: #32 bytes 77..81
cerune_origin_n32_fn_2_6:
# cerune-mir: v1 fn_2 bb0 i1 -> lir 6 (source)
cerune_origin_mir_fn_2_bb0_i1_lir6:
  movq %rax, -24(%rbp)
# cerune-origin: #9 bytes 83..86
cerune_origin_n9_fn_2_7:
# cerune-mir: v1 fn_2 bb0 i2 -> lir 7 (source)
cerune_origin_mir_fn_2_bb0_i2_lir7:
  leaq .Lcerune_string_2(%rip), %rax
# cerune-origin: #9 bytes 83..86
cerune_origin_n9_fn_2_8:
# cerune-mir: v1 fn_2 bb0 i2 -> lir 8 (source)
cerune_origin_mir_fn_2_bb0_i2_lir8:
  movq %rax, -32(%rbp)
# cerune-origin: #34 bytes 83..86
cerune_origin_n34_fn_2_9:
# cerune-mir: v1 fn_2 bb0 i3 -> lir 9 (source)
cerune_origin_mir_fn_2_bb0_i3_lir9:
  movq -32(%rbp), %rax
# cerune-origin: #34 bytes 83..86
cerune_origin_n34_fn_2_10:
# cerune-mir: v1 fn_2 bb0 i3 -> lir 10 (source)
cerune_origin_mir_fn_2_bb0_i3_lir10:
  movq %rax, -40(%rbp)
# cerune-origin: #33 bytes 77..81
cerune_origin_n33_fn_2_11:
# cerune-mir: v1 fn_2 bb0 i4 -> lir 11 (source)
cerune_origin_mir_fn_2_bb0_i4_lir11:
  movq -24(%rbp), %rax
# cerune-origin: #33 bytes 77..81
cerune_origin_n33_fn_2_12:
# cerune-mir: v1 fn_2 bb0 i4 -> lir 12 (source)
cerune_origin_mir_fn_2_bb0_i4_lir12:
  movq %rax, -48(%rbp)
# cerune-origin: #35 bytes 83..86
cerune_origin_n35_fn_2_13:
# cerune-mir: v1 fn_2 bb0 i5 -> lir 13 (source)
cerune_origin_mir_fn_2_bb0_i5_lir13:
  movq -40(%rbp), %rax
# cerune-origin: #35 bytes 83..86
cerune_origin_n35_fn_2_14:
# cerune-mir: v1 fn_2 bb0 i5 -> lir 14 (source)
cerune_origin_mir_fn_2_bb0_i5_lir14:
  movq %rax, -56(%rbp)
# cerune-origin: #7 bytes 70..87
cerune_origin_n7_fn_2_15:
# cerune-mir: v1 fn_2 bb0 i6 -> lir 15 (source)
cerune_origin_mir_fn_2_bb0_i6_lir15:
  movq -56(%rbp), %rax
# cerune-origin: #7 bytes 70..87
cerune_origin_n7_fn_2_16:
# cerune-mir: v1 fn_2 bb0 i6 -> lir 16 (source)
cerune_origin_mir_fn_2_bb0_i6_lir16:
  movq %rax, %rsi
  movq -48(%rbp), %rdi
  callq cerune_string_concat
  testq %rdx, %rdx
  je .Lfn_2_concat_ok_3
  cmpq $1, %rdx
  jne .Lfn_2_concat_limit_3
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_3(%rip), %rsi
  movl $69, %edx
  callq write
  ud2
.Lfn_2_concat_limit_3:
  cmpq $2, %rdx
  jne .Lfn_2_concat_failed_3
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_4(%rip), %rsi
  movl $70, %edx
  callq write
  ud2
.Lfn_2_concat_failed_3:
  xorl %edi, %edi
  callq fflush
  movl $2, %edi
  leaq .Lcerune_failure_5(%rip), %rsi
  movl $62, %edx
  callq write
  ud2
.Lfn_2_concat_ok_3:
# cerune-origin: #7 bytes 70..87
cerune_origin_n7_fn_2_17:
# cerune-mir: v1 fn_2 bb0 i6 -> lir 17 (source)
cerune_origin_mir_fn_2_bb0_i6_lir17:
  movq %rax, -64(%rbp)
# cerune-origin: #36 bytes 70..87
cerune_origin_n36_fn_2_18:
# cerune-mir: v1 fn_2 bb0 i7 -> lir 18 (source)
cerune_origin_mir_fn_2_bb0_i7_lir18:
  movq -64(%rbp), %rax
# cerune-origin: #36 bytes 70..87
cerune_origin_n36_fn_2_19:
# cerune-mir: v1 fn_2 bb0 i7 -> lir 19 (source)
cerune_origin_mir_fn_2_bb0_i7_lir19:
  movq %rax, -72(%rbp)
# cerune-origin: #37 bytes 70..87
cerune_origin_n37_fn_2_20:
# cerune-mir: v1 fn_2 bb0 i8 -> lir 20 (source)
cerune_origin_mir_fn_2_bb0_i8_lir20:
  movq -72(%rbp), %rax
# cerune-origin: #37 bytes 70..87
cerune_origin_n37_fn_2_21:
# cerune-mir: v1 fn_2 bb0 i8 -> lir 21 (source)
cerune_origin_mir_fn_2_bb0_i8_lir21:
  movq %rax, -80(%rbp)
# cerune-origin: #38 bytes 70..87
cerune_origin_n38_fn_2_22:
# cerune-mir: v1 fn_2 bb0 i9 -> lir 22 (source)
cerune_origin_mir_fn_2_bb0_i9_lir22:
  movq -80(%rbp), %rax
# cerune-origin: #38 bytes 70..87
cerune_origin_n38_fn_2_23:
# cerune-mir: v1 fn_2 bb0 i9 -> lir 23 (source)
cerune_origin_mir_fn_2_bb0_i9_lir23:
  addq $80, %rsp
  popq %rbp
  retq
# cerune-origin: #38 bytes 70..87
cerune_origin_n38_fn_2_24:
# cerune-mir: v1 fn_2 bb1 block -> lir 24 (after-return)
cerune_origin_mir_fn_2_bb1_block_lir24:
.Lcerune_fn_2_block_1: # mir_block
# cerune-origin: synthetic
# cerune-mir: v1 fn_2 bb1 i10 -> lir 25 (function-end)
cerune_origin_mir_fn_2_bb1_i10_lir25:
  ud2

# cerune-origin: synthetic
.p2align 4
cerune_fn__ownership3_3:
  pushq %rbp
  movq %rsp, %rbp
  subq $32, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  movq %rdi, -8(%rbp)
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  movq %rsi, -16(%rbp)
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_fn_3_block_0
# cerune-origin: synthetic
# cerune-mir: v1 fn_3 bb0 block -> lir 3 (function-entry)
cerune_origin_mir_fn_3_bb0_block_lir3:
.Lcerune_fn_3_block_0: # mir_block
# cerune-origin: #41 bytes 70..87
cerune_origin_n41_fn_3_4:
# cerune-mir: v1 fn_3 bb0 i0 -> lir 4 (source)
cerune_origin_mir_fn_3_bb0_i0_lir4:
  movq -8(%rbp), %rax
# cerune-origin: #41 bytes 70..87
cerune_origin_n41_fn_3_5:
# cerune-mir: v1 fn_3 bb0 i0 -> lir 5 (source)
cerune_origin_mir_fn_3_bb0_i0_lir5:
  movq %rax, -24(%rbp)
# cerune-origin: #42 bytes 70..87
cerune_origin_n42_fn_3_6:
# cerune-mir: v1 fn_3 bb0 i1 -> lir 6 (source)
cerune_origin_mir_fn_3_bb0_i1_lir6:
  movq -24(%rbp), %rax
# cerune-origin: #42 bytes 70..87
cerune_origin_n42_fn_3_7:
# cerune-mir: v1 fn_3 bb0 i1 -> lir 7 (source)
cerune_origin_mir_fn_3_bb0_i1_lir7:
  movq %rax, %rdi
  callq cerune_string_release
# cerune-origin: #43 bytes 70..87
cerune_origin_n43_fn_3_8:
# cerune-mir: v1 fn_3 bb0 i2 -> lir 8 (source)
cerune_origin_mir_fn_3_bb0_i2_lir8:
  movq -16(%rbp), %rax
# cerune-origin: #43 bytes 70..87
cerune_origin_n43_fn_3_9:
# cerune-mir: v1 fn_3 bb0 i2 -> lir 9 (source)
cerune_origin_mir_fn_3_bb0_i2_lir9:
  movq %rax, -32(%rbp)
# cerune-origin: #44 bytes 70..87
cerune_origin_n44_fn_3_10:
# cerune-mir: v1 fn_3 bb0 i3 -> lir 10 (source)
cerune_origin_mir_fn_3_bb0_i3_lir10:
  movq -32(%rbp), %rax
# cerune-origin: #44 bytes 70..87
cerune_origin_n44_fn_3_11:
# cerune-mir: v1 fn_3 bb0 i3 -> lir 11 (source)
cerune_origin_mir_fn_3_bb0_i3_lir11:
  addq $32, %rsp
  popq %rbp
  retq
# cerune-origin: #44 bytes 70..87
cerune_origin_n44_fn_3_12:
# cerune-mir: v1 fn_3 bb1 block -> lir 12 (after-return)
cerune_origin_mir_fn_3_bb1_block_lir12:
.Lcerune_fn_3_block_1: # mir_block
# cerune-origin: synthetic
# cerune-mir: v1 fn_3 bb1 i4 -> lir 13 (function-end)
cerune_origin_mir_fn_3_bb1_i4_lir13:
  ud2

# cerune-origin: synthetic
.globl main
.p2align 4
main:
  pushq %rbp
  movq %rsp, %rbp
  subq $144, %rsp
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
  jmp .Lcerune_block_0
# cerune-origin: synthetic
# cerune-mir: v1 main bb0 block -> lir 1 (function-entry)
cerune_origin_mir_main_bb0_block_lir1:
.Lcerune_block_0: # mir_block
# cerune-origin: #21 bytes 19..39
cerune_origin_n21_main_2:
# cerune-mir: v1 main bb0 i0 -> lir 2 (source)
cerune_origin_mir_main_bb0_i0_lir2:
  callq cerune_fn__ownership0_0
# cerune-origin: #21 bytes 19..39
cerune_origin_n21_main_3:
# cerune-mir: v1 main bb0 i0 -> lir 3 (source)
cerune_origin_mir_main_bb0_i0_lir3:
  movq %rax, -8(%rbp)
# cerune-origin: #0 bytes 0..40
cerune_origin_n0_main_4:
# cerune-mir: v1 main bb0 i1 -> lir 4 (source)
cerune_origin_mir_main_bb0_i1_lir4:
  movq -8(%rbp), %rax
# cerune-origin: #0 bytes 0..40
cerune_origin_n0_main_5:
# cerune-mir: v1 main bb0 i1 -> lir 5 (source)
cerune_origin_mir_main_bb0_i1_lir5:
  movq %rax, -16(%rbp)
# cerune-origin: #29 bytes 57..61
cerune_origin_n29_main_6:
# cerune-mir: v1 main bb0 i2 -> lir 6 (source)
cerune_origin_mir_main_bb0_i2_lir6:
  movq -16(%rbp), %rax
# cerune-origin: #29 bytes 57..61
cerune_origin_n29_main_7:
# cerune-mir: v1 main bb0 i2 -> lir 7 (source)
cerune_origin_mir_main_bb0_i2_lir7:
  movq %rax, -24(%rbp)
# cerune-origin: #30 bytes 57..61
cerune_origin_n30_main_8:
# cerune-mir: v1 main bb0 i3 -> lir 8 (source)
cerune_origin_mir_main_bb0_i3_lir8:
  movq -24(%rbp), %rdi
  callq cerune_fn__ownership1_1
# cerune-origin: #30 bytes 57..61
cerune_origin_n30_main_9:
# cerune-mir: v1 main bb0 i3 -> lir 9 (source)
cerune_origin_mir_main_bb0_i3_lir9:
  movq %rax, -32(%rbp)
# cerune-origin: #4 bytes 41..62
cerune_origin_n4_main_10:
# cerune-mir: v1 main bb0 i4 -> lir 10 (source)
cerune_origin_mir_main_bb0_i4_lir10:
  movq -32(%rbp), %rax
# cerune-origin: #4 bytes 41..62
cerune_origin_n4_main_11:
# cerune-mir: v1 main bb0 i4 -> lir 11 (source)
cerune_origin_mir_main_bb0_i4_lir11:
  movq %rax, -40(%rbp)
# cerune-origin: #31 bytes 63..88
cerune_origin_n31_main_12:
# cerune-mir: v1 main bb0 i5 -> lir 12 (source)
cerune_origin_mir_main_bb0_i5_lir12:
  movq -16(%rbp), %rax
# cerune-origin: #31 bytes 63..88
cerune_origin_n31_main_13:
# cerune-mir: v1 main bb0 i5 -> lir 13 (source)
cerune_origin_mir_main_bb0_i5_lir13:
  movq %rax, -48(%rbp)
# cerune-origin: #39 bytes 77..81
cerune_origin_n39_main_14:
# cerune-mir: v1 main bb0 i6 -> lir 14 (source)
cerune_origin_mir_main_bb0_i6_lir14:
  movq -16(%rbp), %rax
# cerune-origin: #39 bytes 77..81
cerune_origin_n39_main_15:
# cerune-mir: v1 main bb0 i6 -> lir 15 (source)
cerune_origin_mir_main_bb0_i6_lir15:
  movq %rax, -56(%rbp)
# cerune-origin: #40 bytes 70..87
cerune_origin_n40_main_16:
# cerune-mir: v1 main bb0 i7 -> lir 16 (source)
cerune_origin_mir_main_bb0_i7_lir16:
  movq -56(%rbp), %rdi
  callq cerune_fn__ownership2_2
# cerune-origin: #40 bytes 70..87
cerune_origin_n40_main_17:
# cerune-mir: v1 main bb0 i7 -> lir 17 (source)
cerune_origin_mir_main_bb0_i7_lir17:
  movq %rax, -64(%rbp)
# cerune-origin: #45 bytes 70..87
cerune_origin_n45_main_18:
# cerune-mir: v1 main bb0 i8 -> lir 18 (source)
cerune_origin_mir_main_bb0_i8_lir18:
  movq -48(%rbp), %rdi
  movq -64(%rbp), %rsi
  callq cerune_fn__ownership3_3
# cerune-origin: #45 bytes 70..87
cerune_origin_n45_main_19:
# cerune-mir: v1 main bb0 i8 -> lir 19 (source)
cerune_origin_mir_main_bb0_i8_lir19:
  movq %rax, -72(%rbp)
# cerune-origin: #6 bytes 63..88
cerune_origin_n6_main_20:
# cerune-mir: v1 main bb0 i9 -> lir 20 (source)
cerune_origin_mir_main_bb0_i9_lir20:
  movq -72(%rbp), %rax
# cerune-origin: #6 bytes 63..88
cerune_origin_n6_main_21:
# cerune-mir: v1 main bb0 i9 -> lir 21 (source)
cerune_origin_mir_main_bb0_i9_lir21:
  movq %rax, -16(%rbp)
# cerune-origin: #11 bytes 95..100
cerune_origin_n11_main_22:
# cerune-mir: v1 main bb0 i10 -> lir 22 (source)
cerune_origin_mir_main_bb0_i10_lir22:
  movq -40(%rbp), %rax
# cerune-origin: #11 bytes 95..100
cerune_origin_n11_main_23:
# cerune-mir: v1 main bb0 i10 -> lir 23 (source)
cerune_origin_mir_main_bb0_i10_lir23:
  movq %rax, -80(%rbp)
# cerune-origin: #46 bytes 95..100
cerune_origin_n46_main_24:
# cerune-mir: v1 main bb0 i11 -> lir 24 (source)
cerune_origin_mir_main_bb0_i11_lir24:
  movq -80(%rbp), %rax
# cerune-origin: #46 bytes 95..100
cerune_origin_n46_main_25:
# cerune-mir: v1 main bb0 i11 -> lir 25 (source)
cerune_origin_mir_main_bb0_i11_lir25:
  movq %rax, -88(%rbp)
# cerune-origin: #48 bytes 95..100
cerune_origin_n48_main_26:
# cerune-mir: v1 main bb0 i12 -> lir 26 (source)
cerune_origin_mir_main_bb0_i12_lir26:
  movq -88(%rbp), %rax
# cerune-origin: #48 bytes 95..100
cerune_origin_n48_main_27:
# cerune-mir: v1 main bb0 i12 -> lir 27 (source)
cerune_origin_mir_main_bb0_i12_lir27:
  movq %rax, -96(%rbp)
# cerune-origin: #10 bytes 89..102
cerune_origin_n10_main_28:
# cerune-mir: v1 main bb0 i13 -> lir 28 (source)
cerune_origin_mir_main_bb0_i13_lir28:
  movq -96(%rbp), %rax
# cerune-origin: #10 bytes 89..102
cerune_origin_n10_main_29:
# cerune-mir: v1 main bb0 i13 -> lir 29 (source)
cerune_origin_mir_main_bb0_i13_lir29:
  movl $1, %r10d
  leaq .Lcerune_output_origin_6(%rip), %r11
  callq cerune_output_string
# cerune-origin: #13 bytes 109..113
cerune_origin_n13_main_30:
# cerune-mir: v1 main bb0 i14 -> lir 30 (source)
cerune_origin_mir_main_bb0_i14_lir30:
  movq -16(%rbp), %rax
# cerune-origin: #13 bytes 109..113
cerune_origin_n13_main_31:
# cerune-mir: v1 main bb0 i14 -> lir 31 (source)
cerune_origin_mir_main_bb0_i14_lir31:
  movq %rax, -104(%rbp)
# cerune-origin: #49 bytes 109..113
cerune_origin_n49_main_32:
# cerune-mir: v1 main bb0 i15 -> lir 32 (source)
cerune_origin_mir_main_bb0_i15_lir32:
  movq -104(%rbp), %rax
# cerune-origin: #49 bytes 109..113
cerune_origin_n49_main_33:
# cerune-mir: v1 main bb0 i15 -> lir 33 (source)
cerune_origin_mir_main_bb0_i15_lir33:
  movq %rax, -112(%rbp)
# cerune-origin: #51 bytes 109..113
cerune_origin_n51_main_34:
# cerune-mir: v1 main bb0 i16 -> lir 34 (source)
cerune_origin_mir_main_bb0_i16_lir34:
  movq -112(%rbp), %rax
# cerune-origin: #51 bytes 109..113
cerune_origin_n51_main_35:
# cerune-mir: v1 main bb0 i16 -> lir 35 (source)
cerune_origin_mir_main_bb0_i16_lir35:
  movq %rax, -120(%rbp)
# cerune-origin: #12 bytes 103..115
cerune_origin_n12_main_36:
# cerune-mir: v1 main bb0 i17 -> lir 36 (source)
cerune_origin_mir_main_bb0_i17_lir36:
  movq -120(%rbp), %rax
# cerune-origin: #12 bytes 103..115
cerune_origin_n12_main_37:
# cerune-mir: v1 main bb0 i17 -> lir 37 (source)
cerune_origin_mir_main_bb0_i17_lir37:
  movl $1, %r10d
  leaq .Lcerune_output_origin_7(%rip), %r11
  callq cerune_output_string
# cerune-origin: #52 bytes 41..62
cerune_origin_n52_main_38:
# cerune-mir: v1 main bb0 i18 -> lir 38 (source)
cerune_origin_mir_main_bb0_i18_lir38:
  movq -40(%rbp), %rax
# cerune-origin: #52 bytes 41..62
cerune_origin_n52_main_39:
# cerune-mir: v1 main bb0 i18 -> lir 39 (source)
cerune_origin_mir_main_bb0_i18_lir39:
  movq %rax, -128(%rbp)
# cerune-origin: #53 bytes 41..62
cerune_origin_n53_main_40:
# cerune-mir: v1 main bb0 i19 -> lir 40 (source)
cerune_origin_mir_main_bb0_i19_lir40:
  movq -128(%rbp), %rax
# cerune-origin: #53 bytes 41..62
cerune_origin_n53_main_41:
# cerune-mir: v1 main bb0 i19 -> lir 41 (source)
cerune_origin_mir_main_bb0_i19_lir41:
  movq %rax, %rdi
  callq cerune_string_release
# cerune-origin: #54 bytes 0..40
cerune_origin_n54_main_42:
# cerune-mir: v1 main bb0 i20 -> lir 42 (source)
cerune_origin_mir_main_bb0_i20_lir42:
  movq -16(%rbp), %rax
# cerune-origin: #54 bytes 0..40
cerune_origin_n54_main_43:
# cerune-mir: v1 main bb0 i20 -> lir 43 (source)
cerune_origin_mir_main_bb0_i20_lir43:
  movq %rax, -136(%rbp)
# cerune-origin: #55 bytes 0..40
cerune_origin_n55_main_44:
# cerune-mir: v1 main bb0 i21 -> lir 44 (source)
cerune_origin_mir_main_bb0_i21_lir44:
  movq -136(%rbp), %rax
# cerune-origin: #55 bytes 0..40
cerune_origin_n55_main_45:
# cerune-mir: v1 main bb0 i21 -> lir 45 (source)
cerune_origin_mir_main_bb0_i21_lir45:
  movq %rax, %rdi
  callq cerune_string_release
# cerune-origin: synthetic
# cerune-mir: v1 main bb0 i22 -> lir 46 (function-end)
cerune_origin_mir_main_bb0_i22_lir46:
  jmp .Lcerune_block_1
# cerune-origin: synthetic
# cerune-mir: v1 synthetic ABI setup/exit
.Lcerune_block_1: # main_exit
# cerune-origin: synthetic
  xorl %eax, %eax
  addq $144, %rsp
  popq %rbp
  retq

.section .rodata
.Lcerune_failure_0:
  .asciz "cerune: runtime-v1 code=allocation-size-overflow node=1 bytes=19..39\n"
.Lcerune_failure_1:
  .asciz "cerune: runtime-v1 code=allocation-limit-exceeded node=1 bytes=19..39\n"
.Lcerune_failure_2:
  .asciz "cerune: runtime-v1 code=allocation-failed node=1 bytes=19..39\n"
.Lcerune_failure_3:
  .asciz "cerune: runtime-v1 code=allocation-size-overflow node=7 bytes=70..87\n"
.Lcerune_failure_4:
  .asciz "cerune: runtime-v1 code=allocation-limit-exceeded node=7 bytes=70..87\n"
.Lcerune_failure_5:
  .asciz "cerune: runtime-v1 code=allocation-failed node=7 bytes=70..87\n"
.Lcerune_output_origin_6:
  .asciz " node=10 bytes=89..102\n"
.Lcerune_output_origin_7:
  .asciz " node=12 bytes=103..115\n"

.section .note.GNU-stack,"",@progbits
