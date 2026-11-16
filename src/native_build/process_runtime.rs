//! Explicit subprocess link input and its saved, versioned requirements.
use cerune_lang::codegen::x86_64::Target;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
mod archive;
pub(crate) const POLICY: &str = "target-system-runtime+process-runtime-v1";
const LIMIT: u64 = 128 * 1024 * 1024;
pub(super) struct Snapshot {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub record: Value,
}
pub(crate) fn filename(t: Target) -> &'static str {
    match t {
        Target::X86_64PcWindowsMsvc => "process-runtime.lib",
        Target::X86_64UnknownLinuxGnu => "process-runtime.a",
    }
}
pub(crate) fn libraries(t: Target) -> Vec<&'static str> {
    match t {
        Target::X86_64PcWindowsMsvc => vec![
            "-lkernel32",
            "-lntdll",
            "-luserenv",
            "-lws2_32",
            "-ldbghelp",
            "-ladvapi32",
            "-lbcrypt",
        ],
        Target::X86_64UnknownLinuxGnu => vec![
            "-lgcc_s",
            "-lutil",
            "-lrt",
            "-lpthread",
            "-lm",
            "-ldl",
            "-lc",
        ],
    }
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn record(t: Target, source: &str, bytes: &[u8]) -> Value {
    json!({
     "schema":"cerune-native-process-runtime-v1","target":t.triple(),"linkage":"static",
     "required_abi":{"name":"cerune-process-v1","version":1,"report_words":32},
     "archive":{"path":filename(t),"source_path":source,"bytes":bytes.len(),"sha256":hash(bytes),"validation":"archive-members-target-and-external-link"},
     "system_link_args":libraries(t),
     "configuration":{"schema":"cerune-process-v1","binding":"execution-time-json","embedded":false,
     "startup":{"marker":"--cerune-runtime-v1","option":"--process-config","delimiter":"--","recognition":"leading-marker-only"}},
     "linux_supervisor":{"required":t==Target::X86_64UnknownLinuxGnu,"bundled":false,"binding":"execution-time-json"},
     "executed_for_validation":false
    })
}
pub(super) fn snapshot(path: &Path, target: Target) -> Result<Snapshot, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("process runtime archive: {e}"))?;
    let link = meta.file_type().is_symlink();
    #[cfg(windows)]
    let link = {
        use std::os::windows::fs::MetadataExt;
        link || meta.file_attributes() & 0x400 != 0
    };
    if !meta.is_file() || link {
        return Err("process runtime archive must be a regular non-link file".into());
    }
    if meta.len() > LIMIT {
        return Err("process runtime archive exceeds 128 MiB".into());
    }
    let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("process runtime archive exceeds 128 MiB".into());
    }
    archive::validate(&bytes, target)?;
    let record = record(
        target,
        path.to_str().ok_or("process runtime path must be UTF-8")?,
        &bytes,
    );
    Ok(Snapshot {
        name: filename(target),
        bytes,
        record,
    })
}
pub(crate) fn validate(value: &Value, target: Target, bytes: &[u8]) -> Result<(), String> {
    let path = value["archive"]["source_path"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("missing process runtime input path")?;
    if *value != record(target, path, bytes) {
        return Err("invalid or inconsistent process runtime record/hash".into());
    }
    archive::validate(bytes, target)
}
pub(crate) fn guide(english: bool, enabled: bool) -> String {
    if !enabled {
        return String::new();
    }
    if english {
        "This executable statically links the subprocess runtime. The exact archive is preserved in build/ as a link input, not a dynamic library to install. At execution, pass --cerune-runtime-v1 --process-config <json> -- <program-arguments...>; combine --file-root <directory> before -- if file operations are also used. Without configuration, subprocess calls return Unconfigured. Build/release do not run children or embed the local JSON, registered executables, working directories or environment values. Linux additionally requires a separately supplied matching Cerune supervisor, explicitly named by linux_supervisor in the JSON. It is not bundled or resolved through PATH. These records identify inputs and requirements; hashes are not signatures or proof of implementation correctness.\n\n".into()
    } else {
        "この実行ファイルには子プロセスruntimeを静的リンクしています。build/には実際にリンクしたarchiveを保存しますが、実行時に配置する動的ライブラリではありません。実行時に --cerune-runtime-v1 --process-config <json> -- <プログラム引数...> を指定します。ファイル操作も使う場合は -- の前に --file-root <directory> を併記します。設定がなければ子プロセス操作はUnconfiguredを返します。build/releaseは子を起動せず、ローカルのJSON・登録先実行ファイル・作業ディレクトリ・環境値を埋め込みません。Linuxでは別途、対応するCerune監督を用意し、JSONのlinux_supervisorへパスを明示します。監督は同梱せず、PATHから探しません。記録は入力と必要条件を示し、ハッシュは署名や実装の正しさの証明ではありません。\n\n".into()
    }
}

/// Compare all three saved records to the exact archive that will be packaged.
pub(crate) fn validate_records(
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    target: Target,
) -> Result<Option<Value>, String> {
    let parse = |name: &str| -> Result<Value, String> {
        serde_json::from_slice(&files[name]).map_err(|e| e.to_string())
    };
    let manifest = parse("manifest.json")?;
    let runtime = parse("runtime.json")?;
    let plan = parse("link-plan.json")?;
    if manifest["schema"] == "cerune-native-build-v1" {
        if [&manifest, &runtime, &plan]
            .iter()
            .any(|v| v.get("process_runtime").is_some())
            || runtime["external_symbols"]
                .as_array()
                .is_some_and(|symbols| {
                    symbols.iter().any(|s| {
                        s.as_str()
                            .is_some_and(|s| s.starts_with("cerune_process_v1_"))
                    })
                })
        {
            return Err("legacy records cannot describe a subprocess runtime dependency".into());
        }
        return Ok(None);
    }
    let record = manifest
        .get("process_runtime")
        .ok_or("missing process runtime record")?;
    if runtime.get("process_runtime") != Some(record) || plan.get("process_runtime") != Some(record)
    {
        return Err("inconsistent process runtime records".into());
    }
    let bytes = files
        .get(filename(target))
        .ok_or("missing saved process runtime archive")?;
    validate(record, target, bytes)?;
    if plan["inputs"] != json!(["program.o", filename(target)])
        || plan["runtime"] != POLICY
        || plan["adapter"] != "clang-driver-v1"
        || plan["args"] != json!(super::link::process_arguments(target, record))
        || runtime["cerune_runtime"] != "in-object-and-static-process-archive"
    {
        return Err("inconsistent process runtime link plan".into());
    }
    Ok(Some(record.clone()))
}
