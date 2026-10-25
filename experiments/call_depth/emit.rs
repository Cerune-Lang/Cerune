//! 各生成表現で同じ明示上限を試す入口。外部ツールは実行しません。
use cerune_lang::codegen::{llvm, qbe, wat, x86_64};
use std::{io::Write, num::NonZeroU32};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [route, target, source, limit, destination] = args.as_slice() else {
        return Err("usage: call_depth_emit <llvm|qbe|wat|asm|obj|lir> <target|wasm32> <file.ceru> <positive-limit> <new-output>".into());
    };
    let limit: NonZeroU32 = limit.parse().map_err(|_| "invalid positive depth limit")?;
    let loaded =
        cerune_lang::modules::load(std::path::Path::new(source)).map_err(|e| format!("{e:?}"))?;
    let hir = loaded.to_ir().map_err(|e| format!("{e:?}"))?;
    let data = match route.as_str() {
        "llvm" => llvm::emit_llvm_with_call_limit(
            &hir,
            llvm::Target::parse(target).ok_or("unsupported LLVM target")?,
            limit,
        )
        .map(String::into_bytes),
        "qbe" => qbe::emit_qbe_with_call_limit(
            &hir,
            qbe::Target::parse(target).ok_or("unsupported QBE target")?,
            limit,
        )
        .map(String::into_bytes),
        "wat" if target == "wasm32" => {
            wat::emit_wat_with_call_limit(&hir, limit).map(String::into_bytes)
        }
        "asm" | "obj" | "lir" => {
            let mir = cerune_lang::mir::lower(&hir).map_err(|e| format!("{e:?}"))?;
            let lir = x86_64::lower_mir_with_call_limit(
                &mir,
                x86_64::Target::parse(target).ok_or("unsupported Native target")?,
                limit,
            )
            .map_err(|e| format!("{e:?}"))?;
            match route.as_str() {
                "asm" => x86_64::emit_asm_from_lir(&lir, true).map(String::into_bytes),
                "obj" => x86_64::emit_object_from_lir(&lir, true),
                _ => x86_64::lir::emit(&lir).map(String::into_bytes),
            }
        }
        _ => return Err("unsupported route/target".into()),
    }
    .map_err(|e| format!("{e:?}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| e.to_string())?;
    file.write_all(&data).map_err(|e| e.to_string())
}
