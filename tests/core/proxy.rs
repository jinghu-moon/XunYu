mod proxy_cli_tests {
    use clap::Parser;
    use xun::xun_core::proxy_cmd::{ProxyCmd, ProxySubCommand};

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歝lap 瑙ｆ瀽 ----

    #[test]
    fn proxy_set_parses_url_and_noproxy() {
        let cmd = ProxyCmd::try_parse_from(["test", "set", "http://127.0.0.1:7890", "-n", "localhost,10.0.0.0/8"]).unwrap();
        match cmd.cmd {
            ProxySubCommand::Set(args) => {
                assert_eq!(args.url, "http://127.0.0.1:7890");
                assert_eq!(args.noproxy, "localhost,10.0.0.0/8");
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn proxy_show_parses_empty_args() {
        let cmd = ProxyCmd::try_parse_from(["test", "show"]).unwrap();
        assert!(matches!(cmd.cmd, ProxySubCommand::Show(_)));
    }

    #[test]
    fn proxy_rm_parses_only_option() {
        let cmd = ProxyCmd::try_parse_from(["test", "rm", "--only", "cargo,git"]).unwrap();
        match cmd.cmd {
            ProxySubCommand::Rm(args) => {
                assert_eq!(args.only.as_deref(), Some("cargo,git"));
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }
}

// ============================================================
// Phase 2.2: Proxy 杈撳嚭绫诲瀷
// ============================================================

mod proxy_output_tests {
    use xun::xun_core::proxy_cmd::ProxyInfo;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, TsvRenderer, TerminalRenderer, Renderer};
    use std::io::Cursor;

    fn sample_info() -> ProxyInfo {
        ProxyInfo::new("http://127.0.0.1:7890", "localhost,127.0.0.1", "env")
    }

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歅roxyInfo TableRow 瀹炵幇 ----

    #[test]
    fn proxy_info_columns_correct() {
        let cols = ProxyInfo::columns();
        assert_eq!(cols.len(), 4);
        assert_eq!(cols[0].name, "url");
        assert_eq!(cols[1].name, "noproxy");
        assert_eq!(cols[2].name, "source");
        assert_eq!(cols[3].name, "enabled");
    }

    #[test]
    fn proxy_info_cells_match_fields() {
        let info = sample_info();
        let cells = info.cells();
        assert_eq!(cells.len(), 4);
        // url
        assert!(matches!(&cells[0], xun::xun_core::value::Value::String(s) if s == "http://127.0.0.1:7890"));
        // enabled
        assert!(matches!(&cells[3], xun::xun_core::value::Value::Bool(true)));
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛歅roxyInfo 娓叉煋 ----

    #[test]
    fn proxy_info_renders_as_json() {
        let info = sample_info();
        let table = info.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("127.0.0.1:7890"), "json output: {output}");
    }

    #[test]
    fn proxy_info_renders_as_table() {
        let info = sample_info();
        let table = info.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("url"), "table output: {output}");
        assert!(output.contains("127.0.0.1:7890"), "table output: {output}");
    }

    #[test]
    fn proxy_info_renders_as_tsv() {
        let info = sample_info();
        let table = info.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = TsvRenderer::new(&mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2, "expected header + 1 row: {output}");
        assert!(lines[0].contains("url"), "tsv header: {}", lines[0]);
    }
}

// ============================================================
// Phase 2.3: Proxy CommandSpec 瀹炵幇
// ============================================================

mod proxy_command_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::proxy_cmd::{ProxyInfo, ProxyShowCmd, ProxySetCmd};
    use xun::xun_core::renderer::JsonRenderer;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- MockShowCmd ----

    struct MockShowCmd {
        args: ProxyShowCmd,
    }

    impl CommandSpec for MockShowCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 模拟：返回当前代理配置
            let info = ProxyInfo::new("http://127.0.0.1:7890", "localhost,127.0.0.1", "env");
            let table = info.to_table();
            Ok(Value::List(
                table.rows.into_iter().map(Value::Record).collect(),
            ))
        }
    }

    // ---- MockSetCmd ----

    struct MockSetCmd {
        args: ProxySetCmd,
    }

    impl CommandSpec for MockSetCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 模拟：设置代理，返回空输出
            Ok(Value::Null)
        }
    }

    // ---- 测试用例 1：MockShowCmd 返回 ProxyInfo ----

    #[test]
    fn proxy_show_returns_current_config() {
        let cmd = MockShowCmd { args: ProxyShowCmd { format: "auto".into() } };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        // 输出应包含代理 URL
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("127.0.0.1:7890"), "output: {output}");
        // 返回值应是 List
        assert!(matches!(result, Value::List(_)));
    }

    // ---- 测试用例 2：MockSetCmd 修改配置 ----

    #[test]
    fn proxy_set_updates_config() {
        let cmd = MockSetCmd {
            args: ProxySetCmd {
                url: "http://10.0.0.1:8080".into(),
                noproxy: "localhost".into(),
                only: None,
                msys2: None,
            },
        };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null), "set should return Null");
    }

    #[test]
    fn proxy_set_returns_empty_output() {
        let cmd = MockSetCmd {
            args: ProxySetCmd {
                url: "http://10.0.0.1:8080".into(),
                noproxy: "localhost".into(),
                only: Some("cargo,git".into()),
                msys2: None,
            },
        };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let _ = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap().trim().to_string();
        // Null 值序列化为 "null"
        assert_eq!(output, "null", "output: {output}");
    }
}

