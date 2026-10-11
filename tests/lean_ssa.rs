use cerune_lang::{
    bytecode, compile_to_ir, ir, ir_executor,
    mir::{self, InstructionKind as I, Operation as O, ssa},
    mir_executor, run_bytecode,
    runtime::FailureCode,
    ssa_executor,
};
use std::{fs, path::Path, process::Command, time::Duration};
#[path = "../experiments/lean/ssa.rs"]
mod experiment;
#[path = "support/process.rs"]
mod process;
const PROOFS: &str = include_str!("../experiments/lean/SsaCorrespondence.lean");
const FIXTURES: &[(&str, &str, &str)] = &[
    (
        "Increment",
        include_str!("../experiments/lean/increment.ceru"),
        "increment",
    ),
    (
        "Branch",
        include_str!("../experiments/lean/branch.ceru"),
        "choose",
    ),
    (
        "Loop",
        include_str!("../experiments/lean/loop.ceru"),
        "advance",
    ),
    (
        "LoopControl",
        include_str!("../experiments/lean/loop_control.ceru"),
        "advance",
    ),
    (
        "NestedControl",
        include_str!("../experiments/lean/nested_loop_control.ceru"),
        "advance",
    ),
    (
        "For",
        include_str!("../experiments/lean/for_control.ceru"),
        "advance",
    ),
    (
        "ForFailure",
        include_str!("../experiments/lean/for_update_failure.ceru"),
        "advance",
    ),
    ("Join", experiment::JOIN, "choose"),
    ("Swap", experiment::SWAP, "swap"),
    ("Parallel", experiment::SWAP, "swap"),
];

fn snapshots(source: &str, parallel: bool) -> (ir::Program, ssa::Program, mir::Program) {
    let hir = compile_to_ir(source).unwrap();
    let mut ssa = ssa::construct(&mir::lower(&hir).unwrap()).unwrap();
    if parallel {
        // 元のcopy命令は残し、backedgeをheader引数同士の直接交換にします。
        // 逐次束縛の誤りを、偶然別のTemporaryが隠さないようにする等価なAPI入力です。
        let f = &mut ssa.functions[0];
        let head = f
            .blocks
            .iter()
            .position(|b| b.arguments.len() == 3)
            .unwrap();
        let args = f.blocks[head].arguments.clone();
        let local = |name: &str| {
            ssa.original.functions[0]
                .locals
                .iter()
                .position(
                    |l| matches!(&l.kind, mir::LocalKind::Binding { name: n, .. } if n == name),
                )
                .unwrap()
        };
        let left = local("left");
        let right = local("right");
        let left_arg = *args
            .iter()
            .find(|v| f.values[v.0].original_local.0 == left)
            .unwrap();
        let right_arg = *args
            .iter()
            .find(|v| f.values[v.0].original_local.0 == right)
            .unwrap();
        let mut changed = false;
        for block in &mut f.blocks {
            if block.original_block.0 == 0 {
                continue;
            }
            if let ssa::TerminatorKind::Jump(edge) = &mut block.terminator.kind
                && edge.target.0 == head
            {
                for (arg, value) in args.iter().zip(&mut edge.arguments) {
                    if *arg == left_arg {
                        *value = right_arg;
                    }
                    if *arg == right_arg {
                        *value = left_arg;
                    }
                }
                changed = true;
            }
        }
        assert!(changed);
        ssa::validate(&ssa).unwrap();
    }
    let lowered = ssa::lower(&ssa).unwrap().program;
    (hir, ssa, lowered)
}

#[test]
fn exports_actual_snapshots_and_rejects_unsupported_operations() {
    for &(name, source, _) in FIXTURES {
        let (_, ssa, lowered) = snapshots(source, name == "Parallel");
        let generated = experiment::emit(&ssa.original, &ssa, &lowered, 0).unwrap();
        assert!(generated.contains("def ssaReference : SsaFunction"));
        assert!(generated.contains("def loweredReference : MirFunction"));
        assert!(generated.contains("def mirReference : MirFunction"));
        assert_eq!(
            generated,
            experiment::emit(&ssa.original, &ssa, &lowered, 0).unwrap()
        );
    }
    for source in [
        "fn f(x:u16)->u16{return x+1;} print(f(0));",
        "fn f(x:u8)->u8{return x*2;} print(f(0));",
        "fn f(x:u8)->u8{print(x);return x;} print(f(0));",
        "fn f(x:u8)->u8{s:string=\"hello\";return x;} print(f(0));",
    ] {
        let (_, ssa, lowered) = snapshots(source, false);
        assert!(experiment::emit(&ssa.original, &ssa, &lowered, 0).is_err());
    }
}

