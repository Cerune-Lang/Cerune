use super::{Target, encode, parse};

#[test]
fn instruction_bytes_match_independent_llvm_assembler_reference() {
    // LLVM 22のアセンブラで照合した命令バイト。REX/SIB/変位の境界とSSE2を含みます。
    for (source, hex) in [
        ("pushq %r12", "4154"),
        ("movb $10, (%rcx,%rax)", "c604010a"),
        ("movb %al, (%rcx)", "8801"),
        ("movb $255, 127(%r12,%r9,8)", "43c644cc7fff"),
        ("movb %sil, %r9b", "4188f1"),
        ("movb $1, %r8b", "41b001"),
        ("movb 128(%r13), %sil", "418ab580000000"),
        ("popq %r13", "415d"),
        ("movabsq $-1, %r9", "49b9ffffffffffffffff"),
        ("movq %r9, %r10", "4d89ca"),
        ("movq -128(%rbp), %rax", "488b4580"),
        ("movq -129(%rbp), %rax", "488b857fffffff"),
        ("movq 127(%r12,%r9,8), %r10", "4f8b54cc7f"),
        ("movq 128(%r13), %r8", "4d8b8580000000"),
        ("movzbl %sil, %eax", "400fb6c6"),
        ("movzbq %r8b, %r9", "4d0fb6c8"),
        ("testb $0, (%rsp)", "f6042400"),
        ("setne %r9b", "410f95c1"),
        ("cmovne %r9, %r10", "4d0f45d1"),
        ("cvtsi2ssq %r10, %xmm9", "f34d0f2aca"),
        ("cvtsi2sdq %r10, %xmm9", "f24d0f2aca"),
        ("cvttsd2siq %xmm9, %r10", "f24d0f2cd1"),
        ("movq %r9, %xmm10", "664d0f6ed1"),
        ("movq %xmm9, %r10", "664d0f7eca"),
        ("movss %xmm9, -8(%r13)", "f3450f114df8"),
        ("movsd -8(%r13), %xmm9", "f2450f104df8"),
        ("ucomisd %xmm9, %xmm10", "66450f2ed1"),
        ("btcq $63, %r10", "490fbafa3f"),
        ("shlq %cl, %r10", "49d3e2"),
        ("mulq %r9", "49f7e1"),
        ("divq %r9", "49f7f1"),
        ("idivq %r9", "49f7f9"),
        ("cqto", "4899"),
        ("ud2", "0f0b"),
        ("int $0x29", "cd29"),
        ("retq", "c3"),
    ] {
        let (bytes, fixup) = encode::instruction(source).unwrap();
        assert!(fixup.is_none());
        let actual: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(actual, hex, "{source}");
    }
}

#[test]
fn invalid_forms_and_unresolved_internal_labels_are_diagnostics() {
    for instruction in [
        "vaddps %xmm0, %xmm1",
        "movq %eax, %rbx",
        "movb $256, (%rcx)",
        "movb $-129, %al",
        "movb %eax, (%rcx)",
        "movq %xmm0, %xmm1",
        "movss %rax, %xmm0",
        "shlq %dl, %rax",
        "movq (%rax,%rsp,8), %rax",
        "movq 2147483648(%rbp), %rax",
        "movabsq $18446744073709551616, %rax",
        "setne %rax",
        "int $256",
        "int %rcx",
        "int $0x80",
    ] {
        assert!(encode::instruction(instruction).is_err(), "{instruction}");
    }
    for assembly in [
        ".text\n.Lsame:\n.Lsame:\nretq\n",
        ".text\njmp .Lmissing\n",
        ".section .unknown\n",
        ".text\n.p2align 32\n",
        ".section .rodata\n.byte 256\n",
    ] {
        assert!(parse::assemble(assembly).is_err(), "{assembly}");
    }
}

#[test]
fn branches_resolve_both_directions_and_rip_addends_include_trailing_immediates() {
    let object = parse::assemble(".text\nstart:\njmp done\nud2\ndone:\njmp start\n").unwrap();
    assert_eq!(
        object.sections[0],
        [0xe9, 2, 0, 0, 0, 0x0f, 0x0b, 0xe9, 0xf4, 0xff, 0xff, 0xff]
    );
    assert!(object.relocations.is_empty());
    let object = parse::assemble(
        ".section .rodata\nitem:\n.quad 0\n.text\ntestb $0, item(%rip)\ncallq printf\n",
    )
    .unwrap();
    assert_eq!(object.relocations[0].offset, 2);
    assert_eq!(object.relocations[0].addend, -5);
    assert_eq!(object.relocations[1].addend, -4);
    assert!(object.relocations[1].call);
}

#[test]
fn origins_preserve_encoded_sections_and_relocations_for_all_examples() {
    for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/examples")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "ceru") {
            continue;
        }
        let source = std::fs::read_to_string(path).unwrap();
        let program = crate::compile_to_ir(&source).unwrap();
        for target in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
            let plain = super::super::emit_asm(&program, target).unwrap();
            let annotated = super::super::emit_asm_with_origins(&program, target).unwrap();
            let plain = parse::assemble(&plain).unwrap();
            let annotated = parse::assemble(&annotated).unwrap();
            assert_eq!(plain.sections, annotated.sections);
            assert_eq!(plain.relocations, annotated.relocations);
            let bytes = super::format::write(&annotated, target).unwrap();
            assert_eq!(bytes, super::format::write(&annotated, target).unwrap());
            assert!(
                annotated
                    .symbols
                    .iter()
                    .any(|symbol| symbol.name.starts_with("cerune_origin_n"))
            );
        }
    }
}

#[test]
fn writable_data_is_separate_from_code_and_read_only_literals() {
    let object=parse::assemble(".data\n.p2align 3\nstate:\n.quad 0\n.section .rodata\nliteral:\n.quad 7\n.text\nmovq state(%rip), %rax\nmovq %rax, state(%rip)\nretq\n").unwrap();
    assert_eq!(object.sections[2], [0; 8]);
    assert_eq!(object.sections[1], [7, 0, 0, 0, 0, 0, 0, 0]);
    fn number(bytes: &[u8], at: usize, width: usize) -> u64 {
        bytes[at..at + width]
            .iter()
            .enumerate()
            .fold(0, |n, (i, b)| n | ((*b as u64) << (i * 8)))
    }
    let elf = super::format::write(&object, Target::X86_64UnknownLinuxGnu).unwrap();
    let headers = number(&elf, 40, 8) as usize;
    assert_eq!(number(&elf, headers + 8 * 64 + 8, 8), 3); // SHF_ALLOC | SHF_WRITE、実行不可
    assert_eq!(number(&elf, headers + 2 * 64 + 8, 8), 2); // 読み取り専用
    let coff = super::format::write(&object, Target::X86_64PcWindowsMsvc).unwrap();
    assert_eq!(number(&coff, 2, 2), 3);
    assert_eq!(number(&coff, 20 + 2 * 40 + 36, 4) & 0xe0000000, 0xc0000000); // READ | WRITE
    assert_eq!(number(&coff, 20 + 40 + 36, 4) & 0xe0000000, 0x40000000);
}
