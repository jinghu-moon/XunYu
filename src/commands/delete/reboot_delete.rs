use std::ptr;

type PCWSTR = *const u16;
type DWORD = u32;
type BOOL = i32;

const MOVEFILE_DELAY_UNTIL_REBOOT: DWORD = 0x00000004;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: PCWSTR, new_name: PCWSTR, flags: DWORD) -> BOOL;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn schedule_delete_on_reboot(path: &str) -> bool {
    let src = to_wide(&format!("\\\\?\\{}", path));
    let ok = unsafe { MoveFileExW(src.as_ptr(), ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
    ok != 0
}
