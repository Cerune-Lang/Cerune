//! 試作C ABI v1。呼出し側は有効な領域、非重複、直列呼出しを保証します。
//! 公開ヘッダと日英契約に、状態・寿命・失敗の責務を記載します。
#[cfg(not(all(
    target_arch = "x86_64",
    any(
        all(target_os = "linux", target_env = "gnu"),
        all(windows, target_env = "msvc")
    )
)))]
compile_error!("process-c-abi supports x86_64 Linux GNU and Windows MSVC only");
mod input;
#[cfg(target_os = "linux")]
use super::linux::Linux as OsBackend;
#[cfg(windows)]
use super::windows::Windows as OsBackend;
use super::*;
use input::OwnedRequest;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
};

const OK: u32 = 0;
const PENDING: u32 = 1;
const READY: u32 = 2;
const VERSION: u32 = 10;
const SHAPE: u32 = 11;
const STATE: u32 = 12;
const RANGE: u32 = 13;
const UTF8: u32 = 14;
const ALLOCATION: u32 = 15;
const CONFIG: u32 = 16;
const PANIC: u32 = 17;
const TARGET: u32 = 18;
type Result<T> = std::result::Result<T, u32>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    pub data: *const u8,
    pub len: u64,
}
#[repr(C)]
pub struct RequestV1 {
    pub version: u64,
    pub size: u64,
    pub target: Bytes,
    pub arguments: *const Bytes,
    pub argument_count: u64,
    pub stdin: Bytes,
    pub stdout_limit: u64,
    pub stderr_limit: u64,
    pub timeout_ms: u64,
    pub node: u64,
    pub source: u64,
    pub start: u64,
    pub end: u64,
}
struct Pending {
    request: OwnedRequest,
    outcome: Outcome,
    transfers: Transfers,
}
#[allow(clippy::large_enum_variant)]
enum State {
    Idle,
    Pending(Pending),
    Ready(Outcome),
    Poisoned,
}
/// 不透明context。要求・結果・未回収OS資源を所有します。
pub struct Context {
    processes: Processes<OsBackend>,
    available: usize,
    state: State,
}
fn width(v: u64) -> Result<usize> {
    usize::try_from(v).map_err(|_| RANGE)
}
fn boundary(f: impl FnOnce() -> Result<u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => e,
        Err(_) => PANIC,
    }
}
unsafe fn input_slice<'a, T>(p: *const T, n: u64) -> Result<&'a [T]> {
    let n = width(n)?;
    if n.checked_mul(size_of::<T>())
        .is_none_or(|v| v > isize::MAX as usize)
    {
        return Err(RANGE);
    }
    if n == 0 {
        return Ok(&[]);
    }
    if p.is_null() || !(p as usize).is_multiple_of(align_of::<T>()) {
        return Err(SHAPE);
    }
    // 呼出し契約：n個の初期化済みTが読取り可能で、この呼出し中は変更されない。
    Ok(unsafe { slice::from_raw_parts(p, n) })
}
unsafe fn output_ptr<T>(p: *mut T) -> Result<()> {
    if p.is_null() || !(p as usize).is_multiple_of(align_of::<T>()) {
        Err(SHAPE)
    } else {
        Ok(())
    }
}
unsafe fn with_context(p: *mut Context, f: impl FnOnce(&mut Context) -> Result<u32>) -> u32 {
    boundary(|| {
        unsafe {
            output_ptr(p)?;
        }
        // 同じcontextへの同時・再入呼出しは禁止。入力/出力はcontextと非重複。
        let c = unsafe { &mut *p };
        if matches!(c.state, State::Poisoned) {
            return Err(STATE);
        }
        match catch_unwind(AssertUnwindSafe(|| f(c))) {
            Ok(v) => v,
            Err(_) => {
                c.state = State::Poisoned;
                Err(PANIC)
            }
        }
    })
}
/// version=1、target=1 Windows MSVC / 2 Linux GNU。空設定は未接続。
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_create(
    version: u64,
    target: u64,
    config: Bytes,
    out: *mut *mut Context,
) -> u32 {
    boundary(|| {
        unsafe {
            output_ptr(out)?;
            *out = ptr::null_mut();
        }
        if version != 1 {
            return Err(VERSION);
        }
        if target != if cfg!(windows) { 1 } else { 2 } {
            return Err(TARGET);
        }
        let bytes = unsafe { input_slice(config.data, config.len)? };
        let (processes, available) = if bytes.is_empty() {
            (
                Processes::unconfigured(Limits {
                    arguments: usize::MAX,
                    argument_bytes: usize::MAX,
                    stdin_bytes: usize::MAX,
                    output_bytes: usize::MAX,
                    timeout: Duration::MAX,
                    cleanup_timeout: Duration::MAX,
                }),
                usize::MAX,
            )
        } else {
            configuration::decode(bytes).map_err(|_| CONFIG)?
        };
        let context = Box::new(Context {
            processes,
            available,
            state: State::Idle,
        });
        unsafe {
            *out = Box::into_raw(context);
        }
        Ok(OK)
    })
}
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_prepare(c: *mut Context, r: *const RequestV1) -> u32 {
    unsafe {
        with_context(c, |c| {
            if !matches!(c.state, State::Idle) {
                return Err(STATE);
            }
            let r = input_slice(r, 1)?.first().unwrap();
            if r.version != 1 {
                return Err(VERSION);
            }
            if r.size != size_of::<RequestV1>() as u64 {
                return Err(SHAPE);
            }
            let origin = Origin {
                node_id: crate::ir::NodeId(width(r.node)?),
                span: {
                    if r.start > r.end {
                        return Err(RANGE);
                    }
                    crate::source::Span::in_source(
                        crate::source::SourceId::from_index(width(r.source)?),
                        width(r.start)?,
                        width(r.end)?,
                    )
                },
            };
            let request = OwnedRequest::copy(r)?;
            match c.processes.prepare(request.view(), c.available, origin) {
                Preparation::Complete(out) => {
                    c.state = State::Ready(out);
                    Ok(READY)
                }
                Preparation::Pending(prepared) => {
                    // 同じ親module内でだけ分解。接続・要求はcontextが非公開のまま固定。
                    let Prepared {
                        outcome, transfers, ..
                    } = prepared;
                    c.state = State::Pending(Pending {
                        request,
                        outcome,
                        transfers,
                    });
                    Ok(PENDING)
                }
            }
        })
    }
}
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_execute(c: *mut Context, reservation: u64) -> u32 {
    unsafe {
        with_context(c, |c| {
            let reservation = match reservation {
                0 => Ok(()),
                1 => Err(ReservationError::Capacity),
                2 => Err(ReservationError::Allocation),
                _ => return Err(RANGE),
            };
            if !matches!(c.state, State::Pending(_)) {
                return Err(STATE);
            }
            let State::Pending(Pending {
                request,
                outcome,
                transfers,
            }) = std::mem::replace(&mut c.state, State::Poisoned)
            else {
                unreachable!()
            };
            // 既存Preparedへ戻すことで失敗選択・記録・後始末を二重実装しません。
            let prepared = Prepared {
                processes: &mut c.processes,
                request: request.view(),
                outcome,
                transfers,
            };
            let (outcome, _) = prepared.execute(reservation);
            c.state = State::Ready(outcome);
            Ok(READY)
        })
    }
}
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_report(
    c: *mut Context,
    out: *mut u64,
    count: u64,
) -> u32 {
    unsafe {
        with_context(c, |c| {
            if count != report_codec::WORD_COUNT as u64 {
                return Err(SHAPE);
            }
            output_ptr(out)?;
            let report = match &c.state {
                State::Pending(p) => &p.outcome.report,
                State::Ready(o) => &o.report,
                _ => return Err(STATE),
            };
            let v = report_codec::ReportV1::encode(report).map_err(|_| RANGE)?;
            ptr::copy_nonoverlapping(v.words().as_ptr(), out, report_codec::WORD_COUNT);
            Ok(OK)
        })
    }
}
/// channel=1 stdout / 2 stderr。位置指定のコピーは状態・読取り位置を変更しません。
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_copy(
    c: *mut Context,
    channel: u64,
    offset: u64,
    out: *mut u8,
    capacity: u64,
    written: *mut u64,
) -> u32 {
    unsafe {
        with_context(c, |c| {
            output_ptr(written)?;
            let State::Ready(o) = &c.state else {
                return Err(STATE);
            };
            let bytes = match channel {
                1 => &o.stdout,
                2 => &o.stderr,
                _ => return Err(RANGE),
            };
            let offset = width(offset)?;
            let rest = bytes.get(offset..).ok_or(RANGE)?;
            let capacity = width(capacity)?;
            if capacity > isize::MAX as usize {
                return Err(RANGE);
            }
            let n = capacity.min(rest.len());
            if n > 0 {
                output_ptr(out)?;
                ptr::copy_nonoverlapping(rest.as_ptr(), out, n);
            }
            *written = n as u64;
            Ok(OK)
        })
    }
}
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_release(c: *mut Context) -> u32 {
    unsafe {
        with_context(c, |c| {
            if matches!(c.state, State::Idle) {
                return Err(STATE);
            }
            c.state = State::Idle; // Backend側の未回収資源・disabledは維持。
            Ok(OK)
        })
    }
}
/// # Safety
/// ヘッダの長さに対応する有効・初期化済みの入力と、非重複の書込み領域が必要です。
/// contextはこのABIで作成した未破棄のものに限り、呼出しは直列・再入なしです。
/// destroy後のhandleや不正な非NULLポインタは検査できません。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cerune_process_v1_destroy(c: *mut *mut Context) -> u32 {
    boundary(|| {
        unsafe {
            output_ptr(c)?;
            let p = *c;
            output_ptr(p)?;
            *c = ptr::null_mut();
            drop(Box::from_raw(p));
        }
        Ok(OK)
    })
}

#[cfg(test)]
mod tests;
