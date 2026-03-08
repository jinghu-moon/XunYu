use std::ffi::OsString;
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
    RmStartSession,
};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

#[derive(Debug, Clone)]
pub(crate) struct LockerInfo {
    pub pid: u32,
    pub name: String,
    pub app_type: u32,
}

pub(crate) const ERROR_ACCESS_DENIED_CODE: u32 = 5;
pub(crate) const ERROR_WRITE_FAULT_CODE: u32 = 29;
pub(crate) const ERROR_SEM_TIMEOUT_CODE: u32 = 121;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockQueryCodeSpace {
    Win32,
    NtStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockQueryStage {
    RegistryProbe,
    StartSession,
    RegisterResources,
    GetListSize,
    GetListData,
    HandleEnumerate,
    HandleEngine,
}

impl LockQueryStage {
    fn as_str(self) -> &'static str {
        match self {
            LockQueryStage::RegistryProbe => "registry_probe",
            LockQueryStage::StartSession => "rm_start_session",
            LockQueryStage::RegisterResources => "rm_register_resources",
            LockQueryStage::GetListSize => "rm_get_list_size",
            LockQueryStage::GetListData => "rm_get_list_data",
            LockQueryStage::HandleEnumerate => "handle_enumerate",
            LockQueryStage::HandleEngine => "handle_engine",
        }
    }

    fn is_rm_stage(self) -> bool {
        matches!(
            self,
            LockQueryStage::RegistryProbe
                | LockQueryStage::StartSession
                | LockQueryStage::RegisterResources
                | LockQueryStage::GetListSize
                | LockQueryStage::GetListData
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LockQueryError {
    pub code: u32,
    pub code_space: LockQueryCodeSpace,
    pub stage: LockQueryStage,
    pub detail: String,
}

impl LockQueryError {
    pub(crate) fn from_win32(code: u32, stage: LockQueryStage, detail: impl Into<String>) -> Self {
        Self {
            code,
            code_space: LockQueryCodeSpace::Win32,
            stage,
            detail: detail.into(),
        }
    }

    pub(crate) fn from_ntstatus(
        nt_status: i32,
        stage: LockQueryStage,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: nt_status as u32,
            code_space: LockQueryCodeSpace::NtStatus,
            stage,
            detail: detail.into(),
        }
    }

    pub(crate) fn new(code: u32, stage: LockQueryStage, detail: impl Into<String>) -> Self {
        Self::from_win32(code, stage, detail)
    }

    pub(crate) fn is_registry_unavailable(&self) -> bool {
        self.code_space == LockQueryCodeSpace::Win32 && self.code == ERROR_WRITE_FAULT_CODE
    }

    pub(crate) fn is_registry_mutex_timeout(&self) -> bool {
        self.code_space == LockQueryCodeSpace::Win32 && self.code == ERROR_SEM_TIMEOUT_CODE
    }

    pub(crate) fn is_directory_path_error(&self) -> bool {
        self.code_space == LockQueryCodeSpace::Win32
            && self.code == ERROR_ACCESS_DENIED_CODE
            && self.stage.is_rm_stage()
    }

    pub(crate) fn stage_name(&self) -> &'static str {
        self.stage.as_str()
    }

    pub(crate) fn guidance(&self) -> &'static str {
        if self.is_registry_unavailable() {
            "Restart Manager cannot read/write registry. Check account permissions, loaded user profile, and HKCU\\SOFTWARE\\Microsoft\\RestartManager ACL/policy."
        } else if self.is_registry_mutex_timeout() {
            "Restart Manager registry mutex timed out. Retry shortly and avoid concurrent installer/update operations."
        } else if self.is_directory_path_error() {
            "Restart Manager expects file paths. A registered path is a directory; pass a specific file or use a non-RM fallback for directory targets."
        } else if self.code_space == LockQueryCodeSpace::Win32
            && self.code == ERROR_ACCESS_DENIED_CODE
        {
            "Access denied while reading process handles. Try elevated privileges for fuller lock visibility."
        } else if self.code_space == LockQueryCodeSpace::NtStatus {
            "Native handle query failed (NTSTATUS). Fallback engine may be required; rerun with verbose diagnostics."
        } else {
            "Restart Manager lock query failed. Retry with --verbose and inspect runtime account/profile context."
        }
    }
}

impl fmt::Display for LockQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code_space {
            LockQueryCodeSpace::Win32 => write!(
                f,
                "OS Error {} at {}: {}",
                self.code,
                self.stage_name(),
                self.detail
            ),
            LockQueryCodeSpace::NtStatus => write!(
                f,
                "NTSTATUS 0x{:08X} at {}: {}",
                self.code,
                self.stage_name(),
                self.detail
            ),
        }
    }
}

fn io_error_code(err: &std::io::Error) -> u32 {
    err.raw_os_error().map(|code| code as u32).unwrap_or(1)
}

