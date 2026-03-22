use crate::output::CliError;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

pub(crate) fn pick_color() -> Result<Color, CliError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{CLR_INVALID, GetDC, GetPixel, ReleaseDC};
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

        // SAFETY: Win32 API requires raw pointers; we validate return codes and release the DC.
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) == 0 {
                return Err(CliError::new(2, "Failed to read cursor position."));
            }
            let hdc = GetDC(std::ptr::null_mut());
            if hdc.is_null() {
                return Err(CliError::new(2, "Failed to acquire screen device context."));
            }
            let colorref = GetPixel(hdc, pt.x, pt.y);
            let _ = ReleaseDC(std::ptr::null_mut(), hdc);
            if colorref == CLR_INVALID {
                return Err(CliError::new(2, "Failed to read pixel color."));
            }
            let r = (colorref & 0xFF) as u8;
            let g = ((colorref >> 8) & 0xFF) as u8;
            let b = ((colorref >> 16) & 0xFF) as u8;
            return Ok(Color { r, g, b });
        }
    }
    #[cfg(not(windows))]
    {
        Err(CliError::new(2, "desktop color is Windows-only."))
    }
}

pub(crate) fn color_to_hex(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

pub(crate) fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::GlobalFree;
        use windows_sys::Win32::System::DataExchange::*;
        use windows_sys::Win32::System::Memory::*;

        // SAFETY: Win32 clipboard API requires raw pointers; we guard handles and close clipboard.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return false;
            }
            EmptyClipboard();

            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = wide.len() * 2;
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if hmem.is_null() {
                CloseClipboard();
                return false;
            }
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                GlobalFree(hmem);
                CloseClipboard();
                return false;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes);
            GlobalUnlock(hmem);
            if SetClipboardData(13, hmem as _).is_null() {
                GlobalFree(hmem);
                CloseClipboard();
                return false;
            }
            CloseClipboard();
            true
        }
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        false
    }
}
