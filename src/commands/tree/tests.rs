use super::collect::collect_items;
use super::constants::EXCLUDE_EXTS;
use super::filters::{is_version_dir, should_exclude};
use super::types::{SortKey, TreeFilters};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(windows)]
fn set_file_mtime(path: &Path, unix_secs: u64) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_WRITE_ATTRIBUTES, OPEN_EXISTING, SetFileTime,
    };

    // FILETIME is 100ns ticks since 1601-01-01 UTC.
    let ticks = (unix_secs + 11_644_473_600u64) * 10_000_000u64;
    let ft = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_WRITE_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    assert!(h != INVALID_HANDLE_VALUE, "CreateFileW failed");

    let ok = unsafe { SetFileTime(h, std::ptr::null(), std::ptr::null(), &ft) };
    unsafe {
        CloseHandle(h);
    }
    assert!(ok != 0, "SetFileTime failed");
}

fn mk_filters() -> TreeFilters {
    TreeFilters {
        hidden: false,
        exclude_names: Vec::new(),
        exclude_paths: Vec::new(),
        exclude_exts: EXCLUDE_EXTS.iter().map(|s| s.to_lowercase()).collect(),
        exclude_patterns: Vec::new(),
        include_patterns: Vec::new(),
    }
}

#[test]
fn is_version_dir_detects_v_number_prefix() {
    assert!(is_version_dir("v1"));
    assert!(is_version_dir("v2.3"));
    assert!(is_version_dir("v10-anything"));

    assert!(!is_version_dir("v"));
    assert!(!is_version_dir("vx"));
    assert!(!is_version_dir("1v2"));
}

#[test]
fn should_exclude_hides_dotfiles_by_default_and_allows_when_hidden_enabled() {
    let mut f = mk_filters();
    assert!(should_exclude("", ".hidden", ".hidden", false, &f));

    f.hidden = true;
    assert!(!should_exclude("", ".hidden", ".hidden", false, &f));
}

#[test]
fn should_exclude_filters_version_dirs_and_excluded_exts() {
    let f = mk_filters();
    assert!(should_exclude("v1", "v1", "v1", true, &f));
    assert!(should_exclude("a.exe", "a.exe", "a.exe", false, &f));
    assert!(!should_exclude("a.txt", "a.txt", "a.txt", false, &f));
}

#[test]
fn should_exclude_include_patterns_override_exclude_patterns() {
    let mut f = mk_filters();
    f.exclude_patterns = vec!["*.log".into()];
    f.include_patterns = vec!["keep.log".into()];

    // Include match should prevent exclusion.
    assert!(!should_exclude(
        "keep.log", "keep.log", "keep.log", false, &f
    ));
    // Non-included log should be excluded.
    assert!(should_exclude(
        "drop.log", "drop.log", "drop.log", false, &f
    ));
}

#[test]
fn collect_items_sorts_by_size_descending() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("small.txt"), "x").unwrap();
    fs::write(root.join("big.txt"), "xxxxxxxxxx").unwrap();

    let f = mk_filters();
    let items = collect_items(root, root, &f, SortKey::Size, false, false);
    let names: Vec<String> = items.into_iter().map(|i| i.name).collect();
    assert_eq!(names, vec!["big.txt".to_string(), "small.txt".to_string()]);
}

#[cfg(windows)]
#[test]
fn collect_items_sorts_by_mtime_descending() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let a = root.join("old.txt");
    let b = root.join("new.txt");
    fs::write(&a, "x").unwrap();
    fs::write(&b, "x").unwrap();

    set_file_mtime(&a, 1_600_000_000);
    set_file_mtime(&b, 1_600_000_010);

    let f = mk_filters();
    let items = collect_items(root, root, &f, SortKey::Mtime, false, false);
    let names: Vec<String> = items.into_iter().map(|i| i.name).collect();
    assert_eq!(names, vec!["new.txt".to_string(), "old.txt".to_string()]);
}
