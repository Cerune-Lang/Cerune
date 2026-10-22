//! Frozen migration comparisons and direct typed-Program execution.
use crate::codegen::x86_64::{Target, array, heap, ir::Module, machine::*, object_reference};
use sashimi_assembler::program::{assemble_program, render_asm};
#[path = "native_runtime_reference/array.rs"]
mod old_array;
#[path = "native_runtime_reference/heap.rs"]
mod old_heap;

pub(in crate::codegen::x86_64) fn with_frozen_owners(asm: &str, module: &Module) -> String {
    let mut text = asm.to_owned();
    let linux = module.target.is_linux();
    if let Some(limit) = module.string_heap_limit {
        let current = heap::support(limit, linux);
        assert_eq!(text.matches(&current).count(), 1);
        text = text.replacen(&current, &old_heap::support(limit, linux), 1);
    }
    if let Some(limit) = module.array_heap_limit {
        let current = array::support(limit, linux);
        assert_eq!(text.matches(&current).count(), 1);
        text = text.replacen(&current, &old_array::support(limit, linux), 1);
    }
    text
}
#[test]
fn typed_owners_preserve_objects_state_ranges_and_shared_externals() {
    for target in [Target::X86_64UnknownLinuxGnu, Target::X86_64PcWindowsMsvc] {
        for limit in [0, 1, 64, i64::MAX as u64] {
            for reverse in [false, true] {
                let mut p = Program::new(crate::codegen::x86_64::machine::target(target));
                let linux = target.is_linux();
                let (reference, rendered) = if reverse {
                    array::append(&mut p, limit);
                    heap::append(&mut p, limit);
                    (
                        old_array::support(limit, linux) + &old_heap::support(limit, linux),
                        array::support(limit, linux) + &heap::support(limit, linux),
                    )
                } else {
                    heap::append(&mut p, limit);
                    array::append(&mut p, limit);
                    (
                        old_heap::support(limit, linux) + &old_array::support(limit, linux),
                        heap::support(limit, linux) + &array::support(limit, linux),
                    )
                };
                let saved = p.clone();
                let artifact = assemble_program(&p).unwrap();
                assert_eq!(
                    artifact.object,
                    object_reference::assemble(&reference, target).unwrap()
                );
                assert_eq!(
                    artifact.object,
                    object_reference::assemble(&rendered, target).unwrap()
                );
                for name in ["malloc", "free", "memcpy"] {
                    assert_eq!(
                        p.symbols
                            .iter()
                            .filter(|s| s.name == name && s.linkage == Linkage::External)
                            .count(),
                        1
                    );
                }
                let mut data_count = 0;
                for (e, m) in p.entries.iter().zip(&artifact.entries) {
                    assert!(m.provenance.synthetic && m.provenance.origins.is_empty());
                    if let EntryKind::Data64(_) = e.kind {
                        let range = m.range.unwrap();
                        assert_eq!(range.section, Section::Data);
                        assert_eq!(range.end - range.start, 8);
                        assert_eq!(range.start % 8, 0);
                        data_count += 1;
                    }
                }
                assert_eq!(data_count, 9);
                let asm = render_asm(&p).unwrap();
                assert_eq!(asm.contains("callq malloc@PLT"), linux);
                assert_eq!(p, saved);
                assert_eq!(assemble_program(&p).unwrap(), artifact);
            }
        }
    }
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "linux", target_os = "windows")
))]
fn typed_owners_execute_capacity_recovery_and_copy_lifetimes() {
    use Opcode::*;
    use Operand::{Immediate as imm, Symbol as sym};
    use sashimi_assembler::program::SymbolId;
    use std::{fs, process::Command, time::Duration};
    let Some(clang) = std::env::var_os("CERUNE_TEST_ASM_CLANG") else {
        eprintln!("typed owner execution not run: set CERUNE_TEST_ASM_CLANG");
        return;
    };
    let target = if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    };
    let mut p = Program::new(crate::codegen::x86_64::machine::target(target));
    heap::append(&mut p, 6);
    array::append(&mut p, 16);
    // Export test entry points only. Production helpers retain local linkage.
    for s in &mut p.symbols {
        if s.name.starts_with("cerune_") {
            s.linkage = Linkage::Global;
        }
    }
    for (wrapper, callee) in [
        ("concat_error", "cerune_string_concat"),
        ("array_error", "cerune_array_allocate"),
    ] {
        let callee = SymbolId(p.symbols.iter().position(|s| s.name == callee).unwrap());
        let wrapper = p.declare(wrapper, Linkage::Global);
        push(&mut p, EntryKind::Section(Section::Text));
        push(&mut p, EntryKind::Label(wrapper));
        instruction(&mut p, Push, [r64(5)]);
        instruction(&mut p, Mov64, [r64(4), r64(5)]);
        instruction(&mut p, Sub64, [imm(32), r64(4)]);
        instruction(&mut p, Call, [sym(callee)]);
        instruction(&mut p, Mov64, [r64(2), r64(0)]);
        instruction(&mut p, Add64, [imm(32), r64(4)]);
        instruction(&mut p, Pop, [r64(5)]);
        instruction(&mut p, Ret, []);
    }
    let harness = r#"
