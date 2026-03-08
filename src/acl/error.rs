use std::path::PathBuf;
use thiserror::Error;

/// All errors produced by the `acl` layer.
///
/// `commands/` layer wraps these via `anyhow::Error` for rich context chains.
#[derive(Debug, Error)]
pub enum AclError {
    // ── Windows API errors ───────────────────────────────────────────────────
    #[error("Win32 error {code:#010x}: {msg}")]
    Win32 { code: u32, msg: String },

    #[error("path not found: {0}")]
    #[allow(dead_code)]
    PathNotFound(PathBuf),

    #[error("permission denied: {0}")]
    #[allow(dead_code)]
    PermissionDenied(PathBuf),

    #[error("invalid principal '{0}' (cannot resolve to a SID)")]
    InvalidPrincipal(String),

    #[error("access denied — ACL may be corrupt; use `xun acl repair` to unlock")]
    AccessDenied,

    // ── Data / serialization errors ──────────────────────────────────────────
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // ── Logic errors ─────────────────────────────────────────────────────────
    #[error("backup version mismatch: expected v{expected}, got v{found}")]
    BackupVersionMismatch { expected: u32, found: u32 },

    #[error("no explicit ACE entries to operate on")]
    #[allow(dead_code)]
    NoExplicitEntries,

    #[error("path is protected and cannot be modified: {0}")]
    #[allow(dead_code)]
    ProtectedPath(PathBuf),

    #[error("{0}")]
    #[allow(dead_code)]
    Other(String),
}

impl AclError {
    /// Returns `true` when this error represents a Win32 `ERROR_ACCESS_DENIED`
    /// (0x00000005) or closely related denial code.
    pub fn is_access_denied(&self) -> bool {
        match self {
            AclError::AccessDenied => true,
            AclError::PermissionDenied(_) => true,
            AclError::Win32 { code, .. } => matches!(*code, 0x5 | 0x522 | 0x546),
            _ => false,
        }
    }

    pub fn from_win32(code: u32) -> Self {
        if matches!(code, 0x5 | 0x522 | 0x546) {
            AclError::AccessDenied
        } else {
            AclError::Win32 {
                code,
                msg: win32_error_message(code),
            }
        }
    }

    /// Build an `AclError::Win32` from the value returned by `GetLastError()`.
    ///
    /// # Safety
    /// Must be called immediately after a failing Win32 call while the thread
    /// error value is still set.
    pub fn last_win32() -> Self {
        use windows::Win32::Foundation::GetLastError;
        let code = unsafe { GetLastError().0 };
        let msg = win32_error_message(code);
        if code == 5 || code == 0x522 {
            AclError::AccessDenied
        } else {
            AclError::Win32 { code, msg }
        }
    }
}

/// Format a Win32 error code as a human-readable message using
/// `FormatMessageW`.
pub fn win32_error_message(code: u32) -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::System::Diagnostics::Debug::{
        FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
        FormatMessageW,
    };
    use windows::core::PWSTR;

    unsafe {
        let mut buf: *mut u16 = std::ptr::null_mut();
        let len = FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_IGNORE_INSERTS,
            None,
            code,
            0,
            // Cast: FormatMessageW with ALLOCATE_BUFFER expects *mut PWSTR cast as PWSTR
            PWSTR(&mut buf as *mut *mut u16 as *mut u16),
            0,
            None,
        );
        if len == 0 || buf.is_null() {
            return format!("unknown error {code:#010x}");
        }
        let slice = std::slice::from_raw_parts(buf, len as usize);
        let msg = OsString::from_wide(slice)
            .to_string_lossy()
            .trim_end_matches(|c: char| c.is_whitespace())
            .to_string();
        LocalFree(HLOCAL(buf as *mut _));
        msg
    }
}
