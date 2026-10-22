.section .rdata,"dr"
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
.Lcerune_string_0:
.quad 0x0000000000000004
.byte 230
.byte 151
.byte 165
.byte 0
.p2align 3
.Lcerune_string_1:
.quad 0x0000000000000000

.text
.section .rdata,"dr"
.Loutput_record:
.byte 99
.byte 101
.byte 114
.byte 117
.byte 110
.byte 101
.byte 58
.byte 32
.byte 105
.byte 111
.byte 45
.byte 118
.byte 49
.byte 32
.byte 111
.byte 112
.byte 101
.byte 114
.byte 97
.byte 116
.byte 105
.byte 111
.byte 110
.byte 61
.byte 119
.byte 114
.byte 105
.byte 116
.byte 101
.byte 32
.byte 114
.byte 101
.byte 113
.byte 117
.byte 101
.byte 115
.byte 116
.byte 101
.byte 100
.byte 61
.byte 37
.byte 108
.byte 108
.byte 117
.byte 32
.byte 97
.byte 99
.byte 99
.byte 101
.byte 112
.byte 116
.byte 101
.byte 100
.byte 61
.byte 37
.byte 108
.byte 108
.byte 117
.byte 37
.byte 115
.byte 0
.Loutput_format_error:
.byte 99
.byte 101
.byte 114
.byte 117
.byte 110
.byte 101
.byte 58
.byte 32
.byte 111
.byte 117
.byte 116
.byte 112
.byte 117
.byte 116
.byte 32
.byte 102
.byte 111
.byte 114
.byte 109
.byte 97
.byte 116
.byte 116
.byte 105
.byte 110
.byte 103
.byte 32
.byte 102
.byte 97
.byte 105
.byte 108
.byte 101
.byte 100
.byte 37
.byte 115
.byte 0
.Loutput_i64:
.byte 37
.byte 108
.byte 108
.byte 100
.byte 0
.Loutput_u64:
.byte 37
.byte 108
.byte 108
.byte 117
.byte 0
.Loutput_f32:
.byte 37
.byte 46
.byte 57
.byte 103
.byte 0
.Loutput_f64:
.byte 37
.byte 46
.byte 49
.byte 55
.byte 103
.byte 0
.Loutput_true:
.byte 116
.byte 114
.byte 117
.byte 101
.byte 10
.byte 0
.Loutput_false:
.byte 102
.byte 97
.byte 108
.byte 115
.byte 101
.byte 10
.byte 0
.Loutput_lf:
.byte 10
.byte 0
.Loutput_quote:
.byte 34
.byte 0
.text
cerune_output_write:
  subq $40, %rsp
  movl %edx, %r8d
  movq %rcx, %rdx
  movl $1, %ecx
  callq _write
  shlq $32, %rax
  sarq $32, %rax
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
  movq 56(%rsp), %rcx
  movq 72(%rsp), %rdx
  callq cerune_output_write
  testq %rax, %rax
  jns .Loutput_part_check
  callq _errno
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
  leaq 64(%rsp), %rcx
  movl $512, %edx
  leaq .Loutput_record(%rip), %r8
  movq 0(%r10), %r9
  movq 8(%r10), %rax
  movq %rax, 32(%rsp)
  movq 16(%r10), %rax
  movq %rax, 40(%rsp)
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
  movl %eax, %r8d
  movq %r10, %rdx
  movl $2, %ecx
  callq _write
.Loutput_exit:
  movl $1, %ecx
  callq _Exit
  ud2
cerune_output_i64:
  subq $936, %rsp
  movq %rax, 96(%rsp)
  movq %r10, 104(%rsp)
  movq %r11, 80(%rsp)
  movq $0, 152(%rsp)
  leaq 160(%rsp), %rcx
  movl $128, %edx
  leaq .Loutput_i64(%rip), %r8
  movq %rax, %r9
  callq snprintf
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
  leaq 160(%rsp), %rcx
  movl $128, %edx
  leaq .Loutput_u64(%rip), %r8
  movq %rax, %r9
  callq snprintf
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
  leaq 160(%rsp), %rcx
  movl $128, %edx
  leaq .Loutput_f32(%rip), %r8
  movq %rax, %r9
  movapd %xmm0, %xmm3
  callq snprintf
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
  leaq 160(%rsp), %rcx
  movl $128, %edx
  leaq .Loutput_f64(%rip), %r8
  movq %rax, %r9
  movapd %xmm0, %xmm3
  callq snprintf
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
  leaq 416(%rsp), %rcx
  movl $512, %edx
  leaq .Loutput_format_error(%rip), %r8
  movq 80(%rsp), %r9
  callq snprintf
  leaq 416(%rsp), %r10
  callq cerune_output_diagnostic
  ud2
.Loutput_begin:
  movq $0, 72(%rsp)
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
.text
.p2align 4
cerune_string_equal:
  movq 0(%rcx), %r8
  cmpq 0(%rdx), %r8
  jne .Lstring_different
  xorq %r9, %r9
.Lstring_compare:
  cmpq %r8, %r9
  je .Lstring_equal
  movzbl 8(%rcx,%r9,1), %eax
  cmpb 8(%rdx,%r9,1), %al
  jne .Lstring_different
  incq %r9
  jmp .Lstring_compare
.Lstring_equal:
  movl $1, %eax
  retq
.Lstring_different:
  xorl %eax, %eax
  retq
.globl main
.p2align 4
main:
  pushq %rbp
  movq %rsp, %rbp
  subq $80, %rsp
  movl $1, %ecx
  movl $32768, %edx
  callq _setmode
  cmpl $-1, %eax
  jne .Lstdout_ready
  movl $1, %eax
  addq $80, %rsp
  popq %rbp
  retq
.Lstdout_ready:
  jmp .Lcerune_block_0
.Lcerune_block_0: # mir_block
  leaq .Lcerune_string_0(%rip), %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq (%rax), %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movl $1, %r10d
  leaq .Lcerune_output_origin_0(%rip), %r11
  callq cerune_output_i64
  leaq .Lcerune_string_1(%rip), %rax
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq (%rax), %rax
  movq %rax, -48(%rbp)
  movq -48(%rbp), %rax
  movl $1, %r10d
  leaq .Lcerune_output_origin_1(%rip), %r11
  callq cerune_output_i64
  jmp .Lcerune_block_1
.Lcerune_block_1: # main_exit
  xorl %eax, %eax
  addq $80, %rsp
  popq %rbp
  retq

.section .rdata,"dr"
.Lcerune_output_origin_0:
  .asciz " node=2 bytes=24..46\n"
.Lcerune_output_origin_1:
  .asciz " node=5 bytes=47..67\n"
