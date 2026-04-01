use super::*;

pub(super) fn try_acquire_lock(
    base_path: &std::path::Path,
) -> Option<crate::bookmark::storage::Lock> {
    let lock_path = base_path.with_extension("lock");
    crate::bookmark::storage::Lock::acquire(&lock_path).ok()
}