fn compare(source: &str, parallel: bool) -> String {
    let (hir, ssa, lowered) = snapshots(source, parallel);
    let a = ir_executor::run(&hir);
    let b = mir_executor::run(&ssa.original);
    let c = ssa_executor::run(&ssa);
    let d = mir_executor::run(&lowered);
    let e = run_bytecode(&bytecode::lower(&hir).unwrap());
    match (a, b, c, d, e) {
        (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e)) => {
            assert_eq!(a, b);
            assert_eq!(a, c);
            assert_eq!(a, d);
            assert_eq!(a, e);
            format!("ok {}", a.trim_end())
        }
        (Err(a), Err(b), Err(c), Err(d), Err(e)) => {
            let failure = a.runtime_failure().unwrap();
            assert_eq!(Some(failure), b.runtime_failure());
            assert_eq!(Some(failure), c.runtime_failure());
            assert_eq!(Some(failure), d.runtime_failure());
            assert_eq!(Some(failure), e.runtime_failure());
            assert_eq!(failure.code, FailureCode::IntegerOverflow);
            for output in [
                a.output(),
                b.output(),
                c.output(),
                d.output(),
                e.vm_error().output(),
            ] {
                assert!(output.is_empty());
            }
            format!(
                "integer-overflow node={} source={} bytes={}..{}",
                failure.node_id.0,
                failure.span.source_id().index(),
                failure.span.start(),
                failure.span.end()
            )
        }
        results => panic!("{results:?}"),
    }
}
#[test]
fn proof_fixtures_execute_on_all_five_routes() {
    for &(name, source, _) in FIXTURES {
        let result = compare(source, name == "Parallel");
        match name {
            "Join" => assert_eq!(result, "ok 3\n130\n130\n255"),
            "Swap" | "Parallel" => assert_eq!(result, "ok 0\n127\n255"),
            _ => {}
        }
    }
}

fn lean(directory: &Path, path: &Path, execute: bool) -> std::process::Output {
    let mut command = Command::new(std::env::var_os("CERUNE_TEST_LEAN").unwrap_or("lean".into()));
    command.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("experiments/lean"));
    if execute {
        command.arg("--run");
    }
    command.arg(path);
    process::bounded_output(
        &mut command,
        directory,
        &path.to_string_lossy(),
        Duration::from_secs(120),
    )
    .unwrap()
}
fn accepted(output: std::process::Output) -> String {
    let log = String::from_utf8(output.stdout).unwrap();
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{log}\n{:?}",
        output.stderr
    );
    assert!(
        !log.contains("warning:") && !log.contains("error:"),
        "{log}"
    );
    log
}
fn rejected(directory: &Path, name: &str, text: &str) {
    eprintln!("Lean SSA rejected mutation: {name}");
    let path = directory.join(format!("Rejected{name}.lean"));
    fs::write(&path, text).unwrap();
    let output = lean(directory, &path, false);
    let log = String::from_utf8(output.stdout).unwrap();
    assert!(
        !output.status.success() && output.stderr.is_empty(),
        "{log}"
    );
    assert!(
        log.contains("error: Tactic `decide` proved that the proposition")
            && log.contains("is false"),
        "{log}"
    );
    assert!(
        log.lines()
            .filter(|line| line.contains("error:"))
            .all(|line| line.contains("error: Tactic `decide` proved that the proposition")),
        "{log}"
    );
    fs::write(directory.join(format!("Rejected{name}.stdout")), log).unwrap();
}

