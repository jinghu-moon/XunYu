use std::ffi::c_void;
use std::mem::size_of;

use crate::windows::restart_manager::{LockQueryError, LockQueryStage};

const SYSTEM_EXTENDED_HANDLE_INFORMATION_CLASS: u32 = 64;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC0000004u32 as i32;

pub(super) const DEFAULT_MAX_HANDLE_BUFFER_BYTES: usize = 256usize << 20;
pub(super) const MIN_MAX_HANDLE_BUFFER_BYTES: usize = 64usize << 20;
pub(super) const MAX_MAX_HANDLE_BUFFER_BYTES: usize = 1024usize << 20;

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct SystemHandleTableEntryInfoEx {
    pub(super) object: *mut c_void,
    pub(super) unique_process_id: usize,
    pub(super) handle_value: usize,
    pub(super) granted_access: u32,
    pub(super) creator_back_trace_index: u16,
    pub(super) object_type_index: u16,
    pub(super) handle_attributes: u32,
    pub(super) reserved: u32,
}

#[repr(C)]
struct SystemHandleInformationEx {
    number_of_handles: usize,
    reserved: usize,
    handles: [SystemHandleTableEntryInfoEx; 1],
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

pub(super) fn enumerate_system_handles() -> Result<Vec<SystemHandleTableEntryInfoEx>, LockQueryError>
{
    let max_cap = max_handle_buffer_bytes();
    let mut cap = 1usize << 20;
    loop {
        // Avoid aborting the whole process under low-commit / low-pagefile environments:
        // `vec![0; cap]` would abort on OOM, so we allocate fallibly.
        let mut buf: Vec<u8> = Vec::new();
        if let Err(e) = buf.try_reserve_exact(cap) {
            return Err(LockQueryError::from_win32(
                1455, // ERROR_COMMITMENT_LIMIT / commit/pagefile exhaustion (best-effort mapping)
                LockQueryStage::HandleEnumerate,
                format!(
                    "failed to allocate handle table buffer (cap={}MB, max={}MB): {:?}",
                    cap >> 20,
                    max_cap >> 20,
                    e
                ),
            ));
        }
        // NtQuerySystemInformation writes into this buffer; we only read the written portion.
        unsafe {
            buf.set_len(cap);
        }
        let mut needed: u32 = 0;
        let status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_EXTENDED_HANDLE_INFORMATION_CLASS as i32,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                &mut needed,
            )
        };

        if status == STATUS_INFO_LENGTH_MISMATCH {
            let grow_to = needed as usize + 0x10000;
            cap = cap.max(grow_to).saturating_mul(2);
            if cap > max_cap {
                return Err(LockQueryError::from_ntstatus(
                    status,
                    LockQueryStage::HandleEnumerate,
                    format!(
                        "NtQuerySystemInformation requires too much memory for handle table (cap={}MB). Use XUN_MAX_HANDLE_BUFFER_MB to adjust.",
                        max_cap >> 20
                    ),
                ));
            }
            continue;
        }

        if status < 0 {
            return Err(LockQueryError::from_ntstatus(
                status,
                LockQueryStage::HandleEnumerate,
                format!(
                    "NtQuerySystemInformation failed with NTSTATUS=0x{:08X}",
                    status as u32
                ),
            ));
        }

        if buf.len() < size_of::<SystemHandleInformationEx>() {
            return Err(LockQueryError::from_win32(
                1,
                LockQueryStage::HandleEnumerate,
                "invalid handle table buffer",
            ));
        }

        let info = unsafe { &*(buf.as_ptr() as *const SystemHandleInformationEx) };
        let count = info.number_of_handles;
        let expected_bytes = size_of::<SystemHandleInformationEx>()
            + count.saturating_sub(1) * size_of::<SystemHandleTableEntryInfoEx>();
        if expected_bytes > buf.len() {
            return Err(LockQueryError::from_win32(
                1,
                LockQueryStage::HandleEnumerate,
                "truncated handle table buffer",
            ));
        }

        let mut out = Vec::with_capacity(count);
        let first = info.handles.as_ptr();
        for i in 0..count {
            out.push(unsafe { *first.add(i) });
        }
        return Ok(out);
    }
}

fn max_handle_buffer_bytes() -> usize {
    let raw = std::env::var("XUN_MAX_HANDLE_BUFFER_MB").ok();
    max_handle_buffer_bytes_from_env(raw.as_deref())
}

pub(super) fn max_handle_buffer_bytes_from_env(raw: Option<&str>) -> usize {
    let parsed = raw
        .and_then(|v| v.parse::<usize>().ok())
        .map(|mb| mb.saturating_mul(1usize << 20))
        .unwrap_or(DEFAULT_MAX_HANDLE_BUFFER_BYTES);
    parsed.clamp(MIN_MAX_HANDLE_BUFFER_BYTES, MAX_MAX_HANDLE_BUFFER_BYTES)
}
