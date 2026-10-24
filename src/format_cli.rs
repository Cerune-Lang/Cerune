use cerune_lang::{
    format::{self, LetStyle},
    source::SourceMap,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

const USAGE: &str = "cerune fmt <file> [--let preserve|show|hide] [--check | --write | -o <file>]";
enum Destination {
    Stdout,
    Check,
    Write,
    File(PathBuf),
}
pub(super) fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let input = PathBuf::from(args.next().ok_or_else(|| format!("usage: {USAGE}"))?);
    let mut style = None;
    let mut destination = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--let" if style.is_none() => {
                style = Some(match args.next().as_deref() {
                    Some("preserve") => LetStyle::Preserve,
                    Some("show") => LetStyle::Show,
                    Some("hide") => LetStyle::Hide,
                    _ => return Err("--let expects preserve, show or hide".into()),
                })
            }
            "--check" if destination.is_none() => destination = Some(Destination::Check),
            "--write" if destination.is_none() => destination = Some(Destination::Write),
            "-o" | "--output" if destination.is_none() => {
                destination = Some(Destination::File(PathBuf::from(
                    args.next().ok_or("missing output file")?,
                )))
            }
            _ => {
                return Err(format!(
                    "unexpected or conflicting option {arg}; usage: {USAGE}"
                ));
            }
        }
    }
    let limit = cerune_lang::modules::LoadLimits::default().max_file_bytes;
    let mut data = Vec::new();
    fs::File::open(&input)
        .map_err(|e| format!("{}: {e}", input.display()))?
        .take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > limit {
        return Err(format!("source file exceeds {limit} bytes"));
    }
    let text = String::from_utf8(data).map_err(|_| "input is not valid UTF-8")?;
    let mut sources = SourceMap::new();
    let id = sources.add(input.display().to_string(), text.clone());
    let formatted = format::format_source(sources.get(id).unwrap(), style.unwrap_or_default())
        .map_err(|e| cerune_lang::diagnostic::render::render_compact_with_sources(&e, &sources))?;
    match destination.unwrap_or(Destination::Stdout) {
        Destination::Stdout => std::io::stdout()
            .lock()
            .write_all(formatted.as_bytes())
            .map_err(|e| e.to_string()),
        Destination::Check => {
            if formatted == text {
                Ok(())
            } else {
                Err(format!("{} needs formatting", input.display()))
            }
        }
        Destination::Write => {
            if formatted == text {
                Ok(())
            } else {
                save(&input, &formatted)
            }
        }
        Destination::File(path) => save(&path, &formatted),
    }
}

// Prepare and sync a sibling file before replacing the explicitly selected path.
fn save(path: &Path, contents: &str) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => {
            if m.file_type().is_symlink() || !m.is_file() {
                return Err("fmt output must be a regular file, not a link or directory".into());
            }
            if m.permissions().readonly() {
                return Err("fmt output is read-only".into());
            }
            Some(m)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (temp, mut file) = loop {
        let temp = parent.join(format!(
            ".cerune-fmt-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    };
    let result = (|| {
        file.write_all(contents.as_bytes())?;
        if let Some(metadata) = metadata {
            file.set_permissions(metadata.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.map_err(|e| format!("could not save {}: {e}", path.display()))
}
