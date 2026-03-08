use super::*;

pub(super) fn try_acquire_lock(base_path: &std::path::Path) -> Option<store::Lock> {
    let lock_path = base_path.with_extension("lock");
    store::Lock::acquire(&lock_path).ok()
}
