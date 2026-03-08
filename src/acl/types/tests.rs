use super::*;

#[test]
fn rights_short_known_values() {
    assert_eq!(rights_short(2_032_127), "FullControl");
    assert_eq!(rights_short(1_245_631), "Modify");
    assert_eq!(rights_short(1_179_785), "Read");
}

#[test]
fn rights_short_strips_synchronize() {
    // 0x00100000 = Synchronize; FullControl mask already includes it
    let mask = 2_032_127u32 | 0x00100000;
    assert_eq!(rights_short(mask), "FullControl");
}

#[test]
fn inheritance_flags_display() {
    assert_eq!(
        InheritanceFlags::BOTH.to_string(),
        "ContainerInherit|ObjectInherit"
    );
    assert_eq!(InheritanceFlags::NONE.to_string(), "None");
}

#[test]
fn ace_entry_diff_key_stable() {
    let e = AceEntry {
        principal: "BUILTIN\\Administrators".to_string(),
        raw_sid: "S-1-5-32-544".to_string(),
        rights_mask: 2_032_127,
        ace_type: AceType::Allow,
        inheritance: InheritanceFlags::BOTH,
        propagation: PropagationFlags::NONE,
        is_inherited: false,
        is_orphan: false,
    };
    let key = e.diff_key();
    assert!(key.contains("BUILTIN\\Administrators"));
    assert!(key.contains("Allow"));
    // Calling twice must return the same string
    assert_eq!(key, e.diff_key());
}

#[test]
fn repair_stats_summary() {
    let mut s = RepairStats::default();
    s.total = 10;
    s.owner_ok = 9;
    s.owner_fail.push((PathBuf::from("a"), "err".into()));
    s.acl_ok = 10;
    assert_eq!(s.total_fail(), 1);
    assert!(s.summary().contains("10"));
}
#[test]
fn propagation_flags_display() {
    assert_eq!(PropagationFlags::NONE.to_string(), "None");
    assert_eq!(
        PropagationFlags::NO_PROPAGATE.to_string(),
        "NoPropagateInherit"
    );
    assert_eq!(PropagationFlags::INHERIT_ONLY.to_string(), "InheritOnly");
}

#[test]
fn ace_type_display() {
    assert_eq!(AceType::Allow.to_string(), "Allow");
    assert_eq!(AceType::Deny.to_string(), "Deny");
}

#[test]
fn tri_state_display() {
    assert_eq!(TriState::Allow.to_string(), "允许");
    assert_eq!(TriState::Deny.to_string(), "拒绝");
    assert_eq!(TriState::NoRule.to_string(), "无规则");
}

#[test]
fn acl_snapshot_counts() {
    let entries = vec![
        AceEntry {
            principal: "A".into(),
            raw_sid: "S-1-0".into(),
            rights_mask: 0x1F01FF,
            ace_type: AceType::Allow,
            inheritance: InheritanceFlags::NONE,
            propagation: PropagationFlags::NONE,
            is_inherited: false,
            is_orphan: false,
        },
        AceEntry {
            principal: "B".into(),
            raw_sid: "S-1-1".into(),
            rights_mask: 0x1200A9,
            ace_type: AceType::Deny,
            inheritance: InheritanceFlags::NONE,
            propagation: PropagationFlags::NONE,
            is_inherited: true,
            is_orphan: false,
        },
    ];
    let snap = AclSnapshot {
        path: PathBuf::from("C:\\t"),
        owner: "Admin".into(),
        is_protected: false,
        entries,
    };
    assert_eq!(snap.allow_count(), 1);
    assert_eq!(snap.deny_count(), 1);
    assert_eq!(snap.explicit_count(), 1);
    assert_eq!(snap.inherited_count(), 1);
}

#[test]
fn rights_desc_all_table_entries_nonempty() {
    for &(mask, _, _) in RIGHTS_TABLE {
        assert!(!rights_desc(mask).is_empty());
    }
}
