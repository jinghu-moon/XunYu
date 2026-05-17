use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};

// Returns Ok(()) or the OS error code returned by GetLastError()
pub(crate) fn schedule_delete_on_reboot(path: &Path) -> Result<(), u32> {
    if is_unc_path(path) {
        return Err(windows_sys::Win32::Foundation::ERROR_NOT_SUPPORTED);
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let res = unsafe {
        MoveFileExW(
            wide_path.as_ptr(),
            std::ptr::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        )
    };

    if res != 0 {
        Ok(())
    } else {
        Err(unsafe { GetLastError() })
    }
}

// Check if it is a network path
fn is_unc_path(path: &Path) -> bool {
    let p = path.to_string_lossy();
    p.starts_with(r"\\") || p.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_delete_on_reboot_rejects_unc_paths() {
        let p = Path::new(r"\\server\\share\\file.txt");
        assert_eq!(
            schedule_delete_on_reboot(p),
            Err(windows_sys::Win32::Foundation::ERROR_NOT_SUPPORTED)
        );

        let p2 = Path::new(r"//server/share/file.txt");
        assert_eq!(
            schedule_delete_on_reboot(p2),
            Err(windows_sys::Win32::Foundation::ERROR_NOT_SUPPORTED)
        );
    }

    #[test]
    fn is_unc_path_detects_both_slash_styles() {
        assert!(is_unc_path(Path::new(r"\\server\\share\\x")));
        assert!(is_unc_path(Path::new(r"//server/share/x")));
        assert!(!is_unc_path(Path::new(r"C:\\tmp\\x")));
    }
}
