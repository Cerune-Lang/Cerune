//! 実行器の物理stackを測る子プロセス。通常のCerune CLIではありません。
use cerune_lang::{bytecode, ir_executor, mir, mir_executor, runtime::call, ssa_executor, vm};
use serde_json::{Value, json};
use std::{io::Write, num::NonZeroU32};
#[path = "../../tests/support/crash_dialogs.rs"]
mod crash_dialogs;

fn event(value: Value) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}
fn result(output: Result<String, (Option<call::Failure>, String, String)>) -> Value {
    match output {
        Ok(output) => json!({"kind":"completed", "output":output}),
        Err((Some(failure), output, _)) => json!({"kind":"depth-limit", "output":output,
            "limit":failure.limit().get(), "active":failure.active().len(), "record":failure.record()}),
        Err((None, output, message)) => {
            json!({"kind":"runtime-error", "output":output, "message":message})
        }
    }
}
fn main() -> Result<(), String> {
    crash_dialogs::suppress();
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [route, file, limit, stack] = args.as_slice() else {
        return Err(
            "usage: call_depth_probe <hir|mir|ssa|vm> <file.ceru> <positive-limit> <stack-bytes>"
                .into(),
        );
    };
    if !["hir", "mir", "ssa", "vm"].contains(&route.as_str()) {
        return Err("unknown route".into());
    }
    let limit: NonZeroU32 = limit.parse().map_err(|_| "invalid positive depth limit")?;
    let stack: usize = stack.parse().map_err(|_| "invalid stack size")?;
    if !(64 * 1024..=256 * 1024 * 1024).contains(&stack) {
        return Err("probe stack request must be 64 KiB..256 MiB".into());
    }
    event(
        json!({"stage":"preparing", "route":route, "limit":limit.get(),
        "requested_stack_bytes":stack, "os":std::env::consts::OS, "arch":std::env::consts::ARCH,
        "debug_assertions":cfg!(debug_assertions)}),
    );
    let source = std::fs::read_to_string(file).map_err(|e| format!("{e:?}"))?;
    let hir = cerune_lang::compile_to_ir(&source).map_err(|e| format!("{e:?}"))?;
    // 変換・検証用の構築処理は測定threadへ入る前に終えます。
    let mir = mir::lower(&hir).map_err(|e| format!("{e:?}"))?;
    let ssa = mir::ssa::construct(&mir).map_err(|e| format!("{e:?}"))?;
    let bytecode = bytecode::lower(&hir).map_err(|e| format!("{e:?}"))?;
    let source_functions = hir
        .function_definitions
        .iter()
        .filter(|f| f.lowering.is_none())
        .count();
    event(
        json!({"stage":"prepared", "source_functions":source_functions,
        "generated_helpers":hir.function_definitions.len()-source_functions}),
    );
    let route = route.clone();
    let value = std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("cerune-depth-probe".into())
            .stack_size(stack)
            .spawn_scoped(scope, || {
                event(json!({"stage":"executing"}));
                match route.as_str() {
                    "hir" => result(ir_executor::run_with_call_limit(&hir, limit).map_err(|e| {
                        (
                            e.call_failure().cloned(),
                            e.output().into(),
                            format!("{e:?}"),
                        )
                    })),
                    "mir" => result(mir_executor::run_with_call_limit(&mir, limit).map_err(|e| {
                        (
                            e.call_failure().cloned(),
                            e.output().into(),
                            format!("{e:?}"),
                        )
                    })),
                    "ssa" => result(ssa_executor::run_with_call_limit(&ssa, limit).map_err(|e| {
                        (
                            e.call_failure().cloned(),
                            e.output().into(),
                            format!("{e:?}"),
                        )
                    })),
                    "vm" => result(vm::run_with_call_limit(&bytecode, limit).map_err(|e| {
                        (
                            e.call_failure().cloned(),
                            e.output().into(),
                            format!("{e:?}"),
                        )
                    })),
                    _ => unreachable!(),
                }
            })
            .map_err(|e| format!("thread creation failed: {e}"))?;
        // OSによるabortはjoinのErrとして回収できません。外側のdriverが終了状態を残します。
        worker.join().map_err(|_| "worker panicked".to_owned())
    })?;
    event(json!({"stage":"finished", "result":value}));
    Ok(())
}
