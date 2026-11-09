//! 同じCeruneプログラムを明示した実OS接続で4回実行し、結果を比較します。
use cerune_lang::{
    bytecode, ir_executor, mir, mir_executor,
    runtime::process::{Backend, Limits, Processes},
    ssa_executor, vm,
};
use std::io;
pub fn check<B: Backend>(
    mut backend: impl FnMut() -> B,
    limits: Limits,
) -> io::Result<serde_json::Value> {
    let source = r#"
import "std:process" as p;
import "std:process/run" as process;
request:p::Request=p::Request {
    target:"fixture",arguments:array_copy(["--fixture","transport-echo"]),
    stdin:array_copy([u8(255),u8(0),u8(13),u8(10)]),
    stdout_limit:32,stderr_limit:32,timeout_ms:10000,
};
out:p::Outcome=process::run(request);
print(out.report.failure);print(out.report.exit);
print(out.stdout);print(out.stderr);print(out.report.stdin_accepted);
print("continued");
"#;
    let p = cerune_lang::compile_to_ir(source).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let m = mir::lower(&p).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let s = mir::ssa::construct(&m).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let b = bytecode::lower(&p).map_err(|e| io::Error::other(format!("{e:?}")))?;
    let expected =
        "None{}\nCode{value: 7}\n[255, 0, 13, 10]\n[101, 114, 114, 0, 13, 10]\n4\ncontinued\n";
    for route in 0..4 {
        let mut processes = Processes::new(backend(), limits);
        let output = match route {
            0 => ir_executor::run_with_processes(&p, &mut processes, 32768)
                .map_err(|e| format!("{e:?}")),
            1 => mir_executor::run_with_processes(&m, &mut processes, 32768)
                .map_err(|e| format!("{e:?}")),
            2 => ssa_executor::run_with_processes(&s, &mut processes, 32768)
                .map_err(|e| format!("{e:?}")),
            _ => vm::run_with_processes(&b, &mut processes, 32768).map_err(|e| format!("{e:?}")),
        }
        .map_err(io::Error::other)?;
        if output != expected || processes.is_disabled() {
            return Err(io::Error::other(format!("executor {route}: {output}")));
        }
    }
    Ok(serde_json::json!({"executors":4,"exit":7,"stdout":[255,0,13,10],"continued":true}))
}
