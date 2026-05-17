use std::mem::size_of;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleFileNameExW};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

use super::core::OwnedHandle;
use super::device_map::normalize_path_like;
use super::target::{TargetPath, is_same_or_child, path_eq};

pub(super) fn enumerate_process_modules(pid: u32) -> Vec<String> {
    enumerate_process_modules_with_initial_slots(pid, 256)
}

pub(super) fn enumerate_process_modules_with_initial_slots(
    pid: u32,
    initial_slots: usize,
) -> Vec<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if handle.is_null() {
        return Vec::new();
    }
    let _guard = OwnedHandle::new(handle);

    let slots = initial_slots.max(1);
    let mut modules: Vec<HMODULE> = vec![std::ptr::null_mut(); slots];
    let mut needed: u32 = 0;
    loop {
        let ok = unsafe {
            EnumProcessModules(
                handle,
                modules.as_mut_ptr(),
                (modules.len() * size_of::<HMODULE>()) as u32,
                &mut needed,
            )
        };
        if ok == 0 {
            return Vec::new();
        }
        let count = needed as usize / size_of::<HMODULE>();
        if count <= modules.len() {
            modules.truncate(count);
            break;
        }
        modules.resize(count, std::ptr::null_mut());
    }

    let mut paths = Vec::with_capacity(modules.len());
    let mut buf = vec![0u16; 32768];
    for &m in &modules {
        let len = unsafe { GetModuleFileNameExW(handle, m, buf.as_mut_ptr(), buf.len() as u32) };
        if len > 0 {
            let s = String::from_utf16_lossy(&buf[..len as usize]);
            paths.push(normalize_path_like(&s));
        }
    }
    paths
}

pub(super) fn modules_match_targets(modules: &[String], targets: &[TargetPath]) -> bool {
    modules.iter().any(|mod_path| {
        targets.iter().any(|target| {
            if target.is_dir {
                is_same_or_child(mod_path, &target.dos_path)
            } else {
                path_eq(mod_path, &target.dos_path)
            }
        })
    })
}

pub(super) fn match_pids_by_modules<I, F>(
    pids: I,
    matched_pids: &mut std::collections::BTreeSet<u32>,
    targets: &[TargetPath],
    mut enumerate_modules: F,
) where
    I: IntoIterator<Item = u32>,
    F: FnMut(u32) -> Vec<String>,
{
    for pid in pids {
        if matched_pids.contains(&pid) {
            continue;
        }
        let modules = enumerate_modules(pid);
        if modules_match_targets(&modules, targets) {
            matched_pids.insert(pid);
        }
    }
}
