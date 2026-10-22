//! Compare production objects with the frozen encoder and pre-migration runtime.
use super::super::{emit_asm_from_lir, lower_mir, object_reference};
use super::{Target, assemble};
use std::path::{Path, PathBuf};
const TARGETS: [Target; 2] = [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu];
fn compare(asm: &str, baseline: &str, target: Target, context: &str) -> bool {
    let original = super::super::machine::tests::with_frozen_runtime(baseline, target);
    let expected =
        object_reference::assemble(&original, target).map_err(|e| e.message().to_owned());
    let actual = assemble(asm, target).map_err(|e| e.message().to_owned());
    match (&expected, &actual) {
        (Ok(a), Ok(b)) => {
            assert_eq!(a.len(), b.len(), "{context} {target:?}: object size");
            assert!(
                a == b,
                "{context} {target:?}: first differing byte {:?}",
                a.iter().zip(b).position(|(a, b)| a != b)
            );
        }
        _ => assert_eq!(expected, actual, "{context} {target:?}"),
    }
    expected.is_ok()
}
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "ceru") {
            out.push(path);
        }
    }
}
#[test]
fn all_examples_keep_exact_objects_with_both_targets_ssa_and_origins() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut examples = Vec::new();
    files(&root.join("examples"), &mut examples);
    examples.sort();
    let mut count = 0;
    for file in examples {
        if file.parent() == Some(root.join("examples/source_files").as_path()) {
            continue;
        }
        let hir = crate::modules::load(&file).unwrap().to_ir().unwrap();
        let mir = crate::mir::lower(&hir).unwrap();
        let ssa = crate::mir::ssa::lower(&crate::mir::ssa::construct(&mir).unwrap())
            .unwrap()
            .program;
        for program in [&mir, &ssa] {
            for target in TARGETS {
                let lir = lower_mir(program, target).unwrap();
                for origins in [false, true] {
                    let asm = emit_asm_from_lir(&lir, origins).unwrap();
                    let baseline = super::super::data::tests::with_frozen_data(&asm, &lir);
                    let baseline = super::super::heap::tests::with_frozen_owners(&baseline, &lir);
                    assert!(
                        compare(&asm, &baseline, target, &file.display().to_string()),
                        "{} must assemble",
                        file.display()
                    );
                }
            }
        }
        count += 1;
    }
    assert!(count >= 120);
    eprintln!("SASHIMI: {count} runnable examples, two targets, MIR/SSA, with/without origins");
}
#[test]
fn adapter_preserves_diagnostics_and_relocations() {
    for asm in [
        ".text\ncallq printf\nretq\n",
        ".text\n.globl cerune_origin_n1\ncerune_origin_n1:\n.globl cerune_origin_n2\ncerune_origin_n2:\nretq\n",
        ".data\n.p2align 3\nstate:\n.quad 0\n.section .rodata\ntext:\n.asciz \"ok\"\n.text\nmovq state(%rip), %rax\nmovq %rax, state(%rip)\nleaq text(%rip), %rcx\ncallq puts\nretq\n",
        ".text\nstart:\njmp done\nud2\ndone:\njmp start\n",
        ".section .rodata\nitem:\n.quad 0\n.text\ntestb $0, item(%rip)\ncallq printf\n",
        ".text\nvaddps %xmm0, %xmm1\n",
        ".text\nmovq %eax, %rbx\n",
        ".text\nmovb $256, (%rcx)\n",
        ".text\nmovq (%rax,%rsp,8), %rax\n",
        ".text\nmovq 2147483648(%rbp), %rax\n",
        ".text\nmovabsq $18446744073709551616, %rax\n",
        ".text\n.Lsame:\n.Lsame:\nretq\n",
        ".text\njmp .Lmissing\n",
        ".text\ncallq cerune_missing\n",
        ".section .unknown\n",
        ".text\n.p2align 32\n",
        ".section .rodata\n.byte 256\n",
        ".text\nint $0x80\n",
    ] {
        for target in TARGETS {
            compare(asm, asm, target, asm);
        }
    }
}
