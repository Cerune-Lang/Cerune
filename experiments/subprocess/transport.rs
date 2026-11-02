//! OS接続の通信・buffer寿命を検証します。製品のBackendではありません。
use std::io;
#[cfg(any(windows, target_os = "linux"))]
#[path = "fixture.rs"]
mod fixture;

#[cfg(target_os = "linux")]
#[path = "transport/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "transport/windows.rs"]
mod platform;

#[cfg(any(windows, target_os = "linux"))]
fn require(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(message.to_owned()))
    }
}
#[cfg(any(windows, target_os = "linux"))]
fn case(
    log: &mut std::fs::File,
    name: &str,
    run: impl FnOnce() -> io::Result<serde_json::Value>,
) -> io::Result<()> {
    use std::io::Write;
    match run() {
        Ok(evidence) => writeln!(
            log,
            "{}",
            serde_json::json!({"case":name,"passed":true,"evidence":evidence})
        ),
        Err(error) => {
            writeln!(
                log,
                "{}",
                serde_json::json!({"case":name,"passed":false,"error_kind":format!("{:?}",error.kind())})
            )?;
            Err(error)
        }
    }
}
fn main() -> io::Result<()> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("--fixture") {
            if args.len() < 2 {
                return Err(io::Error::other("missing fixture"));
            }
            return fixture::run(&args[1..]);
        }
        if let Some(result) = platform::helper(&args) {
            return result;
        }
        if args.len() != 1 {
            return Err(io::Error::other(
                "usage: subprocess_transport_probe <new-output-directory>",
            ));
        }
        let root = std::path::absolute(&args[0])?;
        std::fs::create_dir(&root)?;
        let mut log = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("observations.jsonl"))?;
        platform::run(&mut log)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err(io::Error::other(
            "transport probe supports Windows and Linux",
        ))
    }
}
