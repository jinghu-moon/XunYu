use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{
    BOOL, GetLastError, HMODULE, INVALID_HANDLE_VALUE, NTSTATUS, STATUS_ACCESS_DENIED,
    STATUS_BAD_NETWORK_PATH, STATUS_INVALID_PARAMETER, STATUS_NAME_TOO_LONG,
    STATUS_NOT_SUPPORTED, STATUS_OBJECT_NAME_NOT_FOUND, STATUS_OBJECT_PATH_NOT_FOUND,
    STATUS_SHARING_VIOLATION, UNICODE_STRING, ERROR_NO_MORE_FILES,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FindClose, FindFirstFileExW, FindNextFileW, GetFileAttributesW,
    GetFileInformationByHandleEx, GetFinalPathNameByHandleW, GetFullPathNameW, FileAttributeTagInfo,
    FindExInfoBasic, FindExSearchNameMatch, WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_TAG_INFO, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, INVALID_FILE_ATTRIBUTES, OPEN_EXISTING, VOLUME_NAME_DOS,
};
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{FILE_BASIC_INFORMATION, FILE_STAT_INFORMATION};

use crate::path_guard::policy::{PathIssueKind, PathPolicy};

thread_local! {
    static WIDE_BUF: RefCell<Vec<u16>> = RefCell::new(Vec::with_capacity(512));
}

const OBJ_CASE_INSENSITIVE: u32 = 0x40;
const FILE_STAT_BY_NAME_INFO: u32 = 0;

type GetFileInformationByNameFn =
    unsafe extern "system" fn(*const u16, u32, *mut std::ffi::c_void, u32) -> BOOL;
type NtQueryAttributesFileFn =
    unsafe extern "system" fn(*const OBJECT_ATTRIBUTES, *mut FILE_BASIC_INFORMATION) -> NTSTATUS;

static GET_FILE_INFO_BY_NAME: OnceLock<Option<GetFileInformationByNameFn>> = OnceLock::new();
static NT_QUERY_ATTRIBUTES: OnceLock<Option<NtQueryAttributesFileFn>> = OnceLock::new();

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
    if let Some(result) = probe_by_name(wide) {
        return result;
    }
    if let Some(result) = probe_by_nt(wide) {
        return result;
    }
    probe_by_attributes(wide)
}

fn probe_by_name(wide: &[u16]) -> Option<Result<u32, PathIssueKind>> {
    let func = *GET_FILE_INFO_BY_NAME.get_or_init(load_get_file_info_by_name);
    let func = func?;
    let mut info: FILE_STAT_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        func(
            wide.as_ptr(),
            FILE_STAT_BY_NAME_INFO,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_STAT_INFORMATION>() as u32,
        )
    };
    if ok != 0 {
        return Some(Ok(info.FileAttributes));
    }
    let code = unsafe { GetLastError() };
    if should_fallback_error(code) {
        return None;
    }
    Some(Err(map_error_code(code)))
}

fn probe_by_nt(wide: &[u16]) -> Option<Result<u32, PathIssueKind>> {
    let func = *NT_QUERY_ATTRIBUTES.get_or_init(load_nt_query_attributes_file);
    let func = func?;
    let nt_path = build_nt_path(wide);
    let name = UNICODE_STRING {
        Length: (nt_path.len() * 2) as u16,
        MaximumLength: (nt_path.len() * 2) as u16,
        Buffer: nt_path.as_ptr() as *mut _,
    };
    let attrs = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: std::ptr::null_mut(),
        ObjectName: &name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut info: FILE_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let status = unsafe { func(&attrs, &mut info) };
    if nt_status_is_success(status) {
        return Some(Ok(info.FileAttributes));
    }
    if should_fallback_status(status) {
        return None;
    }
    Some(Err(map_nt_status(status)))
}

