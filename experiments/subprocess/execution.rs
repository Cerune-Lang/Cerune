//! 同じCeruneプログラムを明示した実OS接続で4回実行し、結果を比較します。
use cerune_lang::{
    bytecode, ir_executor, mir, mir_executor,
    runtime::process::{
        Backend, Exit, Limits, Origin, Preparation, Processes, Request, report_codec::ReportV1,
    },
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
    // 言語実行とは別に同じ専用fixtureで段階APIと固定Reportを確認します。
    let mut processes = Processes::new(backend(), limits);
    let origin = Origin {
        node_id: cerune_lang::ir::NodeId(17),
        span: cerune_lang::source::Span::new(30, 41),
    };
    let request = Request {
        target: "fixture",
        arguments: &["--fixture", "transport-echo"],
        stdin: &[255, 0, 13, 10],
        stdout_limit: 32,
        stderr_limit: 32,
        timeout: std::time::Duration::from_millis(10000),
    };
    let Preparation::Pending(prepared) = processes.prepare(request, 32768, origin) else {
        return Err(io::Error::other("expected pending preparation"));
    };
    let (outcome, _) = prepared.execute(Ok(()));
    let record =
        ReportV1::encode(&outcome.report).map_err(|e| io::Error::other(format!("{e:?}")))?;
    if outcome.report.failure.is_some()
        || outcome.report.exit != Some(Exit::Code(7))
        || outcome.report.origin != origin
        || outcome.stdout != [255, 0, 13, 10]
        || outcome.stderr != b"err\0\r\n"
        || outcome.report.stdin_accepted != 4
        || ReportV1::from_le_bytes(&record.to_le_bytes())
            .map_err(|e| io::Error::other(format!("{e:?}")))?
            .to_report()
            != outcome.report
    {
        return Err(io::Error::other(format!(
            "staged process result: {outcome:?}"
        )));
    }
    Ok(
        serde_json::json!({"executors":4,"exit":7,"stdout":[255,0,13,10],"continued":true,"staged":true,"report_words":32}),
    )
}
