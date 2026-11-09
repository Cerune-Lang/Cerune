use cerune_lang::{
    compile_to_ir,
    mir::{
        self, InstructionKind as I, LocalId, Operation as O, TerminatorKind as T,
        ssa::{self, Operand},
    },
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
fn convert(source: &str) -> ssa::Program {
    ssa::construct(&mir::lower(&compile_to_ir(source).unwrap()).unwrap()).unwrap()
}
fn original_local(f: &ssa::Function, op: Operand) -> LocalId {
    match op {
        Operand::Value(v) => f.values[v.0].original_local,
        Operand::Slot(id) => id,
    }
}
type State = Vec<BTreeSet<usize>>;
fn write_definitions(f: &ssa::Function, b: &ssa::Block, state: &mut State) {
    for &v in &b.arguments {
        state[f.values[v.0].original_local.0] = BTreeSet::from([v.0]);
    }
    for i in &b.instructions {
        if let I::Assign {
            destination: Operand::Value(v),
            ..
        } = &i.kind
        {
            state[f.values[v.0].original_local.0] = BTreeSet::from([v.0]);
        }
    }
}
/// 支配木を使わず、流入する最新の定義集合を不動点まで求めて読取りと辺を照合します。
fn correspondence(p: &ssa::Program) -> Result<(), String> {
    for (f, old) in p
        .functions
        .iter()
        .chain([&p.main])
        .zip(p.original.functions.iter().chain([&p.original.main]))
    {
        let mut input = vec![vec![BTreeSet::new(); old.locals.len()]; f.blocks.len()];
        let mut output = input.clone();
        let mut entry = input[0].clone();
        for param in &f.parameters {
            if let Operand::Value(v) = param {
                entry[f.values[v.0].original_local.0].insert(v.0);
            }
        }
        loop {
            let mut next_input = vec![vec![BTreeSet::new(); old.locals.len()]; f.blocks.len()];
            next_input[f.entry.0] = entry.clone();
            for (id, b) in f.blocks.iter().enumerate() {
                for e in b.terminator.kind.edges() {
                    for (local, set) in output[id].iter().enumerate() {
                        next_input[e.target.0][local].extend(set);
                    }
                }
            }
            let mut next_output = next_input.clone();
            for (id, b) in f.blocks.iter().enumerate() {
                write_definitions(f, b, &mut next_output[id]);
            }
            if next_input == input && next_output == output {
                break;
            }
            input = next_input;
            output = next_output;
        }
        let read = |op: Operand, state: &State| -> Result<(), String> {
            if let Operand::Value(v) = op {
                let local = f.values[v.0].original_local.0;
                if state[local] != BTreeSet::from([v.0]) {
                    return Err(format!(
                        "stale value v{} for %{local}: {:?}",
                        v.0, state[local]
                    ));
                }
            }
            Ok(())
        };
        for (id, b) in f.blocks.iter().enumerate() {
            let mut state = input[id].clone();
            for v in &b.arguments {
                state[f.values[v.0].original_local.0] = BTreeSet::from([v.0]);
            }
            let source = &old.blocks[b.original_block.0];
            for (i, original) in b.instructions.iter().zip(&source.instructions) {
                let expected = match &original.kind {
                    I::Store { root, path, value }
                        if path.is_empty() && ssa::is_scalar(&old.locals[root.0].ty) =>
                    {
                        I::Assign {
                            destination: *root,
                            value: O::Copy(*value),
                        }
                    }
                    kind => kind.clone(),
                };
                if i.kind.map_operands(|op| original_local(f, op)) != expected {
                    return Err("operation changed".into());
                }
                // destinationだけを除き、他のoperandはすべてその時点の値です。
                let mut failure = None;
                match &i.kind {
                    I::Assign { value, .. } => {
                        value.map_operands(|op| {
                            if let Err(e) = read(op, &state) {
                                failure = Some(e);
                            }
                        });
                    }
                    kind => {
                        kind.map_operands(|op| {
                            if let Err(e) = read(op, &state) {
                                failure = Some(e);
                            }
                        });
                    }
                }
                if let Some(e) = failure {
                    return Err(e);
                }
                if let I::Assign {
                    destination: Operand::Value(v),
                    ..
                } = i.kind
                {
                    state[f.values[v.0].original_local.0] = BTreeSet::from([v.0]);
                }
                assert_eq!(i.origin, original.origin);
                assert_eq!(i.original_instruction, original.id);
            }
            let expected = match &b.terminator.kind {
                ssa::TerminatorKind::Jump(e) => T::Jump(f.blocks[e.target.0].original_block),
                ssa::TerminatorKind::Branch {
                    condition,
                    then_edge,
                    else_edge,
                } => {
                    read(Operand::Value(*condition), &state)?;
                    T::Branch {
                        condition: original_local(f, Operand::Value(*condition)),
                        then_block: f.blocks[then_edge.target.0].original_block,
                        else_block: f.blocks[else_edge.target.0].original_block,
                    }
                }
                ssa::TerminatorKind::Return(v) => {
                    if let Some(v) = v {
                        read(*v, &state)?;
                    }
                    T::Return(v.map(|v| original_local(f, v)))
                }
            };
            if expected != source.terminator.kind {
                return Err("control flow changed".into());
            }
            for e in b.terminator.kind.edges() {
                for (&v, &arg) in e.arguments.iter().zip(&f.blocks[e.target.0].arguments) {
                    let local = f.values[arg.0].original_local.0;
                    if state[local] != BTreeSet::from([v.0]) {
                        return Err("wrong incoming definition".into());
                    }
                }
            }
        }
    }
    Ok(())
}
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "ceru") {
            out.push(path);
        }
    }
}
#[test]
fn every_example_constructs_deterministically_and_preserves_baseline_and_operations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut all = vec![];
    files(&root.join("examples"), &mut all);
    all.sort();
    let mut count = 0;
    for path in all {
        if path.parent() == Some(root.join("examples/source_files").as_path()) {
            continue;
        }
        let hir = cerune_lang::modules::load(&path).unwrap().to_ir().unwrap();
        let mir = mir::lower(&hir).unwrap();
        let before = mir.clone();
        let baseline = mir::text::emit(&mir);
        let p = ssa::construct(&mir).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert_eq!(mir, before);
        assert_eq!(p.original, before);
        correspondence(&p).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let text = ssa::text::emit(&p).unwrap();
        assert!(text.contains(&baseline));
        assert_eq!(p, ssa::construct(&mir).unwrap());
        assert_eq!(
            text,
            ssa::text::emit(&ssa::construct(&mir).unwrap()).unwrap()
        );
        count += 1;
    }
    assert!(count >= 115, "coverage: {count}");
}
#[test]
fn branch_loop_and_short_circuit_merge_current_values() {
    for source in [
        include_str!("../examples/ir_stages/ssa_values.ceru"),
        include_str!("../examples/ir_stages/control_flow.ceru"),
        include_str!("../experiments/lean/for_control.ceru"),
        include_str!("../experiments/lean/for_update_failure.ceru"),
        "mut a:u8=0; mut i:u8=0; while i<4 {if i==1 {a=9;} else {a=a+1;} i=i+1;} print(a);",
        "fn hit()->bool {print(9);return true;} print(false && hit()); print(true || hit());",
    ] {
        let p = convert(source);
        correspondence(&p).unwrap();
        assert!(
            p.functions
                .iter()
                .chain([&p.main])
                .flat_map(|f| &f.blocks)
                .any(|b| !b.arguments.is_empty())
        );
    }
}
#[test]
fn does_not_create_phis_for_dead_locals_or_delete_their_computations() {
    let p = convert("mut x:u8=0; if true {x=1;} else {x=2;} print(3);");
    assert!(p.main.blocks.iter().all(|b| b.arguments.is_empty()));
    assert_eq!(
        p.main
            .blocks
            .iter()
            .map(|b| b.instructions.len())
            .sum::<usize>(),
        p.original
            .main
            .blocks
            .iter()
            .map(|b| b.instructions.len())
            .sum::<usize>()
    );
    correspondence(&p).unwrap();
}
#[test]
fn independent_checker_rejects_well_typed_stale_reads_and_wrong_edges() {
    let p = convert("mut x:u8=0; if true {x=1;} else {x=2;} print(x);");
    let join = p
        .main
        .blocks
        .iter()
        .position(|b| !b.arguments.is_empty())
        .unwrap();
    let merge = p.main.blocks[join].arguments[0];
    let local = p.main.values[merge.0].original_local;
    let stale = p.main.blocks[0]
        .instructions
        .iter()
        .find_map(|i| match i.kind {
            I::Assign {
                destination: Operand::Value(v),
                ..
            } if p.main.values[v.0].original_local == local => Some(v),
            _ => None,
        })
        .unwrap();
    let mut bad = p.clone();
    for i in &mut bad.main.blocks[join].instructions {
        i.kind = i.kind.map_operands(|op| {
            if op == Operand::Value(merge) {
                Operand::Value(stale)
            } else {
                op
            }
        });
    }
    ssa::validate(&bad).unwrap();
    assert!(correspondence(&bad).unwrap_err().contains("stale"));
    let mut bad = p;
    for b in &mut bad.main.blocks {
        if let ssa::TerminatorKind::Jump(e) = &mut b.terminator.kind
            && e.target.0 == join
        {
            e.arguments[0] = stale;
        }
    }
    ssa::validate(&bad).unwrap();
    assert!(correspondence(&bad).unwrap_err().contains("incoming"));
}
#[test]
fn rejects_invalid_input_and_keeps_independent_snapshots() {
    let mut mir = mir::lower(&compile_to_ir("print(1);").unwrap()).unwrap();
    let ssa = ssa::construct(&mir).unwrap();
    mir.main.blocks[0].instructions.clear();
    mir.main.blocks[0].terminator.kind = T::Jump(mir::BlockId(999));
    assert!(ssa::construct(&mir).is_err());
    assert!(!ssa.original.main.blocks[0].instructions.is_empty());
}

