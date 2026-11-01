use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
    time::Duration,
};

pub const BLOCK: usize = 128 * 1024;
pub fn run(args: &[String]) -> io::Result<()> {
    match args[0].as_str() {
        "context" => {
            println!(
                "{}",
                serde_json::json!({
                    "args": &args[1..], "cwd": std::env::current_dir()?,
                    "value": std::env::var("CERUNE_PROBE").ok(),
                    "path_absent": std::env::var_os("PATH").is_none()
                })
            );
            io::stderr().write_all(b"err\0\r\n")
        }
        "duplex" => {
            io::stdout().write_all(&vec![255; BLOCK])?;
            io::stderr().write_all(&vec![128; BLOCK])?;
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input)?;
            if input != vec![42; BLOCK] {
                return Err(io::Error::other("stdin mismatch"));
            }
            io::stdout().write_all(&input)
        }
        "exit" => std::process::exit(-1),
        "early-close" => Ok(()),
        "limit" => io::stdout().write_all(&vec![77; BLOCK]),
        "exact" => io::stdout().write_all(b"1234567"),
        "sleep" => {
            io::stdout().write_all(b"ready\n")?;
            io::stdout().flush()?;
            io::stderr().write_all(b"partial\0\r\n")?;
            io::stderr().flush()?;
            std::thread::sleep(Duration::from_secs(60));
            Ok(())
        }
        "descendant" => {
            let mut command = Command::new(std::env::current_exe()?);
            command
                .args(["--fixture", "sleep"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0800_0000);
            }
            // 直接の子が終わっても、孫がpipeを保持する状態を作ります。
            drop(command.spawn()?);
            Ok(())
        }
        _ => Err(io::Error::other("unknown fixture")),
    }
}
