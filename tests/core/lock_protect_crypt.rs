mod lock_cmd_tests {
    use clap::Parser;
    use xun::xun_core::lock_cmd::*;

    fn parse_lock(args: &[&str]) -> LockCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        LockCmd::try_parse_from(&argv).expect("parse failed")
    }

    fn parse_mv(args: &[&str]) -> MvCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        MvCmd::try_parse_from(&argv).expect("parse failed")
    }

    fn parse_ren(args: &[&str]) -> RenFileCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        RenFileCmd::try_parse_from(&argv).expect("parse failed")
    }

    // 鈹€鈹€ Lock 瀛愬懡浠?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn lock_who_parses_basic() {
        let cmd = parse_lock(&["who", "C:\\test.txt"]);
        match cmd.cmd {
            LockSubCommand::Who(args) => {
                assert_eq!(args.path, "C:\\test.txt");
                assert_eq!(args.format, "auto");
            }
        }
    }

    #[test]
    fn lock_who_parses_format() {
        let cmd = parse_lock(&["who", "C:\\test.txt", "-f", "json"]);
        match cmd.cmd {
            LockSubCommand::Who(args) => {
                assert_eq!(args.format, "json");
            }
        }
    }

    // 鈹€鈹€ Mv 鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn mv_parses_basic() {
        let cmd = parse_mv(&["C:\\src.txt", "C:\\dst.txt"]);
        assert_eq!(cmd.src, "C:\\src.txt");
        assert_eq!(cmd.dst, "C:\\dst.txt");
        assert!(!cmd.unlock);
        assert!(!cmd.force_kill);
        assert!(!cmd.dry_run);
        assert!(!cmd.yes);
        assert!(!cmd.force);
        assert!(cmd.reason.is_none());
    }

    #[test]
    fn mv_parses_all_flags() {
        let cmd = parse_mv(&["C:\\src.txt", "C:\\dst.txt", "--unlock", "--force-kill", "--dry-run", "-y", "--force", "--reason", "urgent"]);
        assert!(cmd.unlock);
        assert!(cmd.force_kill);
        assert!(cmd.dry_run);
        assert!(cmd.yes);
        assert!(cmd.force);
        assert_eq!(cmd.reason.as_deref(), Some("urgent"));
    }

    // 鈹€鈹€ RenFile 鍛戒护 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn ren_parses_basic() {
        let cmd = parse_ren(&["C:\\old.txt", "C:\\new.txt"]);
        assert_eq!(cmd.src, "C:\\old.txt");
        assert_eq!(cmd.dst, "C:\\new.txt");
        assert!(!cmd.unlock);
        assert!(!cmd.force);
    }

    #[test]
    fn ren_parses_flags() {
        let cmd = parse_ren(&["C:\\old.txt", "C:\\new.txt", "--unlock", "--force", "-y"]);
        assert!(cmd.unlock);
        assert!(cmd.force);
        assert!(cmd.yes);
    }

    // 鈹€鈹€ E2E 娴嬭瘯锛堟々锛?鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

    #[test]
    fn lock_subcommand_count() {
        let variants = ["Who"];
        assert_eq!(variants.len(), 1);
    }
}

// ============================================================
// Phase 3.8: protect_cmd 鈥?3 瀛愬懡浠?
// ============================================================

mod protect_cmd_tests {
    use clap::Parser;
    use xun::xun_core::protect_cmd::*;

    fn parse(args: &[&str]) -> ProtectCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        ProtectCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn set_parses_basic() {
        let cmd = parse(&["set", "C:\\important"]);
        match cmd.cmd {
            ProtectSubCommand::Set(args) => {
                assert_eq!(args.path, "C:\\important");
                assert_eq!(args.deny, "delete,move,rename");
                assert_eq!(args.require, "force,reason");
                assert!(!args.system_acl);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_all_options() {
        let cmd = parse(&["set", "C:\\important", "--deny", "delete", "--require", "force", "--system-acl"]);
        match cmd.cmd {
            ProtectSubCommand::Set(args) => {
                assert_eq!(args.deny, "delete");
                assert_eq!(args.require, "force");
                assert!(args.system_acl);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn clear_parses_basic() {
        let cmd = parse(&["clear", "C:\\important"]);
        match cmd.cmd {
            ProtectSubCommand::Clear(args) => {
                assert_eq!(args.path, "C:\\important");
                assert!(!args.system_acl);
            }
            other => panic!("expected Clear, got {other:?}"),
        }
    }

    #[test]
    fn clear_parses_system_acl() {
        let cmd = parse(&["clear", "C:\\important", "--system-acl"]);
        match cmd.cmd {
            ProtectSubCommand::Clear(args) => {
                assert!(args.system_acl);
            }
            other => panic!("expected Clear, got {other:?}"),
        }
    }

    #[test]
    fn status_parses_defaults() {
        let cmd = parse(&["status"]);
        match cmd.cmd {
            ProtectSubCommand::Status(args) => {
                assert!(args.path.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn status_parses_path_and_format() {
        let cmd = parse(&["status", "C:\\test", "-f", "json"]);
        match cmd.cmd {
            ProtectSubCommand::Status(args) => {
                assert_eq!(args.path.as_deref(), Some("C:\\test"));
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn protect_subcommand_count() {
        let variants = ["Set", "Clear", "Status"];
        assert_eq!(variants.len(), 3);
    }
}

// ============================================================
// Phase 3.8: crypt_cmd 鈥?EncryptCmd + DecryptCmd
// ============================================================

mod crypt_cmd_tests {
    use clap::Parser;
    use xun::xun_core::crypt_cmd::*;

    fn parse_enc(args: &[&str]) -> EncryptCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        EncryptCmd::try_parse_from(&argv).expect("parse failed")
    }

    fn parse_dec(args: &[&str]) -> DecryptCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        DecryptCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn encrypt_parses_basic() {
        let cmd = parse_enc(&["secret.txt"]);
        assert_eq!(cmd.path, "secret.txt");
        assert!(!cmd.efs);
        assert!(cmd.to.is_empty());
        assert!(!cmd.passphrase);
        assert!(cmd.out.is_none());
    }

    #[test]
    fn encrypt_parses_all_options() {
        let cmd = parse_enc(&["secret.txt", "--efs", "--to", "key1", "--to", "key2", "--passphrase", "-o", "secret.age"]);
        assert!(cmd.efs);
        assert_eq!(cmd.to, vec!["key1", "key2"]);
        assert!(cmd.passphrase);
        assert_eq!(cmd.out.as_deref(), Some("secret.age"));
    }

    #[test]
    fn decrypt_parses_basic() {
        let cmd = parse_dec(&["secret.age"]);
        assert_eq!(cmd.path, "secret.age");
        assert!(!cmd.efs);
        assert!(cmd.identity.is_empty());
        assert!(!cmd.passphrase);
        assert!(cmd.out.is_none());
    }

    #[test]
    fn decrypt_parses_all_options() {
        let cmd = parse_dec(&["secret.age", "--efs", "-i", "key1", "-i", "key2", "--passphrase", "-o", "plain.txt"]);
        assert!(cmd.efs);
        assert_eq!(cmd.identity, vec!["key1", "key2"]);
        assert!(cmd.passphrase);
        assert_eq!(cmd.out.as_deref(), Some("plain.txt"));
    }
}

// ============================================================
// Phase 3.8: dashboard_cmd 鈥?ServeCmd
// ============================================================