#[test]
fn arbitrary_cfg_with_two_loop_entries_keeps_current_definitions() {
    let source = "mut x:u8=0; if true {x=x+1;} else {x=x+2;} print(x);";
    let mut mir = mir::lower(&compile_to_ir(source).unwrap()).unwrap();
    let T::Branch {
        condition,
        then_block,
        else_block,
    } = mir.main.blocks[0].terminator.kind
    else {
        panic!()
    };
    let T::Jump(exit) = mir.main.blocks[else_block.0].terminator.kind else {
        panic!()
    };
    mir.main.blocks[then_block.0].terminator.kind = T::Jump(else_block);
    mir.main.blocks[else_block.0].terminator.kind = T::Branch {
        condition,
        then_block,
        else_block: exit,
    };
    mir.main.blocks[else_block.0].terminator.origin = mir.main.blocks[0].terminator.origin;
    mir::validate(&mir).unwrap();
    let p = ssa::construct(&mir).unwrap();
    correspondence(&p).unwrap();
    assert!(
        p.main
            .blocks
            .iter()
            .filter(|b| !b.arguments.is_empty())
            .count()
            >= 2
    );
}
#[test]
fn multiple_sources_and_failure_operations_keep_exact_origins() {
    use cerune_lang::{ast, lexer, parser, source::SourceMap};
    let mut sources = SourceMap::new();
    let a = sources.add("private-library.ceru", "fn advance(x:u8)->u8{return x+1;}");
    let b = sources.add("private-entry.ceru", "print(advance(255));");
    let mut ast = ast::Program { items: vec![] };
    for id in [a, b] {
        ast.items.extend(
            parser::parse(lexer::lex_source(sources.get(id).unwrap()).unwrap())
                .unwrap()
                .items,
        );
    }
    let hir = cerune_lang::ir::builder::build(&ast).unwrap();
    let mir = mir::lower(&hir).unwrap();
    let failure = cerune_lang::mir_executor::run(&mir).unwrap_err();
    let p = ssa::construct(&mir).unwrap();
    correspondence(&p).unwrap();
    let origin = failure.origin().unwrap();
    assert_eq!(origin.span.source_id(), a);
    assert!(
        p.functions
            .iter()
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .any(|i| i.origin.source() == Some(origin))
    );
    let text = ssa::text::emit(&p).unwrap();
    assert!(text.contains("construction=scalar-ssa-v1 options=none"));
    assert!(!text.contains("private-"));
    assert_eq!(p.construction, Some(ssa::Construction::ScalarSsaV1));
}
