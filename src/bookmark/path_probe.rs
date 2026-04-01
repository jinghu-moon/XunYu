use std::path::{Component, Path};
use std::sync::mpsc;
use std::time::Duration;

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookmarkPathStatus {
    Existing,
    Missing,
    Unknown,
}

#[cfg(windows)]
const DRIVE_FIXED_TYPE: u32 = 3;
#[cfg(windows)]
const DRIVE_REMOTE_TYPE: u32 = 4;
#[cfg(windows)]
const DRIVE_RAMDISK_TYPE: u32 = 6;

pub(crate) fn path_status(path: &Path) -> BookmarkPathStatus {
    if !should_probe_path_exists(path) {
        return BookmarkPathStatus::Unknown;
    }
    if is_network_like(path) {
        return probe_with_timeout(path, network_probe_timeout());
    }
    if path.exists() {
        BookmarkPathStatus::Existing
    } else {
        BookmarkPathStatus::Missing
    }
}

pub(crate) fn should_probe_path_exists(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    #[cfg(windows)]
    {
        if is_unc(path) {
            return true;
        }
        if !path.is_absolute() {
            return true;
        }
        let Some(root) = drive_root(path) else {
            return false;
        };
        let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
        return matches!(
            unsafe { GetDriveTypeW(wide.as_ptr()) },
            DRIVE_FIXED_TYPE | DRIVE_REMOTE_TYPE | DRIVE_RAMDISK_TYPE
        );
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn probe_with_timeout(path: &Path, timeout: Duration) -> BookmarkPathStatus {
    let owned = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(owned.exists());
    });
    match rx.recv_timeout(timeout) {
        Ok(true) => BookmarkPathStatus::Existing,
        Ok(false) => BookmarkPathStatus::Missing,
        Err(_) => BookmarkPathStatus::Unknown,
    }
}

fn network_probe_timeout() -> Duration {
    let ms = std::env::var("_BM_NETWORK_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300);
    Duration::from_millis(ms)
}

#[cfg(windows)]
fn is_network_like(path: &Path) -> bool {
    if is_unc(path) {
        return true;
    }
    let Some(root) = drive_root(path) else {
        return false;
    };
    let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
    (unsafe { GetDriveTypeW(wide.as_ptr()) }) == DRIVE_REMOTE_TYPE
}

#[cfg(not(windows))]
fn is_network_like(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn is_unc(path: &Path) -> bool {
    let raw = path.to_string_lossy();
    raw.starts_with(r"\\") || raw.starts_with("//")
}

#[cfg(not(windows))]
fn is_unc(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn drive_root(path: &Path) -> Option<OsString> {
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Prefix(prefix)), Some(Component::RootDir)) => {
            let mut root = prefix.as_os_str().to_os_string();
            root.push("\\");
            Some(root)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn local_missing_path_is_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing");
        assert_eq!(path_status(&missing), BookmarkPathStatus::Missing);
    }

    #[test]
    fn unc_path_is_network_like_unknown_without_fast_probe() {
        let unc = Path::new(r"\\server\share\folder");
        assert!(should_probe_path_exists(unc));
    }
}
