//! Package an existing successful build; never compile, link, or execute it.
mod dependencies;
use cerune_lang::codegen::x86_64::Target;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FILE_LIMIT: u64 = 128 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 512 * 1024 * 1024;
const JSON_LIMIT: usize = 16 * 1024 * 1024;
const BASE: &[&str] = &[
    "program.ceir",
    "program.mir.txt",
    "program.lir.txt",
    "program.origins.s",
    "program.machine.txt",
    "program.object-map.txt",
    "program.origins.json",
    "runtime.json",
    "source-index.json",
    "program.o",
];
const SSA: &[&str] = &[
    "program.ssa.txt",
    "program.ssa-map.txt",
    "program.ssa-lowered.mir.txt",
    "program.ssa-lowered-map.txt",
];
const RECORDS: &[&str] = &[
    "manifest.json",
    "link-plan.json",
    "link-result.json",
    "version.stdout.log",
    "version.stderr.log",
    "version.result.json",
    "link.stdout.log",
    "link.stderr.log",
    "link.result.json",
    "program.link.map",
];

pub(super) fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    if args.next().as_deref() != Some("native") {
        return Err("release requires the explicit route native".into());
    }
    let input = PathBuf::from(
        args.next()
            .ok_or("release native requires a build directory")?,
    );
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" if output.is_none() => {
                output = Some(PathBuf::from(
                    args.next().ok_or("missing output directory")?,
                ))
            }
            _ => {
                return Err(format!(
                    "unexpected or duplicate native release option {arg}; target and build options come from the saved build"
                ));
            }
        }
    }
    let output = output.ok_or("release native requires -o <new-directory>")?;
    if fs::symlink_metadata(&output).is_ok() {
        return Err("native release output already exists".into());
    }
    let metadata = fs::symlink_metadata(&input).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || is_link(&metadata) {
        return Err("release input must be a regular build directory, not a link".into());
    }
    let input = fs::canonicalize(input).map_err(|e| e.to_string())?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|e| format!("release output parent: {e}"))?;
    if parent.starts_with(&input) {
        return Err("release output must be outside its input build directory".into());
    }
    let mut remaining = TOTAL_LIMIT;
    let manifest_bytes = read(&input.join("manifest.json"), &mut remaining)?;
    let manifest = parse(&manifest_bytes)?;
    let (target, executable, expected) = validate_manifest(&manifest)?;
    let mut files = BTreeMap::new();
    files.insert("manifest.json".to_owned(), manifest_bytes);
    for name in &expected {
        if name != "manifest.json" {
            files.insert(name.clone(), read(&input.join(name), &mut remaining)?);
        }
    }
    // A fixed allowlist prevents path traversal and accidental inclusion of unrelated files.
    let found = fs::read_dir(&input)
        .map_err(|e| e.to_string())?
        .map(|entry| {
            entry
                .map_err(|e| e.to_string())?
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF-8 build filename".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if found != expected {
        return Err("build directory contains missing or unexpected files".into());
    }
    validate_records(&files, target, executable)?;
    let requirements = dependencies::inspect(&files[executable], target)?;
    #[cfg(unix)]
    if target == Target::X86_64UnknownLinuxGnu {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(input.join(executable))
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err("build executable lacks execute permission".into());
        }
    }
    let mut payload: BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(name, data)| (format!("build/{name}"), data))
        .collect();
    payload.insert("runtime-requirements.json".into(), encoded(&requirements)?);
    payload.insert(
        "README.md".into(),
        guide(target, executable, false).into_bytes(),
    );
    payload.insert(
        "README.en.md".into(),
        guide(target, executable, true).into_bytes(),
    );
    payload.insert(
        "CERUNE-LICENSE-MIT".into(),
        include_bytes!("../LICENSE-MIT").to_vec(),
    );
    payload.insert(
        "CERUNE-LICENSE-APACHE".into(),
        include_bytes!("../LICENSE-APACHE").to_vec(),
    );
    let entries: Vec<_> = payload.iter().map(|(name,bytes)| json!({"path":name,"bytes":bytes.len(),"sha256":hash(bytes),"executable":name == &format!("build/{executable}")})).collect();
    let release = encoded(&json!({
        "schema":"cerune-native-release-v1", "status":"success", "route":"native",
        "cerune_version":env!("CARGO_PKG_VERSION"), "build_cerune_version":manifest["cerune_version"],
        "target":target.triple(), "entry":format!("build/{executable}"),
        "build_manifest":"build/manifest.json", "runtime_requirements":"runtime-requirements.json",
        "runtime_policy":"target-system-runtime", "libraries_bundled":false,
        "compiled":false, "linked":false, "executed":false, "optimization_passes":[],
        "sources_included":manifest["sources_included"], "files":entries,
        "call_depth_limit":manifest["call_depth_limit"]
    }))?;
    let mut sums = payload
        .iter()
        .map(|(name, bytes)| format!("{}  {name}\n", hash(bytes)))
        .collect::<String>();
    sums.push_str(&format!("{}  release.json\n", hash(&release)));
    // Validation and snapshotting finish before any output is created.
    fs::create_dir(&output).map_err(|e| format!("cannot create release directory: {e}"))?;
    fs::create_dir(output.join("build")).map_err(|e| e.to_string())?;
    for (name, bytes) in &payload {
        write(&output.join(name), bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if name == &format!("build/{executable}") {
                0o755
            } else {
                0o644
            };
            fs::set_permissions(output.join(name), fs::Permissions::from_mode(mode))
                .map_err(|e| e.to_string())?;
        }
    }
    write(&output.join("SHA256SUMS"), sums.as_bytes())?;
    // Written last: no success marker for partial packaging failures.
    write(&output.join("release.json"), &release)
}
fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}
fn read(path: &Path, remaining: &mut u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(format!(
            "expected a regular non-link build file: {}",
            path.display()
        ));
    }
    let limit = FILE_LIMIT.min(*remaining);
    if metadata.len() > limit {
        return Err("release input size limit exceeded".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("release input size limit exceeded".into());
    }
    *remaining -= bytes.len() as u64;
    Ok(bytes)
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))
}
fn parse(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > JSON_LIMIT {
        return Err("release JSON size limit exceeded".into());
    }
    serde_json::from_slice(bytes).map_err(|e| format!("invalid build JSON: {e}"))
}
fn encoded(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value).map_err(|e| e.to_string())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn require(value: &Value, key: &str, expected: Value) -> Result<(), String> {
    if value.get(key) != Some(&expected) {
        return Err(format!(
            "invalid build record field {key}: expected {expected}"
        ));
    }
    Ok(())
}
fn validate_manifest(m: &Value) -> Result<(Target, &'static str, BTreeSet<String>), String> {
    for (key, value) in [
        ("schema", json!("cerune-native-build-v1")),
        ("status", json!("success")),
        ("route", json!("native")),
        ("executed", json!(false)),
        ("error", Value::Null),
        ("optimization_passes", json!([])),
        ("runtime_policy", json!("target-system-runtime")),
        ("link_plan", json!("link-plan.json")),
        ("link_result", json!("link-result.json")),
    ] {
        require(m, key, value)?;
    }
    if m["cerune_version"].as_str().is_none_or(str::is_empty) {
        return Err("missing build Cerune version".into());
    }
    for key in [
        "string_heap_limit",
        "array_heap_limit",
        "argument_count_limit",
        "argument_byte_limit",
    ] {
        if m[key].as_u64().is_none() {
            return Err(format!("invalid build limit {key}"));
        }
    }
    // Older builds have no recorded limit. Preserve that absence; never invent a default.
    if let Some(limit) = m.get("call_depth_limit")
        && limit
            .as_u64()
            .is_none_or(|n| !(1..=u64::from(super::call_depth_cli::MAX)).contains(&n))
    {
        return Err("invalid build limit call_depth_limit".into());
    }
    let target = m["target"]
        .as_str()
        .and_then(Target::parse)
        .ok_or("unsupported build target")?;
    let executable = match target {
        Target::X86_64UnknownLinuxGnu => "program",
        Target::X86_64PcWindowsMsvc => "program.exe",
    };
    require(m, "executable", json!(executable))?;
    let ssa = m["ssa"].as_bool().ok_or("missing build SSA selection")?;
    let sources = m["sources_included"]
        .as_bool()
        .ok_or("missing source inclusion selection")?;
    let mut artifacts: BTreeSet<String> = BASE.iter().map(|s| s.to_string()).collect();
    if ssa {
        artifacts.extend(SSA.iter().map(|s| s.to_string()));
    }
    if sources {
        artifacts.insert("sources.json".into());
    }
    let recorded = m["artifacts"].as_array().ok_or("missing build artifacts")?;
    let recorded_set = recorded
        .iter()
        .map(|v| v.as_str().map(str::to_owned).ok_or("invalid artifact name"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if recorded_set != artifacts || recorded.len() != artifacts.len() {
        return Err("invalid build artifact inventory".into());
    }
    artifacts.extend(RECORDS.iter().map(|s| s.to_string()));
    artifacts.insert(executable.into());
    Ok((target, executable, artifacts))
}
fn validate_records(
    files: &BTreeMap<String, Vec<u8>>,
    target: Target,
    executable: &str,
) -> Result<(), String> {
    let result = parse(&files["link-result.json"])?;
    require(&result, "schema", json!("cerune-link-result-v1"))?;
    require(&result, "status", json!("success"))?;
    require(&result, "executed", json!(false))?;
    for phase in ["version", "link"] {
        let result = parse(&files[&format!("{phase}.result.json")])?;
        require(&result, "phase", json!(phase))?;
        require(&result, "exit_code", json!(0))?;
        if result.get("error").is_some() {
            return Err("failed linker phase record".into());
        }
    }
    let plan = parse(&files["link-plan.json"])?;
    for (key, value) in [
        ("schema", json!("cerune-link-plan-v1")),
        ("target", json!(target.triple())),
        ("input", json!("program.o")),
        ("output", json!(executable)),
    ] {
        require(&plan, key, value)?;
    }
    let runtime = parse(&files["runtime.json"])?;
    for (key, value) in [
        ("schema", json!("cerune-native-runtime-v1")),
        ("policy", json!("target-system-runtime")),
        ("target", json!(target.triple())),
        ("system_libraries_bundled", json!(false)),
    ] {
        require(&runtime, key, value)?;
    }
    if files["program.link.map"].is_empty() {
        return Err("empty link map".into());
    }
    // Validate JSON syntax without interpreting observation formats as an executable Image.
    for (name, bytes) in files {
        if name.ends_with(".json") {
            parse(bytes)?;
        }
    }
    Ok(())
}
fn guide(target: Target, executable: &str, english: bool) -> String {
    let command = match target {
        Target::X86_64UnknownLinuxGnu => format!("./build/{executable} 3"),
        Target::X86_64PcWindowsMsvc => format!(".\\build\\{executable} 3"),
    };
    if english {
        format!(
            "# Native distribution\n\n[日本語](README.md)\n\nTarget: {}. Entry: build/{executable}. From this directory, run the entry with your program arguments, for example:\n\n    {command}\n\nNo Cerune compiler or Clang is needed to run it. The target OS must supply the dependencies recorded in runtime-requirements.json, including their own transitive dependencies. This records direct binary requirements, not a tested minimum OS or a guarantee of compatibility. No system libraries are bundled.\n\nCopy the whole directory. Preserve execute permission on Linux (chmod +x build/program if transferred through Windows). SHA256SUMS covers all payload files and release.json; verify it with sha256sum -c SHA256SUMS or a SHA-256 tool such as Get-FileHash. These hashes detect changed contents; they are not signatures or proof of provenance.\n\nThe build/ files are unchanged snapshots, including generation stages and failure-origin maps. Paths in link logs describe the original build environment. Packaging performs no compilation, link, execution, stripping, or optimization. Source contents are included only if selected at build time, but code literals, filenames and local toolchain paths may still appear. See release.json for the source-inclusion choice and file inventory.\n",
            target.triple()
        )
    } else {
        format!(
            "# Native配布物\n\n[English](README.en.md)\n\ntarget: {}。実行入口: build/{executable}。このディレクトリから、用途に応じた引数で実行します。例：\n\n    {command}\n\n実行にCeruneコンパイラ・Clangは不要です。対象OSにはruntime-requirements.jsonの依存物と、その依存先が必要です。記録は実行ファイルの直接の要求であり、検証済み最低OSや互換性の保証ではありません。システムライブラリは同梱しません。\n\nディレクトリ全体をコピーします。Linuxでは実行権限を保ってください（Windows経由の転送時はchmod +x build/program）。SHA256SUMSはpayload全体とrelease.jsonを対象とします。sha256sum -c SHA256SUMSまたはGet-FileHash等のSHA-256ツールで確認できます。ハッシュは内容の変更検知であり、署名や出自の証明ではありません。\n\nbuild/には生成過程・停止出自を含む元のバイト列を保持します。リンク記録内のパスは元の構築環境の履歴です。配布処理はコンパイル・リンク・実行・strip・最適化を行いません。本文はbuild時の選択を引き継ぎますが、省略時もリテラル・ファイル名・toolchainのローカルパスは含まれることがあります。本文同梱の有無と一覧はrelease.jsonに記録します。\n",
            target.triple()
        )
    }
}
