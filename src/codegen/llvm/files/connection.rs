use super::super::Target;
mod linux;
mod windows;
pub(super) fn emit(target: Target, out: &mut String) {
    out.push_str("; File OS connection begin\n");
    let declarations = match target {
        Target::X86_64UnknownLinuxGnu => linux::DECLARATIONS,
        Target::X86_64PcWindowsMsvc => windows::DECLARATIONS,
    };
    for line in declarations.lines().filter(|s| !s.is_empty()) {
        if !out.lines().any(|old| old == line) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str(COMMON);
    match target {
        Target::X86_64UnknownLinuxGnu => {
            super::data(out, "@cerune.files.proc.fd", "/proc/self/fd/%d");
            out.push_str(linux::SOURCE);
        }
        Target::X86_64PcWindowsMsvc => windows::emit(out),
    }
    out.push_str("; File OS connection end\n");
}
const COMMON: &str = r#"
@cerune.files.configured = internal global i1 false
@cerune.files.root = internal global i64 0
define internal i32 @cerune_file_host_configured() {
entry:
  %configured = load i1, ptr @cerune.files.configured
  %result = zext i1 %configured to i32
  ret i32 %result
}
define internal void @cerune.files.connect_optional(ptr %root) {
entry:
  %specified = icmp ne ptr %root, null
  br i1 %specified, label %connect, label %done
connect:
  call void @cerune.files.connect(ptr %root)
  br label %done
done:
  ret void
}
define internal void @cerune.files.disconnect() {
entry:
  %configured = load i1, ptr @cerune.files.configured
  br i1 %configured, label %close, label %done
close:
  %root = load i64, ptr @cerune.files.root
  call i32 @cerune_file_host_close(i64 %root)
  store i1 false, ptr @cerune.files.configured
  br label %done
done:
  ret void
}
"#;
