use std::mem;
use std::sync::OnceLock;

use super::{DWORD, NtQuerySystemInformation, STATUS_INFO_LENGTH_MISMATCH, SysHandleEntry};

static HANDLE_SNAPSHOT: OnceLock<Vec<SysHandleEntry>> = OnceLock::new();

pub(crate) fn handle_snapshot() -> &'static Vec<SysHandleEntry> {
    HANDLE_SNAPSHOT.get_or_init(snapshot_handles)
}

fn snapshot_handles() -> Vec<SysHandleEntry> {
    let mut buf_size: DWORD = 0x100000;
    loop {
        let mut buf = vec![0u8; buf_size as usize];
        let mut ret_len: DWORD = 0;
        let status = unsafe {
            NtQuerySystemInformation(0x10, buf.as_mut_ptr() as _, buf_size, &mut ret_len)
        };
        if status == STATUS_INFO_LENGTH_MISMATCH {
            buf_size = ret_len * 2;
            continue;
        }
        if status != 0 {
            return Vec::new();
        }

        let count = unsafe { *(buf.as_ptr() as *const u32) } as usize;
        let entry_size = mem::size_of::<SysHandleEntry>();
        let mut entries = Vec::with_capacity(count);
        let base = buf.as_ptr() as usize + 4;

        for i in 0..count {
            let ptr = (base + i * entry_size) as *const SysHandleEntry;
            if (ptr as usize + entry_size) > (buf.as_ptr() as usize + buf.len()) {
                break;
            }
            entries.push(unsafe { ptr.read_unaligned() });
        }
        return entries;
    }
}
