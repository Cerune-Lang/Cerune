//! 実行前に明示されたディレクトリを、開いたhandleで固定します。
use super::{Backend, Handle, OpenError, Operation, validate_name};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use windows as platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectError {
    InvalidRoot,
    NotFound,
    UnsupportedTarget,
    OpenFailed,
    UnsupportedPlatform,
}
impl ConnectError {
    pub fn record(self) -> String {
        let code = match self {
            Self::InvalidRoot => "invalid-root",
            Self::NotFound => "not-found",
            Self::UnsupportedTarget => "unsupported-target",
            Self::OpenFailed => "open-failed",
            Self::UnsupportedPlatform => "unsupported-platform",
        };
        format!("file-v1 operation=connect phase=open-root code={code} origin=host")
    }
}
impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.record())
    }
}
impl std::error::Error for ConnectError {}
impl From<OpenError> for ConnectError {
    fn from(error: OpenError) -> Self {
        match error {
            OpenError::NotFound => Self::NotFound,
            OpenError::UnsupportedTarget => Self::UnsupportedTarget,
            _ => Self::OpenFailed,
        }
    }
}
pub struct Directory {
    #[cfg(any(windows, target_os = "linux"))]
    root: File,
}
impl Directory {
    /// rootは接続時だけ解決します。省略時の作業ディレクトリ探索は行いません。
    pub fn open(root: &Path) -> Result<Self, ConnectError> {
        if root.as_os_str().is_empty() {
            return Err(ConnectError::InvalidRoot);
        }
        #[cfg(any(windows, target_os = "linux"))]
        {
            platform::connect(root).map(|root| Self { root })
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            Err(ConnectError::UnsupportedPlatform)
        }
    }
}
impl Backend for Directory {
    type Handle = FileHandle;
    fn open(&mut self, name: &str, operation: Operation) -> Result<FileHandle, OpenError> {
        validate_name(name).map_err(|_| OpenError::Failed)?;
        #[cfg(any(windows, target_os = "linux"))]
        {
            platform::open(&self.root, name, operation).map(FileHandle)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = operation;
            Err(OpenError::UnsupportedTarget)
        }
    }
}
pub struct FileHandle(File);
impl Read for FileHandle {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Write for FileHandle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Handle for FileHandle {
    fn close(self) -> io::Result<()> {
        #[cfg(any(windows, target_os = "linux"))]
        {
            platform::close(self.0)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            drop(self);
            Err(io::ErrorKind::Unsupported.into())
        }
    }
}
