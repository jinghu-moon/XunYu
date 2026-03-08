use super::utils::path_to_unc_wide;
use super::{
    DWORD, DeleteFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, GetFileAttributesW,
    SetFileAttributesW,
};

pub(crate) fn get_last_error() -> u32 {
    unsafe extern "system" {
        fn GetLastError() -> DWORD;
    }
    unsafe { GetLastError() }
}

pub(crate) fn delete_file(path: &str) -> u32 {
    let w = path_to_unc_wide(path);
    let ok = unsafe { DeleteFileW(w.as_ptr()) };
    if ok != 0 { 0 } else { get_last_error() }
}

pub(crate) fn set_normal_attrs(path: &str) {
    let w = path_to_unc_wide(path);
    unsafe {
        SetFileAttributesW(w.as_ptr(), FILE_ATTRIBUTE_NORMAL);
    }
}

pub(crate) fn get_attrs(path: &str) -> u32 {
    let w = path_to_unc_wide(path);
    unsafe { GetFileAttributesW(w.as_ptr()) }
}

pub(crate) fn is_dir_attr(attrs: u32) -> bool {
    attrs != 0xFFFF_FFFF && (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0
}
