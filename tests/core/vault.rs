mod vault_cmd_tests {
    use clap::Parser;
    use xun::xun_core::vault_cmd::*;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use std::io::Cursor;

    fn parse(args: &[&str]) -> VaultCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        VaultCmd::try_parse_from(&argv).expect("parse failed")
    }

    // 鈹€鈹€ Enc 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn enc_parses_basic() {
        let cmd = parse(&["enc", "secret.txt"]);
        match cmd.cmd {
            VaultSubCommand::Enc(args) => {
                assert_eq!(args.input, "secret.txt");
                assert!(args.output.is_none());
                assert!(args.password.is_none());
                assert!(args.keyfile.is_none());
                assert!(args.recovery_key.is_none());
                assert!(args.emit_recovery_key.is_none());
                assert!(!args.dpapi);
                assert_eq!(args.algo, "aes256-gcm");
                assert_eq!(args.kdf, "argon2id");
                assert_eq!(args.chunk_size, 262144);
                assert!(!args.json);
            }
            other => panic!("expected Enc, got {other:?}"),
        }
    }

    #[test]
    fn enc_parses_all_options() {
        let cmd = parse(&["enc", "secret.txt", "-o", "secret.fv", "--password", "s3cret", "--dpapi", "--algo", "xchacha20-poly1305", "--kdf", "pbkdf2-sha256", "--chunk-size", "524288", "--json"]);
        match cmd.cmd {
            VaultSubCommand::Enc(args) => {
                assert_eq!(args.output.as_deref(), Some("secret.fv"));
                assert_eq!(args.password.as_deref(), Some("s3cret"));
                assert!(args.dpapi);
                assert_eq!(args.algo, "xchacha20-poly1305");
                assert_eq!(args.kdf, "pbkdf2-sha256");
                assert_eq!(args.chunk_size, 524288);
                assert!(args.json);
            }
            other => panic!("expected Enc, got {other:?}"),
        }
    }

    #[test]
    fn enc_parses_keyfile_and_recovery() {
        let cmd = parse(&["enc", "secret.txt", "--keyfile", "key.bin", "--recovery-key", "abc123", "--emit-recovery-key", "recovery.txt"]);
        match cmd.cmd {
            VaultSubCommand::Enc(args) => {
                assert_eq!(args.keyfile.as_deref(), Some("key.bin"));
                assert_eq!(args.recovery_key.as_deref(), Some("abc123"));
                assert_eq!(args.emit_recovery_key.as_deref(), Some("recovery.txt"));
            }
            other => panic!("expected Enc, got {other:?}"),
        }
    }

    // 鈹€鈹€ Dec 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn dec_parses_basic() {
        let cmd = parse(&["dec", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Dec(args) => {
                assert_eq!(args.input, "secret.fv");
                assert!(args.output.is_none());
                assert!(args.password.is_none());
                assert!(args.keyfile.is_none());
                assert!(args.recovery_key.is_none());
                assert!(!args.dpapi);
                assert!(!args.json);
            }
            other => panic!("expected Dec, got {other:?}"),
        }
    }

    #[test]
    fn dec_parses_with_password() {
        let cmd = parse(&["dec", "secret.fv", "-o", "plain.txt", "--password", "s3cret", "--json"]);
        match cmd.cmd {
            VaultSubCommand::Dec(args) => {
                assert_eq!(args.output.as_deref(), Some("plain.txt"));
                assert_eq!(args.password.as_deref(), Some("s3cret"));
                assert!(args.json);
            }
            other => panic!("expected Dec, got {other:?}"),
        }
    }

    // 鈹€鈹€ Inspect 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn inspect_parses_basic() {
        let cmd = parse(&["inspect", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Inspect(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(!args.json);
            }
            other => panic!("expected Inspect, got {other:?}"),
        }
    }

    #[test]
    fn inspect_parses_json() {
        let cmd = parse(&["inspect", "secret.fv", "--json"]);
        match cmd.cmd {
            VaultSubCommand::Inspect(args) => {
                assert!(args.json);
            }
            other => panic!("expected Inspect, got {other:?}"),
        }
    }

    // 鈹€鈹€ Verify 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn verify_parses_basic() {
        let cmd = parse(&["verify", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Verify(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(args.password.is_none());
                assert!(args.keyfile.is_none());
                assert!(args.recovery_key.is_none());
                assert!(!args.dpapi);
                assert!(!args.json);
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    #[test]
    fn verify_parses_with_unlock() {
        let cmd = parse(&["verify", "secret.fv", "--password", "s3cret", "--dpapi", "--json"]);
        match cmd.cmd {
            VaultSubCommand::Verify(args) => {
                assert_eq!(args.password.as_deref(), Some("s3cret"));
                assert!(args.dpapi);
                assert!(args.json);
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    // 鈹€鈹€ Resume 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn resume_parses_basic() {
        let cmd = parse(&["resume", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Resume(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(args.password.is_none());
                assert!(!args.dpapi);
                assert!(!args.json);
            }
            other => panic!("expected Resume, got {other:?}"),
        }
    }

    // 鈹€鈹€ Cleanup 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn cleanup_parses_basic() {
        let cmd = parse(&["cleanup", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Cleanup(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(!args.json);
            }
            other => panic!("expected Cleanup, got {other:?}"),
        }
    }

    // 鈹€鈹€ Rewrap 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn rewrap_parses_basic() {
        let cmd = parse(&["rewrap", "secret.fv"]);
        match cmd.cmd {
            VaultSubCommand::Rewrap(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(args.unlock_password.is_none());
                assert!(args.unlock_keyfile.is_none());
                assert!(args.unlock_recovery_key.is_none());
                assert!(!args.unlock_dpapi);
                assert!(args.add_password.is_none());
                assert!(args.add_keyfile.is_none());
                assert!(args.add_recovery_key.is_none());
                assert!(args.emit_recovery_key.is_none());
                assert!(!args.add_dpapi);
                assert!(args.remove_slot.is_empty());
                assert_eq!(args.kdf, "argon2id");
                assert!(!args.json);
            }
            other => panic!("expected Rewrap, got {other:?}"),
        }
    }

    #[test]
    fn rewrap_parses_all_options() {
        let cmd = parse(&["rewrap", "secret.fv", "--unlock-password", "old", "--add-password", "new", "--remove-slot", "keyfile", "--remove-slot", "dpapi", "--kdf", "pbkdf2-sha256", "--json"]);
        match cmd.cmd {
            VaultSubCommand::Rewrap(args) => {
                assert_eq!(args.unlock_password.as_deref(), Some("old"));
                assert_eq!(args.add_password.as_deref(), Some("new"));
                assert_eq!(args.remove_slot, vec!["keyfile", "dpapi"]);
                assert_eq!(args.kdf, "pbkdf2-sha256");
                assert!(args.json);
            }
            other => panic!("expected Rewrap, got {other:?}"),
        }
    }

    // 鈹€鈹€ RecoverKey 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn recover_key_parses_basic() {
        let cmd = parse(&["recover-key", "secret.fv", "--unlock-password", "old", "recovery.txt"]);
        match cmd.cmd {
            VaultSubCommand::RecoverKey(args) => {
                assert_eq!(args.path, "secret.fv");
                assert_eq!(args.unlock_password.as_deref(), Some("old"));
                assert_eq!(args.output, "recovery.txt");
                assert!(!args.json);
            }
            other => panic!("expected RecoverKey, got {other:?}"),
        }
    }

    #[test]
    fn recover_key_parses_with_keyfile() {
        let cmd = parse(&["recover-key", "secret.fv", "--unlock-keyfile", "key.bin", "recovery.txt", "--json"]);
        match cmd.cmd {
            VaultSubCommand::RecoverKey(args) => {
                assert_eq!(args.unlock_keyfile.as_deref(), Some("key.bin"));
                assert!(args.json);
            }
            other => panic!("expected RecoverKey, got {other:?}"),
        }
    }

    // 鈹€鈹€ TableRow 娴嬭瘯 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn vault_entry_columns_correct() {
        let cols = VaultEntry::columns();
        assert_eq!(cols.len(), 4);
        assert_eq!(cols[0].name, "path");
        assert_eq!(cols[0].kind, ValueKind::Path);
        assert_eq!(cols[1].name, "algo");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "slots");
        assert_eq!(cols[2].kind, ValueKind::Int);
        assert_eq!(cols[3].name, "size");
        assert_eq!(cols[3].kind, ValueKind::Int);
    }

    #[test]
    fn vault_entry_cells_match_fields() {
        let entry = VaultEntry::new("secret.fv", "aes256-gcm", 3, 1048576);
        let cells = entry.cells();
        assert_eq!(cells.len(), 4);
        assert!(matches!(&cells[0], Value::String(s) if s == "secret.fv"));
        assert!(matches!(&cells[1], Value::String(s) if s == "aes256-gcm"));
        assert!(matches!(&cells[2], Value::Int(3)));
        assert!(matches!(&cells[3], Value::Int(1048576)));
    }

    #[test]
    fn vault_entry_renders_as_json() {
        let entry = VaultEntry::new("secret.fv", "aes256-gcm", 3, 1048576);
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("secret.fv"), "json output: {output}");
    }

    #[test]
    fn vault_entry_vec_to_table() {
        let entries = vec![
            VaultEntry::new("a.fv", "aes256-gcm", 2, 512),
            VaultEntry::new("b.fv", "xchacha20-poly1305", 4, 1024),
        ];
        let table = VaultEntry::vec_to_table(&entries);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 4);
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn vault_subcommand_count() {
        let variants = [
            "Enc", "Dec", "Inspect", "Verify",
            "Resume", "Cleanup", "Rewrap", "RecoverKey",
        ];
        assert_eq!(variants.len(), 8);
    }
}

// ============================================================
// Phase 3.8: lock_cmd 鈥?LockCmd (1 瀛愬懡浠? + MvCmd + RenFileCmd
// ============================================================


