
.text
cerune_utf8:
  movq $0, %r8
  movq $128, %r9
  movq $191, %r10
  xorl %r11d, %r11d
.Largs_utf8_utf8_loop:
  cmpq %rcx, %r11
  je .Largs_utf8_utf8_end
  movzbl (%rdx,%r11), %eax
  cmpq $0, %r8
  je .Largs_utf8_lead
  cmpq %r9, %rax
  jb .Largs_utf8_utf8_failed
  cmpq %r10, %rax
  ja .Largs_utf8_utf8_failed
  subq $1, %r8
  movq $128, %r9
  movq $191, %r10
  jmp .Largs_utf8_utf8_next
.Largs_utf8_lead:
  cmpl $128, %eax
  jb .Largs_utf8_utf8_next
  cmpl $194, %eax
  jb .Largs_utf8_utf8_failed
  cmpl $224, %eax
  jae .Largs_utf8_three
  movq $1, %r8
  jmp .Largs_utf8_utf8_next
.Largs_utf8_three:
  cmpl $240, %eax
  jae .Largs_utf8_four
  movq $2, %r8
  cmpl $224, %eax
  jne .Largs_utf8_surrogate
  movq $160, %r9
.Largs_utf8_surrogate:
  cmpl $237, %eax
  jne .Largs_utf8_utf8_next
  movq $159, %r10
  jmp .Largs_utf8_utf8_next
.Largs_utf8_four:
  cmpl $244, %eax
  ja .Largs_utf8_utf8_failed
  movq $3, %r8
  cmpl $240, %eax
  jne .Largs_utf8_maximum
  movq $144, %r9
.Largs_utf8_maximum:
  cmpl $244, %eax
  jne .Largs_utf8_utf8_next
  movq $143, %r10
.Largs_utf8_utf8_next:
  addq $1, %r11
  jmp .Largs_utf8_utf8_loop
.Largs_utf8_utf8_end:
  cmpq $0, %r8
  jne .Largs_utf8_utf8_failed
  movl $1, %eax
  retq
.Largs_utf8_utf8_failed:
  xorl %eax, %eax
  retq
