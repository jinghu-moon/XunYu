use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use rayon::prelude::*;

use crate::acl::reader::{get_acl, list_children};
use crate::acl::types::AceEntry;
use crate::config::AclConfig;

/// A path + orphaned ACE pair found during a scan.
#[derive(Debug, Clone)]
pub struct OrphanEntry {
    pub path: PathBuf,
    pub ace: AceEntry,
}

/// Scan `root` for ACEs that reference unresolvable (orphaned) SIDs.
///
/// * `recursive = true`  → scan all descendants
/// * `recursive = false` → scan root only
///
/// Returns one [`OrphanEntry`] per orphaned ACE found (may be multiple per path).
pub fn scan_orphans(root: &Path, recursive: bool, config: &AclConfig) -> Result<Vec<OrphanEntry>> {
    // Build target list
    let mut targets: Vec<PathBuf> = vec![root.to_path_buf()];
    if recursive {
        match list_children(root, true) {
            Ok(children) => targets.extend(children),
            Err(e) => eprintln!("[warn] orphan scan: failed to enumerate children: {e:#}"),
        }
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.throttle_limit)
        .build()?;

    let results: Arc<Mutex<Vec<OrphanEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();

    pool.install(|| {
        targets.par_iter().for_each(|path| match get_acl(path) {
            Ok(snapshot) => {
                let orphans: Vec<OrphanEntry> = snapshot
                    .entries
                    .into_iter()
                    .filter(|e| e.is_orphan)
                    .map(|ace| OrphanEntry {
                        path: path.clone(),
                        ace,
                    })
                    .collect();
                if !orphans.is_empty() {
                    let mut g = results_clone.lock().unwrap();
                    g.extend(orphans);
                }
            }
            Err(e) => {
                eprintln!(
                    "[warn] orphan scan: cannot read ACL for {}: {e:#}",
                    path.display()
                );
            }
        });
    });

    let out = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    Ok(out)
}

/// Remove all orphaned ACEs collected by [`scan_orphans`].
///
/// Returns `(succeeded, failed)` counts.
pub fn purge_orphan_sids(orphans: &[OrphanEntry]) -> (usize, usize) {
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for o in orphans {
        // purge_principal resolves the name — for orphans we pass the raw SID
        // string directly as the "principal" since LookupAccountNameW will fail;
        // instead we fall back to the low-level delete-by-SID path.
        match purge_by_raw_sid(&o.path, &o.ace.raw_sid) {
            Ok(n) if n > 0 => succeeded += 1,
            Ok(_) => {} // nothing removed (already gone)
            Err(e) => {
                eprintln!(
                    "[warn] purge_orphan_sids: failed for {} SID={}: {e:#}",
                    o.path.display(),
                    o.ace.raw_sid
                );
                failed += 1;
            }
        }
    }
    (succeeded, failed)
}

// ── Internal ──────────────────────────────────────────────────────────────────

/// Remove explicit ACEs matching `raw_sid` from `path`'s DACL.
fn purge_by_raw_sid(path: &Path, raw_sid: &str) -> Result<u32> {
    use crate::acl::error::AclError;
    use crate::acl::reader::sid_to_string;
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::{
        ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW,
    };
    use windows::Win32::Security::{
        ACL, ACL_SIZE_INFORMATION, AclSizeInformation, DACL_SECURITY_INFORMATION, DeleteAce,
        GetAclInformation, PSECURITY_DESCRIPTOR, PSID,
    };
    use windows::core::PCWSTR;

    let sid_wide: Vec<u16> = raw_sid.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // Convert string SID → binary SID
        let mut target_sid = PSID::default();
        ConvertStringSidToSidW(PCWSTR(sid_wide.as_ptr()), &mut target_sid)
            .map_err(|_| AclError::last_win32())?;

        let target_str = sid_to_string(target_sid).unwrap_or_default();

        let pw: Vec<u16> = path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_dacl),
            None,
            &mut p_sd,
        );
        if status.0 != 0 {
            return Err(AclError::from_win32(status.0).into());
        }

        if p_dacl.is_null() {
            LocalFree(HLOCAL(p_sd.0 as *mut _));
            LocalFree(HLOCAL(target_sid.0 as *mut _));
            return Ok(0);
        }

        let mut info = ACL_SIZE_INFORMATION::default();
        GetAclInformation(
            p_dacl,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
        .map_err(|_| AclError::last_win32())?;

        let mut removed = 0u32;
        for i in (0..info.AceCount as i32).rev() {
            let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            if windows::Win32::Security::GetAce(p_dacl, i as u32, &mut ace_ptr).is_err()
                || ace_ptr.is_null()
            {
                continue;
            }
            let header = &*(ace_ptr as *const crate::acl::reader::AceHeaderPublic);
            if header.ace_flags & 0x10 != 0 {
                continue;
            } // skip inherited

            let sid_ptr: *const _ = match header.ace_type {
                0 => {
                    let a = &*(ace_ptr as *const windows::Win32::Security::ACCESS_ALLOWED_ACE);
                    &a.SidStart as *const u32 as *const _
                }
                1 => {
                    let a = &*(ace_ptr as *const windows::Win32::Security::ACCESS_DENIED_ACE);
                    &a.SidStart as *const u32 as *const _
                }
                _ => continue,
            };

            let ace_sid = PSID(sid_ptr as *mut _);
            if sid_to_string(ace_sid).ok().as_deref() == Some(&target_str) {
                if DeleteAce(p_dacl, i as u32).is_ok() {
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            let status = SetNamedSecurityInfoW(
                PCWSTR(pw.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(p_dacl),
                None,
            );
            if status.0 != 0 {
                return Err(AclError::from_win32(status.0).into());
            }
        }

        LocalFree(HLOCAL(p_sd.0 as *mut _));
        LocalFree(HLOCAL(target_sid.0 as *mut _));
        Ok(removed)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::types::{AceType, InheritanceFlags, PropagationFlags};

    fn orphan_ace(sid: &str) -> AceEntry {
        AceEntry {
            principal: sid.to_string(),
            raw_sid: sid.to_string(),
            rights_mask: 0x1F01FF,
            ace_type: AceType::Allow,
            inheritance: InheritanceFlags::NONE,
            propagation: PropagationFlags::NONE,
            is_inherited: false,
            is_orphan: true,
        }
    }

    /// Unit-level test: `scan_orphans` correctly filters non-orphan ACEs.
    /// We test the filtering logic in isolation, not the Win32 calls.
    #[test]
    fn orphan_filter_logic() {
        // Simulate a snapshot where one entry is orphaned
        let entries = vec![
            {
                let mut e = orphan_ace("S-1-5-99-12345");
                e.is_orphan = true;
                e
            },
            {
                let mut e = orphan_ace("BUILTIN\\Users");
                e.is_orphan = false;
                e
            },
        ];

        let orphans: Vec<_> = entries.iter().filter(|e| e.is_orphan).collect();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].raw_sid, "S-1-5-99-12345");
    }
}