fn probe_by_attributes(wide: &[u16]) -> Result<u32, PathIssueKind> {
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

pub(crate) fn get_attribute_tag_info(
    handle: &OwnedHandle,
) -> Result<FILE_ATTRIBUTE_TAG_INFO, PathIssueKind> {
    let mut info = FILE_ATTRIBUTE_TAG_INFO {
        FileAttributes: 0,
        ReparseTag: 0,
    };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle() as *mut _,
            FileAttributeTagInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    if ok == 0 {
        let code = unsafe { GetLastError() };
        return Err(map_error_code(code));
    }
    Ok(info)
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

fn map_nt_status(status: NTSTATUS) -> PathIssueKind {
    match status {
        STATUS_OBJECT_NAME_NOT_FOUND => PathIssueKind::NotFound,
        STATUS_OBJECT_PATH_NOT_FOUND => PathIssueKind::NotFound,
        STATUS_ACCESS_DENIED => PathIssueKind::AccessDenied,
        STATUS_SHARING_VIOLATION => PathIssueKind::SharingViolation,
        STATUS_BAD_NETWORK_PATH => PathIssueKind::NetworkPathNotFound,
        STATUS_NAME_TOO_LONG => PathIssueKind::TooLong,
        _ => PathIssueKind::IoError,
    }
}

fn should_fallback_error(code: u32) -> bool {
    matches!(code, 1 | 50 | 87 | 120)
}

fn should_fallback_status(status: NTSTATUS) -> bool {
    matches!(status, STATUS_NOT_SUPPORTED | STATUS_INVALID_PARAMETER)
}

fn nt_status_is_success(status: NTSTATUS) -> bool {
    status >= 0
}

fn load_get_file_info_by_name() -> Option<GetFileInformationByNameFn> {
    load_proc(&["kernel32.dll", "kernelbase.dll"], b"GetFileInformationByName\0")
        .map(|ptr| unsafe { std::mem::transmute(ptr) })
}

fn load_nt_query_attributes_file() -> Option<NtQueryAttributesFileFn> {
    load_proc(&["ntdll.dll"], b"NtQueryAttributesFile\0")
        .map(|ptr| unsafe { std::mem::transmute(ptr) })
}

fn load_proc(modules: &[&str], proc: &[u8]) -> Option<unsafe extern "system" fn() -> isize> {
    for module in modules {
        if let Some(handle) = module_handle(module) {
            let addr = unsafe { GetProcAddress(handle, proc.as_ptr() as *const _) };
            if let Some(addr) = addr {
                return Some(addr);
            }
        }
    }
    None
}

fn module_handle(name: &str) -> Option<HMODULE> {
    let wide = to_wide_null(name);
    let handle = unsafe { GetModuleHandleW(wide.as_ptr()) };
    if !handle.is_null() {
        return Some(handle);
    }
    let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
    if !handle.is_null() {
        return Some(handle);
    }
    None
}

fn to_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn build_nt_path(wide: &[u16]) -> Vec<u16> {
    const SLASH: u16 = b'\\' as u16;
    const QMARK: u16 = b'?' as u16;
    const DOT: u16 = b'.' as u16;
    const U: u16 = b'U' as u16;
    const N: u16 = b'N' as u16;
    const C: u16 = b'C' as u16;

    let slice = if wide.last() == Some(&0) {
        &wide[..wide.len().saturating_sub(1)]
    } else {
        wide
    };

    if slice.starts_with(&[SLASH, SLASH, QMARK, SLASH, U, N, C, SLASH]) {
        let mut out = Vec::with_capacity(slice.len());
        out.extend_from_slice(&[SLASH, QMARK, QMARK, SLASH, U, N, C, SLASH]);
        out.extend_from_slice(&slice[8..]);
        return out;
    }
    if slice.starts_with(&[SLASH, SLASH, QMARK, SLASH]) || slice.starts_with(&[SLASH, SLASH, DOT, SLASH]) {
        let mut out = Vec::with_capacity(slice.len());
        out.extend_from_slice(&[SLASH, QMARK, QMARK, SLASH]);
        out.extend_from_slice(&slice[4..]);
        return out;
    }

    let mut out = Vec::with_capacity(slice.len() + 4);
    out.extend_from_slice(&[SLASH, QMARK, QMARK, SLASH]);
    out.extend_from_slice(slice);
    out
}

pub(crate) fn probe_dir_entries<F>(dir: &Path, mut on_entry: F) -> Result<(), PathIssueKind>
where
    F: FnMut(&[u16], u32),
{
    const SLASH: u16 = b'\\' as u16;
    const STAR: u16 = b'*' as u16;

    WIDE_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend(dir.as_os_str().encode_wide());
        ensure_long_prefix(&mut buf);
        if !buf.ends_with(&[SLASH]) {
            buf.push(SLASH);
        }
        buf.push(STAR);
        buf.push(0);

        let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
        let handle = unsafe {
            FindFirstFileExW(
                buf.as_ptr(),
                FindExInfoBasic,
                &mut data as *mut _ as *mut _,
                FindExSearchNameMatch,
                std::ptr::null_mut(),
                0,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let code = unsafe { GetLastError() };
            return Err(map_error_code(code));
        }

        loop {
            let len = entry_name_len(&data.cFileName);
            if len > 0 && !is_dot_entry(&data.cFileName, len) {
                on_entry(&data.cFileName[..len], data.dwFileAttributes);
            }
            let ok = unsafe { FindNextFileW(handle, &mut data) };
            if ok == 0 {
                let code = unsafe { GetLastError() };
                if code == ERROR_NO_MORE_FILES {
                    break;
                }
                unsafe { FindClose(handle) };
                return Err(map_error_code(code));
            }
        }

        unsafe { FindClose(handle) };
        Ok(())
    })
}

fn entry_name_len(name: &[u16]) -> usize {
    name.iter().position(|&value| value == 0).unwrap_or(name.len())
}

fn is_dot_entry(name: &[u16], len: usize) -> bool {
    match len {
        1 => name[0] == b'.' as u16,
        2 => name[0] == b'.' as u16 && name[1] == b'.' as u16,
        _ => false,
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
        let old_len = wide.len().saturating_sub(2);
        wide.copy_within(2.., 0);
        wide.truncate(old_len);
        let old_len = wide.len();
        wide.resize(old_len + prefix.len(), 0);
        wide.copy_within(..old_len, prefix.len());
        wide[..prefix.len()].copy_from_slice(&prefix);
        return;
    }

    let prefix = [SLASH, SLASH, QMARK, SLASH];
    let old_len = wide.len();
    wide.resize(old_len + prefix.len(), 0);
    wide.copy_within(..old_len, prefix.len());
    wide[..prefix.len()].copy_from_slice(&prefix);
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
    fn attr_helpers_work() {
        let attr = FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY;
        assert!(is_reparse_point(attr));
        assert!(is_directory(attr));
    }

    #[test]
    fn ensure_long_prefix_adds_device_prefix() {
        let mut wide: Vec<u16> = r"C:\Windows".encode_utf16().collect();
        ensure_long_prefix(&mut wide);
        assert!(wide.len() >= 4);
        assert_eq!(wide[0], b'\\' as u16);
        assert_eq!(wide[1], b'\\' as u16);
        assert_eq!(wide[2], b'?' as u16);
        assert_eq!(wide[3], b'\\' as u16);
    }

    #[test]
    fn ensure_long_prefix_adds_unc_prefix() {
        let mut wide: Vec<u16> = r"\\server\share".encode_utf16().collect();
        ensure_long_prefix(&mut wide);
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
}
