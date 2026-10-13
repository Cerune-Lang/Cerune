//! malloc失敗・サイズ計算・解放後の予算を両符号化経路で検証します。
use super::super::{Target, emit, ir::Instruction, lower_with_target, object};
use crate::{compile_to_ir, ir_executor};
#[path = "../../../tests/support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "../../../tests/support/native_process.rs"]
mod native_process;
use super::super::test_process as process;

fn for_artifacts(
    mut build: impl FnMut(Target) -> String,
    mut check: impl FnMut(std::process::Output),
) {
    for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
        let text = build(target);
        let object = object::assemble(&text, target).unwrap();
        if target.is_linux() != cfg!(target_os = "linux") {
            continue;
        }
        for (bytes, extension) in [(text.as_bytes(), "s"), (object.as_slice(), "o")] {
            if let Some(output) = native_process::execute(bytes, extension) {
                check(output);
            }
        }
    }
}
fn failure(actual: std::process::Output, code: &str, record: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(actual.status.signal(), Some(4), "{actual:?}");
    }
    #[cfg(windows)]
    assert_eq!(
        actual.status.code().map(|c| c as u32),
        Some(0xc0000409),
        "{actual:?}"
    );
    assert_eq!(
        String::from_utf8(actual.stdout)
            .unwrap()
            .replace("\r\n", "\n"),
        "7\n"
    );
    assert_eq!(
        String::from_utf8(actual.stderr)
            .unwrap()
            .replace("\r\n", "\n"),
        format!(
            "cerune: {}\n",
            record.replace("allocation-limit-exceeded", code)
        )
    );
}
#[test]
fn allocation_size_and_malloc_failures_preserve_origin_and_prior_output() {
    let mut p = compile_to_ir("print(7); print(array_copy([1]));").unwrap();
    p.array_heap_limit = 0;
    let record = ir_executor::run(&p)
        .unwrap_err()
        .runtime_failure()
        .unwrap()
        .record();
    for (length, width, stride) in [(-1, 8, 8), (i64::MAX, 8, 8), (i64::MAX, 0, 8)] {
        for_artifacts(
            |target| {
                let mut module = lower_with_target(&p, target).unwrap();
                let mut changed = false;
                for (instructions, origins, mir_origins) in std::iter::once((
                    &mut module.instructions,
                    &mut module.origins,
                    &mut module.mir_origins,
                ))
                .chain(
                    module
                        .functions
                        .iter_mut()
                        .map(|f| (&mut f.instructions, &mut f.origins, &mut f.mir_origins)),
                ) {
                    if let Some(index) = instructions
                        .iter()
                        .position(|i| matches!(i, Instruction::ArrayAllocate { .. }))
                    {
                        let Instruction::ArrayAllocate { label, .. } = instructions[index] else {
                            unreachable!()
                        };
                        instructions[index] = Instruction::ArrayAllocate {
                            width,
                            stride,
                            label,
                        };
                        instructions.insert(index, Instruction::MovI64ImmediateToRax(length));
                        origins.insert(index, origins[index]);
                        mir_origins.insert(index, mir_origins[index]);
                        changed = true;
                        break;
                    }
                }
                assert!(changed);
                emit::emit_with_origins(&module, true)
            },
            |actual| {
                failure(
                    actual,
                    if length < 0 {
                        "array-length-out-of-range"
                    } else {
                        "allocation-size-overflow"
                    },
                    &record,
                )
            },
        );
    }
    p.array_heap_limit = 64;
    for_artifacts(
        |target| {
            let text = emit::emit_with_origins(&lower_with_target(&p, target).unwrap(), true);
            format!(
                "{}\n.text\ncerune_test_malloc:\n  xorl %eax, %eax\n  retq\n",
                text.replace("callq malloc", "callq cerune_test_malloc")
            )
        },
        |actual| failure(actual, "allocation-failed", &record),
    );
}
#[test]
fn empty_zero_width_and_owner_lifecycle_keep_budget_consistent() {
    for_artifacts(
        |target| {
            let (a, b, c) = if target.is_linux() {
                ("%rdi", "%rsi", "%rdx")
            } else {
                ("%rcx", "%rdx", "%r8")
            };
            let call = |length, width, stride| {
                format!(
                    "  movabsq \u{24}{length}, {a}\n  movabsq \u{24}{width}, {b}\n  movabsq \u{24}{stride}, {c}\n  callq cerune_array_allocate\n"
                )
            };
            let mut text = super::support(0, target.is_linux());
            text.push_str(
                ".globl main\nmain:\n  pushq %rbp\n  movq %rsp, %rbp\n  subq $48, %rsp\n",
            );
            text.push_str(&call(0, 8, 8));
            text.push_str(
                "  testq %rax, %rax\n  jne .Ltest_fail\n  testq %rdx, %rdx\n  jne .Ltest_fail\n",
            );
            text.push_str(&call(3, 0, 0));
            text.push_str("  testq %rax, %rax\n  je .Ltest_fail\n  testq %rdx, %rdx\n  jne .Ltest_fail\n  movq %rax, -8(%rbp)\n");
            text.push_str(&format!("  movq %rax, {a}\n  callq cerune_array_length\n  cmpq $3, %rax\n  jne .Ltest_fail\n"));
            text.push_str(&format!("  movq -8(%rbp), {a}\n  callq cerune_array_retain\n  movq -8(%rbp), {a}\n  callq cerune_array_release_owner\n  testq %rax, %rax\n  jne .Ltest_fail\n  movq -8(%rbp), {a}\n  callq cerune_array_release_owner\n  cmpq $1, %rax\n  jne .Ltest_fail\n"));
            text.push_str(&format!("  movq -8(%rbp), {a}\n  callq cerune_array_free\n  cmpq $0, .Lcerune_array_live(%rip)\n  jne .Ltest_fail\n  xorl %eax, %eax\n  jmp .Ltest_done\n.Ltest_fail:\n  movl $99, %eax\n.Ltest_done:\n  addq $48, %rsp\n  popq %rbp\n  retq\n"));
            text
        },
        |actual| {
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty());
        },
    );
}
#[test]
fn repeated_success_releases_nested_arrays_and_strings() {
    let p = compile_to_ir(
        r#"
        fn make()->[[string]]{return array_copy([array_copy([concat("日","本")])]);}
        fn main()->void{a:infer=make(); b:infer=a; print(b);}
    "#,
    )
    .unwrap();
    for_artifacts(
        |target| {
            let mut text = emit::emit_with_origins(&lower_with_target(&p, target).unwrap(), true)
                .replace("\nmain:\n", "\ncerune_test_main:\n");
            text.push_str("\n.text\nmain:\n  pushq %rbp\n  movq %rsp, %rbp\n  subq $32, %rsp\n");
            for _ in 0..3 {
                text.push_str("  callq cerune_test_main\n  cmpq $0, .Lcerune_array_live(%rip)\n  jne .Ltest_leaked\n  cmpq $0, .Lcerune_heap_live(%rip)\n  jne .Ltest_leaked\n  cmpq $0, .Lcerune_heap_head(%rip)\n  jne .Ltest_leaked\n");
            }
            text.push_str("  xorl %eax, %eax\n  jmp .Ltest_done\n.Ltest_leaked:\n  movl $99, %eax\n.Ltest_done:\n  addq $32, %rsp\n  popq %rbp\n  retq\n");
            text
        },
        |actual| {
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty());
            assert_eq!(actual.stdout, "[[\"日本\"]]\n".repeat(3).as_bytes());
        },
    );
}
