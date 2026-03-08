#[cfg(target_os = "windows")]
pub(super) fn copy_to_clipboard(text: &str) {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::*;
    use windows_sys::Win32::System::Memory::*;

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return;
        }
        EmptyClipboard();

        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * 2;
        let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if hmem.is_null() {
            CloseClipboard();
            return;
        }
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            GlobalFree(hmem);
            CloseClipboard();
            return;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes);
        GlobalUnlock(hmem);
        if SetClipboardData(13, hmem as _).is_null() {
            GlobalFree(hmem);
        }
        CloseClipboard();
    }
}
