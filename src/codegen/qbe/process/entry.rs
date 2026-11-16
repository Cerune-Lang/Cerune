use super::super::{Target, ir::Module};
use std::fmt::Write;
pub(in crate::codegen::qbe) fn adapt(entry: &str, files: bool) -> String {
    let mut value=super::super::files::entry::adapt(entry)
        .replace("  %root_slot =l alloc8 8", "  %root_slot =l alloc8 8\n  %config_slot =l alloc8 8")
        .replace("$cerune_files_parse(w %argc, l %argv, l %root_slot)", "$cerune_process_parse(w %argc, l %argv, l %root_slot, l %config_slot)")
        .replace("  %root =l loadl %root_slot", "  %root =l loadl %root_slot\n  %config =l loadl %config_slot")
        .replace("  %result =w call $cerune_program()", "  call $cerune_process_connect(l %config)\n  %result =w call $cerune_program()\n  %destroyed =w call $cerune_process_v1_destroy(l $cerune_process_context)\n  call $cerune_process_check(w %destroyed, w 0, l $cerune_process_host)");
    if !files {
        value = value
            .replace("  call $cerune_files_connect_optional(l %root)\n", "")
            .replace("  call $cerune_files_disconnect()\n", "");
    }
    value
}
pub(super) fn support(m: &Module, s: &mut String) {
    let windows = m.target == Some(Target::X86_64PcWindowsMsvc);
    for word in [
        "configure",
        "invalid-root",
        "unknown-setting",
        "duplicate-setting",
        "missing-delimiter",
    ] {
        super::data(
            m,
            s,
            &format!("cerune_process_label_{}", word.replace('-', "_")),
            word,
        );
    }
    super::data(
        m,
        s,
        "cerune_process_startup_format",
        "cerune-process-startup-v1 phase=%s code=%s origin=host\n",
    );
    for (name, value) in [
        ("marker", "--cerune-runtime-v1"),
        ("root_option", "--file-root"),
        ("config_option", "--process-config"),
        ("delimiter", "--"),
        ("mode", "rb"),
    ] {
        let name = format!("cerune_process_{name}");
        if windows {
            write!(
                s,
                "section \"{}\" data ${name} = align 2 {{ ",
                m.target.unwrap().read_only_section()
            )
            .unwrap();
            for (i, b) in value.encode_utf16().chain([0]).enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write!(s, "h {b}").unwrap();
            }
            s.push_str(" }\n");
        } else {
            super::data(m, s, &name, value);
        }
    }
    s.push_str(
        &include_str!("parse.ssa")
            .replace("@CMP@", if windows { "wcscmp" } else { "strcmp" })
            .replace("@LOAD@", if windows { "loaduh" } else { "loadub" })
            .replace("@STEP@", if windows { "2" } else { "1" })
            .replace(
                "@FILES@",
                if super::super::files::used(m) {
                    "1"
                } else {
                    "0"
                },
            ),
    );
    s.push_str(
        &include_str!("connect.ssa")
            .replace("@TARGET@", if windows { "1" } else { "2" })
            .replace("@OPEN@", if windows { "_wfopen" } else { "fopen" }),
    );
    s.push_str(&include_str!("failure.ssa").replace(
        "@WRITE@",
        if windows {
            "  call $_write(w 2, l %text, w %count)"
        } else {
            "  %wide =l extuw %count\n  call $write(w 2, l %text, l %wide)"
        },
    ));
}
