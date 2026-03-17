use std::collections::HashSet;

use crate::acl::types::{AceType, AclSnapshot, EffectiveAccess, TriState};

// FileSystemRights bit constants
pub const RIGHT_READ_DATA: u32 = 0x0001;
pub const RIGHT_WRITE_DATA: u32 = 0x0002;
pub const RIGHT_EXECUTE: u32 = 0x0020;
pub const RIGHT_DELETE: u32 = 0x0001_0000;
pub const RIGHT_CHANGE_PERMISSIONS: u32 = 0x0004_0000;
pub const RIGHT_TAKE_OWNERSHIP: u32 = 0x0008_0000;

// ── Public API ────────────────────────────────────────────────────────────────

/// Collect the SIDs associated with the current Windows process token
/// (the token's owner SID + all group SIDs).
///
/// Falls back to an empty vec when called on a non-Windows build or when the
/// token cannot be read.
pub fn get_current_user_sids() -> Vec<String> {
    collect_token_sids()
}

/// Resolve a named account to its SID string.
///
/// This function calls `LookupAccountNameW` and returns the canonical SID
/// string (e.g. `"S-1-5-32-544"`).  Returns an error when the name is unknown.
pub fn resolve_user_sid(user: &str) -> anyhow::Result<String> {
    use crate::acl::reader::sid_to_string;
    use crate::acl::writer::lookup_account_sid;
    use windows::Win32::Security::PSID;

    let bytes = lookup_account_sid(user)?;
    let sid = PSID(bytes.as_ptr() as *mut _);
    let s = sid_to_string(sid)?;
    Ok(s)
}

