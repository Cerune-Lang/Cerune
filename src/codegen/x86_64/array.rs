//! 配列の要素は既存Native ABIと同じく、基準アドレスから8byteずつ下へ置きます。
//! 共通IRが要素のコピー/逆順解放を展開し、ここでは保存領域と検査だけを生成します。
use super::{
    Target,
    failure::Reporter,
    ir::{Instruction, Type},
};
use crate::runtime::FailureCode as Failure;
use std::fmt::Write;

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

pub(super) fn emit(
    i: &Instruction,
    prefix: &str,
    target: Target,
    reporter: &mut Reporter,
    out: &mut String,
) -> bool {
    let (first, second, third) = if target.is_linux() {
        ("%rdi", "%rsi", "%rdx")
    } else {
        ("%rcx", "%rdx", "%r8")
    };
    match i {
        Instruction::ArrayAllocate {
            width,
            stride,
            label,
        } => {
            let length_ok = format!(".L{prefix}_array_length_ok_{label}");
            writeln!(out, "  testq %rax, %rax\n  jns {length_ok}").unwrap();
            reporter.emit(Failure::ArrayLengthOutOfRange, out);
            writeln!(out, "{length_ok}:").unwrap();
            writeln!(out, "  movq %rax, {first}\n  movabsq \u{24}{width}, {second}\n  movabsq \u{24}{stride}, {third}\n  callq cerune_array_allocate").unwrap();
            let done = format!(".L{prefix}_array_allocated_{label}");
            let limit = format!(".L{prefix}_array_limit_{label}");
            let failed = format!(".L{prefix}_array_failed_{label}");
            writeln!(
                out,
                "  testq %rdx, %rdx\n  je {done}\n  cmpq $1, %rdx\n  jne {limit}"
            )
            .unwrap();
            reporter.emit(Failure::AllocationSizeOverflow, out);
            writeln!(out, "{limit}:\n  cmpq $2, %rdx\n  jne {failed}").unwrap();
            reporter.emit(Failure::AllocationLimitExceeded, out);
            writeln!(out, "{failed}:").unwrap();
            reporter.emit(Failure::AllocationFailed, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::ArrayRangeCheck {
            length_offset,
            start_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_array_range_trap_{label}");
            let done = format!(".L{prefix}_array_range_done_{label}");
            writeln!(out, "  movq {start_offset}(%rbp), %r10\n  testq %r10, %r10\n  js {trap}\n  cmpq %r10, %rax\n  jl {trap}\n  cmpq {length_offset}(%rbp), %rax\n  jg {trap}\n  jmp {done}\n{trap}:").unwrap();
            reporter.emit(Failure::ArrayRangeOutOfBounds, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::DynamicArrayAddress {
            base_offset,
            base_is_pointer,
            destination_offset,
            label,
        } => {
            let trap = format!(".L{prefix}_dynamic_index_trap_{label}");
            let done = format!(".L{prefix}_dynamic_index_done_{label}");
            writeln!(out, "  movq {base_offset}(%rbp), %r10").unwrap();
            if *base_is_pointer {
                out.push_str("  movq (%r10), %r10\n");
            }
            writeln!(out, "  testq %rax, %rax\n  js {trap}\n  testq %r10, %r10\n  je {trap}\n  cmpq 8(%r10), %rax\n  jge {trap}\n  cmpq 32(%r10), %rax\n  jae .Lcerune_array_invalid_owner\n  imulq 40(%r10), %rax\n  movq (%r10), %r11\n  subq %rax, %r11\n  movq %r11, {destination_offset}(%rbp)\n  jmp {done}\n{trap}:").unwrap();
            reporter.emit(Failure::ArrayIndexOutOfBounds, out);
            writeln!(out, "{done}:").unwrap();
        }
        Instruction::ArrayInitAddress {
            owner_offset,
            destination_offset,
        } => {
            writeln!(out, "  movq {owner_offset}(%rbp), {first}\n  callq cerune_array_init_address\n  movq %rax, {destination_offset}(%rbp)").unwrap();
        }
        Instruction::ArrayInitialized { owner_offset } => {
            writeln!(
                out,
                "  movq {owner_offset}(%rbp), %r10\n  addq $1, 32(%r10)"
            )
            .unwrap();
        }
        Instruction::ArrayLength
        | Instruction::ArrayRetain
        | Instruction::ArrayReleaseOwner
        | Instruction::ArrayFree => {
            let name = match i {
                Instruction::ArrayLength => "length",
                Instruction::ArrayRetain => "retain",
                Instruction::ArrayReleaseOwner => "release_owner",
                Instruction::ArrayFree => "free",
                _ => unreachable!(),
            };
            writeln!(out, "  movq %rax, {first}\n  callq cerune_array_{name}").unwrap();
        }
        Instruction::LoadFromPointer { ty, pointer_offset } => {
            writeln!(out, "  movq {pointer_offset}(%rbp), %r11").unwrap();
            out.push_str(match ty {
                Type::F32 => "  movss (%r11), %xmm0\n",
                Type::F64 => "  movsd (%r11), %xmm0\n",
                _ => "  movq (%r11), %rax\n",
            });
        }
        Instruction::CopyFromPointer {
            pointer_offset,
            destination_offset,
            slots,
        } => {
            writeln!(out, "  movq {pointer_offset}(%rbp), %r11").unwrap();
            for slot in 0..*slots {
                let source = -8 * slot as isize;
                let destination = destination_offset + source;
                writeln!(
                    out,
                    "  movq {source}(%r11), %r10\n  movq %r10, {destination}(%rbp)"
                )
                .unwrap();
            }
        }
        _ => return false,
    }
    true
}

// owner: data-high/length/refs/logical-bytes/initialized/stride (各8bytes)、続いて要素領域。
// 空配列はnull。確保は一度だけ行い、失敗時は予算を加算しません。
// RAX=owner, RDX=error (0/size/limit/allocation) は処理系内部だけの戻り値規約です。
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

#[cfg(test)]
#[path = "array_tests.rs"]
mod tests;
