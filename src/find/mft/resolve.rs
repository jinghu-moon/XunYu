use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path};

use super::types::{ChildrenIndex, MftRecord, WcharPool};
use super::win::{NTFS_ROOT_FILE_REF, mask_file_ref};

pub(super) fn resolve_base_ref(
    base_root: &Path,
    records: &[MftRecord],
    pool: &WcharPool,
    children: &ChildrenIndex,
    case_sensitive: bool,
) -> u64 {
    let components = split_path_components(base_root);
    if components.is_empty() {
        return mask_file_ref(NTFS_ROOT_FILE_REF);
    }
    resolve_path_to_ref(&components, records, pool, children, case_sensitive)
}

fn resolve_path_to_ref(
    components: &[Vec<u16>],
    records: &[MftRecord],
    pool: &WcharPool,
    children: &ChildrenIndex,
    case_sensitive: bool,
) -> u64 {
    let mut current = mask_file_ref(NTFS_ROOT_FILE_REF);
    for comp in components {
        let mut found = 0u64;
        let (lo, hi) = children.find_range(current);
        for idx in lo..hi {
            let rec = records[children.record_index_at(idx)];
            if (rec.attrs & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY) == 0
            {
                continue;
            }
            if rec.name_len as usize != comp.len() {
                continue;
            }
            let name = pool.slice(rec.name_offset, rec.name_len);
            if wide_eq(name, comp, case_sensitive) {
                found = rec.file_ref;
                break;
            }
        }
        if found == 0 {
            return 0;
        }
        current = found;
    }
    current
}

fn wide_eq(a: &[u16], b: &[u16], case_sensitive: bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if case_sensitive {
        return a == b;
    }
    for (lhs, rhs) in a.iter().zip(b.iter()) {
        let mut l = *lhs;
        let mut r = *rhs;
        if (b'A' as u16..=b'Z' as u16).contains(&l) {
            l += 32;
        }
        if (b'A' as u16..=b'Z' as u16).contains(&r) {
            r += 32;
        }
        if l != r {
            return false;
        }
    }
    true
}

fn split_path_components(path: &Path) -> Vec<Vec<u16>> {
    let mut parts = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(os) => parts.push(os.encode_wide().collect()),
            Component::Prefix(_) | Component::RootDir => {}
            _ => {}
        }
    }
    parts
}
