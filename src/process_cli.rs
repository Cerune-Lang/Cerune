//! 明示された設定だけで子プロセスを接続します。
use cerune_lang::runtime::process::{Limits, Processes};
use serde::Deserialize;
use std::{io::Read, path::PathBuf, time::Duration};

#[cfg(target_os = "linux")]
use cerune_lang::runtime::process::linux::{Linux as OsBackend, Target};
#[cfg(windows)]
use cerune_lang::runtime::process::windows::{Target, Windows as OsBackend};
#[cfg(not(any(windows, target_os = "linux")))]
#[path = "process_cli/unsupported.rs"]
mod unsupported;
#[cfg(not(any(windows, target_os = "linux")))]
use unsupported::OsBackend;

const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    platform: String,
    linux_supervisor: Option<PathBuf>,
    limits: ConfigLimits,
    transfer_bytes: usize,
    targets: Vec<Registration>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigLimits {
    arguments: usize,
    argument_bytes: usize,
    stdin_bytes: usize,
    output_bytes: usize,
    timeout_ms: u64,
    cleanup_timeout_ms: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
struct Registration {
    name: String,
    executable: PathBuf,
    directory: PathBuf,
    environment: Vec<(String, String)>,
}
impl Config {
    fn limits(&self) -> Result<Limits, String> {
        if self.schema != "cerune-process-v1" {
            return Err("process-config-v1: unsupported schema".into());
        }
        if self.platform != std::env::consts::OS {
            return Err("process-config-v1: platform does not match this host".into());
        }
        if self.limits.arguments > 4096 || self.limits.argument_bytes > 16384 {
            return Err("process-config-v1: argument limits exceed OS connection capacity".into());
        }
        if self.limits.timeout_ms == 0 || self.limits.cleanup_timeout_ms == 0 {
            return Err("process-config-v1: time limits must be positive".into());
        }
        Ok(Limits {
            arguments: self.limits.arguments,
            argument_bytes: self.limits.argument_bytes,
            stdin_bytes: self.limits.stdin_bytes,
            output_bytes: self.limits.output_bytes,
            timeout: Duration::from_millis(self.limits.timeout_ms),
            cleanup_timeout: Duration::from_millis(self.limits.cleanup_timeout_ms),
        })
    }
    #[cfg(any(windows, target_os = "linux"))]
    fn backend(self) -> Result<OsBackend, String> {
        let targets = self
            .targets
            .into_iter()
            .map(|r| Target {
                name: r.name,
                executable: r.executable,
                directory: r.directory,
                environment: r.environment,
            })
            .collect();
        #[cfg(windows)]
        {
            if self.linux_supervisor.is_some() {
                return Err(
                    "process-config-v1: linux_supervisor is not accepted on Windows".into(),
                );
            }
            OsBackend::new(targets, 0)
                .map_err(|e| format!("process-config-v1: invalid target registration ({e:?})"))
        }
        #[cfg(target_os = "linux")]
        {
            let supervisor = self
                .linux_supervisor
                .ok_or("process-config-v1: linux_supervisor is required on Linux")?;
            OsBackend::new(supervisor, targets, 0).map_err(|e| {
                format!("process-config-v1: invalid supervisor or target registration ({e:?})")
            })
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    fn backend(self) -> Result<OsBackend, String> {
        // 他OSでも構文は同じです。実行可能な接続は作りません。
        let _ = (self.linux_supervisor, self.targets);
        Err("process-config-v1: unsupported host platform".into())
    }
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
    let config: Config = serde_json::from_slice(&bytes).map_err(|e| {
        // 環境変数や設定本文を診断へ転記しません。
        format!(
            "process-config-v1: invalid configuration at line {} column {}",
            e.line(),
            e.column()
        )
    })?;
    let limits = config.limits()?;
    let transfer_bytes = config.transfer_bytes;
    let backend = config.backend()?;
    Ok(Some((Processes::new(backend, limits), transfer_bytes)))
}
