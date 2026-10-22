//! Compare typed output against the frozen emitter and independent output bytes.
use super::*;
use crate::codegen::x86_64::{Target, object_reference};
use sashimi_assembler::program::{assemble_program, render_asm};
#[path = "native_runtime_reference/output.rs"]
mod reference;

pub(in crate::codegen::x86_64) fn with_frozen_output(
    asm: &str,
    module: &crate::codegen::x86_64::ir::Module,
) -> String {
    if !module.uses_write {
        return asm.to_owned();
    }
    let current = support(module.target.is_linux());
    assert_eq!(asm.matches(&current).count(), 1);
    asm.replacen(&current, &reference::support(module.target.is_linux()), 1)
}
#[test]
fn typed_output_matches_frozen_objects_and_observation_is_read_only() {
    for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
        let mut p = Program::new(crate::codegen::x86_64::machine::target(target));
        append(&mut p);
        let saved = p.clone();
        let artifact = assemble_program(&p).unwrap();
        assert_eq!(
            artifact.object,
            object_reference::assemble(&reference::support(target.is_linux()), target).unwrap()
        );
        assert_eq!(
            artifact.object,
            object_reference::assemble(&support(target.is_linux()), target).unwrap()
        );
        for (entry, mapping) in p.entries.iter().zip(&artifact.entries) {
            assert!(mapping.provenance.synthetic && mapping.provenance.origins.is_empty());
            if let EntryKind::Data(bytes) = &entry.kind {
                let range = mapping.range.unwrap();
                assert_eq!(range.section, Section::ReadOnlyData);
                assert_eq!((range.end - range.start) as usize, bytes.len());
                assert_eq!(bytes.last(), Some(&0));
            }
        }
        let asm = render_asm(&p).unwrap();
        assert_eq!(asm.contains("callq write@PLT"), target.is_linux());
        assert_eq!(p, saved);
        assert_eq!(assemble_program(&p).unwrap(), artifact);
    }
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
fn typed_output_executes_all_scalar_formats_and_exact_string_bytes() {
    use std::{fs, process::Command, time::Duration};
    let Some(clang) = std::env::var_os("CERUNE_TEST_ASM_CLANG") else {
        eprintln!("typed output execution not run: set CERUNE_TEST_ASM_CLANG");
        return;
    };
    let target = if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    };
    let mut p = Program::new(crate::codegen::x86_64::machine::target(target));
    append(&mut p);
    let origin = local(&mut p, ".Ltest_origin");
    push(&mut p, EntryKind::Section(Section::ReadOnlyData));
    push(&mut p, EntryKind::Label(origin));
    push(&mut p, EntryKind::Data(vec![0]));
    push(&mut p, EntryKind::Section(Section::Text));
    for kind in ["i64", "u64", "f32", "f64", "bool", "string", "quoted"] {
        let entry = p.declare(format!("test_{kind}"), Linkage::Global);
        let callee = local(&mut p, &format!("cerune_output_{kind}"));
        push(&mut p, EntryKind::Label(entry));
        instruction(&mut p, Push, [r64(5)]);
        instruction(&mut p, Mov64, [r64(4), r64(5)]);
        instruction(&mut p, Sub64, [imm(32), r64(4)]);
        instruction(
            &mut p,
            Mov64,
            [r64(if target.is_linux() { 7 } else { 1 }), r64(0)],
        );
        instruction(&mut p, Mov64, [imm(1), r64(10)]);
        instruction(&mut p, Lea64, [rip(origin), r64(11)]);
        instruction(&mut p, Call, [sym(callee)]);
        instruction(&mut p, Add64, [imm(32), r64(4)]);
        instruction(&mut p, Pop, [r64(5)]);
        instruction(&mut p, Ret, []);
    }
    let harness = r#"
#include <stdint.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#endif
extern void test_i64(int64_t),test_u64(uint64_t),test_f32(float),test_f64(double),test_bool(uint64_t);
extern void test_string(const void *),test_quoted(const void *);
int main(void) {
#ifdef _WIN32
    if(_setmode(1,_O_BINARY)==-1) return 1;
#endif
    const struct { uint64_t n; unsigned char bytes[9]; } s={9,{0xe6,0x97,0xa5,0,13,10,34,92,127}};
    test_i64(INT64_MIN); test_u64(UINT64_MAX); test_f32(-0.0f); test_f64(1.25);
    test_bool(0); test_bool(1); test_string(&s); test_quoted(&s);
    return 0;
}
"#;
    let expected = concat!(
        "-9223372036854775808\n18446744073709551615\n-0\n1.25\nfalse\ntrue\n",
        "日\0\r\n\"\\\u{7f}\n",
        "\"日\\0\\r\\n\\\"\\\\\\u{7f}\"\n"
    )
    .as_bytes();
    let dir = std::env::temp_dir().join(format!("cerune-typed-output-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("harness.c"), harness).unwrap();
    fs::write(dir.join("output.s"), render_asm(&p).unwrap()).unwrap();
    fs::write(dir.join("output.o"), assemble_program(&p).unwrap().object).unwrap();
    for ext in ["s", "o"] {
        let exe = dir.join(format!("output-{ext}.exe"));
        let mut cc = Command::new(&clang);
        if target.is_linux() {
            cc.args(["-fPIE", "-pie"]);
        }
        cc.arg(dir.join("harness.c"))
            .arg(dir.join(format!("output.{ext}")))
            .arg("-o")
            .arg(&exe);
        let out = crate::codegen::x86_64::test_process::bounded_output(
            &mut cc,
            &dir,
            "typed-output/link",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = crate::codegen::x86_64::test_process::bounded_output(
            &mut Command::new(exe),
            &dir,
            "typed-output/run",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(out.status.success(), "{ext}: {:?}", out.status);
        assert_eq!(out.stdout, expected, "{ext}");
        assert!(out.stderr.is_empty());
    }
    fs::remove_dir_all(dir).unwrap();
}
