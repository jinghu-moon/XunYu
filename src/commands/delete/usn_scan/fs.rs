use super::common::{Bool, Dword, to_wide};

pub(super) fn is_ntfs(root: &str) -> bool {
    let drive = root.trim_end_matches(['\\', '/']);
    let root_path = format!("{}\\", drive);
    let wide = to_wide(&root_path);

    let mut fs_name = [0u16; 32];

    unsafe extern "system" {
        fn GetVolumeInformationW(
            root: *const u16,
            vol_name: *mut u16,
            vol_len: Dword,
            serial: *mut Dword,
            max_comp: *mut Dword,
            flags: *mut Dword,
            fs_name: *mut u16,
            fs_len: Dword,
        ) -> Bool;
    }

    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs_name.as_mut_ptr(),
            fs_name.len() as Dword,
        )
    };
    if ok == 0 {
        return false;
    }

    let name = String::from_utf16_lossy(&fs_name);
    name.starts_with("NTFS")
}
