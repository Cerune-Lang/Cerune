//! ファイルの意味処理をQBE ILへ直接展開します。明示ターゲットのOS接続と意味処理を分離します。
mod connection;
pub(super) mod entry;
mod names;
#[cfg(test)]
mod tests;
mod transfer;
use super::{
    Target,
    ir::{FailureOrigin, Instruction, Module},
};
use std::fmt::Write;

pub(super) fn used(module: &Module) -> bool {
    module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
        .any(|i| {
            matches!(
                i,
                Instruction::FileReadText { .. } | Instruction::FileWriteNew { .. }
            )
        })
}
pub(super) fn origin_name(origin: FailureOrigin) -> String {
    format!(
        "cerune_files_origin_{}_{}_{}",
        origin.node.0,
        origin.span.start(),
        origin.span.end()
    )
}
fn data(out: &mut String, section: &str, name: &str, text: &str) {
    write!(out, "section \"{section}\" data ${name} = {{ ").unwrap();
    for (i, b) in text.bytes().chain([0]).enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "b {b}").unwrap();
    }
    out.push_str(" }\n");
}
pub(super) fn emit(module: &Module, out: &mut String) {
    let mut origins = Vec::new();
    for item in module
        .instructions
        .iter()
        .chain(module.functions.iter().flat_map(|f| &f.instructions))
    {
        if let Instruction::FileReadText { origin, .. } | Instruction::FileWriteNew { origin, .. } =
            item
            && !origins.contains(origin)
        {
            origins.push(*origin);
        }
    }
    if origins.is_empty() {
        return;
    }
    let target = module.target.expect("file target");
    let section = target.read_only_section();
    for origin in origins {
        let span = origin.span;
        data(
            out,
            section,
            &origin_name(origin),
            &format!(
                " node={}{} bytes={}..{}\n",
                origin.node.0,
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
        data(
            out,
            section,
            &format!("cerune_file_label_{}", word.replace(['-', '$'], "_")),
            word,
        );
    }
    data(
        out,
        section,
        "cerune_files_format",
        "cerune: file-v1 operation=%s phase=%s code=%s limit=%lld requested=%s transferred=%llu progress=%s created=%s cleanup=%s%s",
    );
    data(out, section, "cerune_files_number", "%llu");
    if !out.contains("function w $cerune_utf8(") {
        out.push_str(super::utf8::SUPPORT);
    }
    out.push_str(FAILURE);
    out.push_str(if target == Target::X86_64PcWindowsMsvc {
        "  call $_write(w 2, l %text, w %count)\n"
    } else {
        "  %wide =l extuw %count\n  call $write(w 2, l %text, l %wide)\n"
    });
    out.push_str("  jmp @exit\n@exit\n  call $_Exit(w 1)\n  hlt\n}\n");
    names::emit(out);
    entry::emit(target, out);
    connection::emit(target, out);
    out.push_str(&transfer::SOURCE.replace(
        "@LIMIT@",
        &module.string_heap_limit.expect("file heap").to_string(),
    ));
}
const FAILURE: &str = r#"
function $cerune_files_fail(l %operation, l %phase, l %code, l %limit, w %requested_known, l %requested, l %transferred, w %known, l %created, l %cleanup, l %origin) {
@start
  %text =l alloc8 1024
  %number =l alloc8 32
  call $fflush(l 0)
  call $snprintf(l %number, l 32, l $cerune_files_number, ..., l %requested)
  jnz %requested_known, @requested_known, @requested_unknown
@requested_known
  jmp @progress
@requested_unknown
  jmp @progress
@progress
  %requested_text =l phi @requested_known %number, @requested_unknown $cerune_file_label_unknown
  jnz %known, @known, @unknown
@known
  jmp @format
@unknown
  jmp @format
@format
  %progress =l phi @known $cerune_file_label_known, @unknown $cerune_file_label_unknown
  %length =w call $snprintf(l %text, l 1024, l $cerune_files_format, ..., l %operation, l %phase, l %code, l %limit, l %requested_text, l %transferred, l %progress, l %created, l %cleanup, l %origin)
  %positive =w csgtw %length, 0
  jnz %positive, @bound, @exit
@bound
  %fits =w cultw %length, 1024
  jnz %fits, @fit, @truncate
@fit
  jmp @write
@truncate
  jmp @write
@write
  %count =w phi @fit %length, @truncate 1023
"#;
