use cerune_lang::{
    codegen::x86_64::{self, Target},
    compile_to_ir, ir_executor, mir, mir_executor,
};
#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/native_process.rs"]
mod native_process;
#[path = "support/process.rs"]
mod process;

fn host_target() -> Target {
    if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    }
}
fn execute(p: &mir::Program, expected: &str) {
    let before = p.clone();
    for (bytes, ext) in [
        (
            x86_64::emit_asm_from_mir(p, host_target(), true)
                .unwrap()
                .into_bytes(),
            "s",
        ),
        (
            x86_64::emit_object_from_mir(p, host_target(), true).unwrap(),
            "o",
        ),
    ] {
        if let Some(actual) = native_process::execute(&bytes, ext) {
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty(), "{actual:?}");
            let stdout = String::from_utf8(actual.stdout).unwrap();
            let stdout = if cfg!(windows) && !p.uses_strings {
                stdout.replace("\r\n", "\n")
            } else {
                stdout
            };
            assert_eq!(stdout, expected);
        }
    }
    assert_eq!(
        p, &before,
        "generation must not alter the observed MIR snapshot"
    );
}
#[test]
fn observed_lir_snapshot_drives_asm_and_object_execution() {
    let mut lir = x86_64::lower_mir(
        &mir::lower(&compile_to_ir("print(1);").unwrap()).unwrap(),
        host_target(),
    )
    .unwrap();
    for i in &mut lir.instructions {
        if let x86_64::ir::Instruction::MovI64ImmediateToRax(n) = i {
            *n = 42;
        }
    }
    let before = lir.clone();
    let observed = x86_64::lir::emit(&lir).unwrap();
    assert!(observed.contains("MovI64ImmediateToRax(42)"));
    for (bytes, ext) in [
        (
            x86_64::emit_asm_from_lir(&lir, true).unwrap().into_bytes(),
            "s",
        ),
        (x86_64::emit_object_from_lir(&lir, true).unwrap(), "o"),
    ] {
        if let Some(actual) = native_process::execute(&bytes, ext) {
            assert!(actual.status.success(), "{actual:?}");
            assert!(actual.stderr.is_empty());
            assert_eq!(
                String::from_utf8(actual.stdout)
                    .unwrap()
                    .replace("\r\n", "\n"),
                "42\n"
            );
        }
    }
    assert_eq!(lir, before);
}

