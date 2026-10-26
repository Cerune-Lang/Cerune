use super::{ConnectError, OpenError, Operation};
use std::{
    ffi::c_void,
    fs::File,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, IntoRawHandle},
    },
    path::{Component, Path, Prefix},
    ptr,
};
type RawHandle = *mut c_void;
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: RawHandle,
    name: *mut UnicodeString,
    attributes: u32,
    security: *mut c_void,
    quality: *mut c_void,
}
#[repr(C)]
struct IoStatus {
    status: usize,
    information: usize,
}
#[repr(C)]
struct AttributeTag {
    attributes: u32,
    tag: u32,
}
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtCreateFile(
        handle: *mut RawHandle,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatus,
        allocation: *const i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea: *const c_void,
        ea_length: u32,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandleEx(
        handle: RawHandle,
        class: i32,
        info: *mut c_void,
        size: u32,
    ) -> i32;
    fn GetFileType(handle: RawHandle) -> u32;
    fn CloseHandle(handle: RawHandle) -> i32;
}
const OBJ_CASE_INSENSITIVE: u32 = 0x40;
const SYNCHRONIZE: u32 = 0x100000;
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_TRAVERSE: u32 = 0x20;
const FILE_READ_DATA: u32 = 1;
const FILE_WRITE_DATA: u32 = 2;
const FILE_DIRECTORY_FILE: u32 = 1;
const FILE_NON_DIRECTORY_FILE: u32 = 0x40;
const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x20;
const FILE_OPEN_REPARSE_POINT: u32 = 0x200000;
const FILE_OPEN: u32 = 1;
const FILE_CREATE: u32 = 2;
const FILE_SHARE_READ_WRITE_DELETE: u32 = 7;
const FILE_ATTRIBUTE_TAG_INFO: i32 = 9;
const FILE_TYPE_DISK: u32 = 1;
const DIRECTORY: u32 = 0x10;
const REPARSE: u32 = 0x400;
fn attributes(file: &File) -> Result<u32, OpenError> {
    // pipe/deviceには通常ファイル用の情報照会を行いません。
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK {
        return Err(OpenError::UnsupportedTarget);
    }
    let mut info = AttributeTag {
        attributes: 0,
        tag: 0,
    };
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FILE_ATTRIBUTE_TAG_INFO,
            (&mut info as *mut AttributeTag).cast(),
            size_of::<AttributeTag>() as u32,
        )
    } == 0
    {
        return Err(OpenError::Failed);
    }
    Ok(info.attributes)
}
fn open_nt(
    root: RawHandle,
    name: &mut [u16],
    directory: bool,
    create: bool,
) -> Result<File, OpenError> {
    let length = u16::try_from(name.len().checked_mul(2).ok_or(OpenError::Failed)?)
        .map_err(|_| OpenError::Failed)?;
    let mut name = UnicodeString {
        length,
        maximum_length: length,
        buffer: name.as_mut_ptr(),
    };
    let mut object = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root,
        name: &mut name,
        attributes: OBJ_CASE_INSENSITIVE,
        security: ptr::null_mut(),
        quality: ptr::null_mut(),
    };
    let mut status = IoStatus {
        status: 0,
        information: 0,
    };
    let mut handle = ptr::null_mut();
    // 同期handle・reparseを追わない指定・型制限を、open/createと同時に渡します。
    let access = SYNCHRONIZE
        | FILE_READ_ATTRIBUTES
        | if directory {
            FILE_TRAVERSE
        } else if create {
            FILE_WRITE_DATA
        } else {
            FILE_READ_DATA
        };
    let options = FILE_SYNCHRONOUS_IO_NONALERT
        | FILE_OPEN_REPARSE_POINT
        | if directory {
            FILE_DIRECTORY_FILE
        } else {
            FILE_NON_DIRECTORY_FILE
        };
    let result = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &mut object,
            &mut status,
            ptr::null(),
            0,
            FILE_SHARE_READ_WRITE_DELETE,
            if create { FILE_CREATE } else { FILE_OPEN },
            options,
            ptr::null(),
            0,
        )
    };
    if result < 0 {
        return Err(match result as u32 {
            0xc0000034 | 0xc000003a => OpenError::NotFound,
            0xc0000035 => OpenError::AlreadyExists,
            0xc00000ba if create => OpenError::AlreadyExists,
            0xc00000ba | 0xc0000103 | 0xc0000279 | 0xc000050b => OpenError::UnsupportedTarget,
            _ => OpenError::Failed,
        });
    }
    if handle.is_null() || handle as isize == -1 {
        return Err(OpenError::InvalidResponse);
    }
    // 成功時だけ所有権をFileへ移します。失敗時に再openしません。
    Ok(unsafe { File::from_raw_handle(handle) })
}
pub(super) fn connect(path: &Path) -> Result<File, ConnectError> {
    let absolute = std::path::absolute(path).map_err(|_| ConnectError::InvalidRoot)?;
    // DOS drive/UNCだけを受け付けます。device namespaceをrootとして解釈しません。
    let mut components = absolute.components();
    let prefix = match components.next() {
        Some(Component::Prefix(prefix)) => prefix.kind(),
        _ => return Err(ConnectError::InvalidRoot),
    };
    let mut name: Vec<u16> = "\\??\\".encode_utf16().collect();
    match prefix {
        Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => {
            name.extend([drive as u16, b':' as u16])
        }
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
            name.extend("UNC\\".encode_utf16());
            name.extend(server.encode_wide());
            name.push(b'\\' as u16);
            name.extend(share.encode_wide());
        }
        _ => return Err(ConnectError::InvalidRoot),
    }
    name.push(b'\\' as u16);
    let mut first = true;
    for part in components {
        match part {
            Component::RootDir | Component::CurDir => continue,
            Component::Normal(value) => {
                if !first {
                    name.push(b'\\' as u16);
                }
                name.extend(value.encode_wide());
                first = false;
            }
            _ => return Err(ConnectError::InvalidRoot),
        }
    }
    if name.contains(&0) {
        return Err(ConnectError::InvalidRoot);
    }
    let file = open_nt(ptr::null_mut(), &mut name, true, false)?;
    let attrs = attributes(&file)?;
    if attrs & REPARSE != 0 || attrs & DIRECTORY == 0 {
        return Err(ConnectError::UnsupportedTarget);
    }
    Ok(file)
}
pub(super) fn open(root: &File, name: &str, operation: Operation) -> Result<File, OpenError> {
    let mut name: Vec<u16> = name.encode_utf16().collect();
    let create = operation == Operation::WriteNew;
    let file = open_nt(root.as_raw_handle(), &mut name, false, create)?;
    // 排他的create成功は新しい通常ファイルです。作成後の失敗を未作成と偽りません。
    if !create && attributes(&file)? & (REPARSE | DIRECTORY) != 0 {
        return Err(OpenError::UnsupportedTarget);
    }
    Ok(file)
}
pub(super) fn close(file: File) -> io::Result<()> {
    if unsafe { CloseHandle(file.into_raw_handle()) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
