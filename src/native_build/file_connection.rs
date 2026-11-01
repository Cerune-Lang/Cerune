//! Build-time connection requirements; never contain an execution-time root.
use cerune_lang::codegen::x86_64::Target;
use serde_json::{Value, json};

pub(crate) fn record(target: Target, enabled: bool) -> Value {
    let mut value = json!({
        "schema": "cerune-native-file-connection-v1",
        "enabled": enabled
    });
    if enabled {
        value["startup"] = json!({
            "marker": "--cerune-runtime-v1",
            "file_root_option": "--file-root",
            "argument_delimiter": "--",
            "recognition": "leading-marker-only"
        });
        value["root_binding"] = json!("execution-time-argument");
        value["missing_root"] = json!("unconfigured-on-file-operation");
        value["diagnostic_schema"] = json!("file-v1");
        value["os_requirements"] = match target {
            Target::X86_64UnknownLinuxGnu => json!({
                "connection": "openat-directory-fd",
                "read_reopen": "/proc/self/fd"
            }),
            Target::X86_64PcWindowsMsvc => json!({
                "connection": "nt-root-directory-handle",
                "dynamic_library": "ntdll.dll",
                "dynamic_symbol": "NtCreateFile"
            }),
        };
    }
    value
}

/// Absence in both old records is unknown, distinct from an explicit disabled record.
pub(crate) fn validate(manifest: &Value, runtime: &Value, target: Target) -> Result<Value, String> {
    match (
        manifest.get("file_connection"),
        runtime.get("file_connection"),
    ) {
        (None, None) => Ok(Value::Null),
        (Some(a), Some(b)) if a == b => {
            let enabled = a["enabled"]
                .as_bool()
                .ok_or("invalid file_connection enabled")?;
            if *a != record(target, enabled) {
                return Err("invalid or unsupported file_connection contract".into());
            }
            Ok(a.clone())
        }
        _ => Err("inconsistent file_connection records".into()),
    }
}
