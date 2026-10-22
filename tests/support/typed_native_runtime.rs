//! Verify migration against fixed pre-migration ASM and independent byte semantics.
use super::super::{Target, object_reference, string, utf8};
use super::*;
use sashimi_assembler::program::{ReferenceKind, assemble_program, render_asm};

const TARGETS: [Target; 2] = [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc];
fn helpers(target: Target) -> Program {
    let mut p = Program::new(super::target(target));
    utf8::append(&mut p);
    string::append(&mut p);
    p
}
fn reference(target: Target) -> String {
    format!(
        "{}{}",
        include_str!("native_runtime_reference/utf8.s"),
        if target == Target::X86_64UnknownLinuxGnu {
            include_str!("native_runtime_reference/string-linux.s")
        } else {
            include_str!("native_runtime_reference/string-windows.s")
        }
    )
}
#[test]
fn typed_helpers_match_frozen_objects_and_preserve_synthetic_ranges() {
    for target in TARGETS {
        let p = helpers(target);
        let saved = p.clone();
        let a = assemble_program(&p).unwrap();
        assert_eq!(
            a.object,
            object_reference::assemble(&reference(target), target).unwrap()
        );
        assert!(
            a.references
                .iter()
                .all(|r| r.kind == ReferenceKind::ResolvedRelative)
        );
        assert_eq!(a.entries.len(), p.entries.len());
        for e in &a.entries {
            assert!(e.provenance.synthetic && e.provenance.origins.is_empty());
            match p.entries[e.entry.0].kind {
                EntryKind::Instruction(_) => {
                    let range = e.range.unwrap();
                    assert_eq!(range.section, Section::Text);
                    assert!(range.end > range.start);
                }
                EntryKind::Label(_) => {
                    let range = e.range.unwrap();
                    assert_eq!(range.start, range.end);
                }
                EntryKind::Section(_) => assert_eq!(e.range, None),
                _ => {}
            }
        }
        render_asm(&p).unwrap();
        assert_eq!(p, saved);
        assert_eq!(assemble_program(&p).unwrap(), a);
        let assembly = format!(
            "{}{}",
            render_support(target, utf8::append),
            render_support(target, string::append)
        );
        assert_eq!(
            object_reference::assemble(&assembly, target).unwrap(),
            a.object
        );
    }
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
fn typed_helpers_execute_utf8_and_raw_string_cases_in_asm_and_object() {
    use Opcode::*;
    use Operand::Symbol as sym;
    use sashimi_assembler::program::SymbolId;
    use std::{fs, process::Command, time::Duration};
    let Some(clang) = std::env::var_os("CERUNE_TEST_ASM_CLANG") else {
        eprintln!("typed runtime native execution not run: set CERUNE_TEST_ASM_CLANG");
        return;
    };
    let target = if cfg!(target_os = "windows") {
        TARGETS[1]
    } else {
        TARGETS[0]
    };
    let mut p = helpers(target);
    let lookup = |name: &str| SymbolId(p.symbols.iter().position(|s| s.name == name).unwrap());
    let utf8 = lookup("cerune_utf8");
    let equal = lookup("cerune_string_equal");
    let wrapper = p.declare("cerune_test_utf8", Linkage::Global);
    let equality = p.declare("cerune_test_equal", Linkage::Global);
    push(&mut p, EntryKind::Section(Section::Text));
    push(&mut p, EntryKind::Label(wrapper));
    if target == TARGETS[1] {
        instruction(&mut p, Mov64, [r64(1), r64(0)]);
        instruction(&mut p, Mov64, [r64(2), r64(1)]);
        instruction(&mut p, Mov64, [r64(0), r64(2)]);
    } else {
        instruction(&mut p, Mov64, [r64(7), r64(2)]);
        instruction(&mut p, Mov64, [r64(6), r64(1)]);
    }
    // Tail branches preserve the caller's stack and use only volatile registers.
    instruction(&mut p, Jump, [sym(utf8)]);
    push(&mut p, EntryKind::Label(equality));
    instruction(&mut p, Jump, [sym(equal)]);
    let special = [
        b"".as_slice(),
        b"hello\0\r\n",
        "日本語".as_bytes(),
        "é".as_bytes(),
        "e\u{301}".as_bytes(),
        &[0xe0, 0xa0, 0x80],
        &[0xe0, 0x9f, 0xbf],
        &[0xed, 0x9f, 0xbf],
        &[0xed, 0xa0, 0x80],
        &[0xef, 0xbf, 0xbf],
        &[0xf0, 0x90, 0x80, 0x80],
        &[0xf0, 0x8f, 0xbf, 0xbf],
        &[0xf4, 0x8f, 0xbf, 0xbf],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xe2, 0x82],
        &[0xf0, 0x90, 0x80],
        &[0xe2, 0x28, 0xa1],
        &[0xf0, 0x90, 0x41, 0x80],
    ];
    let mut expected = Vec::new();
    let mut harness = String::from(
        "#include <stdint.h>\n#include <stdio.h>\nextern int cerune_test_utf8(const unsigned char*, uint64_t);\nextern int cerune_test_equal(const void*, const void*);\nstruct S { uint64_t length; unsigned char bytes[32]; };\nint main(void) {\n",
    );
    harness.push_str("unsigned char b[2]; for (unsigned x=0;x<256;x++) { b[0]=(unsigned char)x; putchar('0'+cerune_test_utf8(b,1)); } for (unsigned x=0;x<65536;x++) { b[0]=(unsigned char)(x>>8); b[1]=(unsigned char)x; putchar('0'+cerune_test_utf8(b,2)); }\n");
    for b in 0..=255u8 {
        expected.push(b'0' + u8::from(std::str::from_utf8(&[b]).is_ok()));
    }
    for n in 0..=65535u32 {
        expected.push(b'0' + u8::from(std::str::from_utf8(&[(n >> 8) as u8, n as u8]).is_ok()));
    }
    for (i, bytes) in special.iter().enumerate() {
        let values = if bytes.is_empty() {
            "0".to_owned()
        } else {
            bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        harness.push_str(&format!("static const unsigned char u{i}[] = {{{values}}}; putchar('0'+cerune_test_utf8(u{i},{}));\n",bytes.len()));
        expected.push(b'0' + u8::from(std::str::from_utf8(bytes).is_ok()));
    }
    let strings = [
        b"".as_slice(),
        b"a",
        b"a\0b",
        b"a\0c",
        b"a\r\n",
        b"a\n",
        "日本語".as_bytes(),
        "é".as_bytes(),
        "e\u{301}".as_bytes(),
    ];
    for (i, bytes) in strings.iter().enumerate() {
        let values = if bytes.is_empty() {
            "0".to_owned()
        } else {
            bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        harness.push_str(&format!(
            "static const struct S s{i} = {{{}, {{{values}}}}};\n",
            bytes.len()
        ));
    }
    for (i, a) in strings.iter().enumerate() {
        for (j, b) in strings.iter().enumerate() {
            harness.push_str(&format!("putchar('0'+cerune_test_equal(&s{i},&s{j}));\n"));
            expected.push(b'0' + u8::from(a == b));
        }
    }
    harness.push_str("return 0;\n}\n");
    let dir = std::env::temp_dir().join(format!("cerune-typed-runtime-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("harness.c"), harness).unwrap();
    fs::write(dir.join("runtime.o"), assemble_program(&p).unwrap().object).unwrap();
    fs::write(dir.join("runtime.s"), render_asm(&p).unwrap()).unwrap();
    for extension in ["o", "s"] {
        let executable = dir.join(format!("runtime-{extension}.exe"));
        let mut command = Command::new(&clang);
        if target == TARGETS[0] {
            command.args(["-fPIE", "-pie"]);
        }
        command
            .arg(dir.join("harness.c"))
            .arg(dir.join(format!("runtime.{extension}")))
            .arg("-o")
            .arg(&executable);
        let output = super::super::test_process::bounded_output(
            &mut command,
            &dir,
            "typed-runtime/link",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = super::super::test_process::bounded_output(
            &mut Command::new(&executable),
            &dir,
            "typed-runtime/run",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(
            output.stdout, expected,
            "{extension}: independent UTF-8/string expectations"
        );
        assert!(output.stderr.is_empty());
    }
    fs::remove_dir_all(dir).unwrap();
}

/// Reconstruct only the migrated helpers from the fixed pre-migration text.
pub(in crate::codegen::x86_64) fn with_frozen_runtime(asm: &str, target: Target) -> String {
    asm.replace(
        &render_support(target, utf8::append),
        include_str!("native_runtime_reference/utf8.s"),
    )
    .replace(
        &render_support(target, string::append),
        if target == Target::X86_64UnknownLinuxGnu {
            include_str!("native_runtime_reference/string-linux.s")
        } else {
            include_str!("native_runtime_reference/string-windows.s")
        },
    )
}
