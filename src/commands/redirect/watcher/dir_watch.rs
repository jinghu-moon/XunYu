use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use windows_sys::Win32::Foundation::{
    BOOL, CloseHandle, ERROR_NOTIFY_ENUM_DIR, ERROR_OPERATION_ABORTED, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    READ_DIRECTORY_NOTIFY_INFORMATION_CLASS, ReadDirectoryChangesW,
    ReadDirectoryNotifyExtendedInformation,
};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};

use super::super::path_utils::is_network_share_path;
use super::super::watch_core;

type ReadDirChangesExWFn = unsafe extern "system" fn(
    HANDLE,
    *mut core::ffi::c_void,
    u32,
    BOOL,
    FILE_NOTIFY_CHANGE,
    *mut u32,
    *mut OVERLAPPED,
    isize,
    READ_DIRECTORY_NOTIFY_INFORMATION_CLASS,
) -> BOOL;

pub(super) enum WatchSignal {
    Overflow,
    Paths(Vec<PathBuf>),
}

pub(super) struct DirectoryWatcher {
    dir_handle: HANDLE,
    buffer_len: u32,
    use_ex: Option<ReadDirChangesExWFn>,
    source: PathBuf,
}

unsafe impl Send for DirectoryWatcher {}

impl DirectoryWatcher {
    pub(super) fn new(source: &Path, buffer_len: u32) -> io::Result<Self> {
        const MAX_NETWORK_SHARE_BUFFER_LEN: u32 = 64 * 1024;
        if is_network_share_path(source) && buffer_len > MAX_NETWORK_SHARE_BUFFER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "network share requires buffer_len <= {MAX_NETWORK_SHARE_BUFFER_LEN}, got {buffer_len}"
                ),
            ));
        }
        let handle = open_directory_handle(source)?;
        let use_ex = resolve_read_dir_changes_exw();
        Ok(Self {
            dir_handle: handle,
            buffer_len,
            use_ex,
            source: source.to_path_buf(),
        })
    }

    pub(super) fn run(self, tx: mpsc::Sender<WatchSignal>) {
        let mut buf = vec![0u8; self.buffer_len as usize];

        loop {
            let h_event = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
            if h_event.is_null() {
                let _ = tx.send(WatchSignal::Overflow);
                break;
            }

            let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
            overlapped.hEvent = h_event;

            let notify = FILE_NOTIFY_CHANGE_FILE_NAME
                | FILE_NOTIFY_CHANGE_LAST_WRITE
                | FILE_NOTIFY_CHANGE_SIZE;

            let ok = unsafe {
                if let Some(f) = self.use_ex {
                    (f)(
                        self.dir_handle,
                        buf.as_mut_ptr() as *mut _,
                        self.buffer_len,
                        1,
                        notify,
                        std::ptr::null_mut(),
                        &mut overlapped as *mut _,
                        0,
                        ReadDirectoryNotifyExtendedInformation,
                    )
                } else {
                    ReadDirectoryChangesW(
                        self.dir_handle,
                        buf.as_mut_ptr() as *mut _,
                        self.buffer_len,
                        1,
                        notify,
                        std::ptr::null_mut(),
                        &mut overlapped as *mut _,
                        None,
                    )
                }
            };

            if ok == 0 {
                unsafe { CloseHandle(h_event) };
                let err = unsafe { GetLastError() };
                if err == ERROR_OPERATION_ABORTED {
                    break;
                }
                let _ = tx.send(WatchSignal::Overflow);
                continue;
            }

            let wait = unsafe { WaitForSingleObject(h_event, INFINITE) };
            let _ = wait;
            let mut transferred: u32 = 0;
            let got = unsafe {
                GetOverlappedResult(
                    self.dir_handle,
                    &mut overlapped as *mut _,
                    &mut transferred,
                    0,
                )
            };
            unsafe { CloseHandle(h_event) };

            if got == 0 {
                let err = unsafe { GetLastError() };
                if err == ERROR_NOTIFY_ENUM_DIR {
                    let _ = tx.send(WatchSignal::Overflow);
                    continue;
                }
                if err == ERROR_OPERATION_ABORTED {
                    break;
                }
                continue;
            }

            if transferred == 0 {
                let _ = tx.send(WatchSignal::Overflow);
                continue;
            }

            let rel_paths = if self.use_ex.is_some() {
                watch_core::parse_extended_events(&buf[..transferred as usize])
            } else {
                watch_core::parse_basic_events(&buf[..transferred as usize])
            };
            let mut paths: Vec<PathBuf> = rel_paths
                .into_iter()
                .map(|p| self.source.join(PathBuf::from(p)))
                .collect();
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                let _ = tx.send(WatchSignal::Paths(paths));
            }
        }
    }
}

impl Drop for DirectoryWatcher {
    fn drop(&mut self) {
        if !self.dir_handle.is_null() && self.dir_handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.dir_handle) };
        }
    }
}

fn open_directory_handle(path: &Path) -> io::Result<HANDLE> {
    let wide = to_wide_path(path);
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(h)
}

#[allow(clippy::manual_c_str_literals)]
fn resolve_read_dir_changes_exw() -> Option<ReadDirChangesExWFn> {
    unsafe {
        let mod_name = to_wide_str("kernel32.dll");
        let h = GetModuleHandleW(mod_name.as_ptr());
        if h.is_null() {
            return None;
        }
        let proc = GetProcAddress(h, b"ReadDirectoryChangesExW\0".as_ptr());
        proc.map(|p| std::mem::transmute(p))
    }
}

fn to_wide_str(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn to_wide_path(path: &Path) -> Vec<u16> {
    // Use extended-length paths to avoid MAX_PATH issues in watch mode.
    let raw = path.to_string_lossy().to_string();
    let ext = if raw.starts_with(r"\\?\") {
        raw
    } else if raw.starts_with(r"\\") {
        format!(r"\\?\UNC\{}", raw.trim_start_matches(r"\\"))
    } else if raw.len() >= 240 && raw.chars().nth(1) == Some(':') {
        format!(r"\\?\{}", raw)
    } else {
        raw
    };
    OsStr::new(&ext)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
