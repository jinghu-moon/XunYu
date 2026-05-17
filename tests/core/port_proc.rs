mod port_cmd_tests {
    use clap::Parser;
    use xun::xun_core::port_cmd::{PortsCmd, PortsSubCommand, PortInfo};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn port_list_parses_defaults() {
        let cmd = PortsCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.cmd {
            PortsSubCommand::List(args) => {
                assert!(!args.all);
                assert!(!args.udp);
                assert!(args.range.is_none());
                assert!(args.pid.is_none());
                assert!(args.name.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn port_list_parses_filters() {
        let cmd = PortsCmd::try_parse_from(["test", "list", "--all", "--udp", "--range", "3000-3999", "--pid", "1234", "--name", "node"]).unwrap();
        match cmd.cmd {
            PortsSubCommand::List(args) => {
                assert!(args.all);
                assert!(args.udp);
                assert_eq!(args.range.as_deref(), Some("3000-3999"));
                assert_eq!(args.pid, Some(1234));
                assert_eq!(args.name.as_deref(), Some("node"));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn port_list_parses_format() {
        let cmd = PortsCmd::try_parse_from(["test", "list", "-f", "json"]).unwrap();
        match cmd.cmd {
            PortsSubCommand::List(args) => assert_eq!(args.format, "json"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn port_kill_parses_ports_and_flags() {
        let cmd = PortsCmd::try_parse_from(["test", "kill", "3000,8080,5173", "-f", "--tcp"]).unwrap();
        match cmd.cmd {
            PortsSubCommand::Kill(args) => {
                assert_eq!(args.ports, "3000,8080,5173");
                assert!(args.force);
                assert!(args.tcp);
                assert!(!args.udp);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    #[test]
    fn port_kill_parses_udp() {
        let cmd = PortsCmd::try_parse_from(["test", "kill", "5353", "--udp"]).unwrap();
        match cmd.cmd {
            PortsSubCommand::Kill(args) => {
                assert_eq!(args.ports, "5353");
                assert!(args.udp);
                assert!(!args.force);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    // ---- PortInfo TableRow 娴嬭瘯 ----

    #[test]
    fn port_info_columns_correct() {
        let cols = PortInfo::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "port");
        assert_eq!(cols[0].kind, ValueKind::Int);
        assert_eq!(cols[1].name, "protocol");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "pid");
        assert_eq!(cols[2].kind, ValueKind::Int);
        assert_eq!(cols[3].name, "process_name");
        assert_eq!(cols[4].name, "local_addr");
    }

    #[test]
    fn port_info_cells_match_fields() {
        let info = PortInfo::new(3000, "tcp", 1234, "node", "0.0.0.0:3000");
        let cells = info.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::Int(3000)));
        assert!(matches!(&cells[1], Value::String(s) if s == "tcp"));
        assert!(matches!(&cells[2], Value::Int(1234)));
        assert!(matches!(&cells[3], Value::String(s) if s == "node"));
        assert!(matches!(&cells[4], Value::String(s) if s == "0.0.0.0:3000"));
    }

    #[test]
    fn port_info_renders_as_json() {
        let info = PortInfo::new(8080, "tcp", 5678, "java", "127.0.0.1:8080");
        let table = info.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("8080"), "json output: {output}");
        assert!(output.contains("java"), "json output: {output}");
    }

    #[test]
    fn port_info_renders_as_table() {
        let infos = vec![
            PortInfo::new(3000, "tcp", 100, "node", "0.0.0.0:3000"),
            PortInfo::new(5432, "tcp", 200, "postgres", "127.0.0.1:5432"),
        ];
        let table = PortInfo::vec_to_table(&infos);
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("port"), "table header: {output}");
        assert!(output.contains("3000"), "table row: {output}");
    }

    // ---- CommandSpec 娴嬭瘯 ----

    struct PortListCmd;
    impl CommandSpec for PortListCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let infos = vec![
                PortInfo::new(3000, "tcp", 100, "node", "0.0.0.0:3000"),
                PortInfo::new(8080, "tcp", 200, "java", "127.0.0.1:8080"),
            ];
            let table = PortInfo::vec_to_table(&infos);
            Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
        }
    }

    struct PortKillCmd { ports: String }
    impl CommandSpec for PortKillCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn port_list_returns_table() {
        let cmd = PortListCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("3000"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn port_kill_returns_null() {
        let cmd = PortKillCmd { ports: "3000,8080".into() };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    // ---- E2E dispatch 娴嬭瘯 ----

    fn dispatch_port(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = PortsCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            PortsSubCommand::List(_) => {
                struct ListCmd;
                impl CommandSpec for ListCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let infos = vec![
                            PortInfo::new(3000, "tcp", 100, "node", "0.0.0.0:3000"),
                        ];
                        let table = PortInfo::vec_to_table(&infos);
                        Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ListCmd, &mut ctx, renderer)
            }
            PortsSubCommand::Kill(_) => {
                struct KillCmd;
                impl CommandSpec for KillCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&KillCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_port_list_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_port(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("3000"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_port_list_table() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = TerminalRenderer::new(false, &mut buf);
        let result = dispatch_port(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("port"), "table header: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_port_kill_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_port(&["test", "kill", "3000,8080", "-f"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_port_invalid_subcommand_fails() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_port(&["test", "invalid"], &mut renderer);
        assert!(result.is_err(), "invalid subcommand should fail");
    }
}

// ============================================================
// Phase 3.2: Proc 鍛戒护锛坈lap derive + CommandSpec + TableRow锛?
// ============================================================

mod proc_cmd_tests {
    use clap::Parser;
    use xun::xun_core::proc_cmd::{PsCmd, PsSubCommand, ProcInfo};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

    #[test]
    fn proc_list_parses_defaults() {
        let cmd = PsCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.cmd {
            PsSubCommand::List(args) => {
                assert!(args.pattern.is_none());
                assert!(args.pid.is_none());
                assert!(args.win.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_list_parses_pattern() {
        let cmd = PsCmd::try_parse_from(["test", "list", "node"]).unwrap();
        match cmd.cmd {
            PsSubCommand::List(args) => {
                assert_eq!(args.pattern.as_deref(), Some("node"));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_list_parses_pid_and_win() {
        let cmd = PsCmd::try_parse_from(["test", "list", "--pid", "1234"]).unwrap();
        match cmd.cmd {
            PsSubCommand::List(args) => {
                assert_eq!(args.pid, Some(1234));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_list_parses_window_title() {
        let cmd = PsCmd::try_parse_from(["test", "list", "-w", "My App"]).unwrap();
        match cmd.cmd {
            PsSubCommand::List(args) => {
                assert_eq!(args.win.as_deref(), Some("My App"));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_kill_parses_target_and_flags() {
        let cmd = PsCmd::try_parse_from(["test", "kill", "node", "-f", "-w"]).unwrap();
        match cmd.cmd {
            PsSubCommand::Kill(args) => {
                assert_eq!(args.target, "node");
                assert!(args.force);
                assert!(args.window);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    #[test]
    fn proc_kill_parses_pid_target() {
        let cmd = PsCmd::try_parse_from(["test", "kill", "1234"]).unwrap();
        match cmd.cmd {
            PsSubCommand::Kill(args) => {
                assert_eq!(args.target, "1234");
                assert!(!args.force);
                assert!(!args.window);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    // ---- ProcInfo TableRow 娴嬭瘯 ----

    #[test]
    fn proc_info_columns_correct() {
        let cols = ProcInfo::columns();
        assert_eq!(cols.len(), 6);
        assert_eq!(cols[0].name, "pid");
        assert_eq!(cols[0].kind, ValueKind::Int);
        assert_eq!(cols[1].name, "ppid");
        assert_eq!(cols[2].name, "name");
        assert_eq!(cols[3].name, "exe_path");
        assert_eq!(cols[4].name, "thread_count");
        assert_eq!(cols[5].name, "window_title");
    }

    #[test]
    fn proc_info_cells_match_fields() {
        let info = ProcInfo::new(1234, 1, "node.exe", "C:\\node.exe", 8, "Node Server");
        let cells = info.cells();
        assert_eq!(cells.len(), 6);
        assert!(matches!(&cells[0], Value::Int(1234)));
        assert!(matches!(&cells[1], Value::Int(1)));
        assert!(matches!(&cells[2], Value::String(s) if s == "node.exe"));
        assert!(matches!(&cells[3], Value::String(s) if s == "C:\\node.exe"));
        assert!(matches!(&cells[4], Value::Int(8)));
        assert!(matches!(&cells[5], Value::String(s) if s == "Node Server"));
    }

    #[test]
    fn proc_info_renders_as_json() {
        let info = ProcInfo::new(5678, 1, "java.exe", "C:\\java.exe", 12, "");
        let table = info.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("5678"), "json output: {output}");
        assert!(output.contains("java.exe"), "json output: {output}");
    }

    #[test]
    fn proc_info_renders_as_table() {
        let infos = vec![
            ProcInfo::new(100, 1, "node.exe", "", 4, ""),
            ProcInfo::new(200, 1, "java.exe", "", 8, "Server"),
        ];
        let table = ProcInfo::vec_to_table(&infos);
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("pid"), "table header: {output}");
        assert!(output.contains("node.exe"), "table row: {output}");
    }

    // ---- CommandSpec 娴嬭瘯 ----

    struct ProcListCmd;
    impl CommandSpec for ProcListCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let infos = vec![
                ProcInfo::new(100, 1, "node.exe", "C:\\node.exe", 4, ""),
                ProcInfo::new(200, 1, "java.exe", "C:\\java.exe", 8, "Server"),
            ];
            let table = ProcInfo::vec_to_table(&infos);
            Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
        }
    }

    struct ProcKillCmd { target: String }
    impl CommandSpec for ProcKillCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn proc_list_returns_table() {
        let cmd = ProcListCmd;
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("node.exe"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn proc_kill_returns_null() {
        let cmd = ProcKillCmd { target: "node".into() };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    // ---- E2E dispatch 娴嬭瘯 ----

    fn dispatch_proc(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = PsCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.cmd {
            PsSubCommand::List(_) => {
                struct ListCmd;
                impl CommandSpec for ListCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let infos = vec![
                            ProcInfo::new(100, 1, "node.exe", "", 4, ""),
                        ];
                        let table = ProcInfo::vec_to_table(&infos);
                        Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ListCmd, &mut ctx, renderer)
            }
            PsSubCommand::Kill(_) => {
                struct KillCmd;
                impl CommandSpec for KillCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&KillCmd, &mut ctx, renderer)
            }
        }
    }

    #[test]
    fn e2e_proc_list_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proc(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("node.exe"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_proc_list_table() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = TerminalRenderer::new(false, &mut buf);
        let result = dispatch_proc(&["test", "list"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("pid"), "table header: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_proc_list_with_pattern() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proc(&["test", "list", "node"], &mut renderer).unwrap();
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn e2e_proc_kill_returns_null() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proc(&["test", "kill", "node", "-f"], &mut renderer).unwrap();
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn e2e_proc_invalid_subcommand_fails() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_proc(&["test", "invalid"], &mut renderer);
        assert!(result.is_err(), "invalid subcommand should fail");
    }
}

// ============================================================
// Phase 3.3: Backup 鍛戒护锛坈lap derive + CommandSpec + TableRow锛?
// ============================================================


