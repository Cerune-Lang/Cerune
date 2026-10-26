use super::{ConnectError, OpenError, Operation};
use std::{
    ffi::CString,
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
};
fn error(error: io::Error) -> OpenError {
    match error.raw_os_error() {
        Some(libc::ENOENT) => OpenError::NotFound,
        Some(libc::EEXIST) => OpenError::AlreadyExists,
        Some(libc::ELOOP | libc::ENOTDIR | libc::EISDIR) => OpenError::UnsupportedTarget,
        _ => OpenError::Failed,
    }
}
fn owned(fd: i32) -> Result<File, OpenError> {
    if fd < 0 {
        Err(error(io::Error::last_os_error()))
    } else {
        /* 成功したfdの唯一の所有者になります。 */
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
pub(super) fn connect(path: &Path) -> Result<File, ConnectError> {
    let bytes = path.as_os_str().as_bytes();
    // 最終要素のリンク検査を末尾のslashで回避させません。
    let end = bytes.iter().rposition(|b| *b != b'/').map_or(1, |i| i + 1);
    let path = CString::new(&bytes[..end]).map_err(|_| ConnectError::InvalidRoot)?;
    // O_PATHにより、deviceやFIFOをデータ入出力用に開きません。
    let root = owned(unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    })?;
    if !root
        .metadata()
        .map_err(|_| ConnectError::OpenFailed)?
        .is_dir()
    {
        return Err(ConnectError::UnsupportedTarget);
    }
    Ok(root)
}
pub(super) fn open(root: &File, name: &str, operation: Operation) -> Result<File, OpenError> {
    let name = CString::new(name).map_err(|_| OpenError::Failed)?;
    if operation == Operation::WriteNew {
        // 検査と作成を分けず、既存の名前には一切書き込みません。
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o666,
            )
        };
        if fd < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EISDIR) {
            return Err(OpenError::AlreadyExists);
        }
        return owned(fd);
    }
    let pinned = owned(unsafe {
        libc::openat(
            root.as_raw_fd(),
            name.as_ptr(),
            libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    })?;
    let before = pinned.metadata().map_err(|_| OpenError::Failed)?;
    if !before.is_file() {
        return Err(OpenError::UnsupportedTarget);
    }
    // 検査済みのinodeを読み直します。名前の差し替え後に元のpathへ戻りません。
    // procfsが使えない場合は失敗し、通常のpath openへfallbackしません。
    let file = File::open(format!("/proc/self/fd/{}", pinned.as_raw_fd()))
        .map_err(|_| OpenError::Failed)?;
    let after = file.metadata().map_err(|_| OpenError::Failed)?;
    if !after.is_file() || before.dev() != after.dev() || before.ino() != after.ino() {
        return Err(OpenError::InvalidResponse);
    }
    Ok(file)
}
pub(super) fn close(file: File) -> io::Result<()> {
    // close失敗でも所有権を戻したり再試行したりしません。
    if unsafe { libc::close(file.into_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
