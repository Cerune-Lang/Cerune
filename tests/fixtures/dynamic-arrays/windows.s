.section .rdata,"dr"
.Lcerune_fmt_i64:
  .asciz "%lld\n"
.Lcerune_fmt_f32:
  .asciz "%.9g\n"
.Lcerune_fmt_f64:
  .asciz "%.17g\n"
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
  .quad 1
  .byte 91
.p2align 3
.Lcerune_string_1:
  .quad 1
  .byte 93
.p2align 3
.Lcerune_string_2:
  .quad 0
.p2align 3
.Lcerune_string_3:
  .quad 2
  .byte 44
  .byte 32
.p2align 3
.Lcerune_string_4:
  .quad 1
  .byte 91
.p2align 3
.Lcerune_string_5:
  .quad 1
  .byte 93
.p2align 3
.Lcerune_string_6:
  .quad 0
.p2align 3
.Lcerune_string_7:
  .quad 2
  .byte 44
  .byte 32

.text
.section .rdata,"dr"
.Lwrite_i64:
  .asciz "%lld"
.Lwrite_u64:
  .asciz "%llu"
.Lwrite_f32:
  .asciz "%.9g"
.Lwrite_f64:
  .asciz "%.17g"
.Lwrite_true:
  .asciz "true"
.Lwrite_false:
  .asciz "false"
.Lwrite_bool:
  .asciz "%s"
.text
cerune_write_escaped_byte:
  subq $40, %rsp
  cmpl $0, %ecx
  je .Lescape_0
  cmpl $1, %ecx
  je .Lescape_1
  cmpl $2, %ecx
  je .Lescape_2
  cmpl $3, %ecx
  je .Lescape_3
  cmpl $4, %ecx
  je .Lescape_4
  cmpl $5, %ecx
  je .Lescape_5
  cmpl $6, %ecx
  je .Lescape_6
  cmpl $7, %ecx
  je .Lescape_7
  cmpl $8, %ecx
  je .Lescape_8
  cmpl $9, %ecx
  je .Lescape_9
  cmpl $10, %ecx
  je .Lescape_10
  cmpl $11, %ecx
  je .Lescape_11
  cmpl $12, %ecx
  je .Lescape_12
  cmpl $13, %ecx
  je .Lescape_13
  cmpl $14, %ecx
  je .Lescape_14
  cmpl $15, %ecx
  je .Lescape_15
  cmpl $16, %ecx
  je .Lescape_16
  cmpl $17, %ecx
  je .Lescape_17
  cmpl $18, %ecx
  je .Lescape_18
  cmpl $19, %ecx
  je .Lescape_19
  cmpl $20, %ecx
  je .Lescape_20
  cmpl $21, %ecx
  je .Lescape_21
  cmpl $22, %ecx
  je .Lescape_22
  cmpl $23, %ecx
  je .Lescape_23
  cmpl $24, %ecx
  je .Lescape_24
  cmpl $25, %ecx
  je .Lescape_25
  cmpl $26, %ecx
  je .Lescape_26
  cmpl $27, %ecx
  je .Lescape_27
  cmpl $28, %ecx
  je .Lescape_28
  cmpl $29, %ecx
  je .Lescape_29
  cmpl $30, %ecx
  je .Lescape_30
  cmpl $31, %ecx
  je .Lescape_31
  cmpl $34, %ecx
  je .Lescape_34
  cmpl $92, %ecx
  je .Lescape_92
  cmpl $127, %ecx
  je .Lescape_127
  callq putchar
  addq $40, %rsp
  retq
.Lescape_0:
  movl $92, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_1:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_2:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $50, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_3:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $51, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_4:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $52, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_5:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $53, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_6:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $54, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_7:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $55, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_8:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $56, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_9:
  movl $92, %ecx
  callq putchar
  movl $116, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_10:
  movl $92, %ecx
  callq putchar
  movl $110, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_11:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $98, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_12:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $99, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_13:
  movl $92, %ecx
  callq putchar
  movl $114, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_14:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $101, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_15:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $102, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_16:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $48, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_17:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_18:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $50, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_19:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $51, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_20:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $52, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_21:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $53, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_22:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $54, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_23:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $55, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_24:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $56, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_25:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $57, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_26:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $97, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_27:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $98, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_28:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $99, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_29:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $100, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_30:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $101, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_31:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $49, %ecx
  callq putchar
  movl $102, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_34:
  movl $92, %ecx
  callq putchar
  movl $34, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_92:
  movl $92, %ecx
  callq putchar
  movl $92, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lescape_127:
  movl $92, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $123, %ecx
  callq putchar
  movl $55, %ecx
  callq putchar
  movl $102, %ecx
  callq putchar
  movl $125, %ecx
  callq putchar
  addq $40, %rsp
  retq
cerune_write_bool:
  subq $40, %rsp
  cmpl $0, %ecx
  je .Lwrite_bool_false
  movl $116, %ecx
  callq putchar
  movl $114, %ecx
  callq putchar
  movl $117, %ecx
  callq putchar
  movl $101, %ecx
  callq putchar
  addq $40, %rsp
  retq
.Lwrite_bool_false:
  movl $102, %ecx
  callq putchar
  movl $97, %ecx
  callq putchar
  movl $108, %ecx
  callq putchar
  movl $115, %ecx
  callq putchar
  movl $101, %ecx
  callq putchar
  addq $40, %rsp
  retq
cerune_write_string:
  subq $56, %rsp
  movq %rcx, 32(%rsp)
  movq $0, 40(%rsp)
