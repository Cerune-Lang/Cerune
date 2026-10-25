//! 明示した上限でCを保存する実験用の入口。通常CLIの設定は変えません。
use std::{io::Write, num::NonZeroU32};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [source, limit, destination] = args.as_slice() else {
        return Err("usage: call_depth_emit_c <file.ceru> <positive-limit> <new-output.c>".into());
    };
    let limit: NonZeroU32 = limit.parse().map_err(|_| "invalid positive depth limit")?;
    let loaded =
        cerune_lang::modules::load(std::path::Path::new(source)).map_err(|e| format!("{e:?}"))?;
    let hir = loaded.to_ir().map_err(|e| format!("{e:?}"))?;
    let code = cerune_lang::codegen::c::emit_c_with_call_limit(&hir, limit)
        .map_err(|e| format!("{e:?}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    file.write_all(code.as_bytes()).map_err(|e| e.to_string())
}
