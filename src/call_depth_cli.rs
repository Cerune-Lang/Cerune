//! CLIの公開設定範囲。物理stack・memoryの安全保証ではありません。
use std::num::NonZeroU32;
pub const DEFAULT: NonZeroU32 = NonZeroU32::new(128).unwrap();
pub const MAX: u32 = 128;
pub fn parse(args: Vec<String>, command: &str) -> Result<(Vec<String>, NonZeroU32), String> {
    let mut rest = Vec::new();
    let mut limit = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg != "--call-depth-limit" {
            rest.push(arg);
            continue;
        }
        if !matches!(
            command,
            "run"
                | "run-ir"
                | "run-mir"
                | "run-vm"
                | "emit-c"
                | "emit-llvm"
                | "emit-qbe"
                | "emit-wat"
                | "emit-lir"
                | "emit-asm"
                | "emit-obj"
                | "build"
                | "observe"
        ) {
            return Err(format!("{arg} is not supported by {command}"));
        }
        if limit.is_some() {
            return Err(format!("duplicate {arg}"));
        }
        let text = args
            .next()
            .ok_or_else(|| format!("missing depth after {arg}"))?;
        limit = Some(
            text.parse::<u32>()
                .ok()
                .filter(|n| (1..=MAX).contains(n))
                .filter(|_| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
                .and_then(NonZeroU32::new)
                .ok_or_else(|| format!("{arg} expects a decimal depth from 1 to {MAX}"))?,
        );
    }
    Ok((rest, limit.unwrap_or(DEFAULT)))
}