.Lwrite_string_loop:
  movq 32(%rsp), %rax
  movq 40(%rsp), %rdx
  cmpq (%rax), %rdx
  je .Lwrite_string_end
  movzbl 8(%rax,%rdx), %ecx
  callq putchar
  incq 40(%rsp)
  jmp .Lwrite_string_loop
.Lwrite_string_end:
  addq $56, %rsp
  retq
cerune_write_quoted:
  subq $56, %rsp
  movq %rcx, 32(%rsp)
  movq $0, 40(%rsp)
  movl $34, %ecx
  callq putchar
.Lwrite_quoted_loop:
  movq 32(%rsp), %rax
  movq 40(%rsp), %rdx
  cmpq (%rax), %rdx
  je .Lwrite_quoted_end
  movzbl 8(%rax,%rdx), %ecx
  callq cerune_write_escaped_byte
  incq 40(%rsp)
  jmp .Lwrite_quoted_loop
.Lwrite_quoted_end:
  movl $34, %ecx
  callq putchar
  addq $56, %rsp
  retq

.p2align 4
cerune_string_equal:
  movq (%rcx), %r8
  cmpq (%rdx), %r8
  jne .Lstring_different
  xorq %r9, %r9
.Lstring_compare:
  cmpq %r8, %r9
  je .Lstring_equal
  movzbl 8(%rcx,%r9), %eax
  cmpb 8(%rdx,%r9), %al
  jne .Lstring_different
  incq %r9
  jmp .Lstring_compare
.Lstring_equal:
  movl $1, %eax
  retq
.Lstring_different:
  xorl %eax, %eax
  retq

.p2align 4
cerune_print_string:
  subq $56, %rsp
  movq %rcx, 32(%rsp)
  movq $0, 40(%rsp)
.Lstring_write:
  movq 32(%rsp), %rax
  movq 40(%rsp), %rdx
  cmpq (%rax), %rdx
  je .Lstring_newline
  movzbl 8(%rax,%rdx), %ecx
  callq putchar
  incq 40(%rsp)
  jmp .Lstring_write
.Lstring_newline:
  movl $10, %ecx
  callq putchar
  addq $56, %rsp
  retq


.data
.p2align 3
.Lcerune_array_live:
  .quad 0
.Lcerune_array_limit:
  .quad 67108864
.Lcerune_array_max:
  .quad 9223372036854775807
.Lcerune_array_physical_max:
  .quad 9223372036854775759
.text
cerune_array_allocate:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  movq %rcx, %rax
  movq %rdx, %r10
  movq %r8, %r11
  testq %rax, %rax
  js .Lcerune_array_size_fail
  je .Lcerune_array_empty
  movq %rax, -8(%rbp)
  movq %r11, -32(%rbp)
  mulq %r10
  testq %rdx, %rdx
  jne .Lcerune_array_size_fail
  cmpq .Lcerune_array_max(%rip), %rax
  ja .Lcerune_array_size_fail
  movq %rax, -16(%rbp)
  movq -8(%rbp), %rax
  mulq %r11
  testq %rdx, %rdx
  jne .Lcerune_array_size_fail
  cmpq .Lcerune_array_physical_max(%rip), %rax
  ja .Lcerune_array_size_fail
  movq %rax, -24(%rbp)
  movq .Lcerune_array_limit(%rip), %r10
  subq .Lcerune_array_live(%rip), %r10
  cmpq -16(%rbp), %r10
  jb .Lcerune_array_limit_fail
  leaq 48(%rax), %rcx
  callq malloc
  testq %rax, %rax
  je .Lcerune_array_allocation_fail
  movq -24(%rbp), %r11
  leaq 48(%rax), %r10
  testq %r11, %r11
  je .Lcerune_array_payload_ready
  subq $8, %r10
  addq %r11, %r10
.Lcerune_array_payload_ready:
  movq %r10, (%rax)
  movq -8(%rbp), %r10
  movq %r10, 8(%rax)
  movq $1, 16(%rax)
  movq -16(%rbp), %r10
  movq %r10, 24(%rax)
  addq %r10, .Lcerune_array_live(%rip)
  movq $0, 32(%rax)
  movq -32(%rbp), %r10
  movq %r10, 40(%rax)
  xorl %edx, %edx
  jmp .Lcerune_array_allocate_done
.Lcerune_array_empty:
  xorl %eax, %eax
  xorl %edx, %edx
  jmp .Lcerune_array_allocate_done
.Lcerune_array_size_fail:
  xorl %eax, %eax
  movl $1, %edx
  jmp .Lcerune_array_allocate_done
.Lcerune_array_limit_fail:
  xorl %eax, %eax
  movl $2, %edx
  jmp .Lcerune_array_allocate_done
.Lcerune_array_allocation_fail:
  movl $3, %edx
.Lcerune_array_allocate_done:
  addq $64, %rsp
  popq %rbp
  retq
cerune_array_length:
  xorl %eax, %eax
  testq %rcx, %rcx
  je .Lcerune_array_length_done
  movq 8(%rcx), %rax
.Lcerune_array_length_done:
  retq
