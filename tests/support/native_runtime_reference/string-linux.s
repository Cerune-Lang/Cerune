
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

