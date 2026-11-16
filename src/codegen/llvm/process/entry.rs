use super::super::{Target, ir::Module};
use std::fmt::Write;
pub(in crate::codegen::llvm) fn adapt(entry: &str, files: bool) -> String {
    let mut value=super::super::files::entry::adapt(entry)
        .replace("{ i64, ptr }","{ i64, ptr, ptr }")
        .replace("@cerune.files.parse","@cerune.process.parse")
        .replace("  %root = extractvalue { i64, ptr, ptr } %configuration, 1",
            "  %root = extractvalue { i64, ptr, ptr } %configuration, 1\n  %config = extractvalue { i64, ptr, ptr } %configuration, 2")
        .replace("  %result = call i32 @cerune.program()",
            "  call void @cerune.process.connect(ptr %config)\n  %result = call i32 @cerune.program()\n  %destroyed = call i32 @cerune_process_v1_destroy(ptr @cerune.process.context)\n  call void @cerune.process.check(i32 %destroyed, i32 0, ptr @cerune.process.host)");
    if !files {
        value = value
            .replace(
                "  call void @cerune.files.connect_optional(ptr %root)\n",
                "",
            )
            .replace("  call void @cerune.files.disconnect()\n", "");
    }
    value
}
pub(super) fn support(m: &Module, out: &mut String) {
    let windows = m.target == Some(Target::X86_64PcWindowsMsvc);
    for word in [
        "configure",
        "invalid-root",
        "unknown-setting",
        "duplicate-setting",
        "missing-delimiter",
    ] {
        super::data(out, &format!("@cerune.process.{word}"), word);
    }
    super::data(
        out,
        "@cerune.process.startup.format",
        "cerune-process-startup-v1 phase=%s code=%s origin=host\n",
    );
    for (name, value) in [
        ("marker", "--cerune-runtime-v1"),
        ("root.option", "--file-root"),
        ("config.option", "--process-config"),
        ("delimiter", "--"),
        ("mode", "rb"),
    ] {
        if windows {
            write!(
                out,
                "@cerune.process.{name} = private constant [{} x i16] [",
                value.len() + 1
            )
            .unwrap();
            for (i, b) in value.bytes().chain([0]).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write!(out, "i16 {b}").unwrap();
            }
            out.push_str("]\n");
        } else {
            super::data(out, &format!("@cerune.process.{name}"), value);
        }
    }
    for decl in [
        "declare i64 @fread(ptr, i64, i64, ptr)\n",
        "declare i32 @fclose(ptr)\n",
        "declare i32 @ferror(ptr)\n",
        if windows {
            "declare ptr @_wfopen(ptr, ptr)\n"
        } else {
            "declare ptr @fopen(ptr, ptr)\n"
        },
        if windows {
            "declare i32 @wcscmp(ptr, ptr)\n"
        } else {
            "declare i32 @strcmp(ptr, ptr)\n"
        },
    ] {
        if !out.contains(decl) {
            out.push_str(decl);
        }
    }
    out.push_str(
        &include_str!("parse.ll")
            .replace("@CHAR@", if windows { "i16" } else { "i8" })
            .replace("@CMP@", if windows { "wcscmp" } else { "strcmp" })
            .replace(
                "@FILES@",
                if super::super::files::used(m) {
                    "true"
                } else {
                    "false"
                },
            ),
    );
    out.push_str(
        &include_str!("connect.ll")
            .replace("@OPEN@", if windows { "_wfopen" } else { "fopen" })
            .replace("@TARGET@", if windows { "1" } else { "2" }),
    );
    out.push_str(&include_str!("failure.ll").replace(
        "@WRITE@",
        if windows {
            "  call i32 @_write(i32 2, ptr %text, i32 %count)"
        } else {
            "  %wide = zext i32 %count to i64\n  call i64 @write(i32 2, ptr %text, i64 %wide)"
        },
    ));
}
