mod apply;
mod error_map;
mod inheritance;

use std::path::Path;

use anyhow::Result;

use crate::acl::types::AceEntry;
use crate::acl::types::{AceType, InheritanceFlags, PropagationFlags};

pub fn lookup_account_sid(principal: &str) -> Result<Vec<u8>> {
    apply::lookup_account_sid(principal)
}

#[allow(dead_code)] // 仅在 #[tokio::test] 中使用
pub fn add_rule(
    path: &Path,
    principal: &str,
    rights_mask: u32,
    ace_type: AceType,
    inheritance: InheritanceFlags,
    propagation: PropagationFlags,
) -> Result<()> {
    apply::add_rule(
        path,
        principal,
        rights_mask,
        ace_type,
        inheritance,
        propagation,
    )
}

pub fn add_rule_with_sid_bytes(
    path: &Path,
    sid_bytes: &[u8],
    rights_mask: u32,
    ace_type: AceType,
    inheritance: InheritanceFlags,
    propagation: PropagationFlags,
) -> Result<()> {
    apply::add_rule_with_sid_bytes(
        path,
        sid_bytes,
        rights_mask,
        ace_type,
        inheritance,
        propagation,
    )
}

pub fn remove_rules(path: &Path, to_remove: &[AceEntry]) -> Result<usize> {
    apply::remove_rules(path, to_remove)
}

pub fn purge_principal(path: &Path, principal: &str) -> Result<u32> {
    apply::purge_principal(path, principal)
}

pub fn set_owner(path: &Path, owner: &str) -> Result<()> {
    apply::set_owner(path, owner)
}

pub fn set_access_rule_protection(
    path: &Path,
    is_protected: bool,
    preserve_existing: bool,
) -> Result<()> {
    inheritance::set_access_rule_protection(path, is_protected, preserve_existing)
}

pub fn copy_acl(src: &Path, dst: &Path) -> Result<()> {
    apply::copy_acl(src, dst)
}
