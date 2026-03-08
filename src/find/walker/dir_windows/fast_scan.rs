use super::entry::build_fast_entry;
use super::eval::evaluate_entry_fast;
use super::time::to_wide_null;
use super::*;

pub(super) fn scan_dir_fast(
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