#[test]
#[ignore = "requires pinned Lean; cargo test --test lean_ssa -- --include-ignored"]
fn lean_proves_ssa_snapshots_completion_and_properties() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/lean-verification-test/ssa");
    fs::create_dir_all(&directory).unwrap();
    check_version(&directory);
    for &(name, source, function) in FIXTURES {
        eprintln!("Lean SSA: {name}");
        let (hir, ssa, lowered) = snapshots(source, name == "Parallel");
        let generated = experiment::emit(&ssa.original, &ssa, &lowered, 0).unwrap();
        for (extension, text) in [
            ("ceru", source.to_owned()),
            ("ceir", cerune_lang::ir::text::emit(&hir)),
            ("mir.txt", mir::text::emit(&ssa.original)),
            ("ssa.txt", ssa::text::emit(&ssa).unwrap()),
            ("ssa-map.txt", ssa::mapping::emit(&ssa).unwrap()),
            ("lowered.mir.txt", mir::text::emit(&lowered)),
            ("lowered-map.txt", ssa::lower(&ssa).unwrap().mapping),
            ("Generated.lean", generated.clone()),
        ] {
            fs::write(directory.join(format!("{name}.{extension}")), text).unwrap();
        }
        let verified = with_properties(name, source, &ssa, &generated);
        let path = directory.join(format!("{name}.Verified.lean"));
        fs::write(&path, &verified).unwrap();
        let log = accepted(lean(&directory, &path, false));
        for theorem in [
            "ssa_translation_correct",
            "ssa_lowering_correct",
            "baseline_completed",
            "ssa_completed",
        ] {
            assert!(
                log.contains(&format!(
                    "'CeruneProof.{theorem}' depends on axioms: [propext]"
                )),
                "{log}"
            );
        }
        for theorem in match name {
            "Join" => &["ssa_join_expected", "lowered_join_expected"][..],
            "Swap" | "Parallel" => &["ssa_swap_expected", "lowered_swap_expected"][..],
            _ => &[],
        } {
            assert!(
                log.contains(&format!(
                    "'CeruneProof.{theorem}' depends on axioms: [propext]"
                )),
                "{log}"
            );
        }
        let path = directory.join(format!("{name}.Execute.lean"));
        fs::write(&path, format!("{generated}\ndef main : IO Unit := do\n  for x in List.range 256 do\n    IO.println (CeruneProof.renderMir (CeruneProof.evalMirWithFuel CeruneProof.mirReference 64 x))\n    IO.println (CeruneProof.renderMir (CeruneProof.evalSsa CeruneProof.ssaReference 64 x))\n    IO.println (CeruneProof.renderMir (CeruneProof.evalMirWithFuel CeruneProof.loweredReference 128 x))\n")).unwrap();
        let log = accepted(lean(&directory, &path, true));
        let lines: Vec<_> = log.lines().collect();
        assert_eq!(lines.len(), 256 * 3);
        let prefix = source.split("print(").next().unwrap();
        for x in 0..256 {
            let result = compare(
                &format!("{prefix}print({function}({x}));"),
                name == "Parallel",
            );
            for line in &lines[x * 3..x * 3 + 3] {
                assert_eq!(*line, result, "{name} input={x}");
            }
        }
    }
}

fn check_version(directory: &Path) {
    let mut version = Command::new(std::env::var_os("CERUNE_TEST_LEAN").unwrap_or("lean".into()));
    version.current_dir("experiments/lean").arg("--version");
    let output = process::bounded_output(
        &mut version,
        directory,
        "Lean version",
        Duration::from_secs(15),
    )
    .unwrap();
    let log = accepted(output);
    let expected = experiment::TOOLCHAIN.trim().split(":v").nth(1).unwrap();
    assert!(log.contains(&format!("version {expected},")), "{log}");
    fs::write(directory.join("lean-toolchain"), experiment::TOOLCHAIN).unwrap();
}

fn with_properties(name: &str, source: &str, ssa: &ssa::Program, generated: &str) -> String {
    let properties = match name {
        "Join" => {
            let origin = ssa.original.functions[0]
                .blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .filter_map(|i| i.origin.source())
                .find(|o| &source[o.span.start()..o.span.end()] == "result + 2")
                .unwrap();
            format!(
                "namespace CeruneProof\ndef joinFailureOrigin : Origin := ⟨{}, {}, {}, {}⟩\nend CeruneProof\n{}",
                origin.node_id.0,
                origin.span.source_id().index(),
                origin.span.start(),
                origin.span.end(),
                include_str!("../experiments/lean/SsaJoinProperties.lean")
            )
        }
        "Swap" | "Parallel" => {
            include_str!("../experiments/lean/SsaSwapProperties.lean").to_owned()
        }
        _ => String::new(),
    };
    format!("{generated}\n{PROOFS}\n{properties}")
}

#[test]
#[ignore = "requires pinned Lean; cargo test --test lean_ssa -- --include-ignored"]
fn lean_rejects_ssa_mutations() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/lean-verification-test/ssa/mutations");
    fs::create_dir_all(&directory).unwrap();
    check_version(&directory);
    for (name, source) in [("Join", experiment::JOIN), ("Parallel", experiment::SWAP)] {
        let (_, ssa, lowered) = snapshots(source, name == "Parallel");
        let generated = experiment::emit(&ssa.original, &ssa, &lowered, 0).unwrap();
        let verified = with_properties(name, source, &ssa, &generated);
        let path = directory.join(format!("{name}.Control.lean"));
        fs::write(&path, &verified).unwrap();
        accepted(lean(&directory, &path, false));
        check_mutations(&directory, name, &ssa, &lowered, &generated, &verified);
    }
}

