// Windows-specific FFI operations

#[cfg(any(feature = "lock", feature = "redirect"))]
pub(crate) mod restart_manager;

#[cfg(any(feature = "lock", feature = "redirect"))]
pub(crate) mod handle_query;

#[cfg(any(feature = "lock", feature = "fs"))]
pub(crate) mod reboot_ops;

pub(crate) mod safety;

pub(crate) mod ctrlc;

#[cfg(feature = "redirect")]
pub(crate) mod trash;
#[cfg(feature = "crypt")]
pub(crate) mod volume;

#[cfg(feature = "protect")]
pub(crate) mod acl;

#[cfg(feature = "crypt")]
pub(crate) mod efs;

#[cfg(feature = "desktop")]
pub(crate) mod window_api;
