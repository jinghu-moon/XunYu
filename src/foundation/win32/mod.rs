#![allow(dead_code)]
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

// Windows-specific FFI operations

pub mod error;

pub fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(any(feature = "lock", feature = "redirect"))]
pub(crate) mod restart_manager;

#[cfg(any(feature = "lock", feature = "redirect"))]
pub(crate) mod handle_query;

#[cfg(any(feature = "lock", feature = "fs"))]
pub(crate) mod reboot_ops;

pub(crate) mod safety;

pub(crate) mod ctrlc;
pub(crate) mod file_copy;
pub(crate) mod mmap;

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
