use super::*;

fn chain(depth: usize, aggregate: bool) -> String {
    use std::fmt::Write;
    let mut source = if aggregate {
        "type Packet {text:string, values:[i64;2],}\n".to_string()
    } else {
        String::new()
    };
    for i in (0..depth).rev() {
        if aggregate {
            let body = if i + 1 == depth {
                "return Packet {text:concat(\"a\",\"b\"),values:[1,2]};".to_string()
            } else {
                format!(
                    "v:Packet=step_{}(); return Packet {{..v,values:[v.values[0]+1,2]}};",
                    i + 1
                )
            };
            writeln!(source, "fn step_{i}()->Packet {{{body}}}").unwrap();
        } else {
            let value = if i + 1 == depth {
                "1".to_string()
            } else {
                format!("step_{}()+1", i + 1)
            };
            writeln!(source, "fn step_{i}()->i64 {{return {value};}}").unwrap();
        }
    }
    source.push_str(if aggregate {
        "print(\"before\"); v:Packet=step_0();print(v.values[0]);print(v.text);print(\"after\");"
    } else {
        "print(\"before\");print(step_0());print(\"after\");"
    });
    source
}
#[test]
fn hir_deep_calls_on_small_stack_preserve_values_limits_and_origins() {
    // 変換処理は通常のtest threadで済ませ、実行だけを明示2 MiBへ載せます。
    // 実行時のホストstack依存を検出する回帰例です。
    for aggregate in [false, true] {
        let source = chain(128, aggregate);
        let p = compile_to_ir(&source).unwrap();
        let before = ir::text::emit(&p);
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn_scoped(scope, || {
                    for limit in [8, 127] {
                        let error =
                            ir_executor::run_with_call_limit(&p, NonZeroU32::new(limit).unwrap())
                                .unwrap_err();
                        let failure = error.call_failure().unwrap();
                        assert_eq!(failure.active().len(), limit as usize);
                        assert_eq!(error.output(), "before\n");
                        let site = failure.attempted().origin.unwrap();
                        assert_eq!(
                            &source[site.span.start()..site.span.end()],
                            format!("step_{limit}()")
                        );
                        assert_eq!(error.origin().unwrap().node_id, site.node_id);
                    }
                    let expected = if aggregate {
                        "before\n128\nab\nafter\n"
                    } else {
                        "before\n128\nafter\n"
                    };
                    assert_eq!(
                        ir_executor::run_with_call_limit(&p, NonZeroU32::new(128).unwrap())
                            .unwrap(),
                        expected
                    );
                    assert_eq!(ir_executor::run(&p).unwrap(), expected);
                })
                .unwrap()
                .join()
                .unwrap();
        });
        assert_eq!(ir::text::emit(&p), before);
    }
}
#[test]
fn suspended_operands_and_loop_updates_match_all_four_executors() {
    assert_eq!(
        compare(
            include_str!("../../examples/call_frames/pending_values.ceru"),
            3
        )
        .unwrap(),
        "1\n2\n3\n900\n"
    );
    let source = r#"
fn mark(x:i64)->i64 {print(x);return x;}
fn leaf(x:i64)->i64 {if x==2 {return 20;} return x;}
fn work()->i64 {
    mut total:i64=0;
    for (mut i:i64=0; i<4; i=mark(i+1)) {
        if i==1 {continue;}
        if i==3 {break;}
        mut j:i64=0;
        while j<2 {j=j+1;if j==1 {continue;} total=total+leaf(i);}
    }
    return total;
}
print(mark(7)+work());
"#;
    assert_eq!(compare(source, 2).unwrap(), "7\n1\n2\n3\n27\n");
}
#[test]
fn assignment_checks_each_index_before_later_indices_and_rhs() {
    let source = r#"fn mark(x:i64)->i64 {print(x);return x;} mut a:[[i64;1];1]=[[0]];a[mark(1)][mark(2)]=mark(3);print("after");"#;
    let p = compile_to_ir(source).unwrap();
    let h = ir_executor::run(&p).unwrap_err();
    let m = mir::lower(&p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(&p).unwrap();
    let mm = mir_executor::run(&m).unwrap_err();
    let ss = ssa_executor::run(&s).unwrap_err();
    let v = vm::run(&b).unwrap_err();
    for output in [h.output(), mm.output(), ss.output(), v.output()] {
        assert_eq!(output, "1\n");
    }
    assert_eq!(h.runtime_failure(), mm.runtime_failure());
    assert_eq!(h.runtime_failure(), ss.runtime_failure());
    assert_eq!(
        cerune_lang::runtime::FailureCode::from_vm(v.kind()),
        Some(h.runtime_failure().unwrap().code)
    );
}

#[test]
fn reentrant_test_ir_keeps_owned_parent_values_and_cleanup() {
    // 再帰はこのIR fixtureだけに導入します。公開ソースの禁止は維持します。
    let source = r#"
fn leaf(depth:i64,value:[string;1])->[string;1] {return value;}
fn walk(depth:i64,value:[string;1])->[string;1] {
    if depth==0 {return value;}
    saved:[string;1]=[concat(value[0],"!")];
    dynamic:[string]=array_copy(saved);
    child:[string;1]=leaf(depth-1,saved);
    return [concat(dynamic[0],child[0])];
}
for (mut i:i64=0;i<100;i=i+1) {
    result:[string;1]=walk(3,["x"]);
    if i==99 {print(result);}
}
"#;
    let mut p = compile_to_ir(source).unwrap();
    let leaf = p
        .function_definitions
        .iter()
        .find(|f| f.name == "leaf")
        .unwrap()
        .id;
    let walk = p
        .function_definitions
        .iter()
        .find(|f| f.name == "walk")
        .unwrap()
        .id;
    let mut replaced = 0;
    for statement in p.function_definitions.iter_mut().flat_map(|f| &mut f.body) {
        if let ir::StatementKind::Binding { value, .. } = &mut statement.kind
            && let ir::ExprKind::Call {
                function_id,
                function_name,
                ..
            } = &mut value.kind
            && *function_id == leaf
        {
            *function_id = walk;
            *function_name = "walk".into();
            replaced += 1;
        }
    }
    assert_eq!(replaced, 1);
    p.string_heap_limit = 128;
    p.array_heap_limit = 256;
    assert_eq!(
        ir_executor::run_with_call_limit(&p, NonZeroU32::new(4).unwrap()).unwrap(),
        "[\"x!x!!x!!!x!!!\"]\n"
    );
    let error = ir_executor::run_with_call_limit(&p, NonZeroU32::new(3).unwrap()).unwrap_err();
    assert_eq!(error.call_failure().unwrap().active().len(), 3);
    assert_eq!(error.call_failure().unwrap().attempted().function, walk);
    assert_eq!(error.output(), "");
    assert!(compile_to_ir("fn recursive()->void {recursive();} recursive();").is_err());
}
