//! Explicit Clang-driver adapter. Object generation is not part of this module.
use super::json;
use cerune_lang::codegen::x86_64::Target;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

pub(super) struct Request {
    driver: PathBuf,
    target: Target,
    directory: PathBuf,
    args: Vec<String>,
    pub executable: &'static str,
    timeout: Duration,
}
impl Request {
    pub fn clang(driver: PathBuf, target: Target, directory: &Path, timeout: Duration) -> Self {
        let executable = match target {
            Target::X86_64UnknownLinuxGnu => "program",
            Target::X86_64PcWindowsMsvc => "program.exe",
        };
        let mut args = vec![
            format!("--target={}", target.triple()),
            "-v".into(),
            "program.o".into(),
            "-o".into(),
            executable.into(),
        ];
        match target {
            Target::X86_64UnknownLinuxGnu => args.extend(
                [
                    "-pie",
                    "-Wl,--no-undefined",
                    "-Wl,-Map,program.link.map",
                    "-lm",
                ]
                .map(str::to_owned),
            ),
            Target::X86_64PcWindowsMsvc => {
                args.extend(["-Xlinker", "/map:program.link.map"].map(str::to_owned))
            }
        }
        Self {
            driver,
            target,
            directory: directory.into(),
            args,
            executable,
            timeout,
        }
    }
    pub fn plan(&self) -> String {
        format!(
            r#"{{"schema":"cerune-link-plan-v1","adapter":"clang-driver-v1","driver":{},"target":{},"cwd":{},"input":"program.o","output":{},"args":[{}],"version_args":["--version"],"timeout_seconds":{},"stdin":"null","environment":"inherited-toolchain","runtime":"target-system-runtime"}}"#,
            json(&self.driver.to_string_lossy()),
            json(self.target.triple()),
            json(&self.directory.to_string_lossy()),
            json(self.executable),
            self.args
                .iter()
                .map(|s| json(s))
                .collect::<Vec<_>>()
                .join(","),
            self.timeout.as_secs()
        )
    }
    pub fn execute(&self) -> Result<(), String> {
        let result = self.execute_inner();
        let record = match &result {
            Ok(()) => r#"{"schema":"cerune-link-result-v1","status":"success","validated":"executable-header","executed":false}"#.to_owned(),
            Err(e) => format!(r#"{{"schema":"cerune-link-result-v1","status":"failed","error":{},"executed":false}}"#, json(e)),
        };
        fs::write(self.directory.join("link-result.json"), record).map_err(|e| e.to_string())?;
        result
    }
    fn execute_inner(&self) -> Result<(), String> {
        let version = self.tool("version", &["--version".into()])?;
        if !version.success() {
            return Err(format!("linker-version-failed: {version}"));
        }
        let bytes =
            fs::read(self.directory.join("version.stdout.log")).map_err(|e| e.to_string())?;
        if !String::from_utf8_lossy(&bytes).contains("clang version") {
            return Err("unsupported-linker: expected Clang driver version output".into());
        }
        let status = self.tool("link", &self.args)?;
        if !status.success() {
            return Err(format!("link-failed: {status}; see link.stderr.log"));
        }
        validate(&self.directory.join(self.executable), self.target)?;
        let map = fs::symlink_metadata(self.directory.join("program.link.map"))
            .map_err(|e| format!("link-map-missing: {e}"))?;
        if !map.is_file() || map.len() == 0 {
            return Err("link-map-invalid: expected a nonempty regular map file".into());
        }
        Ok(())
    }
    fn tool(&self, phase: &str, args: &[String]) -> Result<ExitStatus, String> {
        let start = Instant::now();
        let result = self.tool_inner(phase, args);
        let status = match &result {
            Ok(status) => format!(
                r#""exit_code":{},"termination":{}"#,
                status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or("null".into()),
                json(&status.to_string())
            ),
            Err(e) => format!(r#""exit_code":null,"error":{}"#, json(e)),
        };
        fs::write(
            self.directory.join(format!("{phase}.result.json")),
            format!(
                r#"{{"phase":{},"elapsed_ms":{},{status}}}"#,
                json(phase),
                start.elapsed().as_millis()
            ),
        )
        .map_err(|e| e.to_string())?;
        result
    }
    fn tool_inner(&self, phase: &str, args: &[String]) -> Result<ExitStatus, String> {
        let log = |suffix| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.directory.join(format!("{phase}.{suffix}.log")))
                .map_err(|e| e.to_string())
        };
        let mut command = Command::new(&self.driver);
        command
            .args(args)
            .current_dir(&self.directory)
            .stdin(Stdio::null())
            .stdout(log("stdout")?)
            .stderr(log("stderr")?);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("linker-start-failed ({phase}): {e}"))?;
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) if start.elapsed() < self.timeout => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                other => {
                    // Terminate the driver tree where supported, then reap our own child.
                    #[cfg(unix)]
                    {
                        unsafe extern "C" {
                            fn kill(pid: i32, signal: i32) -> i32;
                        }
                        unsafe {
                            kill(-(child.id() as i32), 9);
                        }
                    }
                    #[cfg(windows)]
                    {
                        if let Some(root) = std::env::var_os("SystemRoot") {
                            let _ = Command::new(PathBuf::from(root).join("System32/taskkill.exe"))
                                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .status();
                        }
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(match other {
                        Err(e) => format!("linker-wait-failed ({phase}): {e}"),
                        _ => format!(
                            "linker-timeout ({phase}): {} seconds",
                            self.timeout.as_secs()
                        ),
                    });
                }
            }
        }
    }
}

// Successful process exit is insufficient: require the selected executable format.
fn validate(path: &Path, target: Target) -> Result<(), String> {
    let invalid =
        || "invalid-link-output: expected an x86-64 executable for the explicit target".to_owned();
    let metadata = fs::symlink_metadata(path).map_err(|_| invalid())?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    let mut file = fs::File::open(path).map_err(|_| invalid())?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header).map_err(|_| invalid())?;
    match target {
        Target::X86_64UnknownLinuxGnu => {
            if &header[..4] != b"\x7fELF"
                || header[4..7] != [2, 1, 1]
                || ![2, 3].contains(&u16::from_le_bytes([header[16], header[17]]))
                || header[18..20] != [62, 0]
            {
                return Err(invalid());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    return Err(invalid());
                }
            }
        }
        Target::X86_64PcWindowsMsvc => {
            if &header[..2] != b"MZ" {
                return Err(invalid());
            }
            let offset = u32::from_le_bytes(header[60..64].try_into().unwrap());
            file.seek(SeekFrom::Start(offset as u64))
                .map_err(|_| invalid())?;
            let mut pe = [0u8; 26];
            file.read_exact(&mut pe).map_err(|_| invalid())?;
            let flags = u16::from_le_bytes([pe[22], pe[23]]);
            if &pe[..4] != b"PE\0\0"
                || pe[4..6] != [0x64, 0x86]
                || pe[24..26] != [0x0b, 0x02]
                || flags & 2 == 0
                || flags & 0x2000 != 0
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
