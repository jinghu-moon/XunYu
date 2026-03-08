use super::time::{filetime_to_secs, wide_cstr_to_string};
use super::*;

pub(super) struct FastEntry {
    pub(super) name: String,
    pub(super) is_dir: bool,
    pub(super) attrs: u32,
    pub(super) size: u64,
    pub(super) mtime: Option<i64>,
    pub(super) ctime: Option<i64>,
    pub(super) atime: Option<i64>,
}

pub(super) fn build_fast_entry(data: &WIN32_FIND_DATAW) -> Option<FastEntry> {
    let name = wide_cstr_to_string(&data.cFileName)?;
    if name == "." || name == ".." {
        return None;
    }
    let attrs = data.dwFileAttributes;
    let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0;
    let size = ((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64;
    let mtime = filetime_to_secs(data.ftLastWriteTime);
    let ctime = filetime_to_secs(data.ftCreationTime);
    let atime = filetime_to_secs(data.ftLastAccessTime);

    Some(FastEntry {
        name,
        is_dir,
        attrs,
        size,
        mtime,
        ctime,
        atime,
    })
}
