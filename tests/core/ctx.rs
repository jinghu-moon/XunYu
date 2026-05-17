mod ctx_cmd_tests {
    use clap::Parser;
    use xun::xun_core::ctx_cmd::{
        CtxCmd, CtxSubCommand, CtxSetCmd, CtxProfile,
    };
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn ctx_set_parses_name_and_path() {
        let cmd = CtxCmd::try_parse_from(["test", "set", "work", "--path", "/projects"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Set(args) => {
                assert_eq!(args.name, "work");
                assert_eq!(args.path.as_deref(), Some("/projects"));
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn ctx_set_parses_proxy_and_tag() {
        let cmd = CtxCmd::try_parse_from(["test", "set", "corp", "--proxy", "http://10.0.0.1:8080", "-t", "dev,staging"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Set(args) => {
                assert_eq!(args.proxy.as_deref(), Some("http://10.0.0.1:8080"));
                assert_eq!(args.tag.as_deref(), Some("dev,staging"));
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn ctx_set_parses_env_args() {
        let cmd = CtxCmd::try_parse_from(["test", "set", "dev", "--env", "NODE_ENV=development", "--env", "DEBUG=true"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Set(args) => {
                assert_eq!(args.env.len(), 2);
                assert_eq!(args.env[0], "NODE_ENV=development");
                assert_eq!(args.env[1], "DEBUG=true");
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn ctx_use_parses_name() {
        let cmd = CtxCmd::try_parse_from(["test", "use", "work"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Use(args) => assert_eq!(args.name, "work"),
            other => panic!("expected Use, got {other:?}"),
        }
    }

    #[test]
    fn ctx_off_parses_empty() {
        let cmd = CtxCmd::try_parse_from(["test", "off"]).unwrap();
        assert!(matches!(cmd.cmd, CtxSubCommand::Off(_)));
    }

    #[test]
    fn ctx_list_parses_format() {
        let cmd = CtxCmd::try_parse_from(["test", "list", "-f", "json"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::List(args) => assert_eq!(args.format, "json"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ctx_list_defaults_to_auto() {
        let cmd = CtxCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::List(args) => assert_eq!(args.format, "auto"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ctx_show_defaults_to_current() {
        let cmd = CtxCmd::try_parse_from(["test", "show"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Show(args) => {
                assert!(args.name.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn ctx_show_parses_name_and_format() {
        let cmd = CtxCmd::try_parse_from(["test", "show", "work", "-f", "json"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Show(args) => {
                assert_eq!(args.name.as_deref(), Some("work"));
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn ctx_del_parses_name() {
        let cmd = CtxCmd::try_parse_from(["test", "del", "old-project"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Del(args) => assert_eq!(args.name, "old-project"),
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn ctx_rename_parses_old_and_new() {
        let cmd = CtxCmd::try_parse_from(["test", "rename", "old", "new"]).unwrap();
        match cmd.cmd {
            CtxSubCommand::Rename(args) => {
                assert_eq!(args.old, "old");
                assert_eq!(args.new, "new");
            }
            other => panic!("expected Rename, got {other:?}"),
        }
    }

    // ---- CtxProfile TableRow 娴嬭瘯 ----

    #[test]
    fn ctx_profile_columns_correct() {
        let cols = CtxProfile::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "path");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "active");
        assert_eq!(cols[2].kind, ValueKind::Bool);
    }

    #[test]
    fn ctx_profile_cells_match_fields() {
        let profile = CtxProfile::new("work", "/projects/work", true);
        let cells = profile.cells();
        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0], Value::String(s) if s == "work"));
        assert!(matches!(&cells[1], Value::String(s) if s == "/projects/work"));
        assert!(matches!(&cells[2], Value::Bool(true)));
    }

    #[test]
    fn ctx_profile_to_record_roundtrip() {
        let profile = CtxProfile::new("dev", "/tmp/dev", false);
        let record = profile.to_record();
        assert!(record.contains_key("name"));
        assert!(record.contains_key("path"));
        assert!(record.contains_key("active"));
    }

    #[test]
    fn ctx_profile_vec_to_table() {
        let profiles = vec![
            CtxProfile::new("work", "/projects", true),
            CtxProfile::new("home", "/home/user", false),
        ];
        let table = CtxProfile::vec_to_table(&profiles);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.columns.len(), 3);
    }

    #[test]
    fn ctx_profile_renders_as_json() {
        let profile = CtxProfile::new("work", "/projects", true);
        let table = profile.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("work"), "json output: {output}");
        assert!(output.contains("/projects"), "json output: {output}");
    }

    #[test]
    fn ctx_profile_renders_as_terminal() {
        let profiles = vec![
            CtxProfile::new("work", "/projects", true),
            CtxProfile::new("home", "/home/user", false),
        ];
        let table = CtxProfile::vec_to_table(&profiles);
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("name"), "table header: {output}");
        assert!(output.contains("work"), "table row: {output}");
    }

    // ---- CommandSpec 娴嬭瘯 ----

    struct CtxListCmd;
    impl CommandSpec for CtxListCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let profiles = vec![
                CtxProfile::new("work", "/projects/work", true),
                CtxProfile::new("home", "/home/user", false),
            ];
            let table = CtxProfile::vec_to_table(&profiles);
            Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
        }
    }

    struct CtxShowCmd {
        name: Option<String>,
    }
    impl CommandSpec for CtxShowCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let name = self.name.as_deref().unwrap_or("work");
            let profile = CtxProfile::new(name, "/projects/work", true);
            Ok(Value::Record(profile.to_record()))
        }
    }

    struct MockCtxSetCmd {
        args: CtxSetCmd,
    }
    impl CommandSpec for MockCtxSetCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    struct CtxUseCmd { name: String }
    impl CommandSpec for CtxUseCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::String(format!("activated: {}", self.name)))
        }
    }

    struct CtxOffCmd;
    impl CommandSpec for CtxOffCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    struct CtxDelCmd { name: String }
    impl CommandSpec for CtxDelCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    struct CtxRenameCmd { old: String, new: String }
    impl CommandSpec for CtxRenameCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn ctx_list_returns_profiles() {
        let cmd = CtxListCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("work"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn ctx_show_returns_record() {
        let cmd = CtxShowCmd { name: Some("dev".into()) };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Record(_)));
    }

    #[test]
    fn ctx_set_returns_null() {
        let cmd = MockCtxSetCmd {
            args: CtxSetCmd {
                name: "work".into(),
                path: Some("/projects".into()),
                proxy: None,
                noproxy: None,
                tag: None,
                env: vec![],
                env_file: None,
            },
        };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn ctx_use_returns_activation_message() {
        let cmd = CtxUseCmd { name: "work".into() };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::String(s) if s.contains("work")));
    }

    #[test]
    fn ctx_off_returns_null() {
        let cmd = CtxOffCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    // ---- E2E dispatch 娴嬭瘯 ----

    fn dispatch_ctx(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = CtxCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            CtxSubCommand::Set(_args) => {
                struct SetCmd;
                impl CommandSpec for SetCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&SetCmd, &mut ctx, renderer)
            }
            CtxSubCommand::Use(args) => {
                struct UseCmd { name: String }
                impl CommandSpec for UseCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::String(format!("activated: {}", self.name)))
                    }
                }
                execute(&UseCmd { name: args.name }, &mut ctx, renderer)
            }
            CtxSubCommand::Off(_) => {
                struct OffCmd;
                impl CommandSpec for OffCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&OffCmd, &mut ctx, renderer)
            }
            CtxSubCommand::List(_) => {
                struct ListCmd;
                impl CommandSpec for ListCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let profiles = vec![
                            CtxProfile::new("work", "/projects", true),
                            CtxProfile::new("home", "/home/user", false),
                        ];
                        let table = CtxProfile::vec_to_table(&profiles);
                        Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ListCmd, &mut ctx, renderer)
            }
            CtxSubCommand::Show(args) => {
                struct ShowCmd { name: Option<String> }
                impl CommandSpec for ShowCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let name = self.name.as_deref().unwrap_or("work");
                        let profile = CtxProfile::new(name, "/projects/work", true);
                        Ok(Value::Record(profile.to_record()))
                    }
                }
                execute(&ShowCmd { name: args.name }, &mut ctx, renderer)
            }
            CtxSubCommand::Del(_args) => {
                struct DelCmd;
                impl CommandSpec for DelCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&DelCmd, &mut ctx, renderer)
            }
            CtxSubCommand::Rename(_args) => {
                struct RenameCmd;
                impl CommandSpec for RenameCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&RenameCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_ctx_list_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("work"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_ctx_list_table() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = TerminalRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("name"), "table header: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_ctx_show_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "show", "dev"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("dev"), "output: {output}");
        assert!(matches!(result, Value::Record(_)));
    }

    #[test]
    fn e2e_ctx_use_returns_activation() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "use", "work"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("work"), "output: {output}");
        assert!(matches!(result, Value::String(_)));
    }

    #[test]
    fn e2e_ctx_set_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "set", "work", "--path", "/projects"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_ctx_off_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "off"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_ctx_del_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "del", "old"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_ctx_rename_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "rename", "old", "new"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_ctx_invalid_subcommand_fails() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_ctx(&["test", "invalid"], &mut renderer);
        assert!(result.is_err(), "invalid subcommand should fail");
    }
}

// ============================================================
// Phase 3.2: Port 鍛戒护锛坈lap derive + CommandSpec + TableRow锛?
// ============================================================


