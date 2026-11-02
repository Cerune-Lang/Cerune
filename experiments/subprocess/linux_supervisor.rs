//! 配布用CLIではなく、Linux Backend検証用の明示監督です。
fn main() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::{FromRawFd, OwnedFd};
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 2 || args[0] != "--cerune-process-supervisor-v1" {
            return Err(std::io::Error::other(
                "expected supervisor marker and inherited descriptor",
            ));
        }
        let fd: i32 = args[1].parse().map_err(std::io::Error::other)?;
        if fd < 3 || unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0 {
            return Err(std::io::Error::other("invalid control descriptor"));
        }
        cerune_lang::runtime::process::linux::supervisor::serve(unsafe { OwnedFd::from_raw_fd(fd) })
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(std::io::Error::other("Linux supervisor requires Linux"))
    }
}
