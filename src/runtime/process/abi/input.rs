//! 呼出し中に外部値をコピーし、execute後まで参照先を固定します。
use super::*;
pub(super) struct OwnedRequest {
    target: String,
    // viewsはstringsの個別heap領域だけを参照し、struct自体を参照しません。
    views: Vec<&'static str>,
    _strings: Vec<String>,
    stdin: Vec<u8>,
    stdout: usize,
    stderr: usize,
    timeout: Duration,
}
fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| ALLOCATION)?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}
unsafe fn text(bytes: Bytes) -> Result<String> {
    let bytes = unsafe { input_slice(bytes.data, bytes.len)? };
    std::str::from_utf8(bytes).map_err(|_| UTF8)?;
    String::from_utf8(copy_bytes(bytes)?).map_err(|_| UTF8)
}
impl OwnedRequest {
    pub(super) unsafe fn copy(r: &RequestV1) -> Result<Self> {
        let target = unsafe { text(r.target)? };
        let args = unsafe { input_slice(r.arguments, r.argument_count)? };
        let mut strings = Vec::new();
        strings
            .try_reserve_exact(args.len())
            .map_err(|_| ALLOCATION)?;
        let mut views = Vec::new();
        views
            .try_reserve_exact(args.len())
            .map_err(|_| ALLOCATION)?;
        for arg in args {
            strings.push(unsafe { text(*arg)? });
        }
        for arg in &strings {
            // SAFETY: Stringのheap領域は移動しても不変。fieldsは非公開で変更・分離せず、
            // view()で返す寿命は&selfへ縮める。viewsはdrop時に参照を読みません。
            // これによりexecuteで引数viewを再確保する必要がありません。
            views.push(unsafe { &*(arg.as_str() as *const str) });
        }
        Ok(Self {
            target,
            views,
            _strings: strings,
            stdin: copy_bytes(unsafe { input_slice(r.stdin.data, r.stdin.len)? })?,
            stdout: width(r.stdout_limit)?,
            stderr: width(r.stderr_limit)?,
            timeout: Duration::from_millis(r.timeout_ms),
        })
    }
    pub(super) fn view(&self) -> Request<'_> {
        Request {
            target: &self.target,
            arguments: &self.views,
            stdin: &self.stdin,
            stdout_limit: self.stdout,
            stderr_limit: self.stderr,
            timeout: self.timeout,
        }
    }
}
