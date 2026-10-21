//! Ceruneのオプションとプログラム引数を、値を解釈する前に分離します。
use cerune_lang::runtime::arguments::{Arguments, Limits};
use std::ffi::OsString;

pub fn parse(
    command: &str,
    values: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<String>, Arguments, Limits), String> {
    let executable = matches!(command, "run" | "run-ir" | "run-mir" | "run-vm");
    let mut compiler = Vec::new();
    let mut program = Vec::new();
    let mut separated = false;
    for value in values {
        if !separated && value == "--" {
            if !executable {
                return Err("program arguments are only accepted by run commands".into());
            }
            separated = true;
        } else if separated {
            let index = program.len();
            program.push(value.into_string().map_err(|_| {
                format!("arguments-v1 phase=capture code=invalid-unicode index={index}")
            })?);
        } else {
            compiler.push(
                value
                    .into_string()
                    .map_err(|_| "invalid Unicode in Cerune command line".to_owned())?,
            );
        }
    }
    if !executable
        && !matches!(
            command,
            "emit-llvm" | "emit-qbe" | "emit-wat" | "emit-lir" | "emit-asm" | "emit-obj"
        )
    {
        return Ok((compiler, Arguments::unconfigured(), Limits::default()));
    }
    let mut limits = Limits::default();
    let mut rest = Vec::new();
    let mut args = compiler.into_iter();
    let (mut count_seen, mut bytes_seen) = (false, false);
    while let Some(arg) = args.next() {
        let (limit, seen) = match arg.as_str() {
            "--argument-count-limit" => (&mut limits.max_count, &mut count_seen),
            "--argument-byte-limit" => (&mut limits.max_bytes, &mut bytes_seen),
            _ => {
                rest.push(arg);
                continue;
            }
        };
        if *seen {
            return Err(format!("duplicate {arg}"));
        }
        *seen = true;
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{arg} expects an unsigned decimal integer"));
        }
        *limit = value.parse().map_err(|_| format!("{arg} exceeds u64"))?;
    }
    if !executable {
        if limits.max_count > i64::MAX as u64 {
            return Err("argument count limit must not exceed i64::MAX".into());
        }
        return Ok((rest, Arguments::unconfigured(), limits));
    }
    let bytes: Vec<&[u8]> = program.iter().map(|s| s.as_bytes()).collect();
    let arguments = Arguments::capture(&bytes, limits).map_err(|e| e.to_string())?;
    Ok((rest, arguments, limits))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_first_delimiter_separates_options() {
        let values = [
            "app.ceru",
            "--argument-count-limit",
            "3",
            "--",
            "--",
            "--argument-count-limit",
            "0",
        ]
        .map(OsString::from);
        let (compiler, args, _) = parse("run", values).unwrap();
        assert_eq!(compiler, ["app.ceru"]);
        assert_eq!(args.count(), Ok(3));
        assert_eq!(args.get(0), Ok(Some("--")));
        assert_eq!(args.get(1), Ok(Some("--argument-count-limit")));
        assert_eq!(args.get(2), Ok(Some("0")));
        assert!(parse("emit-ir", [OsString::from("--")]).is_err());
        assert!(
            parse(
                "run",
                ["--argument-count-limit", "1", "--argument-count-limit", "2"].map(OsString::from)
            )
            .is_err()
        );
    }
    #[cfg(windows)]
    #[test]
    fn unpaired_utf16_is_rejected_without_exposing_contents() {
        use std::os::windows::ffi::OsStringExt;
        let error = parse(
            "run",
            [
                OsString::from("app.ceru"),
                OsString::from("--"),
                OsString::from_wide(&[0x53, 0xd800]),
            ],
        )
        .unwrap_err();
        assert_eq!(
            error,
            "arguments-v1 phase=capture code=invalid-unicode index=0"
        );
    }
}
