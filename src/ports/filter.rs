use std::collections::HashSet;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

use super::model::RawPortEntry;

pub(super) fn dedupe_raw(raw: &mut Vec<RawPortEntry>) {
    let mut seen: HashSet<RawPortEntry> = HashSet::new();
    raw.retain(|r| seen.insert(*r));
}

pub(super) fn terminate_pid_error_message(code: u32) -> String {
    match code {
        5 => "access denied".to_string(),
        87 => "not found".to_string(),
        e => format!("error {}", e),
    }
}

pub(super) fn terminate_pid(pid: u32) -> Result<(), String> {
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return Err(terminate_pid_error_message(GetLastError()));
        }
        let ok = TerminateProcess(h, 1);
        CloseHandle(h);
        if ok != 0 {
            Ok(())
        } else {
            Err(terminate_pid_error_message(GetLastError()))
        }
    }
}
