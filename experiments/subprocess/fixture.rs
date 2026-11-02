use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
    time::Duration,
};

pub const BLOCK: usize = 128 * 1024;
pub fn run(args: &[String]) -> io::Result<()> {
    match args[0].as_str() {
        #[cfg(windows)]
        "transport-zero" => {
            use std::ffi::c_void;
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetStdHandle(kind: u32) -> *mut c_void;
                fn WriteFile(
                    handle: *mut c_void,
                    bytes: *const c_void,
                    size: u32,
                    count: *mut u32,
                    ov: *mut c_void,
                ) -> i32;
            }
            let mut n = 0;
            if unsafe {
                WriteFile(
                    GetStdHandle(-11i32 as u32),
                    b"".as_ptr().cast(),
                    0,
                    &mut n,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            io::stdout().write_all(b"tail")?;
            io::stdout().flush()
        }
        "transport-echo" => {
            let mut bytes = Vec::new();
            io::stdin().take(4097).read_to_end(&mut bytes)?;
            if bytes.len() > 4096 {
                return Err(io::Error::other("fixture input bound"));
            }
            io::stdout().write_all(&bytes)?;
            io::stdout().flush()?;
            io::stderr().write_all(b"err\0\r\n")?;
            io::stderr().flush()?;
            std::process::exit(7)
        }
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
