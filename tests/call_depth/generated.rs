use cerune_lang::{
    bytecode,
    codegen::{llvm, qbe, wat, x86_64},
    compile_to_ir, ir, ir_executor, mir, mir_executor, ssa_executor, vm,
};
use std::{
    fs,
    num::NonZeroU32,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};
#[path = "../support/crash_dialogs.rs"]
mod crash_dialogs;
use super::process;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn run(c: &mut Command, dir: &Path, label: &str) -> Output {
    process::bounded_output(
        c,
        dir,
        label,
        Duration::from_secs(if label == "compile" { 120 } else { 30 }),
    )
    .unwrap()
}
fn tool(key: &str, default: &str, arg: &str) -> Option<std::ffi::OsString> {
    let configured = std::env::var_os(key);
    let tool = configured.clone().unwrap_or_else(|| default.into());
    if Command::new(&tool)
        .arg(arg)
        .output()
        .is_ok_and(|o| o.status.success())
    {
        Some(tool)
    } else {
        assert!(configured.is_none(), "configured {key} unavailable");
        eprintln!("{ROUTE} depth execution skipped: {key}");
        None
    }
}
fn execute(
    p: &ir::Program,
    limit: NonZeroU32,
    opt: &str,
    allocation_failure: bool,
) -> Option<Vec<Output>> {
    execute_configured(p, limit, opt, allocation_failure, None)
}
fn execute_configured(p: &ir::Program, limit: NonZeroU32, opt: &str, allocation_failure: bool, cli_source: Option<&str>) -> Option<Vec<Output>> {
    crash_dialogs::suppress();
    let cc = if ROUTE == "wat" {
        tool("CERUNE_TEST_NODE", "node", "--version")?
    } else {
        tool(
            match ROUTE {
                "llvm" => "CERUNE_TEST_LLVM_CLANG",
                "qbe" => "CERUNE_TEST_QBE_CLANG",
                _ => "CERUNE_TEST_ASM_CLANG",
            },
            "clang",
            "--version",
        )?
    };
    let qbe_tool = if ROUTE == "qbe" {
        Some(tool("CERUNE_TEST_QBE", "qbe", "-h")?)
    } else {
        None
    };
    let wabt = std::env::var_os("CERUNE_TEST_WAT2WASM_JS")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/wasm-tools/node_modules/wabt/bin/wat2wasm"));
    if ROUTE == "wat" && !wabt.is_file() {
        assert!(std::env::var_os("CERUNE_TEST_WAT2WASM_JS").is_none());
        eprintln!("WAT tool unavailable");
        return None;
    }
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = root().join("target").join(format!(
        "call-depth-{ROUTE}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let target = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let (extension, mut text) = match ROUTE {
        "llvm" => (
            "ll",
            llvm::emit_llvm_with_call_limit(p, llvm::Target::parse(target).unwrap(), limit)
                .unwrap(),
        ),
        "qbe" => (
            "ssa",
            qbe::emit_qbe_with_call_limit(p, qbe::Target::parse(target).unwrap(), limit).unwrap(),
        ),
        "wat" => ("wat", wat::emit_wat_with_call_limit(p, limit).unwrap()),
        "native" => {
            let original = mir::lower(p).unwrap();
            let m = if opt == "-O2" {
                mir::ssa::lower(&mir::ssa::construct(&original).unwrap())
                    .unwrap()
                    .program
            } else {
                original
            };
            let lir = x86_64::lower_mir_with_call_limit(
                &m,
                x86_64::Target::parse(target).unwrap(),
                limit,
            )
            .unwrap();
            assert!(
                x86_64::lir::emit(&lir)
                    .unwrap()
                    .contains(&format!("call-depth-limit {limit}"))
            );
            if !allocation_failure {
                fs::write(
                    dir.join("program.o"),
                    x86_64::emit_object_from_lir(&lir, true).unwrap(),
                )
                .unwrap();
            }
            ("s", x86_64::emit_asm_from_lir(&lir, true).unwrap())
        }
        _ => unreachable!(),
    };
    if let Some(source) = cli_source {
        let input = dir.join("source.ceru");
        fs::write(&input, source).unwrap();
        let artifact = dir.join(format!("cli.{extension}"));
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cerune"));
        cmd.arg(match ROUTE { "native"=>"emit-asm", "llvm"=>"emit-llvm", "qbe"=>"emit-qbe", "wat"=>"emit-wat", _=>unreachable!() })
            .arg(&input).args(["--call-depth-limit", &limit.to_string()]).arg("-o").arg(&artifact);
        if ROUTE != "wat" { cmd.args(["--target", target]); }
        if ROUTE == "native" {cmd.arg("--annotate-origins");}
        let result=run(&mut cmd,&dir,"cli-emit");
        assert!(result.status.success(), "{}",String::from_utf8_lossy(&result.stderr));
        text=fs::read_to_string(&artifact).unwrap();
        if ROUTE=="native" {
            let result=run(Command::new(env!("CARGO_BIN_EXE_cerune")).arg("emit-obj").arg(&input)
                .args(["--target",target,"--annotate-origins","--call-depth-limit",&limit.to_string()])
                .arg("-o").arg(dir.join("program.o")),&dir,"cli-object");
            assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
        }
    }
    if allocation_failure {
        text = match ROUTE {
            "llvm" => text.replace(
                "%new = call ptr @realloc(ptr %old, i64 %bytes)",
                "%new = inttoptr i64 0 to ptr",
            ),
            "qbe" => text.replace("%new =l call $realloc(l %old, l %bytes)", "%new =l copy 0"),
            "wat" => text.replace(
                "(call $cerune_frame_allocate (i64.const 16))",
                "(i32.const 0)",
            ),
            "native" => text
                .replace("callq realloc@PLT", "xorq %rax, %rax")
                .replace("callq realloc", "xorq %rax, %rax"),
            _ => unreachable!(),
        };
    }
    let file = dir.join(format!("program.{extension}"));
    fs::write(&file, &text).unwrap();
    let mut inputs = vec![file.clone()];
    if ROUTE == "qbe" {
        let asm = dir.join("program.s");
        let result = run(
            Command::new(qbe_tool.unwrap())
                .args([
                    "-t",
                    if cfg!(windows) {
                        "amd64_win"
                    } else {
                        "amd64_sysv"
                    },
                ])
                .arg("-o")
                .arg(&asm)
                .arg(&file),
            &dir,
            "qbe",
        );
        assert!(
            result.status.success(),
            "{}; {}",
            String::from_utf8_lossy(&result.stderr),
            dir.display()
        );
        inputs = vec![asm];
    }
    if ROUTE == "wat" {
        assert!(!text.contains("(export \"memory\""));
        let wasm = dir.join("program.wasm");
        let result = run(
            Command::new(&cc).arg(wabt).arg(&file).arg("-o").arg(&wasm),
            &dir,
            "wat",
        );
        assert!(
            result.status.success(),
            "{}; {}",
            String::from_utf8_lossy(&result.stderr),
            dir.display()
        );
        return Some(vec![run(
            Command::new(cc)
                .arg(root().join("tests/support/run_wasm.cjs"))
                .arg(wasm),
            &dir,
            "execute",
        )]);
    }
    if ROUTE == "native" && !allocation_failure {
        inputs.push(dir.join("program.o"));
    }
    let mut outputs = Vec::new();
    for (id, input) in inputs.iter().enumerate() {
        let exe = dir.join(format!("program-{id}{}", std::env::consts::EXE_SUFFIX));
        let result = run(
            Command::new(&cc)
                .arg(format!("--target={target}"))
                .arg(opt)
                .arg(input)
                .arg("-o")
                .arg(&exe),
            &dir,
            "compile",
        );
        assert!(
            result.status.success(),
            "{}; {}",
            String::from_utf8_lossy(&result.stderr),
            dir.display()
        );
        outputs.push(run(&mut Command::new(exe), &dir, "execute"));
    }
    Some(outputs)
}
fn compare(p: &ir::Program, limit: u32, opt: &str) {
    let limit = NonZeroU32::new(limit).unwrap();
    let before = ir::text::emit(p);
    let expected = ir_executor::run_with_call_limit(p, limit);
    let m = mir::lower(p).unwrap();
    let s = mir::ssa::construct(&m).unwrap();
    let b = bytecode::lower(p).unwrap();
    let mm = mir_executor::run_with_call_limit(&m, limit);
    let ss = ssa_executor::run_with_call_limit(&s, limit);
    let vv = vm::run_with_call_limit(&b, limit);
    let Some(results) = execute(p, limit, opt, false) else {
        return;
    };
    assert_eq!(before, ir::text::emit(p));
    for result in results {
        let stderr = String::from_utf8(result.stderr)
            .unwrap()
            .replace("\r\n", "\n");
        match &expected {
            Ok(text) => {
                assert_eq!(mm.as_ref().unwrap(), text);
                assert_eq!(ss.as_ref().unwrap(), text);
                assert_eq!(vv.as_ref().unwrap(), text);
                assert!(result.status.success(), "{stderr}");
                assert_eq!(result.stdout, text.as_bytes());
                assert!(stderr.is_empty(), "{stderr}");
            }
            Err(error) => {
                assert!(!result.status.success());
                assert_eq!(result.stdout, error.output().as_bytes());
                if let Some(f) = error.call_failure() {
                    assert_eq!(result.status.code(), Some(1));
                    assert_eq!(stderr, format!("{}\n", f.record()));
                    assert_eq!(mm.as_ref().unwrap_err().call_failure(), Some(f));
                    assert_eq!(ss.as_ref().unwrap_err().call_failure(), Some(f));
                    assert_eq!(vv.as_ref().unwrap_err().call_failure(), Some(f));
                } else {
                    assert!(
                        stderr.starts_with(&format!(
                            "cerune: {}\n",
                            error.runtime_failure().unwrap().record()
                        )),
                        "{stderr}"
                    );
                    assert!(!stderr.contains("call-v1"));
                }
            }
        }
    }
}
#[test]
fn limits_arguments_returns_helpers_and_origins() {
    for source in [
        include_str!("../../examples/call_frames/main.ceru"),
        include_str!("../../examples/call_frames/depth.ceru"),
        include_str!("../../examples/call_frames/pending_values.ceru"),
        "fn leaf()->void{print(7);}fn main()->void{leaf();}",
        "fn id<T>(x:T)->T{return x;}fn outer()->i64{return id::<i64>(42);}print(outer());",
        "fn mark(x:i64)->i64{print(x);return x;}fn pair(a:i64,b:i64)->i64{return a+b;}fn outer()->i64{return pair(mark(1),mark(2));}print(outer());",
        "fn later()->i64{print(99);return 0;}fn pair(a:i64,b:i64)->i64{return a+b;}fn outer()->void{print(7);print(pair(1/0,later()));}outer();",
        "fn yes()->bool{return true;}fn outer()->void{print(false&&yes());print(true||yes());}outer();",
        "fn leaf(n:i64)->i64{if n==2{return 20;}return n;}mut i:i64=0;while i<4{print(leaf(i));i=i+1;}",
        "import \"std:num\" as num;fn parse()->void{print(num::parse_i64(\"42\"));}parse();",
    ] {
        let p = cerune_lang::modules::from_source(source)
            .unwrap()
            .to_ir()
            .unwrap();
        for limit in [1, 2] {
            for opt in if ROUTE == "llvm" {
                &["-O0", "-O2"][..]
            } else {
                &["-O0"][..]
            } {
                compare(&p, limit, opt);
            }
        }
    }
}
#[test]
fn allocation_failure_stops_without_a_depth_record() {
    let p = cerune_lang::compile_to_ir("fn leaf()->void{print(999);}print(7);leaf();print(888);")
        .unwrap();
    let Some(results) = execute(&p, NonZeroU32::new(8).unwrap(), "-O0", true) else {
        return;
    };
    let site = p
        .statements
        .iter()
        .find(|s| matches!(s.kind, ir::StatementKind::Call { .. }))
        .unwrap();
    let record = cerune_lang::runtime::RuntimeFailure {
        code: cerune_lang::runtime::FailureCode::AllocationFailed,
        node_id: site.id,
        span: site.span,
    }
    .record();
    for r in results {
        assert!(!r.status.success());
        assert_eq!(r.stdout, b"7\n");
        assert_eq!(
            String::from_utf8(r.stderr).unwrap().replace("\r\n", "\n"),
            format!("cerune: {record}\n")
        );
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
fn finite_scalar_and_aggregate_chains_compare_resource_behavior() {
    use std::fmt::Write;
    for aggregate in [false, true] {
        let mut source = String::from("type Packet {text:string, values:[i64;2],}\n");
        for i in (0..32).rev() {
            if aggregate {
                let body = if i == 31 {
                    "return Packet {text:concat(\"a\",\"b\"),values:[1,2]};".to_owned()
                } else {
                    format!(
                        "v:Packet=step_{}();return Packet {{..v,values:[v.values[0]+1,2]}};",
                        i + 1
                    )
                };
                writeln!(source, "fn step_{i}()->Packet {{{body}}}").unwrap();
            } else {
                let value = if i == 31 {
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
        for limit in [8, 31, 32] {
            compare(&p, limit, "-O0");
        }
    }
}

#[test]
fn float_and_stack_arguments_and_maximum_explicit_limit() {
    let p=compile_to_ir("fn leaf(a:f64,b:f32,c:i64,d:i64,e:f64,f:i64,g:f64)->f64{return a+f64(b)+e+g;}fn outer()->void{print(leaf(1.0,2.0,3,4,5.0,6,7.0)==15.0);}outer();").unwrap();
    for limit in [1, 2, u32::MAX] {
        compare(&p, limit, "-O0");
    }
}

#[test]
fn deep_reentry_uses_small_ir_and_keeps_owned_parent_values() {
    let mut p = compile_to_ir(
        r#"
type Packet { text: string, n: i64, }
fn leaf(n: i64, p: Packet) -> Packet { return p; }
fn walk(n: i64, p: Packet) -> Packet {
    if n == 0 { return p; }
    saved: Packet = p;
    child: Packet = leaf(n - 1, p);
    return Packet { text: saved.text, n: child.n + 1 };
}
print("before");
result: Packet = walk(127, Packet { text: concat("a", "b"), n: 1 });
print(result.n); print(result.text); print("after");
"#,
    )
    .unwrap();
    redirect(&mut p, "leaf", "walk");
    p.string_heap_limit = 4;
    assert_eq!(
        ir_executor::run_with_call_limit(&p, NonZeroU32::new(128).unwrap()).unwrap(),
        "before\n128\nab\nafter\n"
    );
    for limit in [8, 127, 128] {
        compare(&p, limit, "-O0");
    }
}

#[test]
fn return_paths_example_and_no_source_calls() {
    let p = cerune_lang::modules::load(&root().join("examples/call_frames/return_paths.ceru"))
        .unwrap()
        .to_ir()
        .unwrap();
    assert_eq!(
        ir_executor::run_with_call_limit(&p, NonZeroU32::new(2).unwrap()).unwrap(),
        include_str!("../../examples/call_frames/return_paths.stdout")
    );
    for limit in [1, 2] {
        compare(&p, limit, "-O2");
    }
    compare(&compile_to_ir("print(42);").unwrap(), u32::MAX, "-O0");
}

#[test]
fn cli_call_depth_matches_independent_ir_execution() {
    let source=include_str!("../../examples/call_frames/return_paths.ceru");
    let p=cerune_lang::modules::from_source(source).unwrap().to_ir().unwrap();
    for n in [1,2,128] {
        let limit=NonZeroU32::new(n).unwrap();
        let expected=ir_executor::run_with_call_limit(&p,limit);
        let Some(results)=execute_configured(&p,limit,"-O0",false,Some(source)) else {return};
        for result in results {
            match &expected {
                Ok(value)=>{assert!(result.status.success(),"{:?}",result);assert_eq!(&result.stdout,value.as_bytes());assert!(result.stderr.is_empty());}
                Err(error)=>{assert_eq!(result.status.code(),Some(1));assert_eq!(result.stdout,error.output().as_bytes());assert_eq!(String::from_utf8(result.stderr).unwrap().replace("\r\n","\n"),format!("{}\n",error.call_failure().unwrap().record()));}
            }
        }
    }
}
