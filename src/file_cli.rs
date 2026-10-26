use cerune_lang::runtime::files::{Directory, Files};
use std::path::PathBuf;

pub fn parse(args: Vec<String>) -> Result<(Vec<String>, Option<PathBuf>), String> {
    let mut rest = Vec::new();
    let mut root = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--file-root" {
            if root.is_some() {
                return Err("duplicate --file-root".into());
            }
            let value = args
                .next()
                .filter(|value| !value.is_empty() && !value.starts_with("--"))
                .ok_or("--file-root requires a directory")?;
            root = Some(PathBuf::from(value));
        } else {
            rest.push(arg);
        }
    }
    Ok((rest, root))
}
pub fn connect(root: Option<PathBuf>) -> Result<Files<Directory>, String> {
    match root {
        Some(root) => Directory::open(&root)
            .map(Files::new)
            .map_err(|error| error.record()),
        None => Ok(Files::unconfigured()),
    }
}