#include <stdint.h>
#include <string.h>
typedef struct { uint64_t len; unsigned char bytes[3]; } String;
extern String *cerune_string_concat(const String *, const String *);
extern void cerune_string_retain(const String *);
extern void cerune_string_release(const String *);
extern uint64_t concat_error(const String *, const String *);
extern void *cerune_array_allocate(int64_t, uint64_t, uint64_t);
extern uint64_t array_error(int64_t, uint64_t, uint64_t);
extern uint64_t cerune_array_length(void *);
extern void *cerune_array_init_address(void *);
extern void cerune_array_retain(void *);
extern uint64_t cerune_array_release_owner(void *);
extern void cerune_array_free(void *);
int main(void) {
    const String a={3,{0xe6,0x97,0xa5}}, b={3,{0,13,10}}, empty={0,{0}};
    const unsigned char expected[]={0xe6,0x97,0xa5,0,13,10};
    String *s=cerune_string_concat(&a,&b);
    if(!s || s->len!=6 || memcmp((unsigned char*)s+8,expected,6)) return 1;
    if(concat_error(&a,&b)!=2) return 2;
    cerune_string_retain(s); cerune_string_release(s);
    if(concat_error(&a,&b)!=2 || memcmp((unsigned char*)s+8,expected,6)) return 3;
    cerune_string_release(s);
    s=cerune_string_concat(&a,&b);
    if(!s || s->len!=6) return 4;
    cerune_string_release(s);
    s=cerune_string_concat(&empty,&empty);
    if(!s || s->len!=0) return 5;
    cerune_string_retain(s); cerune_string_release(s);
    cerune_string_retain(&a); cerune_string_release(&a);
    void *p=cerune_array_allocate(2,8,8);
    if(!p || cerune_array_length(p)!=2 || !cerune_array_init_address(p)) return 6;
    if(array_error(1,8,8)!=2) return 7;
    cerune_array_retain(p);
    if(cerune_array_release_owner(p)!=0 || array_error(1,8,8)!=2) return 8;
    if(cerune_array_release_owner(p)!=1) return 9;
    cerune_array_free(p);
    p=cerune_array_allocate(2,8,8);
    if(!p || cerune_array_release_owner(p)!=1) return 10;
    cerune_array_free(p);
    if(array_error(-1,8,8)!=1 || array_error(INT64_MAX,8,8)!=1) return 11;
    if(array_error(INT64_MAX,0,8)!=1 || array_error(3,8,8)!=2) return 12;
    if(cerune_array_allocate(0,8,8)!=0 || cerune_array_length(0)!=0) return 13;
    cerune_array_retain(0);
    if(cerune_array_release_owner(0)!=1) return 14;
    cerune_array_free(0);
    p=cerune_array_allocate(3,0,8);
    if(!p || cerune_array_length(p)!=3 || cerune_array_release_owner(p)!=1) return 15;
    cerune_array_free(p);
    return 0;
}
"#;
    let dir = std::env::temp_dir().join(format!("cerune-typed-owners-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("harness.c"), harness).unwrap();
    fs::write(dir.join("owners.s"), render_asm(&p).unwrap()).unwrap();
    fs::write(dir.join("owners.o"), assemble_program(&p).unwrap().object).unwrap();
    for ext in ["s", "o"] {
        let exe = dir.join(format!("owners-{ext}.exe"));
        let mut cc = Command::new(&clang);
        if target.is_linux() {
            cc.args(["-fPIE", "-pie"]);
        }
        cc.arg(dir.join("harness.c"))
            .arg(dir.join(format!("owners.{ext}")))
            .arg("-o")
            .arg(&exe);
        let out = crate::codegen::x86_64::test_process::bounded_output(
            &mut cc,
            &dir,
            "typed-owners/link",
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
            "typed-owners/run",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(out.status.success(), "{ext}: {:?}", out.status);
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
    fs::remove_dir_all(dir).unwrap();
}
