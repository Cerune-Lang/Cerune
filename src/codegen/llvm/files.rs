//! ファイル操作の意味をLLVMへ展開します。明示ターゲットのOS接続と試験接続を分離します。
mod connection;
pub(super) mod entry;
mod names;
#[cfg(test)]
mod tests;
mod transfer;
use super::{
    Target,
    ir::{Instruction, Module, Origin},
};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| {
            matches!(
                i.instruction,
                Instruction::FileReadText { .. } | Instruction::FileWriteNew { .. }
            )
        })
}
pub(super) fn origin_name(origin: Origin) -> String {
    let Origin::Source { node_id, span } = origin else {
        panic!("files require source origin")
    };
    format!(
        "@cerune.files.origin.{}.{}.{}",
        node_id.0,
        span.start(),
        span.end()
    )
}
fn data(out: &mut String, name: &str, text: &str) {
    write!(
        out,
        "{name} = private constant [{} x i8] c\"",
        text.len() + 1
    )
    .unwrap();
    for b in text.bytes().chain([0]) {
        write!(out, "\\{b:02X}").unwrap();
    }
    out.push_str("\"\n");
}
pub(super) fn emit_support(module: &Module, out: &mut String) {
    let mut origins = Vec::new();
    for i in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if matches!(
            i.instruction,
            Instruction::FileReadText { .. } | Instruction::FileWriteNew { .. }
        ) && !origins.contains(&i.origin)
        {
            origins.push(i.origin);
        }
    }
    if origins.is_empty() {
        return;
    }
    for origin in origins {
        let Origin::Source { node_id, span } = origin else {
            unreachable!()
        };
        data(
            out,
            &origin_name(origin),
            &format!(
                " node={}{} bytes={}..{}\n",
                node_id.0,
                span.source_id().record_field(),
                span.start(),
                span.end()
            ),
        );
    }
    for word in [
        "read-text",
        "write-new",
        "validate",
        "reserve",
        "open",
        "read",
        "write",
        "close",
        "decode",
        "invalid-limit",
        "invalid-name",
        "unconfigured",
        "budget-exceeded",
        "allocation-failed",
        "not-found",
        "already-exists",
        "unsupported-target",
        "open-failed",
        "read-failed",
        "write-failed",
        "close-failed",
        "zero-progress",
        "invalid-response",
        "invalid-utf8",
        "limit-exceeded",
        "known",
        "unknown",
        "no",
        "yes",
        "not-attempted",
        "closed",
        "failed",
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "CONIN$",
        "CONOUT$",
        "COM",
        "LPT",
    ] {
        data(out, &format!("@cerune.files.{word}"), word);
    }
    data(
        out,
        "@cerune.files.format",
        "cerune: file-v1 operation=%s phase=%s code=%s limit=%lld requested=%s transferred=%llu progress=%s created=%s cleanup=%s%s",
    );
    data(out, "@cerune.files.number", "%llu");
    let windows = module.target.expect("file target") == Target::X86_64PcWindowsMsvc;
    for declaration in [
        "declare i32 @snprintf(ptr, i64, ptr, ...)\n",
        "declare i32 @fflush(ptr)\n",
        "declare void @_Exit(i32) noreturn\n",
        if windows {
            "declare i32 @_write(i32, ptr, i32)\n"
        } else {
            "declare i64 @write(i32, ptr, i64)\n"
        },
    ] {
        if !out.contains(declaration) {
            out.push_str(declaration);
        }
    }
    if !out.contains("define internal i1 @cerune.utf8(") {
        out.push_str(super::utf8::SUPPORT);
    }

    out.push_str(FAILURE);
    out.push_str(if windows {
        "  call i32 @_write(i32 2, ptr %text, i32 %count)\n"
    } else {
        "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %text, i64 %wide)\n"
    });
    out.push_str("  br label %exit\nexit:\n  call void @_Exit(i32 1)\n  unreachable\n}\n");
    names::emit(out);
    entry::support(module.target.unwrap(), out);
    connection::emit(module.target.unwrap(), out);
    out.push_str(&transfer::SOURCE.replace(
        "@LIMIT@",
        &module.string_heap_limit.expect("file heap").to_string(),
    ));
}
const FAILURE: &str = r#"
define internal void @cerune.files.fail(ptr %operation, ptr %phase, ptr %code, i64 %limit, i1 %requested.known, i64 %requested, i64 %transferred, i1 %known, ptr %created, ptr %cleanup, ptr %origin) noreturn {
entry:
  %text = alloca [1024 x i8]
  %number = alloca [32 x i8]
  call i32 @fflush(ptr null)
  call i32 (ptr, i64, ptr, ...) @snprintf(ptr %number, i64 32, ptr @cerune.files.number, i64 %requested)
  %requested.text = select i1 %requested.known, ptr %number, ptr @cerune.files.unknown
  %progress = select i1 %known, ptr @cerune.files.known, ptr @cerune.files.unknown
  %length = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %text, i64 1024, ptr @cerune.files.format, ptr %operation, ptr %phase, ptr %code, i64 %limit, ptr %requested.text, i64 %transferred, ptr %progress, ptr %created, ptr %cleanup, ptr %origin)
  %positive = icmp sgt i32 %length, 0
  br i1 %positive, label %write, label %exit
write:
  %fits = icmp ult i32 %length, 1024
  %count = select i1 %fits, i32 %length, i32 1023
"#;
