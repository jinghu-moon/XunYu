use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{ERROR_SHARING_VIOLATION, GetLastError};
use windows_sys::Win32::Storage::FileSystem::{DecryptFileW, EncryptFileW};

use crate::windows::safety::ensure_safe_target;

pub(crate) enum EfsError {
    SharingViolation,
    OsError(u32),
    SafetyRestricted(&'static str),
}

impl std::fmt::Display for EfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EfsError::SharingViolation => {
                write!(f, "File is locked by another process (Sharing Violation)")
            }
            EfsError::OsError(c) => write!(f, "OS Error: {}", c),
            EfsError::SafetyRestricted(msg) => write!(f, "Safety Restriction: {}", msg),
        }
    }
}

pub(crate) fn encrypt_file(path: &Path) -> Result<(), EfsError> {
    if let Err(e) = ensure_safe_target(path) {
        return Err(EfsError::SafetyRestricted(e));
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let res = unsafe { EncryptFileW(wide_path.as_ptr()) };
    if res == 0 {
        let err = unsafe { GetLastError() };
        if err == ERROR_SHARING_VIOLATION {
            return Err(EfsError::SharingViolation);
        }
        return Err(EfsError::OsError(err));
    }

    Ok(())
}

pub(crate) fn decrypt_file(path: &Path) -> Result<(), EfsError> {
    if let Err(e) = ensure_safe_target(path) {
        return Err(EfsError::SafetyRestricted(e));
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    // 0 means standard decryption
    let res = unsafe { DecryptFileW(wide_path.as_ptr(), 0) };
    if res == 0 {
        let err = unsafe { GetLastError() };
        if err == ERROR_SHARING_VIOLATION {
            return Err(EfsError::SharingViolation);
        }
        return Err(EfsError::OsError(err));
    }

    Ok(())
}
