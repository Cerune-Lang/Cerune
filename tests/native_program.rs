use cerune_lang::codegen::x86_64::{self as native, Target};
use sashimi_assembler::program::{EntryKind, Section};

#[test]
fn typed_program_keeps_every_lir_origin_and_observation_does_not_change_objects() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/modules/main.ceru");
    let hir = cerune_lang::modules::load(&source)
        .unwrap()
        .to_ir()
        .unwrap();
    let mir = cerune_lang::mir::lower(&hir).unwrap();
    for target in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
        let mut lir = native::lower_mir(&mir, target).unwrap();
        let source = lir
            .origins
            .iter()
            .copied()
            .find(|o| matches!(o, native::ir::Origin::Source { .. }))
            .unwrap();
        let observed = lir.instructions.len();
        lir.instructions.push(native::ir::Instruction::ObserveOnly);
        lir.origins.push(source);
        lir.mir_origins.push(None);
        for annotate in [false, true] {
            let program = native::build_program_from_lir(&lir, annotate).unwrap();
            let snapshot = program.program().clone();
            let before = program.assemble().unwrap();
            let asm = program.render_asm().unwrap();
            assert_eq!(snapshot, *program.program());
            assert_eq!(before, program.assemble().unwrap());
            assert_eq!(
                before.object,
                native::emit_object_from_lir(&lir, annotate).unwrap()
            );
            assert_eq!(asm, native::emit_asm_from_lir(&lir, annotate).unwrap());
            assert_eq!(
                program.origins().len(),
                lir.instructions.len()
                    + lir
                        .functions
                        .iter()
                        .map(|f| f.instructions.len())
                        .sum::<usize>()
            );
            let mut source_ids = std::collections::BTreeSet::new();
            for (id, origin) in program.origins().iter().enumerate() {
                let (expected_source, expected_mir) = if let Some(function) = origin.function {
                    (
                        lir.functions[function].origins[origin.instruction],
                        lir.functions[function].mir_origins[origin.instruction],
                    )
                } else {
                    (
                        lir.origins[origin.instruction],
                        lir.mir_origins[origin.instruction],
                    )
                };
                assert_eq!(origin.source, expected_source);
                assert_eq!(origin.mir, expected_mir);
                if let native::ir::Origin::Source { span, .. } = origin.source {
                    source_ids.insert(span.source_id().record_field());
                }
                let entries: Vec<_> = before
                    .entries
                    .iter()
                    .filter(|e| e.provenance.origins.iter().any(|o| o.0 == id as u64))
                    .collect();
                assert!(
                    !entries.is_empty(),
                    "LIR operation must retain at least a zero-byte marker"
                );
                if origin.function.is_none() && origin.instruction == observed {
                    assert!(entries.iter().all(|e| {
                        e.range
                            .is_some_and(|r| r.section == Section::Text && r.start == r.end)
                    }));
                    assert!(
                        entries
                            .iter()
                            .any(|e| matches!(snapshot.entries[e.entry.0].kind, EntryKind::Marker))
                    );
                }
            }
            assert!(
                source_ids.len() > 1,
                "module SourceId must not collapse into the entry source"
            );
            for entry in &before.entries {
                for id in &entry.provenance.origins {
                    assert!((id.0 as usize) < program.origins().len());
                }
            }
            let main = snapshot
                .symbols
                .iter()
                .position(|s| s.name == "main")
                .unwrap();
            let entry = snapshot
                .entries
                .iter()
                .position(|e| {
                    e.kind == EntryKind::Label(sashimi_assembler::program::SymbolId(main))
                })
                .unwrap();
            assert!(
                snapshot.entries[entry..]
                    .iter()
                    .take_while(|e| !matches!(e.kind, EntryKind::Marker))
                    .all(|e| e.provenance.synthetic && e.provenance.origins.is_empty())
            );
            let failure_data: Vec<_> = snapshot.entries.iter().filter(|e| matches!(&e.kind, EntryKind::CString(s) if s.starts_with(" node=") || s.starts_with("cerune: runtime-v1"))).collect();
            assert!(!failure_data.is_empty());
            assert!(
                failure_data
                    .iter()
                    .all(|e| !e.provenance.synthetic && !e.provenance.origins.is_empty())
            );
            assert!(
                before
                    .references
                    .iter()
                    .any(|r| !before.entries[r.entry.0].provenance.origins.is_empty())
            );
        }
    }
}

#[path = "native_build/mod.rs"]
mod native_build;
