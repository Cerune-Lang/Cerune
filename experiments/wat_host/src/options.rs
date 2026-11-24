use std::{ffi::OsString, path::PathBuf};
pub(super) struct Options {
    pub(super) trace: Option<cerune_lang::runtime::trace::file::Config>,
    pub(super) source: PathBuf,
    pub(super) root: Option<PathBuf>,
    pub(super) process_target: Option<u64>,
    pub(super) process_config: Option<PathBuf>,
    pub(super) arguments: Vec<Vec<u8>>,
}
pub(super) fn options(mut args: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let usage = concat!(
        "usage: cerune-wat-host <module.wat|module.wasm> ",
        "[--file-root <directory>] [--process-target <triple>] ",
        "[--process-config <json>] [--trace <new-file> [--trace-event-limit <n>] [--trace-byte-limit <n>]] -- <program arguments>"
    );
    let source = args.next().ok_or(usage)?.into();
    let mut trace = cerune_lang::runtime::trace::file::Options::default();
    let mut root = None;
    let mut process_target = None;
    let mut process_config = None;
    let mut delimited = false;
    while let Some(option) = args.next() {
        if option == "--" {
            delimited = true;
            break;
        }
        if let Some(key @ ("--trace" | "--trace-event-limit" | "--trace-byte-limit")) =
            option.to_str()
        {
            trace.set(
                key,
                &args
                    .next()
                    .ok_or_else(|| format!("missing value for {key}"))?,
            )?;
            continue;
        }
        let available = (option == "--file-root" && root.is_none())
            || (option == "--process-target" && process_target.is_none())
            || (option == "--process-config" && process_config.is_none());
        if !available {
            return Err("unknown or duplicate host setting".into());
        }
        let value = args.next().ok_or("missing host setting value")?;
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err("invalid host setting value".into());
        }
        if option == "--file-root" {
            root = Some(value.into());
        } else if option == "--process-config" {
            process_config = Some(value.into());
        } else {
            process_target = Some(match value.to_str() {
                Some("x86_64-pc-windows-msvc") => 1,
                Some("x86_64-unknown-linux-gnu") => 2,
                _ => {
                    return Err(concat!(
                        "unsupported --process-target; use ",
                        "x86_64-pc-windows-msvc or x86_64-unknown-linux-gnu"
                    )
                    .into());
                }
            });
        }
    }
    if !delimited {
        return Err("missing host argument delimiter --".into());
    }
    if process_config.is_some() && process_target.is_none() {
        return Err("--process-config requires explicit --process-target".into());
    }
    let arguments = args
        .enumerate()
        .map(|(i, a)| {
            a.into_string()
                .map(String::into_bytes)
                .map_err(|_| format!("arguments-v1 phase=capture code=invalid-unicode index={i}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(Options {
        source,
        trace: trace.finish()?,
        root,
        process_target,
        process_config,
        arguments,
    })
}
#[cfg(all(
    target_arch = "x86_64",
    any(
        all(windows, target_env = "msvc"),
        all(target_os = "linux", target_env = "gnu")
    )
))]
pub(super) fn configuration(path: Option<&std::path::Path>) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let Some(path) = path else { return Ok(vec![]) };
    let file = std::fs::File::open(path).map_err(|_| "cannot read process configuration")?;
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read process configuration")?;
    if bytes.len() > 1_048_576 {
        return Err("process configuration exceeds 1 MiB".into());
    }
    // Empty configuration bytes are the ABI's explicit unconfigured sentinel, not JSON.
    if bytes.is_empty() {
        return Err("invalid empty process configuration".into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, String> {
        options(
            std::iter::once("module.wat")
                .chain(args.iter().copied())
                .map(OsString::from),
        )
    }
    #[test]
    fn explicit_settings_do_not_consume_program_arguments() {
        let p = parse(&[
            "--process-config",
            "p.json",
            "--file-root",
            "data",
            "--process-target",
            "x86_64-unknown-linux-gnu",
            "--",
            "",
            "--process-config",
            "日本語",
        ])
        .unwrap();
        assert_eq!(p.process_target, Some(2));
        assert_eq!(p.process_config, Some(PathBuf::from("p.json")));
        assert_eq!(p.root, Some(PathBuf::from("data")));
        assert_eq!(
            p.arguments,
            [
                b"".to_vec(),
                b"--process-config".to_vec(),
                "日本語".as_bytes().to_vec()
            ]
        );
        assert!(parse(&["--"]).unwrap().process_target.is_none());
    }
    #[test]
    fn rejects_missing_duplicate_and_invalid_settings() {
        for args in [
            vec![],
            vec!["--process-target"],
            vec!["--process-target", "--"],
            vec!["--process-target", "bad", "--"],
            vec!["--process-config", "", "--"],
            vec!["--process-config", "x", "--"],
            vec!["--file-root", "--process-target", "--"],
            vec!["--unknown", "x", "--"],
            vec![
                "--process-target",
                "x86_64-pc-windows-msvc",
                "--process-target",
                "x86_64-pc-windows-msvc",
                "--",
            ],
            vec!["--process-config", "x", "--process-config", "x", "--"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
}
