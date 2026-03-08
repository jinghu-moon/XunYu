use std::path::Path;

use anyhow::{Context, Result};
use windows::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
use windows::Win32::Security::{OWNER_SECURITY_INFORMATION, PSID};
use windows::core::PCWSTR;

use super::super::error_map::check_win32;
use super::common::path_wide;
use super::sid::lookup_account_sid;

/// Change the owner of `path`.
///
/// Requires `SeRestorePrivilege` to be active on the caller's token.
pub(super) fn set_owner(path: &Path, owner: &str) -> Result<()> {
    crate::acl::privilege::enable_privilege("SeRestorePrivilege")
        .context("set_owner: failed to enable SeRestorePrivilege")?;

    let sid_bytes = lookup_account_sid(owner)
        .with_context(|| format!("set_owner: cannot resolve '{owner}'"))?;
    let sid = PSID(sid_bytes.as_ptr() as *mut _);
    let pw = path_wide(path);

    unsafe {
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            sid,
            PSID::default(),
            None,
            None,
        );
        check_win32(
            status,
            format!(
                "set_owner: SetNamedSecurityInfoW failed for {}",
                path.display()
            ),
        )?;
    }
    Ok(())
}
