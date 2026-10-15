//! 実行用stdin接続です。stdioバッファ・文字コード変換・端末設定の変更を行いません。
use super::{Code, Input};
use std::io::{self, Read};

/// プロセスの同期stdinを明示的に選びます。収集型実行APIはこれを自動使用しません。
/// Windows consoleは読取り前にunsupported-connectionとなります。
pub fn stdin() -> Input<Stdin> {
    let (reader, error) = platform::open();
    let mut input = Input::new(Stdin {
        reader,
        unavailable: error,
    });
    input.connection_error = error;
    input
}

/// stdinを所有・closeせず、要求されたバイトだけを読む接続です。
pub struct Stdin {
    reader: platform::Reader,
    unavailable: Option<Code>,
}
impl Read for Stdin {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if let Some(code) = self.unavailable {
            return Err(if code == Code::UnsupportedConnection {
                io::ErrorKind::Unsupported
            } else {
                io::ErrorKind::Other
            }
            .into());
        }
        self.reader.read(bytes)
    }
}
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::ffi::c_void;
    unsafe extern "C" {
        fn read(fd: i32, buffer: *mut c_void, count: usize) -> isize;
    }
    pub struct Reader;
    pub fn open() -> (Reader, Option<Code>) {
        (Reader, None)
    }
    impl Read for Reader {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if bytes.is_empty() {
                return Ok(0);
            }
            // 無効なfdもOSの読取り失敗として扱い、fdの所有を作りません。
            let count = unsafe { read(0, bytes.as_mut_ptr().cast(), 1) };
            if count < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(count as usize)
            }
        }
    }
}
#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    type Handle = *mut c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(kind: u32) -> Handle;
        fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
        fn GetFileType(handle: Handle) -> u32;
        fn ReadFile(
            handle: Handle,
            buffer: *mut c_void,
            size: u32,
            count: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn GetLastError() -> u32;
    }
    pub struct Reader {
        handle: Handle,
        pipe: bool,
    }
    pub fn open() -> (Reader, Option<Code>) {
        // 標準handleは借用し、CloseHandleしません。入力は同期handleを対象にします。
        let handle = unsafe { GetStdHandle((-10i32) as u32) };
        let mut mode = 0;
        let error = if handle.is_null() || handle as isize == -1 {
            Some(Code::ReadFailed)
        } else if unsafe { GetConsoleMode(handle, &mut mode) } != 0 {
            Some(Code::UnsupportedConnection)
        } else {
            None
        };
        let pipe = error.is_none() && unsafe { GetFileType(handle) } == 3;
        (Reader { handle, pipe }, error)
    }
    impl Read for Reader {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if bytes.is_empty() {
                return Ok(0);
            }
            loop {
                let mut count = 0;
                let ok = unsafe {
                    ReadFile(
                        self.handle,
                        bytes.as_mut_ptr().cast(),
                        1,
                        &mut count,
                        std::ptr::null_mut(),
                    )
                };
                if ok != 0 {
                    // pipeの空メッセージはEOFではありません。
                    if count == 0 && self.pipe {
                        continue;
                    }
                    return Ok(count as usize);
                }
                let error = unsafe { GetLastError() };
                if error == 234 && count > 0 {
                    return Ok(count as usize);
                } // ERROR_MORE_DATA
                if error == 109 || error == 38 {
                    return Ok(0);
                } // BROKEN_PIPE / HANDLE_EOF
                // Win32の取消し等を、Rustのエラー分類によって再試行へ変えません。
                return Err(io::ErrorKind::Other.into());
            }
        }
    }
}
#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use super::*;
    pub struct Reader;
    pub fn open() -> (Reader, Option<Code>) {
        (Reader, Some(Code::UnsupportedConnection))
    }
    impl Read for Reader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::Unsupported.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracted_unavailable_reader_never_bypasses_connection_rejection() {
        let (reader, _) = platform::open();
        let mut input = Input::new(Stdin {
            reader,
            unavailable: Some(Code::UnsupportedConnection),
        });
        input.connection_error = Some(Code::UnsupportedConnection);
        let mut reader = input.into_inner().unwrap();
        assert_eq!(
            reader.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
}