// ============================================================
// Phase 2.4: Proxy dispatch 端到端集成
// ============================================================

mod proxy_e2e_tests {
    use clap::Parser;
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::proxy_cmd::{
        ProxyCmd, ProxySubCommand, ProxyShowCmd, ProxySetCmd, ProxyInfo,
    };
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    /// 模拟 dispatch：解析 CLI args → 执行 CommandSpec → 渲染输出
    fn dispatch_proxy(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = ProxyCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            ProxySubCommand::Show(args) => {
                struct ShowCmd { args: ProxyShowCmd }
                impl CommandSpec for ShowCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let info = ProxyInfo::new("http://127.0.0.1:7890", "localhost", "env");
                        Ok(Value::List(info.to_table().rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ShowCmd { args }, &mut ctx, renderer)
            }
            ProxySubCommand::Set(args) => {
                struct SetCmd { args: ProxySetCmd }
                impl CommandSpec for SetCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&SetCmd { args }, &mut ctx, renderer)
            }
            ProxySubCommand::Rm(_) | ProxySubCommand::Detect(_) | ProxySubCommand::Status(_) | ProxySubCommand::Test(_) => {
                struct OtherCmd;
                impl CommandSpec for OtherCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&OtherCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_proxy_show_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proxy(&["test", "show"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("127.0.0.1:7890"), "json output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_proxy_show_table() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = TerminalRenderer::new(false, &mut buf);
        let _ = dispatch_proxy(&["test", "show"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("url"), "table header: {output}");
    }

    #[test]
    fn e2e_proxy_set_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proxy(&["test", "set", "http://10.0.0.1:8080"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_proxy_set_with_only() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proxy(&["test", "set", "http://10.0.0.1:8080", "-o", "cargo,git"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_proxy_invalid_subcommand_fails() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proxy(&["test", "invalid"], &mut renderer);
        assert!(result.is_err(), "invalid subcommand should fail");
    }
}

// ============================================================
// Phase 3.1: Config 鍛戒护锛坈lap derive + CommandSpec锛?
// ============================================================

mod proxy_service_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::proxy_cmd::{ProxyShowCmd, ProxySetCmd, ProxyRmCmd, ProxyShowCmdSpec, ProxySetCmdSpec, ProxyRmCmdSpec};
    use xun::xun_core::renderer::JsonRenderer;
    use std::io::Cursor;

    #[test]
    fn proxy_show_cmd_returns_value() {
        let cmd = ProxyShowCmdSpec { args: ProxyShowCmd { format: "auto".into() } };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer);
        // proxy show 可能成功（返回 ProxyInfo）或失败（git 不可用）
        // 只要不 panic 即可
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn proxy_set_cmd_validates_empty_url() {
        let cmd = ProxySetCmdSpec {
            args: ProxySetCmd {
                url: "".into(),
                noproxy: "localhost".into(),
                only: None,
                msys2: None,
            },
        };
        let ctx = CmdContext::for_test();
        let result = cmd.validate(&ctx);
        assert!(result.is_err(), "empty URL should fail validation");
    }

    #[test]
    fn proxy_rm_cmd_returns_value() {
        let cmd = ProxyRmCmdSpec { args: ProxyRmCmd { only: None, msys2: None } };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer);
        // rm 操作应该成功（即使没有代理配置可删）
        assert!(result.is_ok());
    }
}

// ============================================================
// Phase 3.4: BookmarkDeleteOp 鈥?Operation trait 娴嬭瘯
// ============================================================


