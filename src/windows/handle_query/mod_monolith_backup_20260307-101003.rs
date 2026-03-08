mod debug_privilege;
mod device_map;
mod handle_table;
mod modules;
mod ntquery;
mod process;
mod target;

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

use debug_privilege::try_enable_debug_privilege;
use device_map::{collect_device_map, nt_to_dos_path};
use handle_table::enumerate_system_handles;
use modules::{enumerate_process_modules, match_pids_by_modules};
use ntquery::{MAX_NTQUERYOBJECT_TIMEOUTS, NtPathResolver, PathQueryResult};
use process::{infer_app_type, process_name_from_pid};
use target::{build_targets, path_matches_target};

#[derive(Debug)]
struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn raw(&self) -> HANDLE {
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

pub(crate) fn ensure_debug_privilege() -> bool {
    try_enable_debug_privilege()
}

pub(crate) fn get_locking_processes(paths: &[&Path]) -> Result<Vec<LockerInfo>, LockQueryError> {
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

#[cfg(test)]
mod tests {
    use super::debug_privilege::{DebugPrivilegeApi, try_enable_debug_privilege_with_api};
    use super::device_map::{
        dos_to_nt_paths, looks_like_unc_root, normalize_path_like, nt_to_dos_path,
        strip_prefix_ascii_insensitive,
    };
    use super::handle_table::{
        DEFAULT_MAX_HANDLE_BUFFER_BYTES, MAX_MAX_HANDLE_BUFFER_BYTES, MIN_MAX_HANDLE_BUFFER_BYTES,
        max_handle_buffer_bytes_from_env,
    };
    use super::modules::{
        enumerate_process_modules, enumerate_process_modules_with_initial_slots,
        match_pids_by_modules, modules_match_targets,
    };
    use super::process::{infer_app_type, infer_app_type_from_session};
    use super::target::{TargetPath, is_same_or_child, path_eq};

    use std::collections::BTreeSet;
    use windows_sys::Win32::Foundation::{HANDLE, LUID};
    use windows_sys::Win32::Security::TOKEN_PRIVILEGES;

    #[test]
    fn normalize_path_like_converts_slashes_and_strips_prefixes() {
        assert_eq!(normalize_path_like("C:/a/b"), r"C:\a\b");
        assert_eq!(normalize_path_like(r"\\?\C:\a\b\"), r"C:\a\b");
        assert_eq!(
            normalize_path_like(r"\\?\UNC\server\share\dir\"),
            r"\\server\share\dir"
        );
        assert_eq!(normalize_path_like(r"\??\c:\a\b"), r"C:\a\b");
    }

    #[test]
    fn normalize_path_like_preserves_drive_root_and_trims_trailing_backslashes() {
        assert_eq!(normalize_path_like(r"C:\\"), r"C:\");
        assert_eq!(normalize_path_like(r"C:\a\b\\"), r"C:\a\b");
    }

    #[test]
    fn looks_like_unc_root_detects_unc_roots() {
        assert!(looks_like_unc_root(r"\\server\share"));
        assert!(!looks_like_unc_root(r"\\server\share\dir"));
        assert!(!looks_like_unc_root(r"C:\a\b"));
    }

    #[test]
    fn strip_prefix_ascii_insensitive_is_case_insensitive() {
        assert_eq!(strip_prefix_ascii_insensitive("AbCdEf", "aBC"), Some("dEf"));
        assert_eq!(strip_prefix_ascii_insensitive("abc", "abcd"), None);
        assert_eq!(strip_prefix_ascii_insensitive("abc", "X"), None);
    }

    #[test]
    fn dos_to_nt_paths_maps_drive_letters_and_rejects_non_drive() {
        let mut map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        map.insert(
            "C:".to_string(),
            vec![r"\Device\HarddiskVolume3".to_string()],
        );

        let out = dos_to_nt_paths(r"C:\foo\bar", &map);
        assert_eq!(out, vec![r"\Device\HarddiskVolume3\foo\bar"]);

        assert!(dos_to_nt_paths(r"\\server\share\file", &map).is_empty());
        assert!(dos_to_nt_paths("relative\\path", &map).is_empty());
    }

    #[test]
    fn nt_to_dos_path_converts_prefixes_and_device_map() {
        let mut map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        map.insert(
            "C:".to_string(),
            vec![r"\Device\HarddiskVolume3".to_string()],
        );

        assert_eq!(
            nt_to_dos_path(r"\??\c:\foo\bar\", &map),
            Some(r"C:\foo\bar".to_string())
        );
        assert_eq!(
            nt_to_dos_path(r"\Device\Mup\server\share\dir\file", &map),
            Some(r"\\server\share\dir\file".to_string())
        );
        assert_eq!(
            nt_to_dos_path(r"\Device\HarddiskVolume3\foo\bar", &map),
            Some(r"C:\foo\bar".to_string())
        );
    }

    #[test]
    fn path_eq_and_is_same_or_child_behave_as_expected() {
        assert!(path_eq(r"C:\a", r"C:\a"));
        assert!(!path_eq(r"C:\a", r"C:\A"));

        assert!(is_same_or_child(r"C:\a", r"C:\a"));
        assert!(is_same_or_child(r"C:\a\b", r"C:\a"));
        assert!(!is_same_or_child(r"C:\ab", r"C:\a"));
    }

    #[test]
    fn max_handle_buffer_bytes_from_env_defaults_and_clamps() {
        assert_eq!(
            max_handle_buffer_bytes_from_env(None),
            DEFAULT_MAX_HANDLE_BUFFER_BYTES
        );
        assert_eq!(
            max_handle_buffer_bytes_from_env(Some("not-a-number")),
            DEFAULT_MAX_HANDLE_BUFFER_BYTES
        );

        assert_eq!(
            max_handle_buffer_bytes_from_env(Some("1")),
            MIN_MAX_HANDLE_BUFFER_BYTES
        );
        assert_eq!(
            max_handle_buffer_bytes_from_env(Some("2048")),
            MAX_MAX_HANDLE_BUFFER_BYTES
        );

        assert_eq!(
            max_handle_buffer_bytes_from_env(Some("300")),
            300usize << 20
        );
    }

    #[test]
    fn infer_app_type_explorer_is_rm_explorer() {
        assert_eq!(infer_app_type(123, "explorer.exe"), 5);
    }

    #[test]
    fn infer_app_type_unknown_when_session_lookup_fails() {
        assert_eq!(infer_app_type(u32::MAX, "not-explorer.exe"), 1);
    }

    #[test]
    fn infer_app_type_session0_is_rm_service() {
        assert_eq!(infer_app_type_from_session("svchost.exe", Some(0)), 4);
    }

    struct FakeDebugPrivilegeApi {
        open_ok: bool,
        lookup_ok: bool,
        last_error: u32,
    }

    impl DebugPrivilegeApi for FakeDebugPrivilegeApi {
        unsafe fn open_process_token(
            &self,
            _process: HANDLE,
            _access: u32,
            token: *mut HANDLE,
        ) -> i32 {
            if self.open_ok {
                if !token.is_null() {
                    unsafe {
                        *token = 1 as HANDLE;
                    }
                }
                1
            } else {
                0
            }
        }

        unsafe fn lookup_privilege_value_w(
            &self,
            _system_name: *const u16,
            _name: *const u16,
            luid: *mut LUID,
        ) -> i32 {
            if self.lookup_ok {
                if !luid.is_null() {
                    unsafe {
                        *luid = std::mem::zeroed();
                    }
                }
                1
            } else {
                0
            }
        }

        unsafe fn adjust_token_privileges(
            &self,
            _token: HANDLE,
            _new_state: *const TOKEN_PRIVILEGES,
        ) {
        }

        unsafe fn get_last_error(&self) -> u32 {
            self.last_error
        }
    }

    #[test]
    fn try_enable_debug_privilege_returns_true_when_granted() {
        let api = FakeDebugPrivilegeApi {
            open_ok: true,
            lookup_ok: true,
            last_error: 0,
        };
        assert!(try_enable_debug_privilege_with_api(false, &api));
    }

    #[test]
    fn try_enable_debug_privilege_returns_false_when_not_granted() {
        let api = FakeDebugPrivilegeApi {
            open_ok: true,
            lookup_ok: true,
            last_error: 1300, // ERROR_NOT_ALL_ASSIGNED
        };
        assert!(!try_enable_debug_privilege_with_api(false, &api));
    }

    #[test]
    fn try_enable_debug_privilege_returns_false_when_open_process_token_fails() {
        let api = FakeDebugPrivilegeApi {
            open_ok: false,
            lookup_ok: true,
            last_error: 0,
        };
        assert!(!try_enable_debug_privilege_with_api(false, &api));
    }

    #[test]
    fn try_enable_debug_privilege_returns_false_when_lookup_privilege_value_fails() {
        let api = FakeDebugPrivilegeApi {
            open_ok: true,
            lookup_ok: false,
            last_error: 0,
        };
        assert!(!try_enable_debug_privilege_with_api(false, &api));
    }

    #[test]
    fn enumerate_process_modules_invalid_pid_returns_empty() {
        assert!(enumerate_process_modules(u32::MAX).is_empty());
    }

    #[test]
    fn enumerate_process_modules_self_returns_non_empty_and_normalized() {
        let pid = std::process::id();
        let modules = enumerate_process_modules(pid);
        assert!(!modules.is_empty());
        for p in &modules {
            assert!(!p.contains('/'), "module path should be normalized: {p}");
            assert_eq!(normalize_path_like(p), *p);
        }
    }

    #[test]
    fn enumerate_process_modules_resizes_when_needed() {
        let pid = std::process::id();
        let modules = enumerate_process_modules_with_initial_slots(pid, 1);
        assert!(modules.len() > 1);
    }

    #[test]
    fn modules_match_targets_uses_file_and_dir_rules() {
        let file_target = TargetPath {
            is_dir: false,
            dos_path: r"C:\a\b.txt".to_string(),
            nt_paths: Vec::new(),
        };
        let dir_target = TargetPath {
            is_dir: true,
            dos_path: r"C:\x".to_string(),
            nt_paths: Vec::new(),
        };

        assert!(modules_match_targets(
            &[r"C:\a\b.txt".to_string()],
            &[file_target.clone()]
        ));
        assert!(modules_match_targets(
            &[r"C:\x\child\m.dll".to_string()],
            &[dir_target.clone()]
        ));

        // A file target must match exactly (no "dir prefix" matching).
        assert!(!modules_match_targets(
            &[r"C:\a\b\c.dll".to_string()],
            &[TargetPath {
                is_dir: false,
                dos_path: r"C:\a\b".to_string(),
                nt_paths: Vec::new(),
            }]
        ));
    }

    #[test]
    fn match_pids_by_modules_only_enumerates_unmatched_pids() {
        let targets = vec![TargetPath {
            is_dir: false,
            dos_path: r"C:\match.dll".to_string(),
            nt_paths: Vec::new(),
        }];

        let mut matched: BTreeSet<u32> = BTreeSet::new();
        matched.insert(2);

        let mut called: Vec<u32> = Vec::new();
        match_pids_by_modules(vec![1, 2, 3], &mut matched, &targets, |pid| {
            if pid == 2 {
                panic!("should not enumerate already-matched pid");
            }
            called.push(pid);
            if pid == 3 {
                vec![r"C:\match.dll".to_string()]
            } else {
                Vec::new()
            }
        });

        assert_eq!(called, vec![1, 3]);
        assert!(matched.contains(&3));
    }
}
