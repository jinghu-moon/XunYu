use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    SetConsoleCtrlHandler,
};

static CANCELLED: AtomicBool = AtomicBool::new(false);
static INSTALLED: OnceLock<()> = OnceLock::new();

pub(crate) fn install_ctrlc_handler_once() {
    INSTALLED.get_or_init(|| unsafe {
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    });
}

pub(crate) fn reset_cancelled() {
    CANCELLED.store(false, Ordering::SeqCst);
}

pub(crate) fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
        | CTRL_SHUTDOWN_EVENT => {
            CANCELLED.store(true, Ordering::SeqCst);
            1
        }
        _ => 0,
    }
}
