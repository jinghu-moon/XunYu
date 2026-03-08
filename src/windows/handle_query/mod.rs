mod core;
mod debug_privilege;
mod device_map;
mod handle_table;
mod modules;
mod ntquery;
mod process;
mod target;

pub(crate) fn ensure_debug_privilege() -> bool {
    core::ensure_debug_privilege()
}

pub(crate) fn get_locking_processes(
    paths: &[&std::path::Path],
) -> Result<
    Vec<crate::windows::restart_manager::LockerInfo>,
    crate::windows::restart_manager::LockQueryError,
> {
    core::get_locking_processes(paths)
}

#[cfg(test)]
mod tests;
