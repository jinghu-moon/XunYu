use super::*;
use std::io as stdio;

pub(super) fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }
    if !path.exists() {
        fs::write(path, bytes)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
        return Ok(());
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;
    replace_file(&tmp, path)
        .with_context(|| format!("Failed to replace file: {}", path.display()))?;
    Ok(())
}

pub(super) fn files_equal(path_a: &Path, bytes_b: &[u8]) -> bool {
    let Ok(meta_a) = fs::metadata(path_a) else {
        return false;
    };
    if meta_a.len() != bytes_b.len() as u64 {
        return false;
    }
    let Ok(bytes_a) = fs::read(path_a) else {
        return false;
    };
    bytes_a == bytes_b
}

pub(super) fn files_equal_path(path_a: &Path, path_b: &Path) -> bool {
    let Ok(meta_a) = fs::metadata(path_a) else {
        return false;
    };
    let Ok(meta_b) = fs::metadata(path_b) else {
        return false;
    };
    if meta_a.len() != meta_b.len() {
        return false;
    }
    let Ok(bytes_a) = fs::read(path_a) else {
        return false;
    };
    let Ok(bytes_b) = fs::read(path_b) else {
        return false;
    };
    bytes_a == bytes_b
}

fn replace_file(from: &Path, to: &Path) -> stdio::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let mut from_w: Vec<u16> = from.as_os_str().encode_wide().collect();
    from_w.push(0);
    let mut to_w: Vec<u16> = to.as_os_str().encode_wide().collect();
    to_w.push(0);

    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(stdio::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> stdio::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
pub(super) fn link_template(src: &Path, dst: &Path) -> Result<bool> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::CreateHardLinkW;

    let mut src_w: Vec<u16> = src.as_os_str().encode_wide().collect();
    src_w.push(0);
    let mut dst_w: Vec<u16> = dst.as_os_str().encode_wide().collect();
    dst_w.push(0);
    let ok = unsafe { CreateHardLinkW(dst_w.as_ptr(), src_w.as_ptr(), std::ptr::null()) };
    Ok(ok != 0)
}

#[cfg(not(windows))]
pub(super) fn link_template(_src: &Path, _dst: &Path) -> Result<bool> {
    Ok(false)
}
