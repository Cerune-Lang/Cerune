use super::*;
use crate::codegen::x86_64::{Target, object_reference};
use sashimi_assembler::program::{SymbolId, assemble_program, render_asm};
#[path = "native_data_reference.rs"]
mod reference;
const TARGETS: [Target; 2] = [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc];

fn sample(target: Target) -> Module {
    let hir = crate::compile_to_ir("").unwrap();
    let mut module =
        crate::codegen::x86_64::lower_mir(&crate::mir::lower(&hir).unwrap(), target).unwrap();
    module.uses_strings = true;
    module.strings = vec![
        "".into(),
        "日本\0\r\n".into(),
        "é".into(),
        "e\u{301}".into(),
        "abc".into(),
    ];
    let f32s = [
        0,
        0x80000000,
        1,
        0x7f800000,
        0xff800000,
        0x7fc12345,
        0x7f812345,
        u32::MAX,
    ];
    let f64s = [
        0,
        0x8000000000000000,
        1,
        0x7ff0000000000000,
        0xfff0000000000000,
        0x7ff80123456789ab,
        0x7ff00123456789ab,
        u64::MAX,
    ];
    module.float_constants = f32s
        .into_iter()
        .enumerate()
        .map(|(id, bits)| FloatConstant::F32 { id, bits })
        .chain(
            f64s.into_iter()
                .enumerate()
                .map(|(id, bits)| FloatConstant::F64 { id, bits }),
        )
        .collect();
    module
}
fn number(bytes: &[u8], at: usize, width: usize) -> u64 {
    let mut value = [0u8; 8];
    value[..width].copy_from_slice(&bytes[at..at + width]);
    u64::from_le_bytes(value)
}
fn rodata(bytes: &[u8], target: Target) -> &[u8] {
    let (at, len) = if target == TARGETS[0] {
        let h = number(bytes, 40, 8) as usize + 2 * 64;
        (number(bytes, h + 24, 8), number(bytes, h + 32, 8))
    } else {
        (
            number(bytes, 20 + 40 + 20, 4),
            number(bytes, 20 + 40 + 16, 4),
        )
    };
    &bytes[at as usize..(at + len) as usize]
}
#[test]
fn constants_keep_exact_objects_bits_alignment_and_empty_content_ranges() {
    for target in TARGETS {
        let module = sample(target);
        let mut program = Program::new(super::super::machine::target(target));
        append(&mut program, &module);
        let saved = program.clone();
        let artifact = assemble_program(&program).unwrap();
        assert_eq!(
            artifact.object,
            object_reference::assemble(&reference::render(&module), target).unwrap()
        );
        assert_eq!(
            artifact.object,
            object_reference::assemble(&render(&module), target).unwrap()
        );
        let bytes = rodata(&artifact.object, target);
        let position = |name: &str| {
            let id = program.symbols.iter().position(|s| s.name == name).unwrap();
            let position = artifact.symbols[id].position.unwrap();
            assert_eq!(position.section, Section::ReadOnlyData);
            position.offset as usize
        };
        assert_eq!(
            &bytes[..16],
            &[0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            &bytes[16..32],
            &[0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        for (id, s) in module.strings.iter().enumerate() {
            let at = position(&format!(".Lcerune_string_{id}"));
            assert_eq!(at % 8, 0);
            assert_eq!(number(bytes, at, 8), s.len() as u64);
            assert_eq!(&bytes[at + 8..at + 8 + s.len()], s.as_bytes());
        }
        for c in &module.float_constants {
            let (name, bits, width) = match c {
                FloatConstant::F32 { id, bits } => (format!(".Lcerune_f32_{id}"), *bits as u64, 4),
                FloatConstant::F64 { id, bits } => (format!(".Lcerune_f64_{id}"), *bits, 8),
            };
            let at = position(&name);
            assert_eq!(at % width, 0);
            assert_eq!(number(bytes, at, width), bits);
        }
        assert!(program.entries.iter().zip(&artifact.entries).any(|(e, m)| {
            matches!(&e.kind,EntryKind::Data(b) if b.is_empty())
                && m.range
                    .is_some_and(|r| r.start == r.end && r.section == Section::ReadOnlyData)
        }));
        assert!(
            artifact
                .entries
                .iter()
                .all(|e| e.provenance.synthetic && e.provenance.origins.is_empty())
        );
        let asm = render_asm(&program).unwrap();
        assert!(asm.contains(".long 0x7FC12345"));
        assert!(asm.contains(".quad 0x7FF80123456789AB"));
        assert_eq!(program, saved);
        assert_eq!(assemble_program(&program).unwrap(), artifact);
    }
}

/// Reconstruct the old constant data before comparing complete program objects.
pub(in crate::codegen::x86_64) fn with_frozen_data(asm: &str, module: &Module) -> String {
    let current = render(module);
    assert_eq!(
        asm.matches(&current).count(),
        1,
        "constant prefix must be present exactly once"
    );
    asm.replacen(&current, &reference::render(module), 1)
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "linux", target_os = "windows")
))]
fn typed_data_links_and_preserves_bits_with_external_assembler() {
    use super::super::machine::{instruction, r64};
    use Opcode::*;
    use Operand::RipRelative as rip;
    use std::{fs, process::Command, time::Duration};
    let Some(clang) = std::env::var_os("CERUNE_TEST_ASM_CLANG") else {
        eprintln!("typed data execution not run: set CERUNE_TEST_ASM_CLANG");
        return;
    };
    let target = if cfg!(target_os = "windows") {
        TARGETS[1]
    } else {
        TARGETS[0]
    };
    let module = sample(target);
    let mut p = Program::new(super::super::machine::target(target));
    append(&mut p, &module);
    let data_count = p.symbols.len();
    let mut harness = String::from("#include <stdint.h>\n#include <string.h>\n");
    for i in 0..data_count {
        harness.push_str(&format!("extern const unsigned char *get_{i}(void);\n"));
    }
    harness.push_str("int main(void) { uint32_t u32; uint64_t u64;\n");
    for i in 0..data_count {
        let name = p.symbols[i].name.clone();
        let function = p.declare(format!("get_{i}"), Linkage::Global);
        push(&mut p, EntryKind::Section(Section::Text));
        push(&mut p, EntryKind::Label(function));
        instruction(&mut p, Lea64, [rip(SymbolId(i)), r64(0)]);
        instruction(&mut p, Ret, []);
        let check = if name == ".Lcerune_sign_f32" {
            format!("memcpy(&u32,get_{i}(),4); if(u32!=UINT32_C(2147483648)) return 1;")
        } else if name == ".Lcerune_sign_f64" {
            format!("memcpy(&u64,get_{i}(),8); if(u64!=UINT64_C(9223372036854775808)) return 2;")
        } else if let Some(id) = name.strip_prefix(".Lcerune_string_") {
            let value = &module.strings[id.parse::<usize>().unwrap()];
            let contents = if value.is_empty() {
                "0".into()
            } else {
                value
                    .bytes()
                    .map(|b| b.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            format!(
                "{{ static const unsigned char expected[]={{{contents}}}; memcpy(&u64,get_{i}(),8); if(u64!={len} || memcmp(get_{i}()+8,expected,{len})) return 3; }}",
                len = value.len()
            )
        } else {
            let c = module
                .float_constants
                .iter()
                .find(|c| match c {
                    FloatConstant::F32 { id, .. } => name == format!(".Lcerune_f32_{id}"),
                    FloatConstant::F64 { id, .. } => name == format!(".Lcerune_f64_{id}"),
                })
                .unwrap();
            match c {
                FloatConstant::F32 { bits, .. } => {
                    format!("memcpy(&u32,get_{i}(),4); if(u32!=UINT32_C({bits})) return 4;")
                }
                FloatConstant::F64 { bits, .. } => {
                    format!("memcpy(&u64,get_{i}(),8); if(u64!=UINT64_C({bits})) return 5;")
                }
            }
        };
        harness.push_str(&check);
        harness.push('\n');
    }
    harness.push_str("return 0;\n}\n");
    let dir = std::env::temp_dir().join(format!("cerune-typed-data-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("harness.c"), harness).unwrap();
    fs::write(dir.join("data.o"), assemble_program(&p).unwrap().object).unwrap();
    fs::write(dir.join("data.s"), render_asm(&p).unwrap()).unwrap();
    for ext in ["o", "s"] {
        let exe = dir.join(format!("data-{ext}.exe"));
        let mut cmd = Command::new(&clang);
        if target == TARGETS[0] {
            cmd.args(["-fPIE", "-pie"]);
        }
        cmd.arg(dir.join("harness.c"))
            .arg(dir.join(format!("data.{ext}")))
            .arg("-o")
            .arg(&exe);
        let out = super::super::test_process::bounded_output(
            &mut cmd,
            &dir,
            "typed-data/link",
            Duration::from_secs(120),
        )
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = super::super::test_process::bounded_output(
            &mut Command::new(exe),
            &dir,
            "typed-data/run",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(out.status.success(), "{ext}: {:?}", out.status);
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
    fs::remove_dir_all(dir).unwrap();
}
