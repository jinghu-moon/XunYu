use super::entry::FastEntry;
use super::eval::evaluate_entry_fast;
use super::time::{filetime_ticks_to_secs, to_wide_null};
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn scan_dir_nt(
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

        let bytes = io_status.Information.min(buffer.len());
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
