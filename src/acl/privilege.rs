use crate::acl::error::AclError;
use anyhow::{Context, Result};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PCWSTR;

/// Enable a single named Windows privilege on the current process token.
///
/// # Example
/// ```ignore
/// enable_privilege("SeRestorePrivilege").unwrap();
/// ```
pub fn enable_privilege(name: &str) -> Result<()> {
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // 1. Open process token
        let mut token = HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|_| AclError::last_win32())
        .with_context(|| format!("OpenProcessToken failed for privilege '{name}'"))?;

        // 2. Lookup LUID for the privilege name
        let mut luid = windows::Win32::Foundation::LUID::default();
        let result = LookupPrivilegeValueW(PCWSTR::null(), PCWSTR(name_wide.as_ptr()), &mut luid);
        if result.is_err() {
            let _ = CloseHandle(token);
            return Err(AclError::last_win32())
                .with_context(|| format!("LookupPrivilegeValueW failed for '{name}'"));
        }

        // 3. Build TOKEN_PRIVILEGES structure
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        // 4. Adjust token
        let ok = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        let _ = CloseHandle(token);

        ok.map_err(|_| AclError::last_win32())
            .with_context(|| format!("AdjustTokenPrivileges failed for '{name}'"))?;
    }
    Ok(())
}

/// Enable the three privileges required for forced ACL repair.
///
/// Returns `Ok(())` even when some privilege grants fail (the caller may still
/// succeed with reduced capability).  Individual errors are logged to stderr.
pub fn enable_repair_privileges() -> Result<()> {
    let privs = [
        "SeRestorePrivilege",
        "SeBackupPrivilege",
        "SeTakeOwnershipPrivilege",
    ];
    let mut last_err: Option<anyhow::Error> = None;
    for priv_name in &privs {
        if let Err(e) = enable_privilege(priv_name) {
            eprintln!("[warn] 无法激活特权 {priv_name}: {e:#}");
            last_err = Some(e);
        }
    }
    if let Some(e) = last_err {
        // Return the last error but don't abort — partial privilege is still useful
        eprintln!("[warn] 部分特权激活失败，修复可能受限: {e:#}");
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke-test: enabling a real privilege should not panic.
    /// This test must run in an elevated process to succeed fully.
    #[test]
    fn enable_restore_privilege_does_not_panic() {
        // Allow failure in non-elevated contexts
        let _ = enable_privilege("SeRestorePrivilege");
    }

    #[test]
    fn invalid_privilege_name_returns_error() {
        let r = enable_privilege("SeThisPrivilegeDoesNotExist");
        assert!(r.is_err(), "expected error for bogus privilege name");
    }
}
