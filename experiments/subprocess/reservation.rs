//! 実OS接続で返却領域予約の失敗・次の明示実行・確保済み領域への転送を確認します。
use cerune_lang::runtime::process::{
    Backend, Cleanup, Code, Exit, Limits, Origin, Phase, Processes, Request, ReservationError,
    Started,
};
use serde_json::json;
use std::{io, time::Duration};

pub fn check<B: Backend>(
    backend: B,
    limits: Limits,
    origin: Origin,
) -> io::Result<serde_json::Value> {
    let mut p = Processes::with_trace_limit(backend, limits, 64);
    let input = &[255u8, 0, 13, 10];
    let request = Request {
        target: "fixture",
        arguments: &["--fixture", "transport-echo"],
        stdin: input,
        stdout_limit: 64,
        stderr_limit: 64,
        timeout: Duration::from_secs(10),
    };
    let (failed, storage) = p.run_with_reservation(request, 1 << 20, origin, || {
        Err::<(), _>(ReservationError::Capacity)
    });
    if storage.is_some()
        || failed.report.started != Started::No
        || failed
            .report
            .failure
            .is_none_or(|f| f.code != Code::OutputBudget || f.phase != Phase::Reserve)
        || failed.report.origin != origin
        || failed.report.cleanup.is_some()
        || p.events().iter().any(|e| e.phase == Phase::Start)
    {
        return Err(io::Error::other(
            "reservation failure launched or lost its report",
        ));
    }
    let (out, storage) = p.run_with_reservation(request, 1 << 20, origin, || {
        let mut arrays = [Vec::<u64>::new(), Vec::<u64>::new()];
        for array in &mut arrays {
            array
                .try_reserve_exact(64)
                .map_err(|_| ReservationError::Allocation)?;
        }
        let locations = arrays.each_ref().map(|a| (a.as_ptr(), a.capacity()));
        Ok((arrays, locations))
    });
    if !out.completed()
        || out.report.exit != Some(Exit::Code(7))
        || out.report.origin != origin
        || out.stdout != input
        || out.stderr != b"err\0\r\n"
        || !matches!(out.report.cleanup, Some(Cleanup::Complete { .. }))
        || out.transfers.is_none_or(|s| s.call.get() != 2)
    {
        return Err(io::Error::other(format!(
            "reserved execution: {:?}",
            out.report
        )));
    }
    let (mut arrays, locations) = storage.ok_or_else(|| io::Error::other("reservation lost"))?;
    for ((array, bytes), location) in arrays
        .iter_mut()
        .zip([&out.stdout, &out.stderr])
        .zip(locations)
    {
        if bytes.len() > array.capacity() {
            return Err(io::Error::other("output exceeded reserved storage"));
        }
        array.extend(bytes.iter().map(|&b| u64::from(b)));
        if (array.as_ptr(), array.capacity()) != location {
            return Err(io::Error::other("result conversion reallocated storage"));
        }
    }
    Ok(json!({
        "reservation_failure":"output-budget",
        "failed_call_started":false,
        "next_call":2,
        "exit":7,
        "stdout_length":arrays[0].len(),
        "stderr_length":arrays[1].len(),
        "conversion_reallocated":false,
        "origin_preserved":true
    }))
}
