//! 値渡しのhost接続と、出自を保持するファイル操作です。
use super::ir::{Instruction, Module, Origin};
use std::fmt::Write;
mod names;
mod transfer;

pub(super) const IMPORT: &str = r#"
  (import "cerune" "files_status_v1" (func $files_status_v1 (result i32)))
  (import "cerune" "file_name_begin_v1" (func $file_name_begin_v1 (param i32) (result i32)))
  (import "cerune" "file_name_chunk_v1" (func $file_name_chunk_v1 (param i64 i32) (result i32)))
  (import "cerune" "file_open_v1" (func $file_open_v1 (param i32) (result i32 i32)))
  (import "cerune" "file_read_v1" (func $file_read_v1 (param i32 i32) (result i64 i32)))
  (import "cerune" "file_write_v1" (func $file_write_v1 (param i32 i64 i32) (result i32)))
  (import "cerune" "file_close_v1" (func $file_close_v1 (param i32) (result i32)))
"#;
fn text(value: &str, out: &mut String) {
    for b in value.bytes() {
        writeln!(out, "    (call $write_error_byte (i32.const {b}))").unwrap();
    }
}
pub(super) fn located(instruction: &Instruction, origin: Origin, name: &str, out: &mut String) {
    let read = matches!(instruction, Instruction::FileReadText);
    writeln!(
        out,
        "  (func ${name} (param $name i32) {} (param $limit i64) {}\n    (local $value i32)",
        if read { "" } else { "(param $text i32)" },
        if read { "(result i32)" } else { "" }
    )
    .unwrap();
    out.push_str("    local.get $name\n");
    if !read {
        out.push_str("    local.get $text\n");
    }
    out.push_str("    local.get $limit\n");
    out.push_str(if read {
        "    call $cerune_file_read\n    local.set $value\n"
    } else {
        "    call $cerune_file_write\n"
    });
    out.push_str(
        "    (if (global.get $cerune_file_code) (then\n      call $flush_stdout_v1\n      drop\n",
    );
    text(
        if read {
            "cerune: file-v1 operation=read-text phase="
        } else {
            "cerune: file-v1 operation=write-new phase="
        },
        out,
    );
    out.push_str("      (call $cerune_file_phase_text (global.get $cerune_file_phase))\n");
    text(" code=", out);
    out.push_str("      (call $cerune_file_code_text (global.get $cerune_file_code))\n");
    text(" limit=", out);
    out.push_str("      (call $cerune_file_error_i64 (local.get $limit))\n");
    text(" requested=", out);
    out.push_str("      (if (i64.eq (global.get $cerune_file_requested) (i64.const -1)) (then\n");
    text("unknown", out);
    out.push_str(
        "      ) (else (call $cerune_file_error_u64 (global.get $cerune_file_requested))))\n",
    );
    text(" transferred=", out);
    out.push_str("      (call $cerune_file_error_u64 (global.get $cerune_file_transferred))\n");
    text(" progress=", out);
    out.push_str("      (call $cerune_file_progress_text (global.get $cerune_file_known))\n");
    text(" created=", out);
    out.push_str("      (call $cerune_file_created_text (global.get $cerune_file_created))\n");
    text(" cleanup=", out);
    out.push_str("      (call $cerune_file_cleanup_text (global.get $cerune_file_cleanup))\n");
    text(
        &format!(
            " node={}{} bytes={}..{}\n",
            origin.node_id.0,
            origin.span.source_id().record_field(),
            origin.span.start(),
            origin.span.end()
        ),
        out,
    );
    out.push_str("      unreachable\n    ))\n");
    if read {
        out.push_str("    local.get $value\n");
    }
    out.push_str("  )\n");
}
pub(super) fn support(module: &Module, out: &mut String) {
    use crate::runtime::files::Code;
    for (name, values) in [
        (
            "phase",
            vec![
                "validate", "reserve", "open", "read", "write", "close", "decode",
            ],
        ),
        (
            "code",
            vec![
                "",
                Code::InvalidLimit.name(),
                Code::InvalidName.name(),
                Code::Unconfigured.name(),
                Code::BudgetExceeded.name(),
                Code::AllocationFailed.name(),
                Code::NotFound.name(),
                Code::AlreadyExists.name(),
                Code::UnsupportedTarget.name(),
                Code::OpenFailed.name(),
                Code::ReadFailed.name(),
                Code::WriteFailed.name(),
                Code::CloseFailed.name(),
                Code::ZeroProgress.name(),
                Code::InvalidResponse.name(),
                Code::InvalidUtf8.name(),
                Code::LimitExceeded.name(),
            ],
        ),
        ("progress", vec!["unknown", "known"]),
        ("created", vec!["no", "yes", "unknown"]),
        ("cleanup", vec!["not-attempted", "closed", "failed"]),
    ] {
        writeln!(out, "  (func $cerune_file_{name}_text (param $value i32)").unwrap();
        for (i, value) in values.iter().enumerate() {
            writeln!(
                out,
                "    (if (i32.eq (local.get $value) (i32.const {i})) (then"
            )
            .unwrap();
            text(value, out);
            out.push_str("    ))\n");
        }
        out.push_str("  )\n");
    }
    names::emit(out);
    out.push_str(&transfer::SOURCE.replace("@EMPTY@", &(module.string_heap_start - 8).to_string()));
}
