use super::super::Target;
use std::fmt::Write;
mod linux;
mod windows;

pub(super) fn emit(target: Target, out: &mut String) {
    out.push_str("# File OS connection begin\n");
    out.push_str(COMMON);
    let section = target.read_only_section();
    match target {
        Target::X86_64UnknownLinuxGnu => {
            super::data(out, section, "cerune_files_proc_fd", "/proc/self/fd/%d");
            out.push_str(linux::SOURCE);
        }
        Target::X86_64PcWindowsMsvc => {
            for (name, text) in [
                ("ntdll", "ntdll.dll"),
                ("nt_prefix", "\\??\\"),
                ("unc_prefix", "UNC\\"),
                ("extended", "\\\\?\\"),
                ("device", "\\\\.\\"),
            ] {
                wide_data(out, section, &format!("cerune_files_{name}"), text);
            }
            super::data(out, section, "cerune_files_nt_create", "NtCreateFile");
            out.push_str(windows::SOURCE);
        }
    }
    out.push_str("# File OS connection end\n");
}
pub(super) fn wide_data(out: &mut String, section: &str, name: &str, text: &str) {
    write!(out, "section \"{section}\" data ${name} = align 2 {{ ").unwrap();
    for (i, c) in text.encode_utf16().chain([0]).enumerate() {
        if i != 0 {
            out.push_str(", ");
        }
        write!(out, "h {c}").unwrap();
    }
    out.push_str(" }\n");
}
const COMMON: &str = r#"
data $cerune_files_configured = align 4 { w 0 }
data $cerune_files_root = align 8 { l 0 }
function w $cerune_file_host_configured() {
@start
  %configured =w loadw $cerune_files_configured
  ret %configured
}
function $cerune_files_connect_optional(l %root) {
@start
  jnz %root, @connect, @done
@connect
  call $cerune_files_connect(l %root)
  jmp @done
@done
  ret
}
function $cerune_files_disconnect() {
@start
  %configured =w loadw $cerune_files_configured
  jnz %configured, @close, @done
@close
  %root =l loadl $cerune_files_root
  call $cerune_file_host_close(l %root)
  storew 0, $cerune_files_configured
  jmp @done
@done
  ret
}
"#;
