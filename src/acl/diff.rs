use std::collections::HashMap;

use crate::acl::types::{AceEntry, AclSnapshot, DiffResult};

/// Compare two [`AclSnapshot`]s and return a [`DiffResult`].
///
/// ACEs are compared by their [`AceEntry::diff_key`], which encodes:
/// `principal | ace_type | rights_mask | inheritance_flags | is_inherited`.
///
/// Owner and inheritance-protection state are compared separately.
pub fn diff_acl(a: &AclSnapshot, b: &AclSnapshot) -> DiffResult {
    // Build lookup maps: key → entry reference
    let map_a: HashMap<String, &AceEntry> = a.entries.iter().map(|e| (e.diff_key(), e)).collect();

    let map_b: HashMap<String, &AceEntry> = b.entries.iter().map(|e| (e.diff_key(), e)).collect();

    let only_in_a: Vec<AceEntry> = map_a
        .iter()
        .filter(|(k, _)| !map_b.contains_key(*k))
        .map(|(_, e)| (*e).clone())
        .collect();

    let only_in_b: Vec<AceEntry> = map_b
        .iter()
        .filter(|(k, _)| !map_a.contains_key(*k))
        .map(|(_, e)| (*e).clone())
        .collect();

    let common_count = map_a.keys().filter(|k| map_b.contains_key(*k)).count();

    let owner_diff = if a.owner != b.owner {
        Some((a.owner.clone(), b.owner.clone()))
    } else {
        None
    };

    let inherit_diff = if a.is_protected != b.is_protected {
        Some((a.is_protected, b.is_protected))
    } else {
        None
    };

    DiffResult {
        only_in_a,
        only_in_b,
        common_count,
        owner_diff,
        inherit_diff,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::types::{AceType, InheritanceFlags, PropagationFlags};
    use std::path::PathBuf;

    fn make_ace(principal: &str, rights: u32, ace_type: AceType, inherited: bool) -> AceEntry {
        AceEntry {
            principal: principal.to_string(),
            raw_sid: format!("S-1-5-{}", principal.len()), // dummy
            rights_mask: rights,
            ace_type,
            inheritance: InheritanceFlags::BOTH,
            propagation: PropagationFlags::NONE,
            is_inherited: inherited,
            is_orphan: false,
        }
    }

    fn make_snapshot(entries: Vec<AceEntry>) -> AclSnapshot {
        AclSnapshot {
            path: PathBuf::from(r"C:\test"),
            owner: "BUILTIN\\Administrators".to_string(),
            is_protected: false,
            entries,
        }
    }

    #[test]
    fn identical_snapshots_no_diff() {
        let e = make_ace("BUILTIN\\Users", 0x1F01FF, AceType::Allow, false);
        let a = make_snapshot(vec![e.clone()]);
        let b = make_snapshot(vec![e]);
        let diff = diff_acl(&a, &b);
        assert!(!diff.has_diff());
        assert_eq!(diff.common_count, 1);
        assert!(diff.only_in_a.is_empty());
        assert!(diff.only_in_b.is_empty());
    }

    #[test]
    fn entry_only_in_a() {
        let shared = make_ace("Everyone", 0x1F01FF, AceType::Allow, false);
        let only_a = make_ace("SYSTEM", 0x1F01FF, AceType::Allow, false);

        let a = make_snapshot(vec![shared.clone(), only_a]);
        let b = make_snapshot(vec![shared]);
        let diff = diff_acl(&a, &b);

        assert_eq!(diff.only_in_a.len(), 1);
        assert_eq!(diff.only_in_a[0].principal, "SYSTEM");
        assert!(diff.only_in_b.is_empty());
        assert_eq!(diff.common_count, 1);
    }

    #[test]
    fn entry_only_in_b() {
        let shared = make_ace("Everyone", 0x1F01FF, AceType::Allow, false);
        let only_b = make_ace("Guest", 0x1200A9, AceType::Deny, false);

        let a = make_snapshot(vec![shared.clone()]);
        let b = make_snapshot(vec![shared, only_b]);
        let diff = diff_acl(&a, &b);

        assert!(diff.only_in_a.is_empty());
        assert_eq!(diff.only_in_b.len(), 1);
        assert_eq!(diff.only_in_b[0].principal, "Guest");
    }

    #[test]
    fn completely_different_snapshots() {
        let a = make_snapshot(vec![make_ace("A", 0x1F01FF, AceType::Allow, false)]);
        let b = make_snapshot(vec![make_ace("B", 0x1F01FF, AceType::Allow, false)]);
        let diff = diff_acl(&a, &b);

        assert!(diff.has_diff());
        assert_eq!(diff.only_in_a.len(), 1);
        assert_eq!(diff.only_in_b.len(), 1);
        assert_eq!(diff.common_count, 0);
    }

    #[test]
    fn both_empty() {
        let a = make_snapshot(vec![]);
        let b = make_snapshot(vec![]);
        let diff = diff_acl(&a, &b);
        assert!(!diff.has_diff());
        assert_eq!(diff.common_count, 0);
    }

    #[test]
    fn owner_diff_detected() {
        let a = make_snapshot(vec![]);
        let mut b = make_snapshot(vec![]);
        b.owner = "NT AUTHORITY\\SYSTEM".to_string();
        let diff = diff_acl(&a, &b);
        assert!(diff.owner_diff.is_some());
        let (oa, ob) = diff.owner_diff.unwrap();
        assert_eq!(oa, "BUILTIN\\Administrators");
        assert_eq!(ob, "NT AUTHORITY\\SYSTEM");
    }

    #[test]
    fn inherit_diff_detected() {
        let a = make_snapshot(vec![]);
        let mut b = make_snapshot(vec![]);
        b.is_protected = true;
        let diff = diff_acl(&a, &b);
        assert!(diff.inherit_diff.is_some());
        assert_eq!(diff.inherit_diff, Some((false, true)));
    }

    #[test]
    fn inherited_vs_explicit_are_different() {
        // Same principal/rights but one is inherited and the other is not
        // → diff_key differs → should appear in only_in_a and only_in_b
        let explicit = make_ace("Everyone", 0x1F01FF, AceType::Allow, false);
        let inherited = make_ace("Everyone", 0x1F01FF, AceType::Allow, true);

        let a = make_snapshot(vec![explicit]);
        let b = make_snapshot(vec![inherited]);
        let diff = diff_acl(&a, &b);
        assert_eq!(diff.only_in_a.len(), 1);
        assert_eq!(diff.only_in_b.len(), 1);
    }
}
