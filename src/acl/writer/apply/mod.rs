mod common;
mod copy;
mod dacl;
mod owner;
mod sid;

use std::path::Path;

use anyhow::Result;

use crate::acl::types::{AceEntry, AceType, InheritanceFlags, PropagationFlags};

pub(super) fn lookup_account_sid(principal: &str) -> Result<Vec<u8>> {
    sid::lookup_account_sid(principal)
}

pub(super) fn path_wide(path: &Path) -> Vec<u16> {
    common::path_wide(path)
}

pub(super) fn add_rule(
    path: &Path,
    principal: &str,
    rights_mask: u32,
    ace_type: AceType,
    inheritance: InheritanceFlags,
    propagation: PropagationFlags,
) -> Result<()> {
    dacl::add_rule(
        path,
        principal,
        rights_mask,
        ace_type,
        inheritance,
        propagation,
    )
}

pub(super) fn add_rule_with_sid_bytes(
    path: &Path,
    sid_bytes: &[u8],
    rights_mask: u32,
    ace_type: AceType,
    inheritance: InheritanceFlags,
    propagation: PropagationFlags,
) -> Result<()> {
    dacl::add_rule_with_sid_bytes(
        path,
        sid_bytes,
        rights_mask,
        ace_type,
        inheritance,
        propagation,
    )
}

pub(super) fn remove_rules(path: &Path, to_remove: &[AceEntry]) -> Result<usize> {
    dacl::remove_rules(path, to_remove)
}

pub(super) fn purge_principal(path: &Path, principal: &str) -> Result<u32> {
    dacl::purge_principal(path, principal)
}

pub(super) fn set_owner(path: &Path, owner: &str) -> Result<()> {
    owner::set_owner(path, owner)
}

pub(super) fn copy_acl(src: &Path, dst: &Path) -> Result<()> {
    copy::copy_acl(src, dst)
}