fn check_mutations(
    directory: &Path,
    name: &str,
    ssa: &ssa::Program,
    lowered: &mir::Program,
    generated: &str,
    verified: &str,
) {
    if name == "Join" {
        let mut wrong = ssa.clone();
        let f = &mut wrong.functions[0];
        let ssa::Operand::Value(param) = f.parameters[0] else {
            panic!()
        };
        let edge = f
            .blocks
            .iter_mut()
            .find_map(|b| match &mut b.terminator.kind {
                ssa::TerminatorKind::Jump(e) if !e.arguments.is_empty() => Some(e),
                _ => None,
            })
            .unwrap();
        edge.arguments[0] = param;
        let bad = experiment::emit(&ssa.original, &wrong, lowered, 0).unwrap();
        rejected(directory, "JoinArgument", &format!("{bad}\n{PROOFS}"));
        let mut wrong = lowered.clone();
        let i = wrong.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|b| &mut b.instructions)
            .filter(|i| {
                matches!(
                    i.kind,
                    I::Assign {
                        value: O::Binary {
                            op: ir::BinaryOp::Add,
                            ..
                        },
                        ..
                    }
                )
            })
            .last()
            .unwrap();
        let mut origin = i.origin.source().unwrap();
        origin.node_id.0 += 10000;
        i.origin = mir::Origin::Source(origin);
        let bad = experiment::emit(&ssa.original, ssa, &wrong, 0).unwrap();
        rejected(directory, "FailureOrigin", &format!("{bad}\n{PROOFS}"));
        let bad_property = verified.replace(".ok (input + 3)", ".ok (input + 4)");
        assert_ne!(bad_property, verified);
        rejected(directory, "JoinProperty", &bad_property);
        // 同じfuel不足で3経路が一致しても完了定理は成立しません。
        let empty = verified.replace(":= 64", ":= 0").replace(":= 128", ":= 0");
        rejected(directory, "Exhaustion", &empty);
    } else {
        let mut wrong = lowered.clone();
        let mut changed = false;
        for block in &mut wrong.functions[0].blocks {
            if block.instructions.first().is_some_and(|i| {
                matches!(
                    i.origin,
                    mir::Origin::Derived {
                        reason: "ssa-edge-copy",
                        ..
                    }
                )
            }) {
                let copies: Vec<_> = block
                    .instructions
                    .iter()
                    .filter_map(|i| match i.kind {
                        I::Assign {
                            destination,
                            value: O::Copy(source),
                        } => Some((destination, source)),
                        _ => None,
                    })
                    .collect();
                let half = copies.len() / 2;
                for i in block.instructions.iter_mut().skip(half) {
                    if let I::Assign {
                        value: O::Copy(source),
                        ..
                    } = &mut i.kind
                        && let Some((_, original)) = copies[..half]
                            .iter()
                            .find(|(temporary, _)| temporary == source)
                    {
                        *source = *original;
                        changed = true;
                    }
                }
            }
        }
        assert!(changed);
        let bad = experiment::emit(&ssa.original, ssa, &wrong, 0).unwrap();
        rejected(directory, "SequentialCopies", &format!("{bad}\n{PROOFS}"));
        let mut wrong = ssa.clone();
        let i = wrong.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|b| &mut b.instructions)
            .find(|i| {
                matches!(
                    i.kind,
                    I::Assign {
                        value: O::Binary {
                            op: ir::BinaryOp::Add,
                            ..
                        },
                        ..
                    }
                )
            })
            .unwrap();
        if let I::Assign { value, .. } = &mut i.kind
            && let O::Binary { left, .. } = value
        {
            *value = O::Copy(*left);
        }
        let bad = experiment::emit(&ssa.original, &wrong, lowered, 0).unwrap();
        rejected(directory, "MissingLoopUpdate", &format!("{bad}\n{PROOFS}"));
        // モデル自体も逐次読取りへ壊し、並列受渡しが検査対象であることを確認します。
        let old = "let incoming ← ssaReadArguments edge.arguments values\n  ssaBindArguments target.arguments incoming values";
        let new = "let pairs := target.arguments.zip edge.arguments\n  pairs.foldlM (fun current (dest, source) => do\n    let value ← current source\n    pure (fun index => if index = dest then some value else current index)) values";
        assert!(generated.contains(old));
        let bad = generated.replace(old, new);
        rejected(directory, "SequentialSsa", &format!("{bad}\n{PROOFS}"));
    }
}
