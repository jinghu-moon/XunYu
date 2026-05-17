use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::UI::Shell::{
    FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
    SHFileOperationW,
};

pub(crate) fn trash_file(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(Some("trash_missing".to_string()));
    }

    // SHFileOperationW expects a double-null terminated list of paths.
    let mut wide: Vec<u16> = OsStr::new(&path.to_string_lossy().to_string())
        .encode_wide()
        .collect();
    wide.push(0);
    wide.push(0);

    let mut op = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE,
        pFrom: wide.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };

    let rc = unsafe { SHFileOperationW(&mut op as *mut _) };
    if rc == 0 && op.fAnyOperationsAborted == 0 {
        return Ok(None);
    }
    let aborted = op.fAnyOperationsAborted != 0;

    // Fallback: if recycle bin failed (e.g., policy), attempt direct delete.
    match std::fs::remove_file(path) {
        Ok(_) => Ok(Some(format!(
            "trash_failed_fallback_deleted:rc={rc}:aborted={aborted}"
        ))),
        Err(e) => Err(format!("trash_failed:rc={rc}:aborted={aborted}:{e}")),
    }
}
