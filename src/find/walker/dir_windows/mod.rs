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

mod entry;
mod eval;
mod fast_scan;
mod nt_scan;
mod time;

use fast_scan::scan_dir_fast;
use nt_scan::scan_dir_nt;

#[allow(clippy::too_many_arguments)]
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
