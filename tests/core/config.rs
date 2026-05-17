mod config_cmd_tests {
    use clap::Parser;
    use xun::xun_core::config_cmd::{ConfigCmd, ConfigSubCommand, ConfigGetCmd, ConfigEntry};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn config_get_parses_key() {
        let cmd = ConfigCmd::try_parse_from(["test", "get", "proxy.defaultUrl"]).unwrap();
        match cmd.cmd {
            ConfigSubCommand::Get(args) => assert_eq!(args.key, "proxy.defaultUrl"),
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn config_set_parses_key_and_value() {
        let cmd = ConfigCmd::try_parse_from(["test", "set", "tree.defaultDepth", "3"]).unwrap();
        match cmd.cmd {
            ConfigSubCommand::Set(args) => {
                assert_eq!(args.key, "tree.defaultDepth");
                assert_eq!(args.value, "3");
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn config_edit_parses_empty() {
        let cmd = ConfigCmd::try_parse_from(["test", "edit"]).unwrap();
        assert!(matches!(cmd.cmd, ConfigSubCommand::Edit(_)));
    }

    // ---- CommandSpec 娴嬭瘯 ----

    struct MockConfigGetCmd {
        args: ConfigGetCmd,
    }

    impl CommandSpec for MockConfigGetCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 妯℃嫙锛氳繑鍥為厤缃潯鐩?
            let entry = ConfigEntry::new(&self.args.key, "mock_value");
            Ok(Value::Record(entry.to_record()))
        }
    }

    #[test]
    fn config_get_returns_entry() {
        let cmd = MockConfigGetCmd { args: ConfigGetCmd { key: "proxy.defaultUrl".into() } };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("proxy.defaultUrl"), "output: {output}");
        assert!(matches!(result, Value::Record(_)));
    }

    #[test]
    fn config_entry_table_row() {
        let entry = ConfigEntry::new("proxy.url", "http://127.0.0.1:7890");
        let cols = ConfigEntry::columns();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "key");
        assert_eq!(cols[1].name, "value");
        let cells = entry.cells();
        assert!(matches!(&cells[0], Value::String(s) if s == "proxy.url"));
    }

    // ---- E2E dispatch 娴嬭瘯 ----

    fn dispatch_config(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = ConfigCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            ConfigSubCommand::Get(args) => {
                struct GetCmd { args: ConfigGetCmd }
                impl CommandSpec for GetCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let entry = ConfigEntry::new(&self.args.key, "mock_value");
                        Ok(Value::Record(entry.to_record()))
                    }
                }
                execute(&GetCmd { args }, &mut ctx, renderer)
            }
            ConfigSubCommand::Set(_args) => {
                struct SetCmd;
                impl CommandSpec for SetCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&SetCmd, &mut ctx, renderer)
            }
            ConfigSubCommand::Edit(_) => {
                struct EditCmd;
                impl CommandSpec for EditCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::String("opened editor".into()))
                    }
                }
                execute(&EditCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_config_get_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_config(&["test", "get", "proxy.url"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("proxy.url"), "output: {output}");
        assert!(matches!(result, Value::Record(_)));
    }

    #[test]
    fn e2e_config_set_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_config(&["test", "set", "tree.depth", "5"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }
}

// ============================================================
// Phase 3.1: Tree 鍛戒护锛坈lap derive + CommandSpec锛?
// ============================================================