/// Compute the effective access a set of `user_sids` has on `snapshot`.
///
/// Algorithm (mirrors Windows DACL evaluation):
/// 1. For each ACE whose SID matches one of `user_sids`:
///    - Deny  → OR into deny mask
///    - Allow → OR into allow mask
/// 2. `effective = allow & !deny`
/// 3. For each of the 6 checked rights: classify as `Allow`, `Deny`, or
///    `NoRule` based on the effective and deny masks.
pub fn compute_effective_access(snapshot: &AclSnapshot, user_sids: &[String]) -> EffectiveAccess {
    let sid_set: HashSet<&str> = user_sids.iter().map(|s| s.as_str()).collect();

    let mut deny_mask: u32 = 0;
    let mut allow_mask: u32 = 0;

    for ace in &snapshot.entries {
        // Match by raw SID or by resolved principal name
        let matches =
            sid_set.contains(ace.raw_sid.as_str()) || sid_set.contains(ace.principal.as_str());

        if matches {
            match ace.ace_type {
                AceType::Deny => deny_mask |= ace.rights_mask,
                AceType::Allow => allow_mask |= ace.rights_mask,
            }
        }
    }

    // Deny overrides Allow
    let effective_mask = allow_mask & !deny_mask;

    EffectiveAccess {
        read: classify(RIGHT_READ_DATA, effective_mask, deny_mask, allow_mask),
        write: classify(RIGHT_WRITE_DATA, effective_mask, deny_mask, allow_mask),
        execute: classify(RIGHT_EXECUTE, effective_mask, deny_mask, allow_mask),
        delete: classify(RIGHT_DELETE, effective_mask, deny_mask, allow_mask),
        change_perms: classify(
            RIGHT_CHANGE_PERMISSIONS,
            effective_mask,
            deny_mask,
            allow_mask,
        ),
        take_ownership: classify(RIGHT_TAKE_OWNERSHIP, effective_mask, deny_mask, allow_mask),
        effective_mask,
        allow_mask,
        deny_mask,
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn classify(bit: u32, _effective: u32, deny: u32, allow: u32) -> TriState {
    if deny & bit != 0 {
        TriState::Deny
    } else if allow & bit != 0 {
        // Note: we check allow here rather than effective so that a "Deny from
        // a different group" doesn't cause NoRule when the user's own Allow
        // would otherwise grant it.
        TriState::Allow
    } else {
        TriState::NoRule
    }
}

/// Collect SID strings from the current process token.
fn collect_token_sids() -> Vec<String> {
    use crate::acl::reader::sid_to_string;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TOKEN_GROUPS, TOKEN_QUERY, TOKEN_USER, TokenGroups, TokenUser,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut sids = Vec::new();

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return sids;
        }

        // --- Token user (owner SID) ---
        let mut needed: u32 = 0;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        if needed > 0 {
            let mut buf = vec![0u8; needed as usize];
            if GetTokenInformation(
                token,
                TokenUser,
                Some(buf.as_mut_ptr() as *mut _),
                needed,
                &mut needed,
            )
            .is_ok()
            {
                let tu = &*(buf.as_ptr() as *const TOKEN_USER);
                if let Ok(s) = sid_to_string(tu.User.Sid) {
                    sids.push(s);
                }
            }
        }

        // --- Token groups ---
        let mut needed: u32 = 0;
        let _ = GetTokenInformation(token, TokenGroups, None, 0, &mut needed);
        if needed > 0 {
            let mut buf = vec![0u8; needed as usize];
            if GetTokenInformation(
                token,
                TokenGroups,
                Some(buf.as_mut_ptr() as *mut _),
                needed,
                &mut needed,
            )
            .is_ok()
            {
                let tg = &*(buf.as_ptr() as *const TOKEN_GROUPS);
                let groups = std::slice::from_raw_parts(tg.Groups.as_ptr(), tg.GroupCount as usize);
                for g in groups {
                    if let Ok(s) = sid_to_string(g.Sid) {
                        sids.push(s);
                    }
                }
            }
        }

        let _ = windows::Win32::Foundation::CloseHandle(token);
    }
    sids
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::types::{AceEntry, AceType, AclSnapshot, InheritanceFlags, PropagationFlags};
    use std::path::PathBuf;

    const SID_A: &str = "S-1-5-32-544"; // Administrators
    const SID_B: &str = "S-1-1-0"; // Everyone

    fn make_ace(sid: &str, rights: u32, ace_type: AceType) -> AceEntry {
        AceEntry {
            principal: sid.to_string(),
            raw_sid: sid.to_string(),
            rights_mask: rights,
            ace_type,
            inheritance: InheritanceFlags::NONE,
            propagation: PropagationFlags::NONE,
            is_inherited: false,
            is_orphan: false,
        }
    }

    fn snapshot(entries: Vec<AceEntry>) -> AclSnapshot {
        AclSnapshot {
            path: PathBuf::from(r"C:\test"),
            owner: "BUILTIN\\Administrators".to_string(),
            is_protected: false,
            entries,
        }
    }

    #[test]
    fn allow_read_granted() {
        let s = snapshot(vec![make_ace(SID_A, RIGHT_READ_DATA, AceType::Allow)]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::Allow);
        assert_eq!(ea.write, TriState::NoRule);
    }

    #[test]
    fn deny_overrides_allow() {
        // Allow FullControl, then Deny Delete — Delete should be Deny
        let s = snapshot(vec![
            make_ace(SID_A, RIGHT_READ_DATA | RIGHT_DELETE, AceType::Allow),
            make_ace(SID_A, RIGHT_DELETE, AceType::Deny),
        ]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::Allow);
        assert_eq!(ea.delete, TriState::Deny);
    }

    #[test]
    fn no_matching_sids_all_no_rule() {
        let s = snapshot(vec![make_ace(SID_B, RIGHT_READ_DATA, AceType::Allow)]);
        // SID_A has no rules in this ACL
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::NoRule);
        assert_eq!(ea.write, TriState::NoRule);
    }

    #[test]
    fn full_control_grants_all() {
        const FULL: u32 = 0x1F01FF;
        let s = snapshot(vec![make_ace(SID_A, FULL, AceType::Allow)]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::Allow);
        assert_eq!(ea.write, TriState::Allow);
        assert_eq!(ea.execute, TriState::Allow);
        assert_eq!(ea.delete, TriState::Allow);
        assert_eq!(ea.change_perms, TriState::Allow);
        assert_eq!(ea.take_ownership, TriState::Allow);
    }

    #[test]
    fn group_membership_via_sid_list() {
        // User is in group SID_B; ACL allows SID_B read
        let s = snapshot(vec![make_ace(SID_B, RIGHT_READ_DATA, AceType::Allow)]);
        // Pass both the user's own SID and the group SID
        let ea = compute_effective_access(&s, &["S-1-5-99-user".to_string(), SID_B.to_string()]);
        assert_eq!(ea.read, TriState::Allow);
    }

    #[test]
    fn empty_snapshot_all_no_rule() {
        // 空快照：所有权限均无规则
        let s = snapshot(vec![]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::NoRule);
        assert_eq!(ea.write, TriState::NoRule);
        assert_eq!(ea.execute, TriState::NoRule);
        assert_eq!(ea.delete, TriState::NoRule);
        assert_eq!(ea.effective_mask, 0);
    }

    #[test]
    fn empty_sid_list_all_no_rule() {
        // 空 SID 列表：即使 ACL 有规则，也不匹配任何主体
        let s = snapshot(vec![make_ace(SID_A, RIGHT_READ_DATA | RIGHT_DELETE, AceType::Allow)]);
        let ea = compute_effective_access(&s, &[]);
        assert_eq!(ea.read, TriState::NoRule);
        assert_eq!(ea.delete, TriState::NoRule);
        assert_eq!(ea.effective_mask, 0);
    }

    #[test]
    fn cross_sid_deny_takes_precedence_over_allow() {
        // 设计决策验证：SID_A deny Delete，SID_B allow Delete
        // 用户同时属于两组 → deny 优先（classify 先检查 deny_mask）
        let s = snapshot(vec![
            make_ace(SID_A, RIGHT_DELETE, AceType::Deny),
            make_ace(SID_B, RIGHT_DELETE, AceType::Allow),
        ]);
        let ea = compute_effective_access(&s, &[SID_A.to_string(), SID_B.to_string()]);
        assert_eq!(ea.effective_mask & RIGHT_DELETE, 0, "effective mask should have DELETE cleared");
        assert_eq!(ea.deny_mask & RIGHT_DELETE, RIGHT_DELETE, "deny mask should include DELETE");
        assert_eq!(ea.delete, TriState::Deny,
            "deny from SID_A should take precedence over allow from SID_B");
    }

    #[test]
    fn deny_only_ace_no_allow() {
        // 仅有 Deny ACE，无 Allow：Deny 位应为 Deny，其余为 NoRule
        let s = snapshot(vec![make_ace(SID_A, RIGHT_DELETE, AceType::Deny)]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.delete, TriState::Deny);
        assert_eq!(ea.read, TriState::NoRule);
        assert_eq!(ea.effective_mask, 0);
        assert_eq!(ea.allow_mask, 0);
    }

    #[test]
    fn multiple_allow_aces_union() {
        // 同一用户多条 Allow：权限取并集
        let s = snapshot(vec![
            make_ace(SID_A, RIGHT_READ_DATA, AceType::Allow),
            make_ace(SID_A, RIGHT_WRITE_DATA, AceType::Allow),
        ]);
        let ea = compute_effective_access(&s, &[SID_A.to_string()]);
        assert_eq!(ea.read, TriState::Allow);
        assert_eq!(ea.write, TriState::Allow);
        assert_eq!(ea.execute, TriState::NoRule);
        assert_eq!(ea.effective_mask & (RIGHT_READ_DATA | RIGHT_WRITE_DATA),
            RIGHT_READ_DATA | RIGHT_WRITE_DATA);
    }
}