#[test]
fn native_consumes_modified_mir_without_rebuilding_hir() {
    let hir = compile_to_ir("print(1);").unwrap();
    let mut p = mir::lower(&hir).unwrap();
    for b in &mut p.main.blocks {
        for i in &mut b.instructions {
            if let mir::InstructionKind::Assign {
                value: mir::Operation::Literal(mir::Literal::Integer(n)),
                ..
            } = &mut i.kind
            {
                *n = 42;
            }
        }
    }
    assert_eq!(ir_executor::run(&hir).unwrap(), "1\n");
    assert_eq!(mir_executor::run(&p).unwrap(), "42\n");
    execute(&p, "42\n");
}
#[test]
fn native_calls_example_runs_mixed_arguments_and_main_once() {
    let hir = compile_to_ir(include_str!("../examples/ir_stages/native_calls.ceru")).unwrap();
    let p = mir::lower(&hir).unwrap();
    let expected =
        "MIR → Native\n0\n18446744073709551615\n7\n9\n4\n2\n18446744073709551615\n7\n9\n4\n";
    assert_eq!(ir_executor::run(&hir).unwrap(), expected);
    assert_eq!(mir_executor::run(&p).unwrap(), expected);
    assert_eq!(
        cerune_lang::vm::run(&cerune_lang::bytecode::lower(&hir).unwrap()).unwrap(),
        expected
    );
    execute(&p, expected);
}
#[test]
fn mir_blocks_instructions_and_derived_origins_remain_visible() {
    let hir = compile_to_ir(include_str!("../examples/ir_stages/native_calls.ceru")).unwrap();
    let p = mir::lower(&hir).unwrap();
    for target in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
        let lir = x86_64::lower_mir(&p, target).unwrap();
        let asm = x86_64::emit_asm_from_mir(&p, target, true).unwrap();
        assert_eq!(asm, x86_64::emit_asm_with_origins(&hir, target).unwrap());
        assert_eq!(asm, x86_64::emit_asm_from_mir(&p, target, true).unwrap());
        let object = x86_64::emit_object_from_mir(&p, target, true).unwrap();
        let symbols = String::from_utf8_lossy(&object);
        for (f, origins, locations) in std::iter::once((&p.main, &lir.origins, &lir.mir_origins))
            .chain(
                p.functions
                    .iter()
                    .zip(&lir.functions)
                    .map(|(f, l)| (f, &l.origins, &l.mir_origins)),
            )
        {
            let name =
                f.id.map_or_else(|| "main".into(), |id| format!("fn_{}", id.0));
            for (bid, b) in f.blocks.iter().enumerate() {
                for (iid, origin) in b
                    .instructions
                    .iter()
                    .map(|i| (i.id, i.origin))
                    .chain([(b.terminator.id, b.terminator.origin)])
                {
                    assert!(
                        locations
                            .iter()
                            .flatten()
                            .any(|l| l.block.0 == bid && l.instruction == Some(iid)),
                        "{name} bb{bid} i{}",
                        iid.0
                    );
                    let label = format!("cerune_origin_mir_{name}_bb{bid}_i{}_lir", iid.0);
                    assert!(asm.contains(&label), "{label}");
                    assert!(symbols.contains(&label), "{label}");
                    for (index, l) in locations.iter().enumerate().filter(|(_, l)| {
                        l.is_some_and(|l| l.block.0 == bid && l.instruction == Some(iid))
                    }) {
                        assert_eq!(l.unwrap().origin, origin);
                        match origin.source() {
                            Some(s) => assert_eq!(
                                origins[index],
                                x86_64::ir::Origin::Source {
                                    node_id: s.node_id,
                                    span: s.span
                                }
                            ),
                            None => assert_eq!(origins[index], x86_64::ir::Origin::Synthetic),
                        }
                    }
                }
            }
        }
        assert!(asm.contains("(entry-main-call)"));
        assert!(asm.contains("(for-update)"));
    }
}
#[test]
fn zero_width_values_keep_mir_observations_without_machine_bytes() {
    let mut p = mir::lower(&compile_to_ir("a: [i64; 1] = [7]; print(1);").unwrap()).unwrap();
    // ソースでは0長配列を受け付けません。ここではMIR API上の幅0を検証します。
    for local in &mut p.main.locals {
        if let cerune_lang::ir::Type::Array { length, .. } = &mut local.ty {
            *length = 0;
        }
    }
    for b in &mut p.main.blocks {
        for i in &mut b.instructions {
            if let mir::InstructionKind::Assign {
                value: mir::Operation::Array(values),
                ..
            } = &mut i.kind
            {
                values.clear();
            }
        }
    }
    let lir = x86_64::lower_mir(&p, host_target()).unwrap();
    assert!(
        lir.instructions
            .contains(&x86_64::ir::Instruction::ObserveOnly)
    );
    for b in &p.main.blocks {
        for i in &b.instructions {
            assert!(
                lir.mir_origins
                    .iter()
                    .flatten()
                    .any(|o| o.instruction == Some(i.id))
            );
        }
    }
    execute(&p, "1\n");
}
#[test]
fn folded_string_expressions_retain_binary_output_requirements() {
    let hir = compile_to_ir("const SAME: bool = \"a\" == \"a\"; print(SAME);").unwrap();
    let p = mir::lower(&hir).unwrap();
    assert!(p.uses_strings);
    assert!(mir::text::emit(&p).contains("source-strings=true"));
    let asm = x86_64::emit_asm_from_mir(&p, Target::X86_64PcWindowsMsvc, false).unwrap();
    assert!(asm.contains("callq _setmode"));
    assert_eq!(
        asm,
        x86_64::emit_asm(&hir, Target::X86_64PcWindowsMsvc).unwrap()
    );
    execute(&p, "true\n");
}
#[test]
fn invalid_mir_is_rejected_before_native_generation() {
    let mut p = mir::lower(&compile_to_ir("print(1);").unwrap()).unwrap();
    p.main.entry = mir::BlockId(usize::MAX);
    assert!(x86_64::lower_mir(&p, host_target()).is_err());
    assert!(x86_64::emit_object_from_mir(&p, host_target(), true).is_err());
}
