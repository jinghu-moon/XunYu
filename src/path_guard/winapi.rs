use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{GetLastError, FILETIME, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileAttributesExW, GetFileAttributesW, GetFinalPathNameByHandleW,
    GetFullPathNameW, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetFileExInfoStandard, INVALID_FILE_ATTRIBUTES, OPEN_EXISTING, VOLUME_NAME_DOS,
    WIN32_FILE_ATTRIBUTE_DATA,
};
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;

use crate::path_guard::policy::{PathIssueKind, PathPolicy};

thread_local! {
    static WIDE_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512));
}

pub(crate) fn to_wide_with_prefix(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    ensure_long_prefix(&mut wide);
    wide.push(0);
    wide
}

pub(crate) fn probe(path: &Path) -> Result<u32, PathIssueKind> {
    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(path.as_os_str().encode_wide());
        ensure_long_prefix(&mut buf);
        buf.push(0);
        probe_path(&buf)
    })
}

pub(crate) fn probe_ex(path: &Path) -> Result<WIN32_FILE_ATTRIBUTE_DATA, PathIssueKind> {
    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(path.as_os_str().encode_wide());
        ensure_long_prefix(&mut buf);
        buf.push(0);

        let zero_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut data = WIN32_FILE_ATTRIBUTE_DATA {
            dwFileAttributes: 0,
            ftCreationTime: zero_time,
            ftLastAccessTime: zero_time,
            ftLastWriteTime: zero_time,
            nFileSizeHigh: 0,
            nFileSizeLow: 0,
        };

        let ok = unsafe {
            GetFileAttributesExW(
                buf.as_ptr(),
                GetFileExInfoStandard,
                &mut data as *mut _ as *mut _,
            )
        };
        if ok == 0 {
            let code = unsafe { GetLastError() };
            return Err(map_error_code(code));
        }

        Ok(data)
    })
}

pub(crate) fn get_full_path(path: &Path) -> Result<PathBuf, PathIssueKind> {
    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(path.as_os_str().encode_wide());
        buf.push(0);

        let mut out: Vec<u16> = vec![0u16; 260];
        let mut written =
            unsafe { GetFullPathNameW(buf.as_ptr(), out.len() as u32, out.as_mut_ptr(), std::ptr::null_mut()) };
        if written == 0 {
            return Err(PathIssueKind::IoError);
        }
        if written as usize > out.len() {
            out.resize(written as usize, 0);
            written = unsafe {
                GetFullPathNameW(buf.as_ptr(), out.len() as u32, out.as_mut_ptr(), std::ptr::null_mut())
            };
            if written == 0 {
                return Err(PathIssueKind::IoError);
            }
        }
        let len = written as usize;
        out.truncate(len);
        Ok(PathBuf::from(OsString::from_wide(&out)))
    })
}

pub(crate) fn get_final_path(handle: &OwnedHandle) -> Result<PathBuf, PathIssueKind> {
    let mut out: Vec<u16> = vec![0u16; 260];
    let mut written = unsafe {
        GetFinalPathNameByHandleW(
            handle.as_raw_handle() as *mut _,
            out.as_mut_ptr(),
            out.len() as u32,
            FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
        )
    };
    if written == 0 {
        return Err(PathIssueKind::IoError);
    }
    if written as usize > out.len() {
        out.resize(written as usize, 0);
        written = unsafe {
            GetFinalPathNameByHandleW(
                handle.as_raw_handle() as *mut _,
                out.as_mut_ptr(),
                out.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        };
        if written == 0 {
            return Err(PathIssueKind::IoError);
        }
    }

    let len = written as usize;
    out.truncate(len);
    Ok(PathBuf::from(OsString::from_wide(&out)))
}

pub(crate) fn expand_env(raw: &OsStr) -> Result<OsString, PathIssueKind> {
    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(raw.encode_wide());
        buf.push(0);

        let mut out = vec![0u16; 260];
        let mut written =
            unsafe { ExpandEnvironmentStringsW(buf.as_ptr(), out.as_mut_ptr(), out.len() as u32) };
        if written == 0 {
            return Err(PathIssueKind::IoError);
        }
        if written as usize > out.len() {
            out.resize(written as usize, 0);
            written = unsafe {
                ExpandEnvironmentStringsW(buf.as_ptr(), out.as_mut_ptr(), out.len() as u32)
            };
            if written == 0 {
                return Err(PathIssueKind::IoError);
            }
        }
        let len = written.saturating_sub(1) as usize;
        Ok(OsString::from_wide(&out[..len]))
    })
}

