//! 子プロセスのOS成立条件を測る実験です。Ceruneの公開実行経路ではありません。
#[cfg(any(windows, target_os = "linux"))]
mod probe;
fn main() -> std::io::Result<()> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        probe::run()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err(std::io::Error::other(
            "subprocess probe supports Windows and Linux only",
        ))
    }
}
