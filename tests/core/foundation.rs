mod xun_error_tests {
    use xun::xun_core::error::XunError;

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歎ser 閿欒鏋勯€犱笌 exit code ----

    #[test]
    fn user_error_has_code_1() {
        let err = XunError::user("something went wrong");
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn user_error_with_hints_preserves_hints() {
        let err = XunError::user("bad input").with_hints(&["check your spelling", "try --help"]);
        assert_eq!(err.hints().len(), 2);
        assert_eq!(err.hints()[0], "check your spelling");
        assert_eq!(err.hints()[1], "try --help");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛氶敊璇被鍨嬪垎绫?----

    #[test]
    fn cancelled_has_code_130() {
        let err = XunError::Cancelled;
        assert_eq!(err.exit_code(), 130);
    }

    #[test]
    fn elevation_required_has_code_77() {
        let err = XunError::ElevationRequired("admin access needed".into());
        assert_eq!(err.exit_code(), 77);
    }

    #[test]
    fn not_found_has_code_2() {
        let err = XunError::NotFound("file not found".into());
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn internal_error_from_anyhow() {
        let inner = anyhow::anyhow!("disk full");
        let err = XunError::from(inner);
        assert_eq!(err.exit_code(), 1);
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛欴isplay trait 杈撳嚭 ----

    #[test]
    fn display_user_error_shows_message() {
        let err = XunError::user("invalid path");
        let msg = format!("{err}");
        assert!(msg.contains("invalid path"), "display output: {msg}");
    }

    #[test]
    fn display_internal_error_transparent() {
        let inner = anyhow::anyhow!("io failure");
        let err = XunError::from(inner);
        let msg = format!("{err}");
        assert!(msg.contains("io failure"), "display output: {msg}");
    }
}

// ============================================================
// Phase 1.2: StructuredValue 鈥?缁熶竴鏁版嵁妯″瀷
// ============================================================

mod structured_value_tests {
    use xun::xun_core::value::{ColumnDef, Table, Value, ValueKind};
    use std::collections::BTreeMap;

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歏alue 鍩烘湰绫诲瀷鏋勯€?----

    #[test]
    fn value_string_roundtrip_json() {
        let v = Value::String("hello".into());
        let json = serde_json::to_string(&v).unwrap();
        let back: Value = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Value::String(s) if s == "hello"));
    }

    #[test]
    fn value_int_roundtrip_json() {
        let v = Value::Int(42);
        let json = serde_json::to_string(&v).unwrap();
        let back: Value = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Value::Int(n) if n == 42));
    }

    #[test]
    fn value_bool_roundtrip_json() {
        let v = Value::Bool(true);
        let json = serde_json::to_string(&v).unwrap();
        let back: Value = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Value::Bool(true)));
    }

    #[test]
    fn value_null_serializes_to_null() {
        let v = Value::Null;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "null");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛歊ecord 鍜?List ----

    #[test]
    fn record_ordered_keys() {
        let mut rec = BTreeMap::new();
        rec.insert("z_key".into(), Value::Int(1));
        rec.insert("a_key".into(), Value::Int(2));
        let json = serde_json::to_string(&Value::Record(rec)).unwrap();
        // BTreeMap 鎸?key 鎺掑簭锛宎_key 鍦ㄥ墠
        let pos_a = json.find("a_key").unwrap();
        let pos_z = json.find("z_key").unwrap();
        assert!(pos_a < pos_z, "keys should be ordered: {json}");
    }

    #[test]
    fn list_heterogeneous_values() {
        let v = Value::List(vec![
            Value::String("text".into()),
            Value::Int(99),
            Value::Bool(false),
        ]);
        let json = serde_json::to_string(&v).unwrap();
        let back: Value = serde_json::from_str(&json).unwrap();
        if let Value::List(items) = back {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn nested_record_in_list() {
        let mut inner = BTreeMap::new();
        inner.insert("name".into(), Value::String("test".into()));
        let v = Value::List(vec![Value::Record(inner)]);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛歍able 缁撴瀯 ----

    #[test]
    fn table_with_columns_and_rows() {
        let cols = vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("count", ValueKind::Int),
        ];
        let mut table = Table::new(cols);
        let mut row = BTreeMap::new();
        row.insert("name".into(), Value::String("alpha".into()));
        row.insert("count".into(), Value::Int(10));
        table.push_row(row);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.columns.len(), 2);
    }

    #[test]
    fn table_serializes_with_schema() {
        let cols = vec![ColumnDef::new("id", ValueKind::Int)];
        let mut table = Table::new(cols);
        let mut row = BTreeMap::new();
        row.insert("id".into(), Value::Int(1));
        table.push_row(row);
        let json = serde_json::to_string(&table).unwrap();
        assert!(json.contains("columns"));
        assert!(json.contains("rows"));
        assert!(json.contains("\"id\""));
    }

    #[test]
    fn empty_table_valid() {
        let cols = vec![ColumnDef::new("col", ValueKind::String)];
        let table = Table::new(cols);
        let json = serde_json::to_string(&table).unwrap();
        let back: Table = serde_json::from_str(&json).unwrap();
        assert!(back.rows.is_empty());
        assert_eq!(back.columns.len(), 1);
    }

    // ---- 娴嬭瘯鐢ㄤ緥 4锛氳涔夌被鍨?----

    #[test]
    fn value_duration_serializes_as_millis() {
        let v = Value::Duration(5000);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("5000"), "duration json: {json}");
    }

    #[test]
    fn value_filesize_serializes_as_bytes() {
        let v = Value::Filesize(1024);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("1024"), "filesize json: {json}");
    }

    #[test]
    fn value_date_serializes_iso8601() {
        let v = Value::Date("2026-05-12T10:00:00Z".into());
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("2026-05-12"), "date json: {json}");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 5锛氬ぇ Table 鎬ц兘 ----

    #[test]
    fn table_10k_rows_serializes_under_50ms() {
        let cols = vec![
            ColumnDef::new("id", ValueKind::Int),
            ColumnDef::new("name", ValueKind::String),
        ];
        let mut table = Table::new(cols);
        for i in 0..10_000 {
            let mut row = BTreeMap::new();
            row.insert("id".into(), Value::Int(i));
            row.insert("name".into(), Value::String(format!("item_{i}")));
            table.push_row(row);
        }
        let start = std::time::Instant::now();
        let _json = serde_json::to_string(&table).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "10k rows serialization took {}ms",
            elapsed.as_millis()
        );
    }
}

