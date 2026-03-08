use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE};
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_DISK, GetFileType};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::windows::restart_manager::{
    LockQueryError, LockQueryStage, LockerInfo, get_locking_processes as rm_get_locking_processes,
};

use super::debug_privilege::try_enable_debug_privilege;
use super::device_map::{collect_device_map, nt_to_dos_path};
use super::handle_table::enumerate_system_handles;
use super::modules::{enumerate_process_modules, match_pids_by_modules};
use super::ntquery::{MAX_NTQUERYOBJECT_TIMEOUTS, NtPathResolver, PathQueryResult};
use super::process::{infer_app_type, process_name_from_pid};
use super::target::{build_targets, path_matches_target};

#[derive(Debug)]
pub(super) struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub(super) fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }
    pub(super) fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub(super) fn ensure_debug_privilege() -> bool {
    try_enable_debug_privilege()
}

pub(super) fn get_locking_processes(paths: &[&Path]) -> Result<Vec<LockerInfo>, LockQueryError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let primary_res = query_with_handles(paths);
    match primary_res {
        Ok(lockers) => Ok(lockers),
        Err(primary_err) => {
            // Restart Manager cannot query directory paths (ERROR_ACCESS_DENIED),
            // so only fallback when all requested paths are files.
            if !paths.iter().all(|p| p.is_file()) {
                return Err(primary_err);
            }

            match rm_get_locking_processes(paths) {
                Ok(lockers) => Ok(lockers),
                Err(secondary_err) => Err(LockQueryError::from_win32(
                    secondary_err.code,
                    LockQueryStage::HandleEngine,
                    format!(
                        "primary(handle_query) failed: {}; fallback(restart_manager) failed: {}",
                        primary_err, secondary_err
                    ),
                )),
            }
        }
    }
}

fn query_with_handles(paths: &[&Path]) -> Result<Vec<LockerInfo>, LockQueryError> {
    try_enable_debug_privilege();
    let device_map = collect_device_map();
    let targets = build_targets(paths, &device_map);
    let all_files = paths.iter().all(|p| p.is_file());
    let entries = enumerate_system_handles()?;
    let mut process_handles: HashMap<u32, Option<OwnedHandle>> = HashMap::new();
    let mut matched_pids: BTreeSet<u32> = BTreeSet::new();
    let cur_process = unsafe { GetCurrentProcess() };
    let mut nt_resolver = NtPathResolver::new();
    let mut nt_query_timeout_count = 0usize;

    for entry in entries {
        let pid = entry.unique_process_id as u32;
        if pid == 0 {
            continue;
        }

        let process_handle = process_handles.entry(pid).or_insert_with(|| {
            OwnedHandle::new(unsafe {
                OpenProcess(
                    PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION,
                    0,
                    pid,
                )
            })
        });
        let Some(process_handle) = process_handle.as_ref() else {
            continue;
        };

        let mut duplicated: HANDLE = std::ptr::null_mut();
        let dup_ok = unsafe {
            DuplicateHandle(
                process_handle.raw(),
                entry.handle_value as HANDLE,
                cur_process,
                &mut duplicated,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            )
        };
        if dup_ok == 0 || duplicated.is_null() {
            continue;
        }

        if unsafe { GetFileType(duplicated) } != FILE_TYPE_DISK {
            unsafe {
                CloseHandle(duplicated);
            }
            continue;
        }

        let nt_path = match nt_resolver.query(duplicated) {
            PathQueryResult::Resolved(Some(path)) => path,
            PathQueryResult::Resolved(None) => continue,
            PathQueryResult::TimedOut => {
                nt_query_timeout_count += 1;
                if nt_query_timeout_count >= MAX_NTQUERYOBJECT_TIMEOUTS {
                    if all_files {
                        return Err(LockQueryError::from_win32(
                            1,
                            LockQueryStage::HandleEngine,
                            format!(
                                "NtQueryObject timed out on {} handles; switching to fallback engine",
                                nt_query_timeout_count
                            ),
                        ));
                    }
                    break;
                }
                continue;
            }
            PathQueryResult::WorkerFailed => continue,
        };

        if nt_path.is_empty() {
            continue;
        }
        let dos_path = nt_to_dos_path(&nt_path, &device_map);

        if targets
            .iter()
            .any(|target| path_matches_target(target, &nt_path, dos_path.as_deref()))
        {
            matched_pids.insert(pid);
        }
    }

    // Second pass: module enumeration for PIDs seen but not yet matched.
    match_pids_by_modules(
        process_handles.keys().copied(),
        &mut matched_pids,
        &targets,
        enumerate_process_modules,
    );

    let lockers = matched_pids
        .into_iter()
        .map(|pid| {
            let name = process_name_from_pid(pid);
            let app_type = infer_app_type(pid, &name);
            LockerInfo {
                pid,
                name,
                app_type,
            }
        })
        .collect();
    Ok(lockers)
}