fn probe_path(wide: &[u16]) -> Result<u32, PathIssueKind> {
    let attr = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attr == INVALID_FILE_ATTRIBUTES {
        let code = unsafe { GetLastError() };
        return Err(map_error_code(code));
    }
    Ok(attr)
}

pub(crate) fn open_path_with_policy(
    path: &Path,
    policy: &PathPolicy,
) -> Result<OwnedHandle, PathIssueKind> {
    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(path.as_os_str().encode_wide());
        ensure_long_prefix(&mut buf);
        buf.push(0);

        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
        if !policy.allow_reparse {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }

        let handle = unsafe {
            CreateFileW(
                buf.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                flags,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let code = unsafe { GetLastError() };
            return Err(map_error_code(code));
        }

        let owned = unsafe { OwnedHandle::from_raw_handle(handle as *mut _) };
        Ok(owned)
    })
}

pub(crate) fn is_reparse_point(attr: u32) -> bool {
    (attr & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

pub(crate) fn is_directory(attr: u32) -> bool {
    (attr & FILE_ATTRIBUTE_DIRECTORY) != 0
}

fn map_error_code(code: u32) -> PathIssueKind {
    match code {
        2 | 3 => PathIssueKind::NotFound,
        5 => PathIssueKind::AccessDenied,
        32 => PathIssueKind::SharingViolation,
        53 => PathIssueKind::NetworkPathNotFound,
        206 => PathIssueKind::TooLong,
        1921 => PathIssueKind::SymlinkLoop,
        _ => PathIssueKind::IoError,
    }
}

fn ensure_long_prefix(wide: &mut Vec<u16>) {
    const SLASH: u16 = b'\\' as u16;
    const QMARK: u16 = b'?' as u16;
    const DOT: u16 = b'.' as u16;

    if wide.len() >= 4 && wide[0] == SLASH && wide[1] == SLASH && wide[2] == QMARK && wide[3] == SLASH
    {
        return;
    }
    if wide.len() >= 4 && wide[0] == SLASH && wide[1] == SLASH && wide[2] == DOT && wide[3] == SLASH
    {
        return;
    }
    if wide.len() >= 2 && wide[0] == SLASH && wide[1] == SLASH {
        let prefix = [
            SLASH,
            SLASH,
            QMARK,
            SLASH,
            b'U' as u16,
            b'N' as u16,
            b'C' as u16,
            SLASH,
        ];
        wide.drain(0..2);
        wide.reserve(prefix.len());
        wide.splice(0..0, prefix);
        return;
    }

    let prefix = [SLASH, SLASH, QMARK, SLASH];
    wide.reserve(prefix.len());
    wide.splice(0..0, prefix);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_existing_and_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("probe.txt");
        std::fs::write(&file, "ok").expect("write");

        let attr = probe(&file).expect("probe ok");
        assert!(!is_directory(attr));

        let missing = dir.path().join("missing.txt");
        let err = probe(&missing).unwrap_err();
        assert_eq!(err, PathIssueKind::NotFound);
    }

    #[test]
    fn to_wide_with_prefix_adds_prefix() {
        let wide = to_wide_with_prefix(Path::new(r"C:\Windows"));
        assert!(wide.len() >= 5);
        assert_eq!(wide[0], b'\\' as u16);
        assert_eq!(wide[1], b'\\' as u16);
        assert_eq!(wide[2], b'?' as u16);
        assert_eq!(wide[3], b'\\' as u16);
        assert_eq!(*wide.last().unwrap(), 0);
    }

    #[test]
    fn to_wide_with_prefix_handles_unc() {
        let wide = to_wide_with_prefix(Path::new(r"\\server\share"));
        let prefix = [
            b'\\' as u16,
            b'\\' as u16,
            b'?' as u16,
            b'\\' as u16,
            b'U' as u16,
            b'N' as u16,
            b'C' as u16,
            b'\\' as u16,
        ];
        assert!(wide.starts_with(&prefix));
    }

    #[test]
    fn attr_helpers_work() {
        let attr = FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY;
        assert!(is_reparse_point(attr));
        assert!(is_directory(attr));
    }
}
