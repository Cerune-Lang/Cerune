//! CLI orchestration; generation and external linking have separate contracts.
mod generate;
mod link;
use cerune_lang::{codegen::x86_64::Target, runtime::arguments::Limits};
use std::{fs, path::PathBuf, time::Duration};

struct Options {
    input: PathBuf,
    output: PathBuf,
    target: Target,
    linker: PathBuf,
    ssa: bool,
    sources: bool,
    timeout: Duration,
}
impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        if args.next().as_deref() != Some("native") {
            return Err(
                "build requires the explicit route native; IR/VM images are not implemented".into(),
            );
        }
        let input = args
            .next()
            .ok_or("build native requires a source file")?
            .into();
        let (mut output, mut target, mut linker, mut timeout) = (None, None, None, None);
        let (mut ssa, mut sources) = (false, false);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--target" if target.is_none() => {
                    let value = args.next().ok_or("missing target")?;
                    target = Some(Target::parse(&value).ok_or("unsupported native build target")?);
                }
                "--linker" if linker.is_none() => {
                    linker = Some(PathBuf::from(args.next().ok_or("missing Clang path")?))
                }
                "-o" | "--output" if output.is_none() => {
                    output = Some(PathBuf::from(
                        args.next().ok_or("missing output directory")?,
                    ))
                }
                "--ssa" if !ssa => ssa = true,
                "--include-sources" if !sources => sources = true,
                "--link-timeout-seconds" if timeout.is_none() => {
                    let value = args.next().ok_or("missing link timeout")?;
                    let seconds = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| (1..=3600).contains(n))
                        .filter(|_| value.bytes().all(|b| b.is_ascii_digit()))
                        .ok_or("link timeout must be a decimal integer from 1 to 3600")?;
                    timeout = Some(Duration::from_secs(seconds));
                }
                _ => return Err(format!("unexpected or duplicate native build option {arg}")),
            }
        }
        let target = target.ok_or("build native requires --target")?;
        let linker = linker
            .ok_or("build native requires --linker <path-to-clang>; PATH discovery is not used")?;
        let linker = fs::canonicalize(&linker)
            .map_err(|e| format!("linker-unavailable: {}: {e}", linker.display()))?;
        #[cfg(windows)]
        if linker
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd"))
        {
            return Err("unsupported-linker: select a Clang executable, not a shell script".into());
        }
        if !linker.is_file() {
            return Err("linker-unavailable: expected a Clang executable file".into());
        }
        let output = output.ok_or("build native requires -o <new-directory>")?;
        match fs::symlink_metadata(&output) {
            Ok(_) => return Err("native build output already exists".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        Ok(Self {
            input,
            output,
            target,
            linker,
            ssa,
            sources,
            timeout: timeout.unwrap_or(Duration::from_secs(60)),
        })
    }
}

pub(super) fn run(
    args: impl Iterator<Item = String>,
    string_limit: u64,
    array_limit: u64,
    limits: Limits,
) -> Result<(), String> {
    let options = Options::parse(args)?;
    let source = super::read_source(&options.input)?;
    let hir = super::ir_for(&source, string_limit, array_limit)?;
    // No external tools or filesystem output until the shared frontend/generator succeeds.
    let generated = generate::prepare(
        &source,
        &hir,
        options.target,
        options.ssa,
        options.sources,
        limits,
    )?;
    fs::create_dir(&options.output)
        .map_err(|e| format!("cannot create native build directory: {e}"))?;
    let output = fs::canonicalize(&options.output).map_err(|e| e.to_string())?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())
    };
    for (name, bytes) in &generated.files {
        write(name, bytes)?;
    }
    let request = link::Request::clang(options.linker, options.target, &output, options.timeout);
    write("link-plan.json", request.plan().as_bytes())?;
    let manifest = |status: &str, error: Option<&str>| {
        format!(
            "{{\n  \"schema\":\"cerune-native-build-v1\",\n  \"status\":{},\n  \"cerune_version\":{},\n  \"route\":\"native\",\n  \"target\":{},\n  \"executable\":{},\n  \"executed\":false,\n  \"optimization_passes\":[],\n  \"ssa\":{},\n  \"string_heap_limit\":{},\n  \"array_heap_limit\":{},\n  \"argument_count_limit\":{},\n  \"argument_byte_limit\":{},\n  \"sources_included\":{},\n  \"artifacts\":[{}],\n  \"link_plan\":\"link-plan.json\",\n  \"link_result\":\"link-result.json\",\n  \"runtime_policy\":\"target-system-runtime\",\n  \"error\":{}\n}}\n",
            json(status),
            json(env!("CARGO_PKG_VERSION")),
            json(options.target.triple()),
            json(request.executable),
            options.ssa,
            hir.string_heap_limit,
            hir.array_heap_limit,
            limits.max_count,
            limits.max_bytes,
            options.sources,
            generated
                .files
                .iter()
                .map(|(name, _)| json(name))
                .collect::<Vec<_>>()
                .join(","),
            error.map(json).unwrap_or("null".into())
        )
    };
    write("manifest.json", manifest("building", None).as_bytes())?;
    let result = request.execute();
    // A failed link remains inspectable, and is never described as a successful build.
    let contents = match &result {
        Ok(()) => manifest("success", None),
        Err(e) => manifest("failed", Some(e)),
    };
    write("manifest.tmp", contents.as_bytes())?;
    fs::rename(output.join("manifest.tmp"), output.join("manifest.json"))
        .map_err(|e| e.to_string())?;
    result.map_err(|e| format!("{e}; build records: {}", output.display()))
}

fn json(text: &str) -> String {
    use std::fmt::Write;
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c <= '\u{1f}' => write!(result, "\\u{:04x}", c as u32).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
