use cerune_lang::{
    bytecode, codegen::c, compile_to_ir, ir, ir_executor, mir, mir_executor, ssa_executor, vm,
};
use std::{
    fs,
    num::NonZeroU32,
    path::PathBuf,
    process::{Command, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[path = "../support/crash_dialogs.rs"]
mod crash_dialogs;

fn directory() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "cerune-c-call-depth-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}
fn execute(c: &str, optimization: &str) -> Option<Output> {
    let cc = super::compiler()?;
    crash_dialogs::suppress();
    let dir = directory();
    let source = dir.join("program.c");
    let exe = dir.join(format!("program{}", std::env::consts::EXE_SUFFIX));
    fs::write(&source, c).unwrap();
    let compile = super::process::bounded_output(
        Command::new(cc)
            .arg("-std=c11")
            .arg(optimization)
            .arg(&source)
            .arg("-o")
            .arg(&exe),
        &dir,
        "call-depth/compile",
        Duration::from_secs(120),
    )
    .unwrap();
    assert!(
        compile.status.success(),
        "{}; artifacts: {}",
        String::from_utf8_lossy(&compile.stderr),
        dir.display()
    );
    let result = super::process::bounded_output(
        &mut Command::new(exe),
        &dir,
        "call-depth/run",
        Duration::from_secs(30),
    )
    .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    Some(result)
}
fn compare(p: &ir::Program, limit: u32, optimization: &str) {
    let limit = NonZeroU32::new(limit).unwrap();
    let before = ir::text::emit(p);
    let code = c::emit_c_with_call_limit(p, limit).unwrap();
    assert_eq!(before, ir::text::emit(p));
    let expected = ir_executor::run_with_call_limit(p, limit);
    let m = mir::lower(p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(p).unwrap();
    let mm = mir_executor::run_with_call_limit(&m, limit);
    let ss = ssa_executor::run_with_call_limit(&s, limit);
    let vv = vm::run_with_call_limit(&b, limit);
    let Some(actual) = execute(&code, optimization) else {
        return;
    };
    let stderr = String::from_utf8(actual.stderr)
        .unwrap()
        .replace("\r\n", "\n");
    match expected {
        Ok(text) => {
            assert_eq!(mm.unwrap(), text);
            assert_eq!(ss.unwrap(), text);
            assert_eq!(vv.unwrap(), text);
            assert!(actual.status.success(), "{stderr}");
            assert_eq!(actual.stdout, text.as_bytes());
            assert!(stderr.is_empty(), "{stderr}");
        }
        Err(error) => {
            assert!(!actual.status.success());
            assert_eq!(actual.stdout, error.output().as_bytes());
            let mm = mm.unwrap_err();
            let ss = ss.unwrap_err();
            let vv = vv.unwrap_err();
            for output in [mm.output(), ss.output(), vv.output()] {
                assert_eq!(output, error.output());
            }
            if let Some(failure) = error.call_failure() {
                assert_eq!(actual.status.code(), Some(1), "{stderr}");
                for other in [mm.call_failure(), ss.call_failure(), vv.call_failure()] {
                    assert_eq!(other, Some(failure));
                }
                assert_eq!(stderr, format!("{}\n", failure.record()));
            } else {
                let failure = error.runtime_failure().unwrap();
                assert_eq!(mm.runtime_failure(), Some(failure));
                assert_eq!(ss.runtime_failure(), Some(failure));
                assert_eq!(
                    cerune_lang::runtime::FailureCode::from_vm(vv.kind()),
                    Some(failure.code)
                );
                assert!(
                    stderr.starts_with(&format!("cerune: {}\n", failure.record())),
                    "{stderr}"
                );
                assert!(!stderr.contains("call-v1"));
            }
        }
    }
}
#[test]
fn limits_order_helpers_main_and_origins_match_four_executors() {
    for source in [
        include_str!("../../examples/call_frames/main.ceru"),
        include_str!("../../examples/call_frames/depth.ceru"),
        include_str!("../../examples/call_frames/pending_values.ceru"),
        r#"fn leaf()->void {print(7);} fn main()->void {leaf();}"#,
        r#"fn id<T>(x:T)->T {return x;} fn outer()->i64 {return id::<i64>(42);} print(outer());"#,
        r#"fn mark(x:i64)->i64 {print(x);return x;} fn pair(a:i64,b:i64)->i64 {return a+b;} fn outer()->i64 {return pair(mark(1),mark(2));} print(outer());"#,
        r#"fn later()->i64 {print(99);return 0;} fn pair(a:i64,b:i64)->i64 {return a+b;} fn outer()->void {print(7); print(pair(1/0,later()));} outer();"#,
        r#"fn yes()->bool {return true;} fn outer()->void {print(false && yes());print(true || yes());} outer();"#,
        r#"fn leaf(n:i64)->i64 {if n==2 {return 20;} return n;} mut i:i64=0; while i<4 {print(leaf(i));i=i+1;}"#,
        r#"import "std:num" as num; fn parse()->void {print(num::parse_i64("42"));} parse();"#,
    ] {
        let p = cerune_lang::modules::from_source(source)
            .unwrap()
            .to_ir()
            .unwrap();
        for limit in [1, 2] {
            for optimization in ["-O0", "-O2"] {
                compare(&p, limit, optimization);
            }
        }
    }
}
fn redirect(p: &mut ir::Program, from: &str, to: &str) {
    let source = p
        .function_definitions
        .iter()
        .find(|f| f.name == from)
        .unwrap()
        .id;
    let target = p
        .function_definitions
        .iter()
        .find(|f| f.name == to)
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
            && *function_id == source
        {
            *function_id = target;
            *function_name = to.into();
            replaced += 1;
        }
    }
    assert_eq!(replaced, 1);
}
#[test]
fn test_ir_reentry_preserves_owned_values_and_mutual_returns() {
    let source = r#"
fn leaf(depth:i64,value:[string;1])->[string;1] {return value;}
fn walk(depth:i64,value:[string;1])->[string;1] {
    if depth==0 {return value;}
    saved:[string;1]=[concat(value[0],"!")];
    dynamic:[string]=array_copy(saved);
    child:[string;1]=leaf(depth-1,saved);
    return [concat(dynamic[0],child[0])];
}
for (mut i:i64=0;i<100;i=i+1) {result:[string;1]=walk(3,["x"]);if i==99 {print(result);}}
"#;
    let mut p = compile_to_ir(source).unwrap();
    redirect(&mut p, "leaf", "walk");
    p.string_heap_limit = 128;
    p.array_heap_limit = 256;
    assert_eq!(
        ir_executor::run_with_call_limit(&p, NonZeroU32::new(4).unwrap()).unwrap(),
        "[\"x!x!!x!!!x!!!\"]\n"
    );
    for limit in [3, 4] {
        for optimization in ["-O0", "-O2"] {
            compare(&p, limit, optimization);
        }
    }
    let mut p=compile_to_ir("fn leaf(n:i64)->i64{return n;} fn odd(n:i64)->i64 {if n==0{return 0;} v:i64=leaf(n-1);return 10+v;} fn even(n:i64)->i64 {if n==0{return 1;}return 100+odd(n-1);}print(even(4));").unwrap();
    redirect(&mut p, "leaf", "even");
    assert_eq!(
        ir_executor::run_with_call_limit(&p, NonZeroU32::new(5).unwrap()).unwrap(),
        "221\n"
    );
    for limit in [4, 5] {
        for optimization in ["-O0", "-O2"] {
            compare(&p, limit, optimization);
        }
    }
}
#[test]
fn history_allocation_is_fallible_and_never_reported_as_depth_overflow() {
    let source = r#"fn leaf()->void {print("body");} print("before");leaf();print("after");"#;
    let p = compile_to_ir(source).unwrap();
    let original = c::emit_c_with_call_limit(&p, NonZeroU32::new(8).unwrap()).unwrap();
    let mut code = original.replace(
        "realloc(cerune_call_frames, (size_t)next * sizeof(cerune_call_frame))",
        "NULL",
    );
    assert_ne!(code, original);
    // 正常な呼出し記録から、確保失敗でも同じcallの位置を要求します。
    let call = p
        .statements
        .iter()
        .find(|s| matches!(s.kind, ir::StatementKind::Call { .. }))
        .unwrap();
    let expected = cerune_lang::runtime::RuntimeFailure {
        code: cerune_lang::runtime::FailureCode::AllocationFailed,
        node_id: call.id,
        span: call.span,
    }
    .record();
    for optimization in ["-O0", "-O2"] {
        let Some(result) = execute(&code, optimization) else {
            return;
        };
        assert!(!result.status.success());
        assert_eq!(result.stdout, b"before\n");
        let error = String::from_utf8(result.stderr)
            .unwrap()
            .replace("\r\n", "\n");
        assert!(
            error.starts_with(&format!("cerune: {expected}\n")),
            "{error}"
        );
        assert!(!error.contains("call-v1"));
    }
    // 未指定の経路は上限や履歴を追加しません。
    code = c::emit_c(&p).unwrap();
    assert!(!code.contains("cerune_call_"));
}
#[test]
fn finite_scalar_and_aggregate_chains_compare_resource_behavior() {
    use std::fmt::Write;
    for aggregate in [false, true] {
        let mut source = String::from("type Packet {text:string, values:[i64;2],}\n");
        for i in (0..128).rev() {
            if aggregate {
                let body = if i == 127 {
                    "return Packet {text:concat(\"a\",\"b\"),values:[1,2]};".to_owned()
                } else {
                    format!(
                        "v:Packet=step_{}();return Packet {{..v,values:[v.values[0]+1,2]}};",
                        i + 1
                    )
                };
                writeln!(source, "fn step_{i}()->Packet {{{body}}}").unwrap();
            } else {
                let value = if i == 127 {
                    "1".to_owned()
                } else {
                    format!("step_{}()+1", i + 1)
                };
                writeln!(source, "fn step_{i}()->i64 {{return {value};}}").unwrap();
            }
        }
        source.push_str(if aggregate {
            "print(7);v:Packet=step_0();print(v.values[0]);print(v.text);"
        } else {
            "print(7);print(step_0());"
        });
        let p = compile_to_ir(&source).unwrap();
        for limit in [8, 127, 128] {
            compare(&p, limit, "-O0");
        }
    }
}

#[test]
fn cli_call_depth_matches_independent_ir_execution() {
    let dir = directory();
    let source = include_str!("../../examples/call_frames/return_paths.ceru");
    let input = dir.join("source.ceru");
    fs::write(&input, source).unwrap();
    let p = cerune_lang::modules::from_source(source)
        .unwrap()
        .to_ir()
        .unwrap();
    for n in [1, 2, 128] {
        let result = super::process::bounded_output(
            Command::new(env!("CARGO_BIN_EXE_cerune"))
                .arg("emit-c")
                .arg(&input)
                .args(["--call-depth-limit", &n.to_string()]),
            &dir,
            "cli-emit",
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(result.status.success(), "{result:?}");
        let Some(actual) = execute(&String::from_utf8(result.stdout).unwrap(), "-O0") else {
            return;
        };
        match ir_executor::run_with_call_limit(&p, NonZeroU32::new(n).unwrap()) {
            Ok(value) => {
                assert!(actual.status.success(), "{actual:?}");
                assert_eq!(actual.stdout, value.as_bytes());
                assert!(actual.stderr.is_empty());
            }
            Err(error) => {
                assert_eq!(actual.status.code(), Some(1));
                assert_eq!(actual.stdout, error.output().as_bytes());
                assert_eq!(
                    String::from_utf8(actual.stderr)
                        .unwrap()
                        .replace("\r\n", "\n"),
                    format!("{}\n", error.call_failure().unwrap().record())
                );
            }
        }
    }
}
