use std::ptr;

use super::guards::HandleGuard;
use super::utils::path_to_unc_wide;
use super::{
    CreateFileW, DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, DWORD, DuplicateHandle,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ,
    GetCurrentProcess, GetCurrentProcessId, GetFinalPathNameByHandleW, HANDLE,
    INVALID_HANDLE_VALUE, OpenProcess, PROCESS_DUP_HANDLE, SysHandleEntry,
};

pub(crate) fn force_close_external_handles(path: &str, snapshot: &[SysHandleEntry]) -> i32 {
    let canonical = match get_canonical_path(path) {
        Some(c) => c,
        None => return 0,
    };

    let my_pid = unsafe { GetCurrentProcessId() };
    let mut closed = 0i32;

    for entry in snapshot {
        if entry.owner_pid == my_pid as u16 {
            continue;
        }

        let h_proc = unsafe { OpenProcess(PROCESS_DUP_HANDLE, 0, entry.owner_pid as u32) };
        if h_proc == 0 {
            continue;
        }
        let _pg = HandleGuard(h_proc);

        unsafe {
            let mut dup: HANDLE = 0;
            if DuplicateHandle(
                h_proc,
                entry.handle_value as isize,
                GetCurrentProcess(),
                &mut dup,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            ) == 0
            {
                continue;
            }
            let _dg = HandleGuard(dup);

            if let Some(p) = path_from_handle(dup)
                && p.eq_ignore_ascii_case(&canonical)
            {
                let mut dummy: HANDLE = 0;
                if DuplicateHandle(
                    h_proc,
                    entry.handle_value as isize,
                    0,
                    &mut dummy,
                    0,
                    0,
                    DUPLICATE_CLOSE_SOURCE,
                ) != 0
                {
                    closed += 1;
                }
            }
        }
    }
    closed
}

fn get_canonical_path(path: &str) -> Option<String> {
    let w = path_to_unc_wide(path);
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            super::OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            0,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    let _guard = HandleGuard(h);
    path_from_handle(h)
}

fn path_from_handle(h: HANDLE) -> Option<String> {
    let mut buf = vec![0u16; 512];
    let len = unsafe { GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), buf.len() as DWORD, 0) };
    if len == 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..len as usize]);
    Some(match s.strip_prefix("\\\\?\\") {
        Some(stripped) => stripped.to_string(),
        None => s,
    })
}