fn probe_registry_access_impl<F>(mut open_subkey_with_flags: F) -> Result<(), LockQueryError>
where
    F: FnMut(&str, u32) -> std::io::Result<()>,
{
    let flags = KEY_READ | KEY_WRITE;

    open_subkey_with_flags("SOFTWARE", flags).map_err(|e| {
        LockQueryError::new(
            io_error_code(&e),
            LockQueryStage::RegistryProbe,
            format!("cannot open HKCU\\SOFTWARE with read/write access: {e}"),
        )
    })?;

    open_subkey_with_flags("SOFTWARE\\Microsoft", flags).map_err(|e| {
        LockQueryError::new(
            io_error_code(&e),
            LockQueryStage::RegistryProbe,
            format!("cannot open HKCU\\SOFTWARE\\Microsoft with read/write access: {e}"),
        )
    })?;

    Ok(())
}

fn probe_registry_access() -> Result<(), LockQueryError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    probe_registry_access_impl(|subkey, flags| {
        hkcu.open_subkey_with_flags(subkey, flags).map(|_| ())
    })
}

fn with_probe_context(detail: &str, probe_issue: Option<&LockQueryError>) -> String {
    if let Some(probe) = probe_issue {
        format!(
            "{}; preflight {} (OS Error {}): {}",
            detail,
            probe.stage_name(),
            probe.code,
            probe.detail
        )
    } else {
        detail.to_string()
    }
}

pub(crate) fn get_locking_processes(paths: &[&Path]) -> Result<Vec<LockerInfo>, LockQueryError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let probe_issue = probe_registry_access().err();

    let mut session_handle: u32 = 0;
    let mut session_key: [u16; CCH_RM_SESSION_KEY as usize + 1] =
        [0; CCH_RM_SESSION_KEY as usize + 1];

    // 1. Start session
    let res = unsafe { RmStartSession(&mut session_handle, 0, session_key.as_mut_ptr()) };
    if res != ERROR_SUCCESS {
        return Err(LockQueryError::new(
            res,
            LockQueryStage::StartSession,
            with_probe_context("RmStartSession failed", probe_issue.as_ref()),
        ));
    }

    // Prepare paths array
    let wide_paths: Vec<Vec<u16>> = paths
        .iter()
        .map(|p| {
            let mut v: Vec<u16> = p.as_os_str().encode_wide().collect();
            v.push(0);
            v
        })
        .collect();

    let path_ptrs: Vec<*const u16> = wide_paths.iter().map(|p| p.as_ptr()).collect();

    // 2. Register resources
    let register_res = unsafe {
        RmRegisterResources(
            session_handle,
            path_ptrs.len() as u32,
            path_ptrs.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };

    let result = if register_res == ERROR_SUCCESS {
        // 3. Get list
        let mut needed: u32 = 0;
        let mut allocated: u32 = 0;
        let mut reason: u32 = 0;

        // First call to get the required size
        let mut rm_res = unsafe {
            RmGetList(
                session_handle,
                &mut needed,
                &mut allocated,
                std::ptr::null_mut(),
                &mut reason,
            )
        };

        if rm_res == ERROR_MORE_DATA {
            allocated = needed;
            let mut proc_info_array: Vec<RM_PROCESS_INFO> =
                vec![unsafe { std::mem::zeroed() }; allocated as usize];

            rm_res = unsafe {
                RmGetList(
                    session_handle,
                    &mut needed,
                    &mut allocated,
                    proc_info_array.as_mut_ptr(),
                    &mut reason,
                )
            };

            if rm_res == ERROR_SUCCESS {
                let mut lockers = Vec::new();
                for i in 0..allocated as usize {
                    let info = &proc_info_array[i];
                    let pid = info.Process.dwProcessId;
                    let app_type = info.ApplicationType as u32;

                    let name_len = info
                        .strAppName
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(info.strAppName.len());

                    let name = OsString::from_wide(&info.strAppName[..name_len])
                        .to_string_lossy()
                        .into_owned();

                    lockers.push(LockerInfo {
                        pid,
                        name,
                        app_type,
                    });
                }
                Ok(lockers)
            } else {
                Err(LockQueryError::new(
                    rm_res,
                    LockQueryStage::GetListData,
                    with_probe_context("RmGetList (data phase) failed", probe_issue.as_ref()),
                ))
            }
        } else if rm_res == ERROR_SUCCESS {
            // No processes are holding it
            Ok(Vec::new())
        } else {
            Err(LockQueryError::new(
                rm_res,
                LockQueryStage::GetListSize,
                with_probe_context("RmGetList (size phase) failed", probe_issue.as_ref()),
            ))
        }
    } else {
        Err(LockQueryError::new(
            register_res,
            LockQueryStage::RegisterResources,
            with_probe_context("RmRegisterResources failed", probe_issue.as_ref()),
        ))
    };

    // 4. End session
    unsafe {
        RmEndSession(session_handle);
    }

    result
}

#[cfg(test)]
mod tests;
