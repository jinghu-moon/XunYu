mod bookmark_operation_tests {
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::bookmark::BookmarkDeleteOp;

    #[test]
    fn bookmark_delete_op_preview_has_correct_risk() {
        let op = BookmarkDeleteOp::new("test-bookmark");
        assert_eq!(op.preview().risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn bookmark_delete_op_preview_has_changes() {
        let op = BookmarkDeleteOp::new("my-bm");
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "delete");
        assert_eq!(changes[0].target(), "my-bm");
    }

    #[test]
    fn bookmark_delete_op_preview_description() {
        let op = BookmarkDeleteOp::new("test");
        assert!(op.preview().description().contains("test"));
    }

    #[test]
    fn bookmark_delete_op_rollback_returns_error() {
        let op = BookmarkDeleteOp::new("test");
        let mut ctx = CmdContext::for_test();
        let result = op.rollback(&mut ctx);
        assert!(result.is_err());
    }
}

// ============================================================
// Phase 3.5: EnvSetOp / EnvDelOp 鈥?Operation trait 娴嬭瘯
// ============================================================

mod env_operation_tests {
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::env::{EnvSetOp, EnvDelOp};
    use xun::EnvScope;

    #[test]
    fn env_set_op_preview_has_correct_risk() {
        let op = EnvSetOp::new("TEST_VAR", "test_value", EnvScope::User);
        assert_eq!(op.preview().risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn env_set_op_preview_has_changes() {
        let op = EnvSetOp::new("MY_VAR", "hello", EnvScope::User);
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "set");
        assert!(changes[0].target().contains("MY_VAR"));
    }

    #[test]
    fn env_del_op_preview_has_correct_risk() {
        let op = EnvDelOp::new("TEST_VAR", EnvScope::User);
        assert_eq!(op.preview().risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn env_del_op_preview_has_changes() {
        let op = EnvDelOp::new("OLD_VAR", EnvScope::System);
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "delete");
        assert!(changes[0].target().contains("OLD_VAR"));
    }
}

// ============================================================
// Phase 3.6: AclAddOp / AclRemoveOp / AclRepairOp 鈥?Operation trait 娴嬭瘯
// ============================================================

mod acl_operation_tests {
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::acl::{AclAddOp, AclRemoveOp, AclRepairOp};

    #[test]
    fn acl_add_op_preview_has_high_risk() {
        let op = AclAddOp::new("C:\\test", "Users", "FullControl", "allow", "all");
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn acl_add_op_preview_has_changes() {
        let op = AclAddOp::new("C:\\test", "Users", "Read", "allow", "all");
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "add");
    }

    #[test]
    fn acl_remove_op_preview_has_high_risk() {
        let op = AclRemoveOp::new("C:\\test", "Everyone");
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn acl_repair_op_preview_has_critical_risk() {
        let op = AclRepairOp::new("C:\\test", true, None);
        assert_eq!(op.preview().risk_level(), RiskLevel::Critical);
    }
}

// ============================================================
// Phase 3.7: RenameOperation 鈥?Operation trait 娴嬭瘯
// ============================================================

#[cfg(feature = "batch_rename")]
mod brn_operation_tests {
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::brn::RenameOperation;

    #[test]
    fn rename_op_preview_has_high_risk() {
        let op = RenameOperation::new("C:\\files", "*.txt", "*.md");
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn rename_op_preview_has_changes() {
        let op = RenameOperation::new("C:\\files", "old", "new");
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "rename");
    }

    #[test]
    fn rename_op_preview_description_contains_params() {
        let op = RenameOperation::new("/tmp", "foo", "bar");
        let desc = op.preview().description();
        assert!(desc.contains("foo"));
        assert!(desc.contains("bar"));
    }
}

// ============================================================
// Phase 3.7: VaultEncOp / VaultDecOp 鈥?Operation trait 娴嬭瘯
// ============================================================

#[cfg(feature = "crypt")]
mod vault_operation_tests {
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::vault::{VaultEncOp, VaultDecOp};

    #[test]
    fn vault_enc_op_preview_has_high_risk() {
        let op = VaultEncOp::new("secret.txt", None);
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn vault_enc_op_preview_has_changes() {
        let op = VaultEncOp::new("data.bin", Some("data.enc".into()));
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "encrypt");
    }

    #[test]
    fn vault_dec_op_preview_has_high_risk() {
        let op = VaultDecOp::new("data.enc", None);
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn vault_dec_op_preview_has_changes() {
        let op = VaultDecOp::new("data.enc", None);
        let changes = op.preview().changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action(), "decrypt");
    }
}
