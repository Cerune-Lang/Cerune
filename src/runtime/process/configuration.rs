//! CLIと外部runtimeで共有する明示接続設定です。
#[cfg(target_os = "linux")]
use super::linux::{Linux as OsBackend, Target};
#[cfg(windows)]
use super::windows::{Target, Windows as OsBackend};
use super::{Limits, Processes};
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
#[cfg(not(any(windows, target_os = "linux")))]
#[path = "configuration/unsupported.rs"]
mod unsupported;
#[cfg(not(any(windows, target_os = "linux")))]
pub use unsupported::OsBackend;
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
pub fn decode(bytes: &[u8]) -> Result<(Processes<OsBackend>, usize), String> {
    if bytes.len() > 1024 * 1024 {
        return Err("process-config-v1: configuration exceeds 1 MiB".into());
    }
    let config: Config = serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "process-config-v1: invalid configuration at line {} column {}",
            e.line(),
            e.column()
        )
    })?;
    let limits = config.limits()?;
    let transfer_bytes = config.transfer_bytes;
    Ok((Processes::new(config.backend()?, limits), transfer_bytes))
}
