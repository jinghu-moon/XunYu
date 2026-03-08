use std::path::{Path, PathBuf};

use crate::util::normalize_glob_path;

use super::super::filters::{
    EmptyFilterMode, FindFilters, attr_filter_match, depth_filter_match, needs_metadata_for_entry,
    size_filters_match, time_filters_match,
};
use super::super::ignore::IgnoreSet;
use super::super::matcher::determine_path_state;
use super::super::rules::{CompiledRules, RuleKind};
use super::common::{
    EntryOutcome, ScanItem, build_rel_path, passes_empty_filter, rel_path, should_prune_dir,
};

use std::os::windows::ffi::{OsStrExt, OsStringExt};

use windows_sys::Wdk::Storage::FileSystem::{
    FILE_ID_BOTH_DIR_INFORMATION, FileIdBothDirectoryInformation, NtQueryDirectoryFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_FILES, FILETIME, GetLastError, INVALID_HANDLE_VALUE,
    STATUS_BUFFER_OVERFLOW, STATUS_NO_MORE_FILES, STATUS_SUCCESS,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FIND_FIRST_EX_LARGE_FETCH, FindClose,
    FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW, OPEN_EXISTING,
    WIN32_FIND_DATAW,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

#[derive(Clone, Debug)]
struct FastEntry {
    name: String,
    is_dir: bool,
    attrs: u32,
    size: u64,
    mtime: Option<i64>,
    ctime: Option<i64>,
    atime: Option<i64>,
}

pub(super) fn scan_dir_windows(
    dir: &Path,
    base_root: &Path,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
    count: &mut usize,
    on_dir: &mut dyn FnMut(PathBuf, RuleKind),
    on_item: &mut dyn FnMut(ScanItem),
) {
    if !scan_dir_nt(
        dir,
        base_root,
        base_display,
        rules,
        filters,
        ignore,
        inherited_state,
        depth,
        force_meta,
        count_only,
        count,
        on_dir,
        on_item,
    ) {
        scan_dir_fast(
            dir,
            base_root,
            base_display,
            rules,
            filters,
            ignore,
            inherited_state,
            depth,
            force_meta,
            count_only,
            count,
            on_dir,
            on_item,
        );
    }
}

fn scan_dir_fast(
    dir: &Path,
    base_root: &Path,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
    count: &mut usize,
    on_dir: &mut dyn FnMut(PathBuf, RuleKind),
    on_item: &mut dyn FnMut(ScanItem),
) {
    let dir_rel = rel_path(base_root, dir);
    let mut rel_buf = String::new();
    let mut pattern = dir.to_path_buf();
    pattern.push("*");
    let wide = to_wide_null(&pattern);

    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let mut handle = unsafe {
        FindFirstFileExW(
            wide.as_ptr(),
            FindExInfoBasic,
            &mut data as *mut _ as *mut _,
            FindExSearchNameMatch,
            std::ptr::null_mut(),
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        handle = unsafe {
            FindFirstFileExW(
                wide.as_ptr(),
                FindExInfoBasic,
                &mut data as *mut _ as *mut _,
                FindExSearchNameMatch,
                std::ptr::null_mut(),
                0,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
    }

    loop {
        if let Some(entry) = build_fast_entry(&data) {
            let rel = build_rel_path(&dir_rel, &entry.name, &mut rel_buf);
            let outcome = evaluate_entry_fast(
                dir,
                &entry,
                rel,
                base_display,
                rules,
                filters,
                ignore,
                inherited_state,
                depth,
                force_meta,
                count_only,
            );
            if let Some((child, state)) = outcome.next_dir {
                on_dir(child, state);
            }
            if outcome.count_inc {
                if count_only {
                    *count += 1;
                } else if let Some(item) = outcome.item {
                    on_item(item);
                }
            }
        }

        let ok = unsafe { FindNextFileW(handle, &mut data) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                break;
            }
            break;
        }
    }

    unsafe {
        FindClose(handle);
    }
}

fn scan_dir_nt(
    dir: &Path,
    base_root: &Path,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
    count: &mut usize,
    on_dir: &mut dyn FnMut(PathBuf, RuleKind),
    on_item: &mut dyn FnMut(ScanItem),
) -> bool {
    let dir_rel = rel_path(base_root, dir);
    let mut rel_buf = String::new();
    let wide = to_wide_null(dir);
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return false;
    }

    let mut buffer = vec![0u8; 64 * 1024];
    let mut io_status: IO_STATUS_BLOCK = unsafe { std::mem::zeroed() };
    let mut restart = true;
    let mut ok = true;

    loop {
        let status = unsafe {
            NtQueryDirectoryFile(
                handle,
                std::ptr::null_mut(),
                None,
                std::ptr::null(),
                &mut io_status as *mut _,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                FileIdBothDirectoryInformation,
                0,
                std::ptr::null(),
                if restart { 1 } else { 0 },
            )
        };
        if status == STATUS_NO_MORE_FILES {
            break;
        }
        if status != STATUS_SUCCESS && status != STATUS_BUFFER_OVERFLOW {
            ok = false;
            break;
        }
        restart = false;

        let bytes = io_status.Information.min(buffer.len()) as usize;
        let buffer_start = buffer.as_ptr() as usize;
        let buffer_end = buffer_start + bytes;
        let mut offset = 0usize;
        while offset < bytes {
            let info =
                unsafe { &*(buffer.as_ptr().add(offset) as *const FILE_ID_BOTH_DIR_INFORMATION) };
            if info.NextEntryOffset == 0 && info.FileNameLength == 0 {
                break;
            }

            let name_len = (info.FileNameLength / 2) as usize;
            let name_ptr = info.FileName.as_ptr() as usize;
            let name_end = name_ptr.saturating_add(name_len.saturating_mul(2));
            if name_ptr < buffer_start || name_end > buffer_end {
                ok = false;
                break;
            }
            let name_ptr = name_ptr as *const u16;
            let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
            let name = std::ffi::OsString::from_wide(name_slice)
                .to_string_lossy()
                .into_owned();
            if name != "." && name != ".." {
                let attrs = info.FileAttributes;
                let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0;
                let size = info.EndOfFile.max(0) as u64;
                let mtime = filetime_ticks_to_secs(info.LastWriteTime);
                let ctime = filetime_ticks_to_secs(info.CreationTime);
                let atime = filetime_ticks_to_secs(info.LastAccessTime);

                let entry = FastEntry {
                    name: name.clone(),
                    is_dir,
                    attrs,
                    size,
                    mtime,
                    ctime,
                    atime,
                };
                let rel = build_rel_path(&dir_rel, &entry.name, &mut rel_buf);
                let outcome = evaluate_entry_fast(
                    dir,
                    &entry,
                    rel,
                    base_display,
                    rules,
                    filters,
                    ignore,
                    inherited_state,
                    depth,
                    force_meta,
                    count_only,
                );
                if let Some((child, state)) = outcome.next_dir {
                    on_dir(child, state);
                }
                if outcome.count_inc {
                    if count_only {
                        *count += 1;
                    } else if let Some(item) = outcome.item {
                        on_item(item);
                    }
                }
            }

            if info.NextEntryOffset == 0 {
                break;
            }
            offset = offset.saturating_add(info.NextEntryOffset as usize);
        }
    }

    unsafe {
        CloseHandle(handle);
    }
    ok
}

fn build_fast_entry(data: &WIN32_FIND_DATAW) -> Option<FastEntry> {
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

fn evaluate_entry_fast(
    dir: &Path,
    entry: &FastEntry,
    rel: &str,
    base_display: &str,
    rules: &CompiledRules,
    filters: &FindFilters,
    ignore: &IgnoreSet,
    inherited_state: RuleKind,
    depth: i32,
    force_meta: bool,
    count_only: bool,
) -> EntryOutcome {
    if !ignore.is_empty() {
        let rel_norm = normalize_glob_path(rel);
        let name_lower = entry.name.to_ascii_lowercase();
        if ignore.should_ignore(&rel_norm, &name_lower, entry.is_dir) {
            return EntryOutcome {
                next_dir: None,
                item: None,
                count_inc: false,
            };
        }
    }

    let decision = determine_path_state(rules, rel, entry.is_dir, inherited_state);
    let depth_val = depth + 1;
    let need_dir_path = entry.is_dir
        && (filters.empty_dirs != EmptyFilterMode::None || !should_prune_dir(&decision));
    let dir_path = if need_dir_path {
        Some(dir.join(&entry.name))
    } else {
        None
    };

    if decision.final_state != RuleKind::Include {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !depth_filter_match(filters.depth.as_ref(), depth_val) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }

    let need_meta = needs_metadata_for_entry(filters, entry.is_dir) || force_meta;
    let (size, mtime, ctime, atime, attrs) = if need_meta {
        (
            Some(entry.size),
            entry.mtime,
            entry.ctime,
            entry.atime,
            entry.attrs,
        )
    } else {
        (None, None, None, None, 0)
    };

    if !attr_filter_match(filters.attr.as_ref(), attrs) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !time_filters_match(&filters.time_filters, mtime, ctime, atime) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if !entry.is_dir && !size_filters_match(&filters.size_filters, size.unwrap_or(0)) {
        return EntryOutcome {
            next_dir: if entry.is_dir && !should_prune_dir(&decision) {
                dir_path.map(|p| (p, decision.final_state))
            } else {
                None
            },
            item: None,
            count_inc: false,
        };
    }
    if entry.is_dir {
        if filters.empty_dirs != EmptyFilterMode::None {
            let Some(path) = dir_path.as_ref() else {
                return EntryOutcome {
                    next_dir: None,
                    item: None,
                    count_inc: false,
                };
            };
            if !passes_empty_filter(filters, entry.is_dir, path, size) {
                return EntryOutcome {
                    next_dir: None,
                    item: None,
                    count_inc: false,
                };
            }
        }
    } else if !passes_empty_filter(filters, false, Path::new(""), size) {
        return EntryOutcome {
            next_dir: None,
            item: None,
            count_inc: false,
        };
    }

    let next_dir = if entry.is_dir && !should_prune_dir(&decision) {
        dir_path.map(|p| (p, decision.final_state))
    } else {
        None
    };

    if count_only {
        return EntryOutcome {
            next_dir,
            item: None,
            count_inc: true,
        };
    }

    EntryOutcome {
        next_dir,
        item: Some(ScanItem {
            base_dir: base_display.to_string(),
            rel_path: rel.to_string(),
            is_dir: entry.is_dir,
            depth: depth_val,
            size,
            mtime,
            rule_idx: decision.rule_idx,
            final_state: decision.final_state,
            explicit: decision.explicit,
        }),
        count_inc: true,
    }
}

fn to_wide_null(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

fn wide_cstr_to_string(buf: &[u16]) -> Option<String> {
    let len = buf.iter().position(|&c| c == 0)?;
    let os = std::ffi::OsString::from_wide(&buf[..len]);
    Some(os.to_string_lossy().into_owned())
}

fn filetime_to_secs(ft: FILETIME) -> Option<i64> {
    let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    if ticks == 0 {
        return None;
    }
    const EPOCH_DIFF_100NS: u64 = 11644473600u64 * 10_000_000u64;
    if ticks < EPOCH_DIFF_100NS {
        return None;
    }
    Some(((ticks - EPOCH_DIFF_100NS) / 10_000_000u64) as i64)
}

fn filetime_ticks_to_secs(ticks: i64) -> Option<i64> {
    if ticks <= 0 {
        return None;
    }
    let ticks = ticks as u64;
    const EPOCH_DIFF_100NS: u64 = 11644473600u64 * 10_000_000u64;
    if ticks < EPOCH_DIFF_100NS {
        return None;
    }
    Some(((ticks - EPOCH_DIFF_100NS) / 10_000_000u64) as i64)
}
