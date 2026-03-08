use std::mem;

use super::guards::HandleGuard;
use super::utils::path_to_unc_wide;
use super::{
    CreateFileW, DWORD, FILE_DISPOSITION_INFORMATION_CLASS, FILE_FLAG_DELETE_ON_CLOSE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileDispositionInfo, GENERIC_WRITE,
    INVALID_HANDLE_VALUE, IoStatusBlock, NtSetInformationFile, OPEN_EXISTING, PVOID,
};

pub(crate) fn mark_delete_on_close(path: &str) -> bool {
    let w = path_to_unc_wide(path);
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_DELETE_ON_CLOSE,
            0,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return false;
    }
    let _guard = HandleGuard(h);

    unsafe {
        let mut iosb = IoStatusBlock {
            status: 0,
            information: 0,
        };
        let mut info = FileDispositionInfo { delete_file: 1 };
        NtSetInformationFile(
            h,
            &mut iosb,
            &mut info as *mut _ as PVOID,
            mem::size_of::<FileDispositionInfo>() as DWORD,
            FILE_DISPOSITION_INFORMATION_CLASS,
        ) == 0
    }
}