cerune_array_init_address:
  movq %rcx, %r10
  testq %r10, %r10
  je .Lcerune_array_invalid_owner
  movq 32(%r10), %rax
  cmpq 8(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, %rax
  retq
cerune_array_retain:
  movq %rcx, %r10
  testq %r10, %r10
  je .Lcerune_array_retain_done
  cmpq $0, 16(%r10)
  je .Lcerune_array_invalid_owner
  cmpq $-1, 16(%r10)
  je .Lcerune_array_invalid_owner
  addq $1, 16(%r10)
.Lcerune_array_retain_done:
  retq
cerune_array_release_owner:
  movq %rcx, %r10
  xorl %eax, %eax
  testq %r10, %r10
  je .Lcerune_array_release_empty
  cmpq $0, 16(%r10)
  je .Lcerune_array_invalid_owner
  subq $1, 16(%r10)
  sete %al
  retq
.Lcerune_array_release_empty:
  movl $1, %eax
  retq
cerune_array_free:
  testq %rcx, %rcx
  je .Lcerune_array_free_empty
  cmpq $0, 16(%rcx)
  jne .Lcerune_array_invalid_owner
  pushq %rbp
  movq %rsp, %rbp
  subq $32, %rsp
  movq 24(%rcx), %r10
  subq %r10, .Lcerune_array_live(%rip)
  callq free
  addq $32, %rsp
  popq %rbp
.Lcerune_array_free_empty:
  retq
.Lcerune_array_invalid_owner:
  movl $7, %ecx
  int $0x29
.p2align 4
cerune_fn__display0_0:
  pushq %rbp
  movq %rsp, %rbp
  subq $304, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_0_block_0
.Lcerune_fn_0_block_0: # mir_block
  leaq .Lcerune_string_0(%rip), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  callq cerune_fn__ownership2_2
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_0_block_1
.Lcerune_fn_0_block_1: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -64(%rbp)
  movq -56(%rbp), %rcx
  movq -64(%rbp), %rdx
  callq cerune_fn__ownership3_3
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_0_block_3
  jmp .Lcerune_fn_0_block_2
.Lcerune_fn_0_block_2: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rcx
  callq cerune_fn__ownership5_5
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_0_block_6
  jmp .Lcerune_fn_0_block_5
.Lcerune_fn_0_block_3: # mir_block
  leaq .Lcerune_string_1(%rip), %rax
  movq %rax, -224(%rbp)
  movq -224(%rbp), %rax
  movq %rax, -232(%rbp)
  movq -232(%rbp), %rax
  movq %rax, -240(%rbp)
  movq -240(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  leaq .Lcerune_string_2(%rip), %rax
  movq %rax, -248(%rbp)
  movq -248(%rbp), %rax
  movq %rax, -256(%rbp)
  movq -256(%rbp), %rax
  movq %rax, -264(%rbp)
  movq -264(%rbp), %rax
  movq %rax, %rcx
  callq cerune_print_string
  addq $304, %rsp
  popq %rbp
  retq
.Lcerune_fn_0_block_4: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -208(%rbp)
  movq -208(%rbp), %rcx
  callq cerune_fn__ownership4_4
  movq %rax, -216(%rbp)
  movq -216(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_0_block_1
.Lcerune_fn_0_block_5: # mir_block
  leaq .Lcerune_string_3(%rip), %rax
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  jmp .Lcerune_fn_0_block_7
.Lcerune_fn_0_block_6: # mir_block
  jmp .Lcerune_fn_0_block_7
.Lcerune_fn_0_block_7: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -120(%rbp)
  movq -120(%rbp), %rax
  movq %rax, -128(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -136(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -144(%rbp)
  movq -144(%rbp), %rax
  movq %rax, -152(%rbp)
  movq -152(%rbp), %rax
  movq %rax, -160(%rbp)
  movq -128(%rbp), %rax
  movq %rax, -168(%rbp)
  movq -160(%rbp), %rax
  movq %rax, -176(%rbp)
  movq -176(%rbp), %rax
  movq -168(%rbp), %r10
  testq %rax, %rax
  js .Lfn_0_dynamic_index_trap_10
  testq %r10, %r10
  je .Lfn_0_dynamic_index_trap_10
  cmpq 8(%r10), %rax
  jge .Lfn_0_dynamic_index_trap_10
  cmpq 32(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, -272(%rbp)
  jmp .Lfn_0_dynamic_index_done_10
.Lfn_0_dynamic_index_trap_10:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_0(%rip), %rdx
  movl $71, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_0_dynamic_index_done_10:
  movq -272(%rbp), %r11
  movq (%r11), %rax
  movq %rax, -184(%rbp)
  movq -184(%rbp), %rax
  movq %rax, -192(%rbp)
  movq -192(%rbp), %rax
  movq %rax, -200(%rbp)
  movq -200(%rbp), %rax
  movq %rax, %rdx
  leaq .Lwrite_i64(%rip), %rcx
  callq printf
  jmp .Lcerune_fn_0_block_4
.Lcerune_fn_0_block_8: # mir_block
  addq $304, %rsp
  popq %rbp
  retq

.p2align 4
cerune_fn__display1_1:
  pushq %rbp
  movq %rsp, %rbp
  subq $304, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_1_block_0
.Lcerune_fn_1_block_0: # mir_block
  leaq .Lcerune_string_4(%rip), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  callq cerune_fn__ownership6_6
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_1_block_1
.Lcerune_fn_1_block_1: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -64(%rbp)
  movq -56(%rbp), %rcx
  movq -64(%rbp), %rdx
  callq cerune_fn__ownership7_7
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_1_block_3
  jmp .Lcerune_fn_1_block_2
.Lcerune_fn_1_block_2: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rcx
  callq cerune_fn__ownership9_9
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_1_block_6
  jmp .Lcerune_fn_1_block_5
.Lcerune_fn_1_block_3: # mir_block
  leaq .Lcerune_string_5(%rip), %rax
  movq %rax, -224(%rbp)
  movq -224(%rbp), %rax
  movq %rax, -232(%rbp)
  movq -232(%rbp), %rax
  movq %rax, -240(%rbp)
  movq -240(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  leaq .Lcerune_string_6(%rip), %rax
  movq %rax, -248(%rbp)
  movq -248(%rbp), %rax
  movq %rax, -256(%rbp)
  movq -256(%rbp), %rax
  movq %rax, -264(%rbp)
  movq -264(%rbp), %rax
  movq %rax, %rcx
  callq cerune_print_string
  addq $304, %rsp
  popq %rbp
  retq
.Lcerune_fn_1_block_4: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -208(%rbp)
  movq -208(%rbp), %rcx
  callq cerune_fn__ownership8_8
  movq %rax, -216(%rbp)
  movq -216(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_1_block_1
.Lcerune_fn_1_block_5: # mir_block
  leaq .Lcerune_string_7(%rip), %rax
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  movq %rax, %rcx
  callq cerune_write_string
  jmp .Lcerune_fn_1_block_7
.Lcerune_fn_1_block_6: # mir_block
  jmp .Lcerune_fn_1_block_7
.Lcerune_fn_1_block_7: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -120(%rbp)
  movq -120(%rbp), %rax
  movq %rax, -128(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -136(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -144(%rbp)
  movq -144(%rbp), %rax
  movq %rax, -152(%rbp)
  movq -152(%rbp), %rax
  movq %rax, -160(%rbp)
  movq -128(%rbp), %rax
  movq %rax, -168(%rbp)
  movq -160(%rbp), %rax
  movq %rax, -176(%rbp)
  movq -176(%rbp), %rax
  movq -168(%rbp), %r10
  testq %rax, %rax
  js .Lfn_1_dynamic_index_trap_10
  testq %r10, %r10
  je .Lfn_1_dynamic_index_trap_10
  cmpq 8(%r10), %rax
  jge .Lfn_1_dynamic_index_trap_10
  cmpq 32(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, -272(%rbp)
  jmp .Lfn_1_dynamic_index_done_10
.Lfn_1_dynamic_index_trap_10:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_1(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_1_dynamic_index_done_10:
  movq -272(%rbp), %r11
  movq (%r11), %rax
  movq %rax, -184(%rbp)
  movq -184(%rbp), %rax
  movq %rax, -192(%rbp)
  movq -192(%rbp), %rax
  movq %rax, -200(%rbp)
  movq -200(%rbp), %rax
  movq %rax, %rdx
  leaq .Lwrite_i64(%rip), %rcx
  callq printf
  jmp .Lcerune_fn_1_block_4
.Lcerune_fn_1_block_8: # mir_block
  addq $304, %rsp
  popq %rbp
  retq

.p2align 4
cerune_fn__ownership2_2:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  jmp .Lcerune_fn_2_block_0
.Lcerune_fn_2_block_0: # mir_block
  movabsq $0, %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  addq $64, %rsp
  popq %rbp
  retq
.Lcerune_fn_2_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership3_3:
  pushq %rbp
  movq %rsp, %rbp
  subq $144, %rsp
  movq %rcx, -8(%rbp)
  movq %rdx, -16(%rbp)
  jmp .Lcerune_fn_3_block_0
.Lcerune_fn_3_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  movq %rax, %rcx
  movq -80(%rbp), %rax
  cmpq %rcx, %rax
  setl %al
  movzbq %al, %rax
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  addq $144, %rsp
  popq %rbp
  retq
.Lcerune_fn_3_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership4_4:
  pushq %rbp
  movq %rsp, %rbp
  subq $112, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_4_block_0
.Lcerune_fn_4_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movabsq $1, %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  movq -48(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_4_integer_ok_3
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_2(%rip), %rdx
  movl $62, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_4_integer_ok_3:
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  addq $112, %rsp
  popq %rbp
  retq
.Lcerune_fn_4_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership5_5:
  pushq %rbp
  movq %rsp, %rbp
  subq $112, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_5_block_0
.Lcerune_fn_5_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movabsq $0, %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  movq -48(%rbp), %rax
  cmpq %rcx, %rax
  setne %al
  movzbq %al, %rax
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  addq $112, %rsp
  popq %rbp
  retq
.Lcerune_fn_5_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership6_6:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  jmp .Lcerune_fn_6_block_0
.Lcerune_fn_6_block_0: # mir_block
  movabsq $0, %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  addq $64, %rsp
  popq %rbp
  retq
.Lcerune_fn_6_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership7_7:
  pushq %rbp
  movq %rsp, %rbp
  subq $144, %rsp
  movq %rcx, -8(%rbp)
  movq %rdx, -16(%rbp)
  jmp .Lcerune_fn_7_block_0
.Lcerune_fn_7_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  movq %rax, %rcx
  movq -80(%rbp), %rax
  cmpq %rcx, %rax
  setl %al
  movzbq %al, %rax
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  addq $144, %rsp
  popq %rbp
  retq
.Lcerune_fn_7_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership8_8:
  pushq %rbp
  movq %rsp, %rbp
  subq $112, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_8_block_0
.Lcerune_fn_8_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movabsq $1, %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  movq -48(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_8_integer_ok_3
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_3(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_8_integer_ok_3:
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  addq $112, %rsp
  popq %rbp
  retq
.Lcerune_fn_8_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership9_9:
  pushq %rbp
  movq %rsp, %rbp
  subq $112, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_9_block_0
.Lcerune_fn_9_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movabsq $0, %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -56(%rbp), %rax
  movq %rax, %rcx
  movq -48(%rbp), %rax
  cmpq %rcx, %rax
  setne %al
  movzbq %al, %rax
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  addq $112, %rsp
  popq %rbp
  retq
.Lcerune_fn_9_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership10_10:
  pushq %rbp
  movq %rsp, %rbp
  subq $448, %rsp
  jmp .Lcerune_fn_10_block_0
.Lcerune_fn_10_block_0: # mir_block
  movabsq $1, %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movabsq $2, %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -64(%rbp)
  movq -56(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -64(%rbp), %rax
  movq %rax, -80(%rbp)
  movabsq $2, %rax
  movq %rax, -88(%rbp)
  movabsq $0, %rax
  movq %rax, -96(%rbp)
  movabsq $2, %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq -96(%rbp), %r10
  testq %r10, %r10
  js .Lfn_10_array_range_trap_7
  cmpq %r10, %rax
  jl .Lfn_10_array_range_trap_7
  cmpq -88(%rbp), %rax
  jg .Lfn_10_array_range_trap_7
  jmp .Lfn_10_array_range_done_7
.Lfn_10_array_range_trap_7:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_4(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_10_array_range_done_7:
  movabsq $2, %rax
  movq %rax, -112(%rbp)
  movabsq $0, %rax
  movq %rax, -120(%rbp)
  movq -120(%rbp), %rax
  movq %rax, %rcx
  movq -112(%rbp), %rax
  subq %rcx, %rax
  jno .Lcerune_fn_10_integer_ok_8
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_5(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_10_integer_ok_8:
  movq %rax, -128(%rbp)
  movq -128(%rbp), %rax
  movq %rax, -136(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -144(%rbp)
  movq -144(%rbp), %rax
  testq %rax, %rax
  jns .Lfn_10_array_length_ok_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_6(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_10_array_length_ok_9:
  movq %rax, %rcx
  movabsq $8, %rdx
  movabsq $8, %r8
  callq cerune_array_allocate
  testq %rdx, %rdx
  je .Lfn_10_array_allocated_9
  cmpq $1, %rdx
  jne .Lfn_10_array_limit_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_7(%rip), %rdx
  movl $71, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_10_array_limit_9:
  cmpq $2, %rdx
  jne .Lfn_10_array_failed_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_8(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_10_array_failed_9:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_9(%rip), %rdx
  movl $64, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_10_array_allocated_9:
  movq %rax, -152(%rbp)
  movq -152(%rbp), %rax
  movq %rax, -160(%rbp)
  movabsq $0, %rax
  movq %rax, -168(%rbp)
  movq -168(%rbp), %rax
  movq %rax, -176(%rbp)
  jmp .Lcerune_fn_10_block_1
.Lcerune_fn_10_block_1: # mir_block
  movq -176(%rbp), %rax
  movq %rax, -184(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -192(%rbp)
  movq -192(%rbp), %rax
  movq %rax, %rcx
  movq -184(%rbp), %rax
  cmpq %rcx, %rax
  setl %al
  movzbq %al, %rax
  movq %rax, -200(%rbp)
  movq -200(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_10_block_3
  jmp .Lcerune_fn_10_block_2
.Lcerune_fn_10_block_2: # mir_block
  movq -72(%rbp), %rax
  movq %rax, -208(%rbp)
  movq -80(%rbp), %rax
  movq %rax, -216(%rbp)
  movq -208(%rbp), %rax
  movq %rax, -224(%rbp)
  movq -216(%rbp), %rax
  movq %rax, -232(%rbp)
  movq -176(%rbp), %rax
  movq %rax, -240(%rbp)
  movq -240(%rbp), %rax
  movq %rax, -248(%rbp)
  movabsq $0, %rax
  movq %rax, -256(%rbp)
  movq -256(%rbp), %rax
  movq %rax, -264(%rbp)
  movq -248(%rbp), %rax
  movq %rax, -272(%rbp)
  movq -264(%rbp), %rax
  movq %rax, -280(%rbp)
  movq -280(%rbp), %rax
  movq %rax, %rcx
  movq -272(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_10_integer_ok_10
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_10(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_10_integer_ok_10:
  movq %rax, -288(%rbp)
  movq -288(%rbp), %rax
  movq %rax, -296(%rbp)
  movq -224(%rbp), %rax
  movq %rax, -304(%rbp)
  movq -232(%rbp), %rax
  movq %rax, -312(%rbp)
  movq -296(%rbp), %rax
  movq %rax, -320(%rbp)
  movq -320(%rbp), %rax
  testq %rax, %rax
  js .Lcerune_fn_10_array_oob_11
  cmpq $2, %rax
  jge .Lcerune_fn_10_array_oob_11
  imulq $-1, %rax
  leaq -304(%rbp,%rax,8), %rcx
  movq %rcx, -408(%rbp)
  jmp .Lcerune_fn_10_array_done_11
.Lcerune_fn_10_array_oob_11:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_11(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_10_array_done_11:
  movq -408(%rbp), %r11
  movq (%r11), %rax
  movq %rax, -328(%rbp)
  movq -328(%rbp), %rax
  movq %rax, -336(%rbp)
  movq -336(%rbp), %rax
  movq %rax, -344(%rbp)
  movq -344(%rbp), %rax
  movq %rax, -352(%rbp)
  movq -160(%rbp), %rax
  movq %rax, -360(%rbp)
  movq -352(%rbp), %rax
  movq %rax, -368(%rbp)
  movq -360(%rbp), %rcx
  callq cerune_array_init_address
  movq %rax, -416(%rbp)
  movq -368(%rbp), %rax
  movq -416(%rbp), %rcx
  movq %rax, (%rcx)
  movq -360(%rbp), %r10
  addq $1, 32(%r10)
  jmp .Lcerune_fn_10_block_4
.Lcerune_fn_10_block_3: # mir_block
  movq -160(%rbp), %rax
  movq %rax, -400(%rbp)
  movq -400(%rbp), %rax
  addq $448, %rsp
  popq %rbp
  retq
.Lcerune_fn_10_block_4: # mir_block
  movq -176(%rbp), %rax
  movq %rax, -376(%rbp)
  movabsq $1, %rax
  movq %rax, -384(%rbp)
  movq -384(%rbp), %rax
  movq %rax, %rcx
  movq -376(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_10_integer_ok_12
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_12(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_10_integer_ok_12:
  movq %rax, -392(%rbp)
  movq -392(%rbp), %rax
  movq %rax, -176(%rbp)
  jmp .Lcerune_fn_10_block_1
.Lcerune_fn_10_block_5: # mir_block
  ud2

.p2align 4
cerune_fn__ownership11_11:
  pushq %rbp
  movq %rsp, %rbp
  subq $416, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_11_block_0
.Lcerune_fn_11_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  movq -48(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -56(%rbp)
  movabsq $0, %rax
  movq %rax, -64(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  movq -64(%rbp), %r10
  testq %r10, %r10
  js .Lfn_11_array_range_trap_7
  cmpq %r10, %rax
  jl .Lfn_11_array_range_trap_7
  cmpq -56(%rbp), %rax
  jg .Lfn_11_array_range_trap_7
  jmp .Lfn_11_array_range_done_7
.Lfn_11_array_range_trap_7:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_13(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_array_range_done_7:
  movq -40(%rbp), %rax
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -96(%rbp)
  movabsq $0, %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, %rcx
  movq -96(%rbp), %rax
  subq %rcx, %rax
  jno .Lcerune_fn_11_integer_ok_8
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_14(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_11_integer_ok_8:
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  movq %rax, -120(%rbp)
  movq -120(%rbp), %rax
  movq %rax, -128(%rbp)
  movq -128(%rbp), %rax
  testq %rax, %rax
  jns .Lfn_11_array_length_ok_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_15(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_array_length_ok_9:
  movq %rax, %rcx
  movabsq $8, %rdx
  movabsq $8, %r8
  callq cerune_array_allocate
  testq %rdx, %rdx
  je .Lfn_11_array_allocated_9
  cmpq $1, %rdx
  jne .Lfn_11_array_limit_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_16(%rip), %rdx
  movl $71, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_array_limit_9:
  cmpq $2, %rdx
  jne .Lfn_11_array_failed_9
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_17(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_array_failed_9:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_18(%rip), %rdx
  movl $64, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_array_allocated_9:
  movq %rax, -136(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -144(%rbp)
  movabsq $0, %rax
  movq %rax, -152(%rbp)
  movq -152(%rbp), %rax
  movq %rax, -160(%rbp)
  jmp .Lcerune_fn_11_block_1
.Lcerune_fn_11_block_1: # mir_block
  movq -160(%rbp), %rax
  movq %rax, -168(%rbp)
  movq -120(%rbp), %rax
  movq %rax, -176(%rbp)
  movq -176(%rbp), %rax
  movq %rax, %rcx
  movq -168(%rbp), %rax
  cmpq %rcx, %rax
  setl %al
  movzbq %al, %rax
  movq %rax, -184(%rbp)
  movq -184(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_11_block_3
  jmp .Lcerune_fn_11_block_2
.Lcerune_fn_11_block_2: # mir_block
  movq -40(%rbp), %rax
  movq %rax, -192(%rbp)
  movq -192(%rbp), %rax
  movq %rax, -200(%rbp)
  movq -160(%rbp), %rax
  movq %rax, -208(%rbp)
  movq -208(%rbp), %rax
  movq %rax, -216(%rbp)
  movabsq $0, %rax
  movq %rax, -224(%rbp)
  movq -224(%rbp), %rax
  movq %rax, -232(%rbp)
  movq -216(%rbp), %rax
  movq %rax, -240(%rbp)
  movq -232(%rbp), %rax
  movq %rax, -248(%rbp)
  movq -248(%rbp), %rax
  movq %rax, %rcx
  movq -240(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_11_integer_ok_10
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_19(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_11_integer_ok_10:
  movq %rax, -256(%rbp)
  movq -256(%rbp), %rax
  movq %rax, -264(%rbp)
  movq -200(%rbp), %rax
  movq %rax, -272(%rbp)
  movq -264(%rbp), %rax
  movq %rax, -280(%rbp)
  movq -280(%rbp), %rax
  movq -272(%rbp), %r10
  testq %rax, %rax
  js .Lfn_11_dynamic_index_trap_11
  testq %r10, %r10
  je .Lfn_11_dynamic_index_trap_11
  cmpq 8(%r10), %rax
  jge .Lfn_11_dynamic_index_trap_11
  cmpq 32(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, -368(%rbp)
  jmp .Lfn_11_dynamic_index_done_11
.Lfn_11_dynamic_index_trap_11:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_20(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lfn_11_dynamic_index_done_11:
  movq -368(%rbp), %r11
  movq (%r11), %rax
  movq %rax, -288(%rbp)
  movq -288(%rbp), %rax
  movq %rax, -296(%rbp)
  movq -296(%rbp), %rax
  movq %rax, -304(%rbp)
  movq -304(%rbp), %rax
  movq %rax, -312(%rbp)
  movq -144(%rbp), %rax
  movq %rax, -320(%rbp)
  movq -312(%rbp), %rax
  movq %rax, -328(%rbp)
  movq -320(%rbp), %rcx
  callq cerune_array_init_address
  movq %rax, -376(%rbp)
  movq -328(%rbp), %rax
  movq -376(%rbp), %rcx
  movq %rax, (%rcx)
  movq -320(%rbp), %r10
  addq $1, 32(%r10)
  jmp .Lcerune_fn_11_block_4
.Lcerune_fn_11_block_3: # mir_block
  movq -144(%rbp), %rax
  movq %rax, -360(%rbp)
  movq -360(%rbp), %rax
  addq $416, %rsp
  popq %rbp
  retq
.Lcerune_fn_11_block_4: # mir_block
  movq -160(%rbp), %rax
  movq %rax, -336(%rbp)
  movabsq $1, %rax
  movq %rax, -344(%rbp)
  movq -344(%rbp), %rax
  movq %rax, %rcx
  movq -336(%rbp), %rax
  addq %rcx, %rax
  jno .Lcerune_fn_11_integer_ok_12
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_21(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_11_integer_ok_12:
  movq %rax, -352(%rbp)
  movq -352(%rbp), %rax
  movq %rax, -160(%rbp)
  jmp .Lcerune_fn_11_block_1
.Lcerune_fn_11_block_5: # mir_block
  ud2

.p2align 4
cerune_fn__ownership12_12:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  jmp .Lcerune_fn_12_block_0
.Lcerune_fn_12_block_0: # mir_block
  movabsq $0, %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  addq $64, %rsp
  popq %rbp
  retq
.Lcerune_fn_12_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership13_13:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  jmp .Lcerune_fn_13_block_0
.Lcerune_fn_13_block_0: # mir_block
  movabsq $9, %rax
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  addq $64, %rsp
  popq %rbp
  retq
.Lcerune_fn_13_block_1: # mir_block
  ud2

.p2align 4
cerune_fn__ownership14_14:
  pushq %rbp
  movq %rsp, %rbp
  subq $144, %rsp
  movq %rcx, -8(%rbp)
  jmp .Lcerune_fn_14_block_0
.Lcerune_fn_14_block_0: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_release_owner
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_14_block_2
  jmp .Lcerune_fn_14_block_1
.Lcerune_fn_14_block_1: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_length
  movq %rax, -40(%rbp)
  movq -40(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_14_block_4
.Lcerune_fn_14_block_2: # mir_block
  jmp .Lcerune_fn_14_block_3
.Lcerune_fn_14_block_3: # mir_block
  addq $144, %rsp
  popq %rbp
  retq
.Lcerune_fn_14_block_4: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movabsq $0, %rax
  movq %rax, -64(%rbp)
  movq -64(%rbp), %rax
  movq %rax, %rcx
  movq -56(%rbp), %rax
  cmpq %rcx, %rax
  setg %al
  movzbq %al, %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  testq %rax, %rax
  je .Lcerune_fn_14_block_6
  jmp .Lcerune_fn_14_block_5
.Lcerune_fn_14_block_5: # mir_block
  jmp .Lcerune_fn_14_block_7
.Lcerune_fn_14_block_6: # mir_block
  movq -8(%rbp), %rax
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq %rax, %rcx
  callq cerune_array_free
  jmp .Lcerune_fn_14_block_3
.Lcerune_fn_14_block_7: # mir_block
  movq -48(%rbp), %rax
  movq %rax, -80(%rbp)
  movabsq $1, %rax
  movq %rax, -88(%rbp)
  movq -88(%rbp), %rax
  movq %rax, %rcx
  movq -80(%rbp), %rax
  subq %rcx, %rax
  jno .Lcerune_fn_14_integer_ok_10
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_22(%rip), %rdx
  movl $63, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lcerune_fn_14_integer_ok_10:
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq %rax, -48(%rbp)
  jmp .Lcerune_fn_14_block_4
.Lcerune_fn_14_block_8: # mir_block
  addq $144, %rsp
  popq %rbp
  retq

.globl main
.p2align 4
main:
  pushq %rbp
  movq %rsp, %rbp
  subq $224, %rsp
  movl $1, %ecx
  movl $32768, %edx
  callq _setmode
  cmpl $-1, %eax
  jne .Lstdout_ready
  movl $1, %eax
  addq $224, %rsp
  popq %rbp
  retq
.Lstdout_ready:
  jmp .Lcerune_block_0
.Lcerune_block_0: # mir_block
  callq cerune_fn__ownership10_10
  movq %rax, -8(%rbp)
  movq -8(%rbp), %rax
  movq %rax, -16(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -24(%rbp)
  movq -24(%rbp), %rcx
  callq cerune_fn__ownership11_11
  movq %rax, -32(%rbp)
  movq -32(%rbp), %rax
  movq %rax, -40(%rbp)
  callq cerune_fn__ownership12_12
  movq %rax, -48(%rbp)
  movq -48(%rbp), %rax
  movq %rax, -56(%rbp)
  movq -16(%rbp), %rax
  movq %rax, -64(%rbp)
  movq -56(%rbp), %rax
  movq %rax, -72(%rbp)
  movq -72(%rbp), %rax
  movq -64(%rbp), %r10
  testq %rax, %rax
  js .Lmain_dynamic_index_trap_2
  testq %r10, %r10
  je .Lmain_dynamic_index_trap_2
  cmpq 8(%r10), %rax
  jge .Lmain_dynamic_index_trap_2
  cmpq 32(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, -184(%rbp)
  jmp .Lmain_dynamic_index_done_2
.Lmain_dynamic_index_trap_2:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_23(%rip), %rdx
  movl $72, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lmain_dynamic_index_done_2:
  movq -184(%rbp), %r11
  movq (%r11), %rax
  movq %rax, -80(%rbp)
  movq -80(%rbp), %rax
  movq %rax, -88(%rbp)
  movq -56(%rbp), %rax
  movq %rax, -96(%rbp)
  movq -96(%rbp), %rax
  movq -16(%rbp), %r10
  testq %rax, %rax
  js .Lmain_dynamic_index_trap_3
  testq %r10, %r10
  je .Lmain_dynamic_index_trap_3
  cmpq 8(%r10), %rax
  jge .Lmain_dynamic_index_trap_3
  cmpq 32(%r10), %rax
  jae .Lcerune_array_invalid_owner
  imulq 40(%r10), %rax
  movq (%r10), %r11
  subq %rax, %r11
  movq %r11, -176(%rbp)
  jmp .Lmain_dynamic_index_done_3
.Lmain_dynamic_index_trap_3:
  xorl %ecx, %ecx
  callq fflush
  movl $2, %ecx
  leaq .Lcerune_failure_24(%rip), %rdx
  movl $70, %r8d
  callq _write
  movl $7, %ecx
  int $0x29
.Lmain_dynamic_index_done_3:
  callq cerune_fn__ownership13_13
  movq %rax, -104(%rbp)
  movq -104(%rbp), %rax
  movq -176(%rbp), %rcx
  movq %rax, (%rcx)
  movq -16(%rbp), %rax
  movq %rax, -112(%rbp)
  movq -112(%rbp), %rax
  movq %rax, -120(%rbp)
  movq -120(%rbp), %rax
  movq %rax, -128(%rbp)
  movq -128(%rbp), %rcx
  callq cerune_fn__display0_0
  movq -40(%rbp), %rax
  movq %rax, -136(%rbp)
  movq -136(%rbp), %rax
  movq %rax, -144(%rbp)
  movq -144(%rbp), %rax
  movq %rax, -152(%rbp)
  movq -152(%rbp), %rcx
  callq cerune_fn__display1_1
  movq -40(%rbp), %rax
  movq %rax, -160(%rbp)
  movq -160(%rbp), %rcx
  callq cerune_fn__ownership14_14
  movq -16(%rbp), %rax
  movq %rax, -168(%rbp)
  movq -168(%rbp), %rcx
  callq cerune_fn__ownership14_14
  jmp .Lcerune_block_1
.Lcerune_block_1: # main_exit
  xorl %eax, %eax
  addq $224, %rsp
  popq %rbp
  retq

.section .rdata,"dr"
.Lcerune_failure_0:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=30 bytes=78..92\n"
.Lcerune_failure_1:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=59 bytes=93..106\n"
.Lcerune_failure_2:
  .asciz "cerune: runtime-v1 code=integer-overflow node=34 bytes=78..92\n"
.Lcerune_failure_3:
  .asciz "cerune: runtime-v1 code=integer-overflow node=63 bytes=93..106\n"
.Lcerune_failure_4:
  .asciz "cerune: runtime-v1 code=array-range-out-of-bounds node=219 bytes=20..38\n"
.Lcerune_failure_5:
  .asciz "cerune: runtime-v1 code=integer-overflow node=221 bytes=20..38\n"
.Lcerune_failure_6:
  .asciz "cerune: runtime-v1 code=array-length-out-of-range node=224 bytes=20..38\n"
.Lcerune_failure_7:
  .asciz "cerune: runtime-v1 code=allocation-size-overflow node=224 bytes=20..38\n"
.Lcerune_failure_8:
  .asciz "cerune: runtime-v1 code=allocation-limit-exceeded node=224 bytes=20..38\n"
.Lcerune_failure_9:
  .asciz "cerune: runtime-v1 code=allocation-failed node=224 bytes=20..38\n"
.Lcerune_failure_10:
  .asciz "cerune: runtime-v1 code=integer-overflow node=237 bytes=20..38\n"
.Lcerune_failure_11:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=239 bytes=20..38\n"
.Lcerune_failure_12:
  .asciz "cerune: runtime-v1 code=integer-overflow node=233 bytes=20..38\n"
.Lcerune_failure_13:
  .asciz "cerune: runtime-v1 code=array-range-out-of-bounds node=275 bytes=55..61\n"
.Lcerune_failure_14:
  .asciz "cerune: runtime-v1 code=integer-overflow node=277 bytes=55..61\n"
.Lcerune_failure_15:
  .asciz "cerune: runtime-v1 code=array-length-out-of-range node=280 bytes=55..61\n"
.Lcerune_failure_16:
  .asciz "cerune: runtime-v1 code=allocation-size-overflow node=280 bytes=55..61\n"
.Lcerune_failure_17:
  .asciz "cerune: runtime-v1 code=allocation-limit-exceeded node=280 bytes=55..61\n"
.Lcerune_failure_18:
  .asciz "cerune: runtime-v1 code=allocation-failed node=280 bytes=55..61\n"
.Lcerune_failure_19:
  .asciz "cerune: runtime-v1 code=integer-overflow node=293 bytes=55..61\n"
.Lcerune_failure_20:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=295 bytes=55..61\n"
.Lcerune_failure_21:
  .asciz "cerune: runtime-v1 code=integer-overflow node=289 bytes=55..61\n"
.Lcerune_failure_22:
  .asciz "cerune: runtime-v1 code=integer-overflow node=354 bytes=40..62\n"
.Lcerune_failure_23:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=330 bytes=69..72\n"
.Lcerune_failure_24:
  .asciz "cerune: runtime-v1 code=array-index-out-of-bounds node=7 bytes=69..72\n"
