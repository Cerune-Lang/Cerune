//! 明示された設定だけで子プロセスを接続します。
use cerune_lang::runtime::process::Processes;
use std::{io::Read, path::PathBuf};

const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
#[cfg(not(any(windows, target_os = "linux")))]
use cerune_lang::runtime::process::configuration::OsBackend;
#[cfg(target_os = "linux")]
use cerune_lang::runtime::process::linux::Linux as OsBackend;
#[cfg(windows)]
use cerune_lang::runtime::process::windows::Windows as OsBackend;
type Connection = Option<(Processes<OsBackend>, usize)>;

pub fn parse(args: Vec<String>) -> Result<(Vec<String>, Option<PathBuf>), String> {
    let mut rest = Vec::new();
    let mut config = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--process-config" {
            if config.is_some() {
                return Err("duplicate --process-config".into());
            }
            let value = args
                .next()
                .filter(|v| !v.is_empty() && !v.starts_with("--"))
                .ok_or("--process-config requires a JSON file")?;
            config = Some(PathBuf::from(value));
        } else {
            rest.push(arg);
        }
    }
    Ok((rest, config))
}

pub fn connect(path: Option<PathBuf>) -> Result<Connection, String> {
    let Some(path) = path else { return Ok(None) };
    let file =
        std::fs::File::open(path).map_err(|_| "process-config-v1: cannot open configuration")?;
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "process-config-v1: cannot read configuration")?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err("process-config-v1: configuration exceeds 1 MiB".into());
    }
    cerune_lang::runtime::process::configuration::decode(&bytes).map(Some)
}
