//! Frozen from Cerune bf81a78; test-only migration reference.
pub(super) fn support(limit: u64, linux: bool) -> String {
    SOURCE
        .replace("@LIMIT@", &limit.to_string())
        .replace("@FIRST@", if linux { "%rdi" } else { "%rcx" })
        .replace("@SECOND@", if linux { "%rsi" } else { "%rdx" })
        .replace("@THIRD@", if linux { "%rdx" } else { "%r8" })
        .replace(
            "@TRAP@",
            if linux {
                "  ud2"
            } else {
                "  movl $7, %ecx\n  int $0x29"
            },
        )
}
const SOURCE: &str = r#"
.data
.p2align 3
.Lcerune_array_live:
  .quad 0
.Lcerune_array_limit:
  .quad @LIMIT@
.Lcerune_array_max:
  .quad 9223372036854775807
.Lcerune_array_physical_max:
  .quad 9223372036854775759
.text
cerune_array_allocate:
  pushq %rbp
  movq %rsp, %rbp
  subq $64, %rsp
  movq @FIRST@, %rax
  movq @SECOND@, %r10
  movq @THIRD@, %r11
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
  leaq 48(%rax), @FIRST@
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
  testq @FIRST@, @FIRST@
  je .Lcerune_array_length_done
  movq 8(@FIRST@), %rax
.Lcerune_array_length_done:
  retq
cerune_array_init_address:
  movq @FIRST@, %r10
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
  movq @FIRST@, %r10
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
  movq @FIRST@, %r10
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
  testq @FIRST@, @FIRST@
  je .Lcerune_array_free_empty
  cmpq $0, 16(@FIRST@)
  jne .Lcerune_array_invalid_owner
  pushq %rbp
  movq %rsp, %rbp
  subq $32, %rsp
  movq 24(@FIRST@), %r10
  subq %r10, .Lcerune_array_live(%rip)
  callq free
  addq $32, %rsp
  popq %rbp
.Lcerune_array_free_empty:
  retq
.Lcerune_array_invalid_owner:
@TRAP@
"#;
