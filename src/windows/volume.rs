use std::collections::HashMap;
use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path};
use std::sync::RwLock;

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;

lazy_static::lazy_static! {
    static ref VOLUME_CAPABILITY_CACHE: RwLock<HashMap<String, VolumeInfo>> = RwLock::new(HashMap::new());
}

#[derive(Clone, Copy)]
pub(crate) struct VolumeInfo {
    pub supports_efs: bool,
}

pub(crate) fn is_volume_efs_capable(path: &Path) -> Result<bool, u32> {
    Ok(get_volume_info(path)?.supports_efs)
}

fn get_volume_info(path: &Path) -> Result<VolumeInfo, u32> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Find the prefix component
    let mut root_path = String::new();
    for comp in canonical.components() {
        if let Component::Prefix(p) = comp {
            root_path = p.as_os_str().to_string_lossy().into_owned();
            root_path.push('\\'); // Volume root must end with backslash
            break;
        }
    }

    if root_path.is_empty() {
        return Err(windows_sys::Win32::Foundation::ERROR_INVALID_NAME);
    }

    let root_key = root_path.to_lowercase();

    // Check cache
    if let Ok(cache) = VOLUME_CAPABILITY_CACHE.read()
        && let Some(info) = cache.get(&root_key)
    {
        return Ok(*info);
    }

    // Call GetVolumeInformationW
    let mut wide_root: Vec<u16> = std::ffi::OsString::from(&root_path).encode_wide().collect();
    wide_root.push(0);

    let mut flags: u32 = 0;

    let res = unsafe {
        GetVolumeInformationW(
            wide_root.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut flags,
            std::ptr::null_mut(),
            0,
        )
    };

    if res == 0 {
        return Err(unsafe { GetLastError() });
    }

    let info = VolumeInfo {
        supports_efs: (flags & 0x00200000) != 0, // FILE_SUPPORTS_ENCRYPTION
    };

    if let Ok(mut cache) = VOLUME_CAPABILITY_CACHE.write() {
        cache.insert(root_key, info);
    }

    Ok(info)
}
