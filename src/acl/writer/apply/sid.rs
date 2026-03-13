use anyhow::Result;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::{GetLengthSid, LookupAccountNameW, PSID};
use windows::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows::core::{PCWSTR, PWSTR};

use crate::acl::error::AclError;

fn try_parse_string_sid(principal: &str) -> Result<Option<Vec<u8>>> {
    let p = principal.trim();
    if !p.to_ascii_lowercase().starts_with("s-1-") {
        return Ok(None);
    }
    let name_wide: Vec<u16> = p.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut sid = PSID::default();
        ConvertStringSidToSidW(PCWSTR(name_wide.as_ptr()), &mut sid)
            .map_err(|_| AclError::InvalidPrincipal(principal.to_string()))?;
        let len = GetLengthSid(sid);
        if len == 0 {
            LocalFree(HLOCAL(sid.0 as *mut _));
            return Err(AclError::InvalidPrincipal(principal.to_string()).into());
        }
        let bytes = std::slice::from_raw_parts(sid.0 as *const u8, len as usize).to_vec();
        LocalFree(HLOCAL(sid.0 as *mut _));
        Ok(Some(bytes))
    }
}

/// Allocate and return the SID bytes for `principal` (e.g. `"BUILTIN\Administrators"`).
///
/// The returned `Vec<u8>` owns the SID memory. The caller must not free it via
/// Win32 — Rust's allocator owns it.
pub(super) fn lookup_account_sid(principal: &str) -> Result<Vec<u8>> {
    if let Some(bytes) = try_parse_string_sid(principal)? {
        return Ok(bytes);
    }
    let name_wide: Vec<u16> = principal.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut sid_len: u32 = 0;
        let mut domain_len: u32 = 0;
        let mut sid_use = windows::Win32::Security::SID_NAME_USE(0);

        // First call: size probe
        let _ = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name_wide.as_ptr()),
            PSID::default(),
            &mut sid_len,
            PWSTR::null(),
            &mut domain_len,
            &mut sid_use,
        );

        if sid_len == 0 {
            return Err(AclError::InvalidPrincipal(principal.to_string()).into());
        }

        let mut sid_buf = vec![0u8; sid_len as usize];
        let mut domain_buf = vec![0u16; domain_len as usize];

        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name_wide.as_ptr()),
            PSID(sid_buf.as_mut_ptr() as *mut _),
            &mut sid_len,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut domain_len,
            &mut sid_use,
        )
        .map_err(|_| AclError::InvalidPrincipal(principal.to_string()))?;

        Ok(sid_buf)
    }
}
