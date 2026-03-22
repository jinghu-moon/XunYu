use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileCopyBackend {
    Std,
    CopyFile2,
}

pub(crate) fn detect_copy_backend_for_backup() -> FileCopyBackend {
    detect_copy_backend_with(FileCopyBackend::Std, |name| std::env::var_os(name))
}

pub(crate) fn detect_copy_backend_for_restore() -> FileCopyBackend {
    detect_copy_backend_with(FileCopyBackend::Std, |name| std::env::var_os(name))
}

fn detect_copy_backend_with<F>(default_backend: FileCopyBackend, mut get_env: F) -> FileCopyBackend
where
    F: FnMut(&str) -> Option<OsString>,
{
    match get_env("XUN_COPY_BACKEND")
        .and_then(|value| value.into_string().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("copyfile2") | Some("copy2") => FileCopyBackend::CopyFile2,
        Some("std") | Some("fs") => FileCopyBackend::Std,
        _ => default_backend,
    }
}

pub(crate) fn copy_file(src: &Path, dst: &Path, backend: FileCopyBackend) -> std::io::Result<u64> {
    match backend {
        FileCopyBackend::Std => std::fs::copy(src, dst),
        FileCopyBackend::CopyFile2 => copy_file_copyfile2(src, dst),
    }
}

#[cfg(windows)]
fn copy_file_copyfile2(src: &Path, dst: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{COPYFILE2_EXTENDED_PARAMETERS, CopyFile2};

    let metadata = std::fs::metadata(src)?;
    let mut src_w: Vec<u16> = src.as_os_str().encode_wide().collect();
    src_w.push(0);
    let mut dst_w: Vec<u16> = dst.as_os_str().encode_wide().collect();
    dst_w.push(0);

    let params = COPYFILE2_EXTENDED_PARAMETERS {
        dwSize: std::mem::size_of::<COPYFILE2_EXTENDED_PARAMETERS>() as u32,
        dwCopyFlags: 0,
        pfCancel: std::ptr::null_mut(),
        pProgressRoutine: None,
        pvCallbackContext: std::ptr::null_mut(),
    };

    let hr = unsafe { CopyFile2(src_w.as_ptr(), dst_w.as_ptr(), &params) };
    if hr < 0 {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("CopyFile2 failed: HRESULT=0x{hr:08x}"),
        ))
    } else {
        Ok(metadata.len())
    }
}

#[cfg(not(windows))]
fn copy_file_copyfile2(src: &Path, dst: &Path) -> std::io::Result<u64> {
    std::fs::copy(src, dst)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;

    use super::{FileCopyBackend, detect_copy_backend_with};

    #[test]
    fn detect_copy_backend_uses_supplied_default_when_env_missing() {
        let env = HashMap::<&str, OsString>::new();
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::Std, |name| env.get(name).cloned()),
            FileCopyBackend::Std
        );
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::CopyFile2, |name| env.get(name).cloned()),
            FileCopyBackend::CopyFile2
        );
    }

    #[test]
    fn detect_copy_backend_accepts_copyfile2_aliases() {
        let env = HashMap::from([("XUN_COPY_BACKEND", OsString::from("copyfile2"))]);
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::Std, |name| env.get(name).cloned()),
            FileCopyBackend::CopyFile2
        );

        let env = HashMap::from([("XUN_COPY_BACKEND", OsString::from("copy2"))]);
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::Std, |name| env.get(name).cloned()),
            FileCopyBackend::CopyFile2
        );
    }

    #[test]
    fn detect_copy_backend_accepts_explicit_std_override() {
        let env = HashMap::from([("XUN_COPY_BACKEND", OsString::from("std"))]);
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::CopyFile2, |name| env.get(name).cloned()),
            FileCopyBackend::Std
        );
    }

    #[test]
    fn detect_copy_backend_default_policy_can_be_kept_std() {
        let env = HashMap::<&str, OsString>::new();
        assert_eq!(
            detect_copy_backend_with(FileCopyBackend::Std, |name| env.get(name).cloned()),
            FileCopyBackend::Std
        );
    }
}
