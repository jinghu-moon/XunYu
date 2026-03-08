use super::{CloseHandle, FreeSid, HANDLE, INVALID_HANDLE_VALUE, PSID};

pub(super) struct HandleGuard(pub(super) HANDLE);
impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != 0 && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub(super) struct SidGuard(pub(super) PSID);
impl Drop for SidGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                FreeSid(self.0);
            }
        }
    }
}
