use super::ir::Module;
use std::fmt::Write;

pub(super) const LINUX_SUPPORT: &str = r#"
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

"#;

pub(super) fn emit_data(module: &Module, output: &mut String) {
    for (id, value) in module.strings.iter().enumerate() {
        writeln!(
            output,
            ".p2align 3\n.Lcerune_string_{id}:\n  .quad {}",
            value.len()
        )
        .unwrap();
        for byte in value.bytes() {
            writeln!(output, "  .byte {byte}").unwrap();
        }
    }
}

// 比較は揮発レジスタだけで行い、出力はshadow spaceと16バイト境界を守ります。
pub(super) const SUPPORT: &str = r#"
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

"#;