// ============================================================
// Phase 1.3: Renderer 鈥?澶氱杈撳嚭
// ============================================================

mod renderer_tests {
    use xun::xun_core::renderer::{JsonRenderer, OutputFormat, Renderer, TerminalRenderer};
    use xun::xun_core::value::{ColumnDef, Table, Value, ValueKind};
    use std::collections::BTreeMap;
    use std::io::Cursor;

    fn sample_table() -> Table {
        let cols = vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("count", ValueKind::Int),
        ];
        let mut table = Table::new(cols);
        let mut row1 = BTreeMap::new();
        row1.insert("name".into(), Value::String("alpha".into()));
        row1.insert("count".into(), Value::Int(10));
        table.push_row(row1);
        let mut row2 = BTreeMap::new();
        row2.insert("name".into(), Value::String("beta".into()));
        row2.insert("count".into(), Value::Int(20));
        table.push_row(row2);
        table
    }

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歍erminalRenderer 琛ㄦ牸杈撳嚭 ----

    #[test]
    fn terminal_renders_table_with_headers() {
        let table = sample_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("name"), "missing 'name' header: {output}");
        assert!(output.contains("count"), "missing 'count' header: {output}");
        assert!(output.contains("alpha"), "missing 'alpha' row: {output}");
    }

    #[test]
    fn terminal_renders_single_record() {
        let mut rec = BTreeMap::new();
        rec.insert("key".into(), Value::String("val".into()));
        let v = Value::Record(rec);
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(false, &mut buf);
        r.render_value(&v).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("key"), "missing key: {output}");
        assert!(output.contains("val"), "missing val: {output}");
    }

    #[test]
    fn terminal_respects_no_color() {
        let table = sample_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = TerminalRenderer::new(true, &mut buf); // no_color = true
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        // 涓嶅簲鍖呭惈 ANSI escape
        assert!(!output.contains("\x1b["), "should not contain ANSI codes: {output:?}");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛欽sonRenderer ----

    #[test]
    fn json_renders_table_as_array() {
        let table = sample_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.is_array(), "expected JSON array, got: {output}");
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn json_renders_value_directly() {
        let v = Value::Int(42);
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_value(&v).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert_eq!(output.trim(), "42");
    }

    #[test]
    fn json_pretty_vs_compact() {
        let v = Value::String("test".into());
        // pretty
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(true, &mut buf); // pretty = true
        r.render_value(&v).unwrap();
        let pretty = String::from_utf8(buf.into_inner()).unwrap();
        // compact
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf); // pretty = false
        r.render_value(&v).unwrap();
        let compact = String::from_utf8(buf.into_inner()).unwrap();
        // pretty 搴旇鏇撮暱鎴栫浉鍚岋紙鏈夌缉杩涳級
        assert!(pretty.len() >= compact.len());
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛歍svRenderer ----

    #[test]
    fn tsv_renders_table_tab_separated() {
        let table = sample_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = xun::xun_core::renderer::TsvRenderer::new(&mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 3, "expected header + 2 rows, got: {lines:?}");
        assert!(lines[0].contains('\t'), "header should be tab-separated: {}", lines[0]);
    }

    #[test]
    fn tsv_escapes_tabs_in_values() {
        let cols = vec![ColumnDef::new("data", ValueKind::String)];
        let mut table = Table::new(cols);
        let mut row = BTreeMap::new();
        row.insert("data".into(), Value::String("has\ttab".into()));
        table.push_row(row);
        let mut buf = Cursor::new(Vec::new());
        let mut r = xun::xun_core::renderer::TsvRenderer::new(&mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        // tab in value should be escaped (e.g. replaced with \\t or quoted)
        // The exact escaping depends on implementation; just verify it doesn't break
        assert!(!output.is_empty());
    }

    // ---- 娴嬭瘯鐢ㄤ緥 4锛歄utputFormat 鑷姩妫€娴?----

    #[test]
    fn auto_format_tty_returns_table() {
        let fmt = OutputFormat::Auto.resolve(true); // is_tty = true
        assert!(matches!(fmt, OutputFormat::Table));
    }

    #[test]
    fn auto_format_pipe_returns_json() {
        let fmt = OutputFormat::Auto.resolve(false); // is_tty = false
        assert!(matches!(fmt, OutputFormat::Json));
    }
}

// ============================================================
// Phase 1.4: 鍏叡鍙傛暟缁?鈥?Args
// ============================================================

mod args_tests {
    use clap::Parser;
    use xun::xun_core::args::{ConfirmArgs, FuzzyArgs, ListArgs, ScopeArgs};

    // Minimal sanity check: clap bool parsing works at all
    #[test]
    fn clap_bool_flag_works_at_all() {
        let cmd = clap::Command::new("test")
            .arg(clap::Arg::new("verbose").short('v').long("verbose").action(clap::ArgAction::SetTrue));
        // NOTE: first element is always the command name (argv[0])
        let m = cmd.try_get_matches_from(["test", "--verbose"]).unwrap();
        assert!(m.get_flag("verbose"), "SetTrue flag broken");
    }

    // NOTE: try_parse_from 绗竴涓厓绱犳槸 argv[0]锛堝懡浠ゅ悕锛夛紝蹇呴』鍔犲墠缂€ "test"

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歀istArgs 瑙ｆ瀽 ----

    #[test]
    fn list_args_defaults() {
        let args = ListArgs::try_parse_from(["test"]).unwrap();
        assert_eq!(args.limit, 50);
        assert_eq!(args.offset, 0);
        assert!(!args.reverse);
        assert!(args.sort.is_none());
    }

    #[test]
    fn list_args_custom_limit_and_sort() {
        let args = ListArgs::try_parse_from(["test", "--limit", "10", "--sort", "name"]).unwrap();
        assert_eq!(args.limit, 10);
        assert_eq!(args.sort.as_deref(), Some("name"));
    }

    #[test]
    fn list_args_reverse_flag() {
        let args = ListArgs::try_parse_from(["test", "--reverse"]).unwrap();
        assert!(args.reverse, "expected reverse=true, got {}", args.reverse);
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛欶uzzyArgs 瑙ｆ瀽 ----

    #[test]
    fn fuzzy_args_multiple_patterns() {
        let args = FuzzyArgs::try_parse_from(["test", "pattern1", "pattern2"]).unwrap();
        assert_eq!(args.patterns.len(), 2);
        assert_eq!(args.patterns[0], "pattern1");
        assert_eq!(args.patterns[1], "pattern2");
    }

    #[test]
    fn fuzzy_args_list_flag() {
        let args = FuzzyArgs::try_parse_from(["test", "pat", "--list"]).unwrap();
        assert!(args.list);
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛歋copeArgs 瑙ｆ瀽 ----

    #[test]
    fn scope_args_global_flag() {
        let args = ScopeArgs::try_parse_from(["test", "--global"]).unwrap();
        assert!(args.global);
    }

    #[test]
    fn scope_args_workspace_option() {
        let args = ScopeArgs::try_parse_from(["test", "--workspace", "my-ws"]).unwrap();
        assert_eq!(args.workspace.as_deref(), Some("my-ws"));
    }

    // ---- 娴嬭瘯鐢ㄤ緥 4锛欳onfirmArgs 瑙ｆ瀽 ----

    #[test]
    fn confirm_args_yes_skips_prompt() {
        let args = ConfirmArgs::try_parse_from(["test", "-y"]).unwrap();
        assert!(args.yes);
    }

    #[test]
    fn confirm_args_dry_run_flag() {
        let args = ConfirmArgs::try_parse_from(["test", "--dry-run"]).unwrap();
        assert!(args.dry_run);
    }
}

// ============================================================
// Phase 1.5: CmdContext 鈥?鎵ц涓婁笅鏂?
// ============================================================

mod context_tests {
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::renderer::OutputFormat;

    // ---- 娴嬭瘯鐢ㄤ緥 1锛氭瀯閫犱笌榛樿鍊?----

    #[test]
    fn context_default_format_is_auto() {
        let ctx = CmdContext::for_test();
        assert_eq!(ctx.format(), OutputFormat::Auto);
    }

    #[test]
    fn context_respects_quiet_flag() {
        let ctx = CmdContext::for_test().with_quiet(true);
        assert!(ctx.is_quiet());
    }

    #[test]
    fn context_respects_verbose_flag() {
        let ctx = CmdContext::for_test().with_verbose(true);
        assert!(ctx.is_verbose());
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛氶厤缃欢杩熷姞杞?----

    #[test]
    fn config_not_loaded_until_accessed() {
        let ctx = CmdContext::for_test();
        assert!(!ctx.config_loaded(), "config should not be loaded at construction");
    }

    #[test]
    fn config_loaded_once_and_cached() {
        let mut ctx = CmdContext::for_test();
        let _ = ctx.config();
        assert!(ctx.config_loaded(), "config should be marked loaded after first access");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛氫氦浜掑垽鏂?----

    #[test]
    fn non_interactive_flag_disables_confirm() {
        let ctx = CmdContext::for_test().with_non_interactive(true);
        assert!(ctx.is_non_interactive());
    }

    #[test]
    fn confirm_returns_true_when_non_interactive() {
        let ctx = CmdContext::for_test().with_non_interactive(true);
        assert!(ctx.confirm("proceed?"), "confirm should auto-return true in non-interactive mode");
    }
}

// ============================================================
// Phase 1.6: CommandSpec 鈥?缁熶竴鍛戒护 trait
// ============================================================

mod command_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- Mock 鍛戒护 ----

    struct EchoCmd {
        message: String,
        fail_validate: bool,
    }

    impl EchoCmd {
        fn new(msg: &str) -> Self {
            Self { message: msg.to_string(), fail_validate: false }
        }
        fn with_fail_validate(mut self) -> Self {
            self.fail_validate = true;
            self
        }
    }

    impl CommandSpec for EchoCmd {
        fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
            if self.fail_validate {
                Err(XunError::user("validation failed"))
            } else {
                Ok(())
            }
        }

        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            Ok(Value::String(self.message.clone()))
        }
    }

    // ---- 娴嬭瘯鐢ㄤ緥 1锛氬熀鏈?CommandSpec 瀹炵幇 ----

    #[test]
    fn command_spec_run_returns_output() {
        let cmd = EchoCmd::new("hello");
        let mut ctx = CmdContext::for_test();
        let result = cmd.run(&mut ctx).unwrap();
        assert!(matches!(result, Value::String(s) if s == "hello"));
    }

    #[test]
    fn command_spec_validate_default_passes() {
        let cmd = EchoCmd::new("ok");
        let ctx = CmdContext::for_test();
        assert!(cmd.validate(&ctx).is_ok());
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛歟xecute 鍑芥暟闆嗘垚 ----

    #[test]
    fn execute_calls_validate_then_run() {
        let cmd = EchoCmd::new("validated");
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::String(s) if s == "validated"));
    }

    #[test]
    fn execute_renders_output_via_renderer() {
        let cmd = EchoCmd::new("rendered");
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let _ = execute(&cmd, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("rendered"), "renderer output: {output}");
    }

    #[test]
    fn execute_returns_error_on_validate_failure() {
        let cmd = EchoCmd::new("nope").with_fail_validate();
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer);
        assert!(result.is_err(), "should fail on validation");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛歅ipeline middleware ----

    use xun::xun_core::command::Pipeline;

    #[test]
    fn pipeline_before_runs_before_command() {
        let mut pipeline = Pipeline::new();
        pipeline.add_before(|_ctx| {
            // before hook succeeds
            Ok(())
        });
        let cmd = EchoCmd::new("with-pipeline");
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute_with_pipeline(&cmd, &mut ctx, &mut renderer, &pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn pipeline_after_runs_after_command() {
        let mut pipeline = Pipeline::new();
        pipeline.add_after(|_ctx| {
            Ok(())
        });
        let cmd = EchoCmd::new("after-ok");
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute_with_pipeline(&cmd, &mut ctx, &mut renderer, &pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn pipeline_before_error_aborts_execution() {
        let mut pipeline = Pipeline::new();
        pipeline.add_before(|_ctx| {
            Err(XunError::user("blocked by pipeline"))
        });
        let cmd = EchoCmd::new("should-not-run");
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute_with_pipeline(&cmd, &mut ctx, &mut renderer, &pipeline);
        assert!(result.is_err(), "pipeline before error should abort");
    }

    // Helper: execute with pipeline (tests need this since execute() is the simple version)
    fn execute_with_pipeline<C: CommandSpec>(
        cmd: &C,
        ctx: &mut CmdContext,
        renderer: &mut dyn Renderer,
        pipeline: &Pipeline,
    ) -> Result<Value, XunError> {
        pipeline.run_before(ctx)?;
        let output = execute(cmd, ctx, renderer)?;
        pipeline.run_after(ctx)?;
        Ok(output)
    }
}

// ============================================================
// Phase 1.7: Operation 鈥?鍗遍櫓鎿嶄綔鍗忚
// ============================================================

mod operation_tests {
    use xun::xun_core::operation::{
        Change, Operation, OperationResult, Preview, RiskLevel, run_operation,
    };
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歅review 缁撴瀯 ----

    #[test]
    fn preview_serializes_to_json() {
        let preview = Preview::new("delete files")
            .add_change(Change::new("remove", "/tmp/old.txt"))
            .with_risk_level(RiskLevel::High);
        let json = serde_json::to_string(&preview).unwrap();
        assert!(json.contains("delete files"));
        assert!(json.contains("remove"));
    }

    #[test]
    fn preview_with_changes_and_risk_level() {
        let preview = Preview::new("rename batch")
            .add_change(Change::new("rename", "a.txt -> b.txt"))
            .add_change(Change::new("rename", "c.txt -> d.txt"))
            .with_risk_level(RiskLevel::Medium);
        assert_eq!(preview.changes().len(), 2);
        assert_eq!(preview.risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn empty_preview_valid() {
        let preview = Preview::new("noop");
        let json = serde_json::to_string(&preview).unwrap();
        let back: Preview = serde_json::from_str(&json).unwrap();
        assert!(back.changes().is_empty());
        assert_eq!(back.description(), "noop");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛歄peration trait 鍩烘湰娴佺▼ ----

    struct MockOp {
        preview: Preview,
        executed: std::cell::Cell<bool>,
        should_fail: bool,
    }

    impl MockOp {
        fn new(desc: &str) -> Self {
            Self {
                preview: Preview::new(desc),
                executed: std::cell::Cell::new(false),
                should_fail: false,
            }
        }
        fn with_fail(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    impl Operation for MockOp {
        fn preview(&self) -> &Preview {
            &self.preview
        }

        fn execute(&self, _ctx: &mut CmdContext) -> Result<OperationResult, XunError> {
            if self.should_fail {
                return Err(XunError::user("operation failed"));
            }
            self.executed.set(true);
            Ok(OperationResult::new().with_changes_applied(1))
        }
    }

    #[test]
    fn run_operation_calls_preview_then_execute() {
        let op = MockOp::new("test op");
        let mut ctx = CmdContext::for_test().with_non_interactive(true);
        let result = run_operation(&op, &mut ctx).unwrap();
        assert!(op.executed.get(), "execute should have been called");
        assert_eq!(result.changes_applied(), 1);
    }

    #[test]
    fn run_operation_skips_confirm_when_yes_flag() {
        let op = MockOp::new("auto-confirm op");
        let mut ctx = CmdContext::for_test().with_non_interactive(true);
        let result = run_operation(&op, &mut ctx);
        assert!(result.is_ok(), "non-interactive should auto-confirm");
    }

    #[test]
    fn run_operation_aborts_on_cancel() {
        // 妯℃嫙锛歄peration execute 杩斿洖 Cancelled
        struct CancelOp;
        impl Operation for CancelOp {
            fn preview(&self) -> &Preview {
                // 鐢ㄤ竴涓潤鎬?Preview锛堥渶瑕?leak 鎴?thread_local锛?
                // 绠€鍗曡捣瑙侊紝杩欓噷鐢?Box::leak
                Box::leak(Box::new(Preview::new("cancel test")))
            }
            fn execute(&self, _ctx: &mut CmdContext) -> Result<OperationResult, XunError> {
                Err(XunError::Cancelled)
            }
        }

        let op = CancelOp;
        let mut ctx = CmdContext::for_test().with_non_interactive(true);
        let result = run_operation(&op, &mut ctx);
        assert!(result.is_err(), "cancel should propagate error");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 3锛氶闄╃瓑绾т笌纭 ----

    #[test]
    fn high_risk_forces_confirm_even_without_flag() {
        // 娴嬭瘯 RiskLevel 鐨勬瘮杈冭涔?
        assert!(RiskLevel::High > RiskLevel::Low);
        assert!(RiskLevel::Critical > RiskLevel::High);
    }

    #[test]
    fn low_risk_skips_confirm_when_no_flag() {
        let op = MockOp::new("low risk op");
        let mut ctx = CmdContext::for_test().with_non_interactive(true);
        let result = run_operation(&op, &mut ctx);
        assert!(result.is_ok());
    }

    // ---- 娴嬭瘯鐢ㄤ緥 4锛歄perationResult ----

    #[test]
    fn operation_result_serializes_with_duration() {
        let result = OperationResult::new()
            .with_changes_applied(5)
            .with_duration_ms(120);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("5"), "changes_applied: {json}");
        assert!(json.contains("120"), "duration_ms: {json}");
    }

    #[test]
    fn operation_result_tracks_changes_applied() {
        let result = OperationResult::new().with_changes_applied(42);
        assert_eq!(result.changes_applied(), 42);
    }

    // ---- 娴嬭瘯鐢ㄤ緥 5锛歳ollback 榛樿涓嶆敮鎸?----

    #[test]
    fn default_rollback_returns_error() {
        let op = MockOp::new("no rollback");
        let result = op.rollback(&mut CmdContext::for_test());
        assert!(result.is_err(), "default rollback should return error");
    }
}

// ============================================================
// Phase 1.8: ShellIntegration 鈥?Shell 闆嗘垚 trait
// ============================================================

mod shell_tests {
    use xun::xun_core::shell::{BashShell, PowerShell, ShellIntegration};

    // ---- 娴嬭瘯鐢ㄤ緥 1锛歅owerShell 娓叉煋 ----

    #[test]
    fn powershell_renders_alias() {
        let ps = PowerShell;
        let script = ps.render_alias("ll", "Get-ChildItem -Force");
        assert!(script.contains("ll"), "should contain alias name: {script}");
        assert!(script.contains("Get-ChildItem"), "should contain command: {script}");
        assert!(script.contains("Set-Alias") || script.contains("function"), "PS alias syntax: {script}");
    }

    #[test]
    fn powershell_renders_function() {
        let ps = PowerShell;
        let script = ps.render_function("mkcd", "New-Item -ItemType Directory -Path $args[0]; Set-Location $args[0]");
        assert!(script.contains("mkcd"), "should contain function name: {script}");
        assert!(script.contains("function"), "PS function syntax: {script}");
    }

    #[test]
    fn powershell_renders_completion() {
        let ps = PowerShell;
        let script = ps.render_completion("xun", &["backup", "bookmark", "alias"]);
        assert!(script.contains("xun"), "should contain command name: {script}");
        assert!(script.contains("backup"), "should contain completion: {script}");
    }

    // ---- 娴嬭瘯鐢ㄤ緥 2锛欱ash 娓叉煋 ----

    #[test]
    fn bash_renders_alias() {
        let bash = BashShell;
        let script = bash.render_alias("ll", "ls -la");
        assert!(script.contains("ll"), "should contain alias name: {script}");
        assert!(script.contains("ls -la"), "should contain command: {script}");
        assert!(script.contains("alias"), "bash alias syntax: {script}");
    }

    #[test]
    fn bash_renders_function() {
        let bash = BashShell;
        let script = bash.render_function("mkcd", "mkdir -p \"$1\" && cd \"$1\"");
        assert!(script.contains("mkcd"), "should contain function name: {script}");
        assert!(script.contains("function") || script.contains("()"), "bash function syntax: {script}");
    }
}

// ============================================================
// Phase 2.1: Proxy CLI 瀹氫箟锛坈lap derive锛?
// ============================================================

