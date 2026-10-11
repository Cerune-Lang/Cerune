use cerune_lang::{
    codegen::x86_64::{self, Target},
    compile_to_ir,
    mir::{self, ssa},
    mir_executor, ssa_executor,
};
#[path = "support/crash_dialogs.rs"]
mod crash_dialogs;
#[path = "support/native_process.rs"]
mod native_process;
#[path = "support/process.rs"]
mod process;
fn program(source: &str) -> ssa::Program {
    ssa::construct(&mir::lower(&compile_to_ir(source).unwrap()).unwrap()).unwrap()
}
fn host() -> Target {
    if cfg!(windows) {
        Target::X86_64PcWindowsMsvc
    } else {
        Target::X86_64UnknownLinuxGnu
    }
}
fn execute(p: &ssa::Program) {
    let expected = ssa_executor::run(p);
    let before = p.clone();
    let lowered = ssa::lower(p).unwrap();
    let evaluated = mir_executor::run(&lowered.program);
    match (&expected, &evaluated) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => {
            assert_eq!(a.runtime_failure(), b.runtime_failure());
            assert_eq!(a.output(), b.output());
        }
        _ => panic!("{expected:?} {evaluated:?}"),
    }
    for (bytes, ext) in [
        (
            x86_64::emit_asm_from_mir(&lowered.program, host(), true)
                .unwrap()
                .into_bytes(),
            "s",
        ),
        (
            x86_64::emit_object_from_mir(&lowered.program, host(), true).unwrap(),
            "o",
        ),
    ] {
        if let Some(actual) = native_process::execute(&bytes, ext) {
            let text = String::from_utf8(actual.stdout).unwrap();
            let text = if cfg!(windows) && !p.original.uses_strings {
                text.replace("\r\n", "\n")
            } else {
                text
            };
            match &expected {
                Ok(output) => {
                    assert!(actual.status.success(), "{:?}", actual.stderr);
                    assert!(actual.stderr.is_empty());
                    assert_eq!(&text, output);
                }
                Err(error) => {
                    // 非ゼロ終了だけで合格にせず、Nativeの意図した停止と診断を照合します。
                    #[cfg(windows)]
                    assert_eq!(actual.status.code().map(|c| c as u32), Some(0xc0000409));
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        assert_eq!(actual.status.signal(), Some(4));
                    }
                    assert_eq!(text, error.output());
                    assert_eq!(
                        String::from_utf8(actual.stderr)
                            .unwrap()
                            .replace("\r\n", "\n"),
                        format!("cerune: {}\n", error.runtime_failure().unwrap().record())
                    );
                }
            }
        }
    }
    assert_eq!(&before, p);
}
#[test]
fn native_ssa_examples_and_failures_match_direct_execution() {
    for source in [
        include_str!("../examples/ir_stages/ssa_values.ceru"),
        include_str!("../examples/ir_stages/constant_candidates.ceru"),
        include_str!("../examples/runtime_failures/constant_overflow.ceru"),
        include_str!("../examples/ir_stages/native_calls.ceru"),
        include_str!("../examples/ir_stages/lir_layout.ceru"),
        include_str!("../examples/ir_stages/owned_values.ceru"),
        include_str!("../examples/string_origins.ceru"),
        include_str!("../examples/rounding_evaluation_order.ceru"),
        include_str!("../examples/ir_stages/control_flow.ceru"),
        "print(7); fn inner(x:i64)->i64{return 1/x;} print(inner(0));",
        include_str!("../experiments/lean/for_update_failure.ceru"),
        "print(7); a:[i64;1]=[1]; print(a[1]);",
    ] {
        execute(&program(source));
    }
    for path in [
        "examples/modules/main.ceru",
        "examples/modules/failure.ceru",
    ] {
        let hir = cerune_lang::modules::load(std::path::Path::new(path))
            .unwrap()
            .to_ir()
            .unwrap();
        execute(&ssa::construct(&mir::lower(&hir).unwrap()).unwrap());
    }
    for limit in [47, 48] {
        let mut hir = compile_to_ir(include_str!(
            "../examples/dynamic_arrays/lowering_order.ceru"
        ))
        .unwrap();
        hir.array_heap_limit = limit;
        execute(&ssa::construct(&mir::lower(&hir).unwrap()).unwrap());
    }
}
#[test]
fn self_loop_parallel_transfer_survives_lowering_and_native_execution() {
    let mut p =
        program("mut a:u8=10; mut b:u8=20; while a<b {c:u8=a; a=b; b=c;} print(a);print(b);");
    let head = p
        .main
        .blocks
        .iter()
        .position(|b| b.arguments.len() == 2)
        .unwrap();
    let args = p.main.blocks[head].arguments.clone();
    let mut changed = false;
    for b in &mut p.main.blocks {
        if let ssa::TerminatorKind::Jump(edge) = &mut b.terminator.kind
            && edge.target.0 == head
            && b.original_block.0 != 0
        {
            edge.arguments = vec![args[1], args[0]];
            changed = true;
        }
    }
    assert!(changed);
    assert_eq!(ssa_executor::run(&p).unwrap(), "20\n10\n");
    let lowered = ssa::lower(&p).unwrap();
    assert_eq!(mir_executor::run(&lowered.program).unwrap(), "20\n10\n");
    assert!(lowered.mapping.contains("helper=mir-bb"));
    assert!(mir::text::emit(&lowered.program).contains("ssa-edge-copy"));
    execute(&p);
}
#[test]
fn changed_ssa_bodies_and_parameters_drive_native_instead_of_original_snapshot() {
    let mut p = program("fn f(x:i64)->i64{return x+1;} print(f(10));");
    for i in p.functions[0]
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
    {
        if let mir::InstructionKind::Assign {
            value: mir::Operation::Literal(mir::Literal::Integer(v)),
            ..
        } = &mut i.kind
        {
            *v = 32;
        }
    }
    assert_eq!(ssa_executor::run(&p).unwrap(), "42\n");
    assert_eq!(mir_executor::run(&p.original).unwrap(), "11\n");
    let lowered = ssa::lower(&p).unwrap();
    assert!(lowered.mapping.contains("parameter-entry="));
    for target in [Target::X86_64PcWindowsMsvc, Target::X86_64UnknownLinuxGnu] {
        let asm = x86_64::emit_asm_from_mir(&lowered.program, target, true).unwrap();
        let object = x86_64::emit_object_from_mir(&lowered.program, target, true).unwrap();
        assert!(asm.contains("ssa-parameter-copy"));
        for f in lowered
            .program
            .functions
            .iter()
            .chain([&lowered.program.main])
        {
            for (bid, b) in f.blocks.iter().enumerate() {
                for i in &b.instructions {
                    let name = f.id.map_or("main".into(), |id| format!("fn_{}", id.0));
                    let label = format!("cerune_origin_mir_{name}_bb{bid}_i{}_lir", i.id.0);
                    assert!(asm.contains(&label));
                    assert!(String::from_utf8_lossy(&object).contains(&label));
                }
            }
        }
    }
    execute(&p);
}
#[test]
fn lowering_keeps_unreachable_records_and_rejects_invalid_ssa() {
    let p = program("fn f(x:u8)->u8 {return x; print(99);} print(f(7));");
    let lowered = ssa::lower(&p).unwrap();
    let mut retained = 0;
    for ((f, old), new) in p
        .functions
        .iter()
        .chain([&p.main])
        .zip(p.original.functions.iter().chain([&p.original.main]))
        .zip(
            lowered
                .program
                .functions
                .iter()
                .chain([&lowered.program.main]),
        )
    {
        for b in &f.retained_unreachable {
            assert_eq!(old.blocks[b.0], new.blocks[b.0]);
            retained += 1;
        }
    }
    assert!(retained > 0);
    let mut bad = p;
    bad.main.entry = mir::BlockId(usize::MAX);
    assert!(ssa::lower(&bad).is_err());
}
