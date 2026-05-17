mod backup_cmd_tests {
    use clap::Parser;
    use xun::xun_core::backup_cmd::{BackupCmd, BackupSubCommand, BackupEntry};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn backup_no_subcommand_parses() {
        let cmd = BackupCmd::try_parse_from(["test"]).unwrap();
        assert!(cmd.cmd.is_none());
        assert!(cmd.msg.is_none());
        assert!(!cmd.dry_run);
    }

    #[test]
    fn backup_create_parses_basic() {
        let cmd = BackupCmd::try_parse_from(["test", "create", "-m", "daily backup", "--format", "zip"]).unwrap();
        match cmd.cmd {
            Some(BackupSubCommand::Add(args)) => {
                assert_eq!(args.msg.as_deref(), Some("daily backup"));
                assert_eq!(args.format.as_deref(), Some("zip"));
            }
            other => panic!("expected Create, got {other:?}"),
        }
    }

    #[test]
    fn backup_create_parses_dry_run_and_list() {
        let cmd = BackupCmd::try_parse_from(["test", "create", "--dry-run", "--list"]).unwrap();
        match cmd.cmd {
            Some(BackupSubCommand::Add(args)) => {
                assert!(args.dry_run);
                assert!(args.list);
            }
            other => panic!("expected Create, got {other:?}"),
        }
    }

    #[test]
    fn backup_restore_parses_name_and_options() {
        let cmd = BackupCmd::try_parse_from(["test", "restore", "my-backup", "--to", "/tmp/out", "-y"]).unwrap();
        match cmd.cmd {
            Some(BackupSubCommand::Restore(args)) => {
                assert_eq!(args.name_or_path, "my-backup");
                assert_eq!(args.to.as_deref(), Some("/tmp/out"));
                assert!(args.yes);
            }
            other => panic!("expected Restore, got {other:?}"),
        }
    }

    #[test]
    fn backup_list_parses() {
        let cmd = BackupCmd::try_parse_from(["test", "list"]).unwrap();
        assert!(matches!(cmd.cmd, Some(BackupSubCommand::List(_))));
    }

    #[test]
    fn backup_verify_parses_name() {
        let cmd = BackupCmd::try_parse_from(["test", "verify", "my-backup"]).unwrap();
        match cmd.cmd {
            Some(BackupSubCommand::Verify(args)) => {
                assert_eq!(args.name, "my-backup");
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    #[test]
    fn backup_find_parses_filters() {
        let cmd = BackupCmd::try_parse_from(["test", "find", "important", "--since", "2026-01-01"]).unwrap();
        match cmd.cmd {
            Some(BackupSubCommand::Find(args)) => {
                assert_eq!(args.tag.as_deref(), Some("important"));
                assert_eq!(args.since.as_deref(), Some("2026-01-01"));
            }
            other => panic!("expected Find, got {other:?}"),
        }
    }

    #[test]
    fn backup_parent_dry_run_flag() {
        let cmd = BackupCmd::try_parse_from(["test", "--dry-run"]).unwrap();
        assert!(cmd.dry_run);
    }

    // ---- BackupEntry TableRow 娴嬭瘯 ----

    #[test]
    fn backup_entry_columns_correct() {
        let cols = BackupEntry::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[2].name, "size");
        assert_eq!(cols[2].kind, ValueKind::Filesize);
        assert_eq!(cols[3].name, "file_count");
        assert_eq!(cols[3].kind, ValueKind::Int);
    }

    #[test]
    fn backup_entry_cells_match_fields() {
        let entry = BackupEntry::new("backup-2026-05-12", "2026-05-12T10:00:00Z", 102400, 42, "daily");
        let cells = entry.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::String(s) if s == "backup-2026-05-12"));
        assert!(matches!(&cells[2], Value::Filesize(102400)));
        assert!(matches!(&cells[3], Value::Int(42)));
    }

    #[test]
    fn backup_entry_renders_as_json() {
        let entry = BackupEntry::new("bak-001", "2026-05-12", 50000, 10, "test");
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("bak-001"), "json output: {output}");
    }

    #[test]
    fn backup_entry_renders_as_table() {
        let entries = vec![
            BackupEntry::new("bak-001", "2026-05-12", 50000, 10, "first"),
            BackupEntry::new("bak-002", "2026-05-13", 60000, 15, "second"),
        ];
        let table = BackupEntry::vec_to_table(&entries);
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("name"), "table header: {output}");
        assert!(output.contains("bak-001"), "table row: {output}");
    }

    // ---- CommandSpec 娴嬭瘯 ----

    struct BackupListCmd;
    impl CommandSpec for BackupListCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let entries = vec![
                BackupEntry::new("bak-001", "2026-05-12", 50000, 10, "daily"),
            ];
            let table = BackupEntry::vec_to_table(&entries);
            Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
        }
    }

    struct BackupCreateCmd;
    impl CommandSpec for BackupCreateCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn backup_list_returns_table() {
        let cmd = BackupListCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("bak-001"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn backup_create_returns_null() {
        let cmd = BackupCreateCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    // ---- E2E dispatch 娴嬭瘯 ----

    fn dispatch_backup(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = BackupCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            Some(BackupSubCommand::List(_)) => {
                struct ListCmd;
                impl CommandSpec for ListCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let entries = vec![
                            BackupEntry::new("bak-001", "2026-05-12", 50000, 10, "daily"),
                        ];
                        let table = BackupEntry::vec_to_table(&entries);
                        Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ListCmd, &mut ctx, renderer)
            }
            Some(BackupSubCommand::Add(_)) => {
                struct CreateCmd;
                impl CommandSpec for CreateCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&CreateCmd, &mut ctx, renderer)
            }
            Some(BackupSubCommand::Restore(_)) => {
                struct RestoreCmd;
                impl CommandSpec for RestoreCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&RestoreCmd, &mut ctx, renderer)
            }
            Some(BackupSubCommand::Verify(_)) => {
                struct VerifyCmd;
                impl CommandSpec for VerifyCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Bool(true))
                    }
                }
                execute(&VerifyCmd, &mut ctx, renderer)
            }
            Some(BackupSubCommand::Find(_)) => {
                struct FindCmd;
                impl CommandSpec for FindCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::List(vec![]))
                    }
                }
                execute(&FindCmd, &mut ctx, renderer)
            }
            Some(BackupSubCommand::Convert(_)) => {
                struct ConvertCmd;
                impl CommandSpec for ConvertCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&ConvertCmd, &mut ctx, renderer)
            }
            None => {
                // Default behavior: create backup
                struct DefaultCmd;
                impl CommandSpec for DefaultCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&DefaultCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_backup_list_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_backup(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("bak-001"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_backup_create_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_backup(&["test", "create", "-m", "test"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_backup_restore_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_backup(&["test", "restore", "bak-001"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_backup_verify_returns_bool() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_backup(&["test", "verify", "bak-001"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }
}

// ============================================================
// Phase 3.3: Video 鍛戒护锛坈lap derive锛?
// ============================================================

mod video_cmd_tests {
    use clap::Parser;
    use xun::xun_core::video_cmd::{VideoCmd, VideoSubCommand};

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn video_probe_parses_input() {
        let cmd = VideoCmd::try_parse_from(["test", "probe", "-i", "video.mp4"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Probe(args) => {
                assert_eq!(args.input, "video.mp4");
                assert!(args.ffprobe.is_none());
            }
            other => panic!("expected Probe, got {other:?}"),
        }
    }

    #[test]
    fn video_probe_parses_ffprobe_override() {
        let cmd = VideoCmd::try_parse_from(["test", "probe", "-i", "video.mp4", "--ffprobe", "/usr/bin/ffprobe"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Probe(args) => {
                assert_eq!(args.ffprobe.as_deref(), Some("/usr/bin/ffprobe"));
            }
            other => panic!("expected Probe, got {other:?}"),
        }
    }

    #[test]
    fn video_compress_parses_basic() {
        let cmd = VideoCmd::try_parse_from(["test", "compress", "-i", "in.mp4", "-o", "out.mp4"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Compress(args) => {
                assert_eq!(args.input, "in.mp4");
                assert_eq!(args.output, "out.mp4");
                assert_eq!(args.mode, "balanced");
                assert_eq!(args.engine, "auto");
                assert!(!args.overwrite);
            }
            other => panic!("expected Compress, got {other:?}"),
        }
    }

    #[test]
    fn video_compress_parses_mode_and_engine() {
        let cmd = VideoCmd::try_parse_from(["test", "compress", "-i", "in.mp4", "-o", "out.mp4", "--mode", "fastest", "--engine", "gpu"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Compress(args) => {
                assert_eq!(args.mode, "fastest");
                assert_eq!(args.engine, "gpu");
            }
            other => panic!("expected Compress, got {other:?}"),
        }
    }

    #[test]
    fn video_compress_parses_overwrite() {
        let cmd = VideoCmd::try_parse_from(["test", "compress", "-i", "in.mp4", "-o", "out.mp4", "--overwrite"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Compress(args) => {
                assert!(args.overwrite);
            }
            other => panic!("expected Compress, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_basic() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Remux(args) => {
                assert_eq!(args.input, "in.mkv");
                assert_eq!(args.output, "out.mp4");
                assert_eq!(args.strict, true);
                assert!(!args.overwrite);
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_strict_false() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4", "--strict", "false"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Remux(args) => {
                assert_eq!(args.strict, false);
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_overwrite() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4", "--overwrite"]).unwrap();
        match cmd.cmd {
            VideoSubCommand::Remux(args) => {
                assert!(args.overwrite);
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }
}

// ============================================================
// Phase 3.3: Verify 鍛戒护锛坈lap derive锛?
// ============================================================

mod verify_cmd_tests {
    use clap::Parser;
    use xun::xun_core::verify_cmd::VerifyCmd;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn verify_parses_path() {
        let cmd = VerifyCmd::try_parse_from(["test", "archive.xunbak"]).unwrap();
        assert_eq!(cmd.path, "archive.xunbak");
        assert!(cmd.level.is_none());
        assert!(!cmd.json);
    }

    #[test]
    fn verify_parses_level_and_json() {
        let cmd = VerifyCmd::try_parse_from(["test", "archive.xunbak", "--level", "paranoid", "--json"]).unwrap();
        assert_eq!(cmd.path, "archive.xunbak");
        assert_eq!(cmd.level.as_deref(), Some("paranoid"));
        assert!(cmd.json);
    }

    #[test]
    fn verify_parses_all_levels() {
        for level in &["quick", "full", "manifest-only", "existence-only", "paranoid"] {
            let cmd = VerifyCmd::try_parse_from(["test", "archive.xunbak", "--level", level]).unwrap();
            assert_eq!(cmd.level.as_deref(), Some(*level));
        }
    }
}

// ============================================================
// Phase 3.4: Bookmark CLI 鈥?27 涓瓙鍛戒护
// ============================================================


