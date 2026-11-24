use cerune_lang::runtime::trace::file::{Config, Options};
pub fn parse(args: Vec<String>) -> Result<(Vec<String>, Option<Config>), String> {
    if !args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--trace" | "--trace-event-limit" | "--trace-byte-limit"
        )
    }) {
        return Ok((args, None));
    }
    let mut options = Options::default();
    let mut rest = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if !matches!(
            arg.as_str(),
            "--trace" | "--trace-event-limit" | "--trace-byte-limit"
        ) {
            let paired = matches!(
                arg.as_str(),
                "--file-root"
                    | "--diagnostic-format"
                    | "--process-config"
                    | "--argument-count-limit"
                    | "--argument-byte-limit"
                    | "--string-heap-limit"
                    | "--array-heap-limit"
                    | "--call-depth-limit"
            );
            if paired {
                let val = args
                    .next()
                    .filter(|s| !s.starts_with("--"))
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                rest.extend([arg, val]);
            } else {
                rest.push(arg);
            }
            continue;
        }
        let value = args
            .next()
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .ok_or_else(|| format!("missing value for {arg}"))?;
        options.set(&arg, value.as_ref())?;
    }
    Ok((rest, options.finish()?))
}
