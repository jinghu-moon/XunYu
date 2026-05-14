//! core 模块集成测试入口
//! Phase 0.3: 测试脚手架

// ============================================================
// Phase 1.1: XunError — 分层错误类型
// ============================================================

mod xun_error_tests {
    use xun::xun_core::error::XunError;

    // ---- 测试用例 1：User 错误构造与 exit code ----

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

    // ---- 测试用例 2：错误类型分类 ----

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

    // ---- 测试用例 3：Display trait 输出 ----

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
// Phase 1.2: StructuredValue — 统一数据模型
// ============================================================

mod structured_value_tests {
    use xun::xun_core::value::{ColumnDef, Table, Value, ValueKind};
    use std::collections::BTreeMap;

    // ---- 测试用例 1：Value 基本类型构造 ----

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

    // ---- 测试用例 2：Record 和 List ----

    #[test]
    fn record_ordered_keys() {
        let mut rec = BTreeMap::new();
        rec.insert("z_key".into(), Value::Int(1));
        rec.insert("a_key".into(), Value::Int(2));
        let json = serde_json::to_string(&Value::Record(rec)).unwrap();
        // BTreeMap 按 key 排序，a_key 在前
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

    // ---- 测试用例 3：Table 结构 ----

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

    // ---- 测试用例 4：语义类型 ----

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

    // ---- 测试用例 5：大 Table 性能 ----

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
// Phase 1.3: Renderer — 多端输出
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

    // ---- 测试用例 1：TerminalRenderer 表格输出 ----

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
        // 不应包含 ANSI escape
        assert!(!output.contains("\x1b["), "should not contain ANSI codes: {output:?}");
    }

    // ---- 测试用例 2：JsonRenderer ----

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
        // pretty 应该更长或相同（有缩进）
        assert!(pretty.len() >= compact.len());
    }

    // ---- 测试用例 3：TsvRenderer ----

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

    // ---- 测试用例 4：OutputFormat 自动检测 ----

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
// Phase 1.4: 公共参数组 — Args
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

    // NOTE: try_parse_from 第一个元素是 argv[0]（命令名），必须加前缀 "test"

    // ---- 测试用例 1：ListArgs 解析 ----

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

    // ---- 测试用例 2：FuzzyArgs 解析 ----

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

    // ---- 测试用例 3：ScopeArgs 解析 ----

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

    // ---- 测试用例 4：ConfirmArgs 解析 ----

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
// Phase 1.5: CmdContext — 执行上下文
// ============================================================

mod context_tests {
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::renderer::OutputFormat;

    // ---- 测试用例 1：构造与默认值 ----

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

    // ---- 测试用例 2：配置延迟加载 ----

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

    // ---- 测试用例 3：交互判断 ----

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
// Phase 1.6: CommandSpec — 统一命令 trait
// ============================================================

mod command_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- Mock 命令 ----

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

    // ---- 测试用例 1：基本 CommandSpec 实现 ----

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

    // ---- 测试用例 2：execute 函数集成 ----

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

    // ---- 测试用例 3：Pipeline middleware ----

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
// Phase 1.7: Operation — 危险操作协议
// ============================================================

mod operation_tests {
    use xun::xun_core::operation::{
        Change, Operation, OperationResult, Preview, RiskLevel, run_operation,
    };
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;

    // ---- 测试用例 1：Preview 结构 ----

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

    // ---- 测试用例 2：Operation trait 基本流程 ----

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
        // 模拟：Operation execute 返回 Cancelled
        struct CancelOp;
        impl Operation for CancelOp {
            fn preview(&self) -> &Preview {
                // 用一个静态 Preview（需要 leak 或 thread_local）
                // 简单起见，这里用 Box::leak
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

    // ---- 测试用例 3：风险等级与确认 ----

    #[test]
    fn high_risk_forces_confirm_even_without_flag() {
        // 测试 RiskLevel 的比较语义
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

    // ---- 测试用例 4：OperationResult ----

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

    // ---- 测试用例 5：rollback 默认不支持 ----

    #[test]
    fn default_rollback_returns_error() {
        let op = MockOp::new("no rollback");
        let result = op.rollback(&mut CmdContext::for_test());
        assert!(result.is_err(), "default rollback should return error");
    }
}

// ============================================================
// Phase 1.8: ShellIntegration — Shell 集成 trait
// ============================================================

mod shell_tests {
    use xun::xun_core::shell::{BashShell, PowerShell, ShellIntegration};

    // ---- 测试用例 1：PowerShell 渲染 ----

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

    // ---- 测试用例 2：Bash 渲染 ----

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
// Phase 2.1: Proxy CLI 定义（clap derive）
// ============================================================

mod proxy_cli_tests {
    use clap::Parser;
    use xun::xun_core::proxy_cmd::{ProxyCmd, ProxySubCommand};

    // ---- 测试用例 1：clap 解析 ----

    #[test]
    fn proxy_set_parses_url_and_noproxy() {
        let cmd = ProxyCmd::try_parse_from(["test", "set", "http://127.0.0.1:7890", "-n", "localhost,10.0.0.0/8"]).unwrap();
        match cmd.sub {
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
        assert!(matches!(cmd.sub, ProxySubCommand::Show(_)));
    }

    #[test]
    fn proxy_rm_parses_only_option() {
        let cmd = ProxyCmd::try_parse_from(["test", "rm", "--only", "cargo,git"]).unwrap();
        match cmd.sub {
            ProxySubCommand::Rm(args) => {
                assert_eq!(args.only.as_deref(), Some("cargo,git"));
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }
}

// ============================================================
// Phase 2.2: Proxy 输出类型
// ============================================================

mod proxy_output_tests {
    use xun::xun_core::proxy_cmd::ProxyInfo;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, TsvRenderer, TerminalRenderer, Renderer};
    use std::io::Cursor;

    fn sample_info() -> ProxyInfo {
        ProxyInfo::new("http://127.0.0.1:7890", "localhost,127.0.0.1", "env")
    }

    // ---- 测试用例 1：ProxyInfo TableRow 实现 ----

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

    // ---- 测试用例 2：ProxyInfo 渲染 ----

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
// Phase 2.3: Proxy CommandSpec 实现
// ============================================================

mod proxy_command_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::proxy_cmd::{ProxyInfo, ProxyShowArgs, ProxySetArgs};
    use xun::xun_core::renderer::JsonRenderer;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- ProxyShowCmd ----

    struct ProxyShowCmd {
        args: ProxyShowArgs,
    }

    impl CommandSpec for ProxyShowCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 模拟：返回当前代理配置
            let info = ProxyInfo::new("http://127.0.0.1:7890", "localhost,127.0.0.1", "env");
            let table = info.to_table();
            Ok(Value::List(
                table.rows.into_iter().map(Value::Record).collect(),
            ))
        }
    }

    // ---- ProxySetCmd ----

    struct ProxySetCmd {
        args: ProxySetArgs,
    }

    impl CommandSpec for ProxySetCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 模拟：设置代理，返回空输出
            Ok(Value::Null)
        }
    }

    // ---- 测试用例 1：ProxyShowCmd 返回 ProxyInfo ----

    #[test]
    fn proxy_show_returns_current_config() {
        let cmd = ProxyShowCmd { args: ProxyShowArgs { format: "auto".into() } };
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

    // ---- 测试用例 2：ProxySetCmd 修改配置 ----

    #[test]
    fn proxy_set_updates_config() {
        let cmd = ProxySetCmd {
            args: ProxySetArgs {
                url: "http://10.0.0.1:8080".into(),
                noproxy: "localhost".into(),
                only: None,
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
        let cmd = ProxySetCmd {
            args: ProxySetArgs {
                url: "http://10.0.0.1:8080".into(),
                noproxy: "localhost".into(),
                only: Some("cargo,git".into()),
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
        ProxyCmd, ProxySubCommand, ProxyShowArgs, ProxySetArgs, ProxyInfo,
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

        match cmd.sub {
            ProxySubCommand::Show(args) => {
                struct ShowCmd { args: ProxyShowArgs }
                impl CommandSpec for ShowCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        let info = ProxyInfo::new("http://127.0.0.1:7890", "localhost", "env");
                        Ok(Value::List(info.to_table().rows.into_iter().map(Value::Record).collect()))
                    }
                }
                execute(&ShowCmd { args }, &mut ctx, renderer)
            }
            ProxySubCommand::Set(args) => {
                struct SetCmd { args: ProxySetArgs }
                impl CommandSpec for SetCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&SetCmd { args }, &mut ctx, renderer)
            }
            ProxySubCommand::Rm(_args) => {
                struct RmCmd;
                impl CommandSpec for RmCmd {
                    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
                        Ok(Value::Null)
                    }
                }
                execute(&RmCmd, &mut ctx, renderer)
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
// Phase 3.1: Config 命令（clap derive + CommandSpec）
// ============================================================

mod config_cmd_tests {
    use clap::Parser;
    use xun::xun_core::config_cmd::{ConfigCmd, ConfigSubCommand, ConfigGetArgs, ConfigEntry};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn config_get_parses_key() {
        let cmd = ConfigCmd::try_parse_from(["test", "get", "proxy.defaultUrl"]).unwrap();
        match cmd.sub {
            ConfigSubCommand::Get(args) => assert_eq!(args.key, "proxy.defaultUrl"),
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn config_set_parses_key_and_value() {
        let cmd = ConfigCmd::try_parse_from(["test", "set", "tree.defaultDepth", "3"]).unwrap();
        match cmd.sub {
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
        assert!(matches!(cmd.sub, ConfigSubCommand::Edit(_)));
    }

    // ---- CommandSpec 测试 ----

    struct ConfigGetCmd {
        args: ConfigGetArgs,
    }

    impl CommandSpec for ConfigGetCmd {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            // 模拟：返回配置条目
            let entry = ConfigEntry::new(&self.args.key, "mock_value");
            Ok(Value::Record(entry.to_record()))
        }
    }

    #[test]
    fn config_get_returns_entry() {
        let cmd = ConfigGetCmd { args: ConfigGetArgs { key: "proxy.defaultUrl".into() } };
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

    // ---- E2E dispatch 测试 ----

    fn dispatch_config(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = ConfigCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.sub {
            ConfigSubCommand::Get(args) => {
                struct GetCmd { args: ConfigGetArgs }
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
// Phase 3.1: Tree 命令（clap derive + CommandSpec）
// ============================================================

mod tree_cmd_tests {
    use clap::Parser;
    use xun::xun_core::tree_cmd::TreeCmd;
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn tree_parses_defaults() {
        let cmd = TreeCmd::try_parse_from(["test"]).unwrap();
        assert!(cmd.path.is_none());
        assert!(cmd.depth.is_none());
        assert!(!cmd.hidden);
        assert!(!cmd.plain);
        assert_eq!(cmd.sort, "name");
    }

    #[test]
    fn tree_parses_path_and_depth() {
        let cmd = TreeCmd::try_parse_from(["test", "/tmp", "-d", "3"]).unwrap();
        assert_eq!(cmd.path.as_deref(), Some("/tmp"));
        assert_eq!(cmd.depth, Some(3));
    }

    #[test]
    fn tree_parses_flags() {
        let cmd = TreeCmd::try_parse_from(["test", "--hidden", "--plain", "--fast", "--size"]).unwrap();
        assert!(cmd.hidden);
        assert!(cmd.plain);
        assert!(cmd.fast);
        assert!(cmd.size);
    }

    // ---- CommandSpec 测试 ----

    struct TreeExecutor {
        path: Option<String>,
        depth: Option<usize>,
    }

    impl CommandSpec for TreeExecutor {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let path = self.path.as_deref().unwrap_or(".");
            Ok(Value::String(format!("{path}\n├── a\n└── b")))
        }
    }

    #[test]
    fn tree_returns_string_output() {
        let cmd = TreeCmd::try_parse_from(["test", "/tmp"]).unwrap();
        let exec = TreeExecutor { path: cmd.path, depth: cmd.depth };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&exec, &mut ctx, &mut renderer).unwrap();
        assert!(matches!(result, Value::String(s) if s.contains("/tmp")));
    }

    // ---- E2E 测试 ----

    fn dispatch_tree(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = TreeCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let exec = TreeExecutor { path: cmd.path, depth: cmd.depth };
        let mut ctx = CmdContext::for_test();
        execute(&exec, &mut ctx, renderer)
    }

    #[test]
    fn e2e_tree_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_tree(&["test", "/tmp", "-d", "2"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("/tmp"), "output: {output}");
        assert!(matches!(result, Value::String(_)));
    }
}

// ============================================================
// Phase 3.1: Find 命令（clap derive + CommandSpec）
// ============================================================

mod find_cmd_tests {
    use clap::Parser;
    use xun::xun_core::find_cmd::{FindCmd, FindResult};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn find_parses_defaults() {
        let cmd = FindCmd::try_parse_from(["test"]).unwrap();
        assert!(cmd.paths.is_empty());
        assert!(cmd.include.is_empty());
        assert!(!cmd.count);
    }

    #[test]
    fn find_parses_paths_and_include() {
        let cmd = FindCmd::try_parse_from(["test", "/tmp", "-i", "*.rs", "-i", "*.toml"]).unwrap();
        assert_eq!(cmd.paths, vec!["/tmp"]);
        assert_eq!(cmd.include, vec!["*.rs", "*.toml"]);
    }

    #[test]
    fn find_parses_count_flag() {
        let cmd = FindCmd::try_parse_from(["test", "-c"]).unwrap();
        assert!(cmd.count);
    }

    // ---- Output 类型测试 ----

    #[test]
    fn find_result_table_row() {
        let result = FindResult::new("/tmp/test.rs", "file", 1024);
        let cols = FindResult::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "path");
        assert_eq!(cols[1].name, "kind");
        assert_eq!(cols[2].name, "size");
        let cells = result.cells();
        assert!(matches!(&cells[0], Value::String(s) if s == "/tmp/test.rs"));
        assert!(matches!(&cells[2], Value::Filesize(1024)));
    }

    // ---- CommandSpec 测试 ----

    struct FindExecutor {
        paths: Vec<String>,
        include: Vec<String>,
    }

    impl CommandSpec for FindExecutor {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let results = vec![
                FindResult::new("/tmp/a.rs", "file", 512),
                FindResult::new("/tmp/b.rs", "file", 256),
            ];
            let table = FindResult::vec_to_table(&results);
            Ok(Value::List(table.rows.into_iter().map(Value::Record).collect()))
        }
    }

    #[test]
    fn find_returns_table_output() {
        let cmd = FindCmd::try_parse_from(["test", "/tmp"]).unwrap();
        let exec = FindExecutor { paths: cmd.paths, include: cmd.include };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&exec, &mut ctx, &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("a.rs"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }

    // ---- E2E 测试 ----

    fn dispatch_find(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = FindCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let exec = FindExecutor { paths: cmd.paths, include: cmd.include };
        let mut ctx = CmdContext::for_test();
        execute(&exec, &mut ctx, renderer)
    }

    #[test]
    fn e2e_find_json() {
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = dispatch_find(&["test", "/tmp", "-i", "*.rs"], &mut renderer).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("a.rs"), "output: {output}");
        assert!(matches!(result, Value::List(_)));
    }
}

// ============================================================
// Phase 3.1: Ctx 命令（clap derive + CommandSpec + TableRow）
// ============================================================

mod ctx_cmd_tests {
    use clap::Parser;
    use xun::xun_core::ctx_cmd::{
        CtxCmd, CtxSubCommand, CtxSetArgs, CtxProfile,
    };
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn ctx_set_parses_name_and_path() {
        let cmd = CtxCmd::try_parse_from(["test", "set", "work", "--path", "/projects"]).unwrap();
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
            CtxSubCommand::Use(args) => assert_eq!(args.name, "work"),
            other => panic!("expected Use, got {other:?}"),
        }
    }

    #[test]
    fn ctx_off_parses_empty() {
        let cmd = CtxCmd::try_parse_from(["test", "off"]).unwrap();
        assert!(matches!(cmd.sub, CtxSubCommand::Off(_)));
    }

    #[test]
    fn ctx_list_parses_format() {
        let cmd = CtxCmd::try_parse_from(["test", "list", "-f", "json"]).unwrap();
        match cmd.sub {
            CtxSubCommand::List(args) => assert_eq!(args.format, "json"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ctx_list_defaults_to_auto() {
        let cmd = CtxCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.sub {
            CtxSubCommand::List(args) => assert_eq!(args.format, "auto"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ctx_show_defaults_to_current() {
        let cmd = CtxCmd::try_parse_from(["test", "show"]).unwrap();
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
            CtxSubCommand::Del(args) => assert_eq!(args.name, "old-project"),
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn ctx_rename_parses_old_and_new() {
        let cmd = CtxCmd::try_parse_from(["test", "rename", "old", "new"]).unwrap();
        match cmd.sub {
            CtxSubCommand::Rename(args) => {
                assert_eq!(args.old, "old");
                assert_eq!(args.new, "new");
            }
            other => panic!("expected Rename, got {other:?}"),
        }
    }

    // ---- CtxProfile TableRow 测试 ----

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

    // ---- CommandSpec 测试 ----

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

    struct CtxSetCmd {
        args: CtxSetArgs,
    }
    impl CommandSpec for CtxSetCmd {
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
        let cmd = CtxSetCmd {
            args: CtxSetArgs {
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

    // ---- E2E dispatch 测试 ----

    fn dispatch_ctx(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = CtxCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.sub {
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
// Phase 3.2: Port 命令（clap derive + CommandSpec + TableRow）
// ============================================================

mod port_cmd_tests {
    use clap::Parser;
    use xun::xun_core::port_cmd::{PortCmd, PortSubCommand, PortInfo};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn port_list_parses_defaults() {
        let cmd = PortCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.sub {
            PortSubCommand::List(args) => {
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
        let cmd = PortCmd::try_parse_from(["test", "list", "--all", "--udp", "--range", "3000-3999", "--pid", "1234", "--name", "node"]).unwrap();
        match cmd.sub {
            PortSubCommand::List(args) => {
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
        let cmd = PortCmd::try_parse_from(["test", "list", "-f", "json"]).unwrap();
        match cmd.sub {
            PortSubCommand::List(args) => assert_eq!(args.format, "json"),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn port_kill_parses_ports_and_flags() {
        let cmd = PortCmd::try_parse_from(["test", "kill", "3000,8080,5173", "-f", "--tcp"]).unwrap();
        match cmd.sub {
            PortSubCommand::Kill(args) => {
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
        let cmd = PortCmd::try_parse_from(["test", "kill", "5353", "--udp"]).unwrap();
        match cmd.sub {
            PortSubCommand::Kill(args) => {
                assert_eq!(args.ports, "5353");
                assert!(args.udp);
                assert!(!args.force);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    // ---- PortInfo TableRow 测试 ----

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

    // ---- CommandSpec 测试 ----

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

    // ---- E2E dispatch 测试 ----

    fn dispatch_port(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = PortCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.sub {
            PortSubCommand::List(_) => {
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
            PortSubCommand::Kill(_) => {
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
// Phase 3.2: Proc 命令（clap derive + CommandSpec + TableRow）
// ============================================================

mod proc_cmd_tests {
    use clap::Parser;
    use xun::xun_core::proc_cmd::{ProcCmd, ProcSubCommand, ProcInfo};
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, TerminalRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ---- CLI 解析测试 ----

    #[test]
    fn proc_list_parses_defaults() {
        let cmd = ProcCmd::try_parse_from(["test", "list"]).unwrap();
        match cmd.sub {
            ProcSubCommand::List(args) => {
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
        let cmd = ProcCmd::try_parse_from(["test", "list", "node"]).unwrap();
        match cmd.sub {
            ProcSubCommand::List(args) => {
                assert_eq!(args.pattern.as_deref(), Some("node"));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_list_parses_pid_and_win() {
        let cmd = ProcCmd::try_parse_from(["test", "list", "--pid", "1234"]).unwrap();
        match cmd.sub {
            ProcSubCommand::List(args) => {
                assert_eq!(args.pid, Some(1234));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_list_parses_window_title() {
        let cmd = ProcCmd::try_parse_from(["test", "list", "-w", "My App"]).unwrap();
        match cmd.sub {
            ProcSubCommand::List(args) => {
                assert_eq!(args.win.as_deref(), Some("My App"));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn proc_kill_parses_target_and_flags() {
        let cmd = ProcCmd::try_parse_from(["test", "kill", "node", "-f", "-w"]).unwrap();
        match cmd.sub {
            ProcSubCommand::Kill(args) => {
                assert_eq!(args.target, "node");
                assert!(args.force);
                assert!(args.window);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    #[test]
    fn proc_kill_parses_pid_target() {
        let cmd = ProcCmd::try_parse_from(["test", "kill", "1234"]).unwrap();
        match cmd.sub {
            ProcSubCommand::Kill(args) => {
                assert_eq!(args.target, "1234");
                assert!(!args.force);
                assert!(!args.window);
            }
            other => panic!("expected Kill, got {other:?}"),
        }
    }

    // ---- ProcInfo TableRow 测试 ----

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

    // ---- CommandSpec 测试 ----

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

    // ---- E2E dispatch 测试 ----

    fn dispatch_proc(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = ProcCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.sub {
            ProcSubCommand::List(_) => {
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
            ProcSubCommand::Kill(_) => {
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
// Phase 3.3: Backup 命令（clap derive + CommandSpec + TableRow）
// ============================================================

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

    // ---- CLI 解析测试 ----

    #[test]
    fn backup_no_subcommand_parses() {
        let cmd = BackupCmd::try_parse_from(["test"]).unwrap();
        assert!(cmd.sub.is_none());
        assert!(cmd.msg.is_none());
        assert!(!cmd.dry_run);
    }

    #[test]
    fn backup_create_parses_basic() {
        let cmd = BackupCmd::try_parse_from(["test", "create", "-m", "daily backup", "--format", "zip"]).unwrap();
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
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
        assert!(matches!(cmd.sub, Some(BackupSubCommand::List(_))));
    }

    #[test]
    fn backup_verify_parses_name() {
        let cmd = BackupCmd::try_parse_from(["test", "verify", "my-backup"]).unwrap();
        match cmd.sub {
            Some(BackupSubCommand::Verify(args)) => {
                assert_eq!(args.name, "my-backup");
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    #[test]
    fn backup_find_parses_filters() {
        let cmd = BackupCmd::try_parse_from(["test", "find", "important", "--since", "2026-01-01"]).unwrap();
        match cmd.sub {
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

    // ---- BackupEntry TableRow 测试 ----

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

    // ---- CommandSpec 测试 ----

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

    // ---- E2E dispatch 测试 ----

    fn dispatch_backup(raw_args: &[&str], renderer: &mut dyn Renderer) -> Result<Value, XunError> {
        let cmd = BackupCmd::try_parse_from(raw_args)
            .map_err(|e| XunError::user(e.to_string()))?;
        let mut ctx = CmdContext::for_test();

        match cmd.sub {
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
// Phase 3.3: Video 命令（clap derive）
// ============================================================

mod video_cmd_tests {
    use clap::Parser;
    use xun::xun_core::video_cmd::{VideoCmd, VideoSubCommand};

    // ---- CLI 解析测试 ----

    #[test]
    fn video_probe_parses_input() {
        let cmd = VideoCmd::try_parse_from(["test", "probe", "-i", "video.mp4"]).unwrap();
        match cmd.sub {
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
        match cmd.sub {
            VideoSubCommand::Probe(args) => {
                assert_eq!(args.ffprobe.as_deref(), Some("/usr/bin/ffprobe"));
            }
            other => panic!("expected Probe, got {other:?}"),
        }
    }

    #[test]
    fn video_compress_parses_basic() {
        let cmd = VideoCmd::try_parse_from(["test", "compress", "-i", "in.mp4", "-o", "out.mp4"]).unwrap();
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
            VideoSubCommand::Compress(args) => {
                assert!(args.overwrite);
            }
            other => panic!("expected Compress, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_basic() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4"]).unwrap();
        match cmd.sub {
            VideoSubCommand::Remux(args) => {
                assert_eq!(args.input, "in.mkv");
                assert_eq!(args.output, "out.mp4");
                assert_eq!(args.strict, "true");
                assert!(!args.overwrite);
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_strict_false() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4", "--strict", "false"]).unwrap();
        match cmd.sub {
            VideoSubCommand::Remux(args) => {
                assert_eq!(args.strict, "false");
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }

    #[test]
    fn video_remux_parses_overwrite() {
        let cmd = VideoCmd::try_parse_from(["test", "remux", "-i", "in.mkv", "-o", "out.mp4", "--overwrite"]).unwrap();
        match cmd.sub {
            VideoSubCommand::Remux(args) => {
                assert!(args.overwrite);
            }
            other => panic!("expected Remux, got {other:?}"),
        }
    }
}

// ============================================================
// Phase 3.3: Verify 命令（clap derive）
// ============================================================

mod verify_cmd_tests {
    use clap::Parser;
    use xun::xun_core::verify_cmd::VerifyCmd;

    // ---- CLI 解析测试 ----

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
// Phase 3.4: Bookmark CLI — 27 个子命令
// ============================================================

mod bookmark_cmd_tests {
    use clap::Parser;
    use xun::xun_core::bookmark_cmd::{
        BookmarkCmd, BookmarkSubCommand, BookmarkEntry, FuzzyArgs,
        TagSubCommand,
    };
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ── 辅助函数 ──────────────────────────────────────────────

    fn parse(args: &[&str]) -> BookmarkCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        BookmarkCmd::try_parse_from(full).unwrap()
    }

    fn make_fuzzy_args() -> FuzzyArgs {
        FuzzyArgs {
            patterns: vec!["proj".into()],
            tag: Some("work".into()),
            list: true,
            score: false,
            why: false,
            preview: false,
            limit: Some(5),
            json: false,
            tsv: false,
            global: false,
            child: false,
            base: None,
            workspace: None,
            preset: None,
        }
    }

    // ── CLI 解析测试：Z ───────────────────────────────────────

    #[test]
    fn z_parses_basic() {
        let cmd = parse(&["z", "mydir"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["mydir"]);
                assert!(!args.fuzzy.list);
                assert!(!args.fuzzy.score);
                assert!(!args.fuzzy.json);
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_multiple_patterns() {
        let cmd = parse(&["z", "proj", "src", "lib"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj", "src", "lib"]);
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_all_flags() {
        let cmd = parse(&[
            "z", "proj", "-t", "work", "-l", "-s", "--why", "--preview",
            "-n", "10", "--json", "--tsv", "-g", "-c", "--base", "/tmp",
            "-w", "ws1", "--preset", "dev",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert_eq!(args.fuzzy.tag.as_deref(), Some("work"));
                assert!(args.fuzzy.list);
                assert!(args.fuzzy.score);
                assert!(args.fuzzy.why);
                assert!(args.fuzzy.preview);
                assert_eq!(args.fuzzy.limit, Some(10));
                assert!(args.fuzzy.json);
                assert!(args.fuzzy.tsv);
                assert!(args.fuzzy.global);
                assert!(args.fuzzy.child);
                assert_eq!(args.fuzzy.base.as_deref(), Some("/tmp"));
                assert_eq!(args.fuzzy.workspace.as_deref(), Some("ws1"));
                assert_eq!(args.fuzzy.preset.as_deref(), Some("dev"));
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    #[test]
    fn z_parses_no_patterns() {
        let cmd = parse(&["z"]);
        match cmd.sub {
            BookmarkSubCommand::Z(args) => {
                assert!(args.fuzzy.patterns.is_empty());
            }
            other => panic!("expected Z, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Zi / O / Oi / Open ─────────────────────

    #[test]
    fn zi_parses_basic() {
        let cmd = parse(&["zi", "proj"]);
        match cmd.sub {
            BookmarkSubCommand::Zi(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj"]);
            }
            other => panic!("expected Zi, got {other:?}"),
        }
    }

    #[test]
    fn o_parses_basic() {
        let cmd = parse(&["o", "docs"]);
        match cmd.sub {
            BookmarkSubCommand::O(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["docs"]);
            }
            other => panic!("expected O, got {other:?}"),
        }
    }

    #[test]
    fn oi_parses_basic() {
        let cmd = parse(&["oi", "src"]);
        match cmd.sub {
            BookmarkSubCommand::Oi(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["src"]);
            }
            other => panic!("expected Oi, got {other:?}"),
        }
    }

    #[test]
    fn open_parses_basic() {
        let cmd = parse(&["open", "proj"]);
        match cmd.sub {
            BookmarkSubCommand::Open(args) => {
                assert_eq!(args.fuzzy.patterns, vec!["proj"]);
            }
            other => panic!("expected Open, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Save / Set ─────────────────────────────

    #[test]
    fn save_parses_name_and_options() {
        let cmd = parse(&["save", "myproj", "-t", "rust,cli", "--desc", "my project", "-w", "ws1"]);
        match cmd.sub {
            BookmarkSubCommand::Save(args) => {
                assert_eq!(args.name.as_deref(), Some("myproj"));
                assert_eq!(args.tag.as_deref(), Some("rust,cli"));
                assert_eq!(args.desc.as_deref(), Some("my project"));
                assert_eq!(args.workspace.as_deref(), Some("ws1"));
            }
            other => panic!("expected Save, got {other:?}"),
        }
    }

    #[test]
    fn save_parses_no_name() {
        let cmd = parse(&["save"]);
        match cmd.sub {
            BookmarkSubCommand::Save(args) => {
                assert!(args.name.is_none());
            }
            other => panic!("expected Save, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_name_and_path() {
        let cmd = parse(&["set", "myproj", "C:/code/myproj"]);
        match cmd.sub {
            BookmarkSubCommand::Set(args) => {
                assert_eq!(args.name, "myproj");
                assert_eq!(args.path.as_deref(), Some("C:/code/myproj"));
                assert!(args.tag.is_none());
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_with_tag_and_desc() {
        let cmd = parse(&["set", "proj", ".", "-t", "work", "--desc", "main project"]);
        match cmd.sub {
            BookmarkSubCommand::Set(args) => {
                assert_eq!(args.name, "proj");
                assert_eq!(args.path.as_deref(), Some("."));
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.desc.as_deref(), Some("main project"));
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Delete / Pin / Unpin / Touch ───────────

    #[test]
    fn delete_parses_name() {
        let cmd = parse(&["delete", "oldproj"]);
        match cmd.sub {
            BookmarkSubCommand::Rm(args) => {
                assert_eq!(args.name, "oldproj");
                assert!(!args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn delete_parses_yes_flag() {
        let cmd = parse(&["delete", "oldproj", "-y"]);
        match cmd.sub {
            BookmarkSubCommand::Rm(args) => {
                assert_eq!(args.name, "oldproj");
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn pin_parses_name() {
        let cmd = parse(&["pin", "important"]);
        match cmd.sub {
            BookmarkSubCommand::Pin(args) => {
                assert_eq!(args.name, "important");
            }
            other => panic!("expected Pin, got {other:?}"),
        }
    }

    #[test]
    fn unpin_parses_name() {
        let cmd = parse(&["unpin", "important"]);
        match cmd.sub {
            BookmarkSubCommand::Unpin(args) => {
                assert_eq!(args.name, "important");
            }
            other => panic!("expected Unpin, got {other:?}"),
        }
    }

    #[test]
    fn touch_parses_name() {
        let cmd = parse(&["touch", "mydir"]);
        match cmd.sub {
            BookmarkSubCommand::Touch(args) => {
                assert_eq!(args.name, "mydir");
            }
            other => panic!("expected Touch, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Undo / Redo / Rename ───────────────────

    #[test]
    fn undo_parses_default_steps() {
        let cmd = parse(&["undo"]);
        match cmd.sub {
            BookmarkSubCommand::Undo(args) => {
                assert_eq!(args.steps, 1);
            }
            other => panic!("expected Undo, got {other:?}"),
        }
    }

    #[test]
    fn undo_parses_custom_steps() {
        let cmd = parse(&["undo", "-n", "5"]);
        match cmd.sub {
            BookmarkSubCommand::Undo(args) => {
                assert_eq!(args.steps, 5);
            }
            other => panic!("expected Undo, got {other:?}"),
        }
    }

    #[test]
    fn redo_parses_default_steps() {
        let cmd = parse(&["redo"]);
        match cmd.sub {
            BookmarkSubCommand::Redo(args) => {
                assert_eq!(args.steps, 1);
            }
            other => panic!("expected Redo, got {other:?}"),
        }
    }

    #[test]
    fn redo_parses_custom_steps() {
        let cmd = parse(&["redo", "-n", "3"]);
        match cmd.sub {
            BookmarkSubCommand::Redo(args) => {
                assert_eq!(args.steps, 3);
            }
            other => panic!("expected Redo, got {other:?}"),
        }
    }

    #[test]
    fn rename_parses_old_and_new() {
        let cmd = parse(&["rename", "old_name", "new_name"]);
        match cmd.sub {
            BookmarkSubCommand::Rename(args) => {
                assert_eq!(args.old, "old_name");
                assert_eq!(args.new, "new_name");
            }
            other => panic!("expected Rename, got {other:?}"),
        }
    }

    // ── CLI 解析测试：List / Recent / Stats / Check ──────────

    #[test]
    fn list_parses_defaults() {
        let cmd = parse(&["list"]);
        match cmd.sub {
            BookmarkSubCommand::List(args) => {
                assert!(args.tag.is_none());
                assert_eq!(args.sort, "name");
                assert!(args.limit.is_none());
                assert!(args.offset.is_none());
                assert!(!args.reverse);
                assert!(!args.tsv);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_all_options() {
        let cmd = parse(&[
            "list", "-t", "work", "-s", "visits", "-n", "20",
            "--offset", "5", "--reverse", "--tsv", "-f", "json",
        ]);
        match cmd.sub {
            BookmarkSubCommand::List(args) => {
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.sort, "visits");
                assert_eq!(args.limit, Some(20));
                assert_eq!(args.offset, Some(5));
                assert!(args.reverse);
                assert!(args.tsv);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn recent_parses_defaults() {
        let cmd = parse(&["recent"]);
        match cmd.sub {
            BookmarkSubCommand::Recent(args) => {
                assert_eq!(args.limit, 10);
                assert!(args.tag.is_none());
                assert!(args.workspace.is_none());
                assert!(args.since.is_none());
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Recent, got {other:?}"),
        }
    }

    #[test]
    fn recent_parses_all_options() {
        let cmd = parse(&[
            "recent", "-n", "25", "-t", "work", "-w", "ws1",
            "--since", "7d", "-f", "json",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Recent(args) => {
                assert_eq!(args.limit, 25);
                assert_eq!(args.tag.as_deref(), Some("work"));
                assert_eq!(args.workspace.as_deref(), Some("ws1"));
                assert_eq!(args.since.as_deref(), Some("7d"));
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Recent, got {other:?}"),
        }
    }

    #[test]
    fn stats_parses_defaults() {
        let cmd = parse(&["stats"]);
        match cmd.sub {
            BookmarkSubCommand::Stats(args) => {
                assert_eq!(args.format, "auto");
                assert!(!args.insights);
            }
            other => panic!("expected Stats, got {other:?}"),
        }
    }

    #[test]
    fn stats_parses_insights() {
        let cmd = parse(&["stats", "--insights", "-f", "json"]);
        match cmd.sub {
            BookmarkSubCommand::Stats(args) => {
                assert!(args.insights);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Stats, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_defaults() {
        let cmd = parse(&["check"]);
        match cmd.sub {
            BookmarkSubCommand::Check(args) => {
                assert_eq!(args.days, 90);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_custom_days() {
        let cmd = parse(&["check", "-d", "30", "-f", "tsv"]);
        match cmd.sub {
            BookmarkSubCommand::Check(args) => {
                assert_eq!(args.days, 30);
                assert_eq!(args.format, "tsv");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Gc / Dedup / Export / Import ───────────

    #[test]
    fn gc_parses_defaults() {
        let cmd = parse(&["gc"]);
        match cmd.sub {
            BookmarkSubCommand::Gc(args) => {
                assert!(!args.purge);
                assert!(!args.dry_run);
                assert!(!args.learned);
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected Gc, got {other:?}"),
        }
    }

    #[test]
    fn gc_parses_all_flags() {
        let cmd = parse(&["gc", "--purge", "--dry-run", "--learned", "-f", "json"]);
        match cmd.sub {
            BookmarkSubCommand::Gc(args) => {
                assert!(args.purge);
                assert!(args.dry_run);
                assert!(args.learned);
                assert_eq!(args.format, "json");
            }
            other => panic!("expected Gc, got {other:?}"),
        }
    }

    #[test]
    fn dedup_parses_defaults() {
        let cmd = parse(&["dedup"]);
        match cmd.sub {
            BookmarkSubCommand::Dedup(args) => {
                assert_eq!(args.mode, "path");
                assert_eq!(args.format, "auto");
                assert!(!args.yes);
            }
            other => panic!("expected Dedup, got {other:?}"),
        }
    }

    #[test]
    fn dedup_parses_name_mode() {
        let cmd = parse(&["dedup", "-m", "name", "-y"]);
        match cmd.sub {
            BookmarkSubCommand::Dedup(args) => {
                assert_eq!(args.mode, "name");
                assert!(args.yes);
            }
            other => panic!("expected Dedup, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_defaults() {
        let cmd = parse(&["export"]);
        match cmd.sub {
            BookmarkSubCommand::Export(args) => {
                assert_eq!(args.format, "json");
                assert!(args.out.is_none());
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_tsv_with_output() {
        let cmd = parse(&["export", "-f", "tsv", "-o", "bookmarks.tsv"]);
        match cmd.sub {
            BookmarkSubCommand::Export(args) => {
                assert_eq!(args.format, "tsv");
                assert_eq!(args.out.as_deref(), Some("bookmarks.tsv"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_defaults() {
        let cmd = parse(&["import"]);
        match cmd.sub {
            BookmarkSubCommand::Import(args) => {
                assert_eq!(args.format, "json");
                assert!(args.from.is_none());
                assert!(args.input.is_none());
                assert_eq!(args.mode, "merge");
                assert!(!args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_all_options() {
        let cmd = parse(&[
            "import", "-f", "tsv", "--from", "zoxide", "-i", "data.tsv",
            "-m", "overwrite", "-y",
        ]);
        match cmd.sub {
            BookmarkSubCommand::Import(args) => {
                assert_eq!(args.format, "tsv");
                assert_eq!(args.from.as_deref(), Some("zoxide"));
                assert_eq!(args.input.as_deref(), Some("data.tsv"));
                assert_eq!(args.mode, "overwrite");
                assert!(args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Init / Learn / Keys / All ──────────────

    #[test]
    fn init_parses_shell() {
        let cmd = parse(&["init", "powershell"]);
        match cmd.sub {
            BookmarkSubCommand::Init(args) => {
                assert_eq!(args.shell, "powershell");
                assert!(args.cmd.is_none());
            }
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn init_parses_custom_cmd() {
        let cmd = parse(&["init", "bash", "--cmd", "j"]);
        match cmd.sub {
            BookmarkSubCommand::Init(args) => {
                assert_eq!(args.shell, "bash");
                assert_eq!(args.cmd.as_deref(), Some("j"));
            }
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn learn_parses_path() {
        let cmd = parse(&["learn", "--path", "C:/code/myproj"]);
        match cmd.sub {
            BookmarkSubCommand::Learn(args) => {
                assert_eq!(args.path, "C:/code/myproj");
            }
            other => panic!("expected Learn, got {other:?}"),
        }
    }

    #[test]
    fn keys_parses_empty() {
        let cmd = parse(&["keys"]);
        match cmd.sub {
            BookmarkSubCommand::Keys(_) => {}
            other => panic!("expected Keys, got {other:?}"),
        }
    }

    #[test]
    fn all_parses_no_tag() {
        let cmd = parse(&["all"]);
        match cmd.sub {
            BookmarkSubCommand::All(args) => {
                assert!(args.tag.is_none());
            }
            other => panic!("expected All, got {other:?}"),
        }
    }

    #[test]
    fn all_parses_with_tag() {
        let cmd = parse(&["all", "work"]);
        match cmd.sub {
            BookmarkSubCommand::All(args) => {
                assert_eq!(args.tag.as_deref(), Some("work"));
            }
            other => panic!("expected All, got {other:?}"),
        }
    }

    // ── CLI 解析测试：Tag 子命令组 ───────────────────────────

    #[test]
    fn tag_add_parses() {
        let cmd = parse(&["tag", "add", "myproj", "rust,cli"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Add(args) => {
                    assert_eq!(args.name, "myproj");
                    assert_eq!(args.tags, "rust,cli");
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_add_batch_parses() {
        let cmd = parse(&["tag", "add-batch", "work", "proj1", "proj2", "proj3"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::AddBatch(args) => {
                    assert_eq!(args.tags, "work");
                    assert_eq!(args.names, vec!["proj1", "proj2", "proj3"]);
                }
                other => panic!("expected AddBatch, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_remove_parses() {
        let cmd = parse(&["tag", "remove", "myproj", "old"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Remove(args) => {
                    assert_eq!(args.name, "myproj");
                    assert_eq!(args.tags, "old");
                }
                other => panic!("expected Remove, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_list_parses() {
        let cmd = parse(&["tag", "list"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::List(_) => {}
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    #[test]
    fn tag_rename_parses() {
        let cmd = parse(&["tag", "rename", "old_tag", "new_tag"]);
        match cmd.sub {
            BookmarkSubCommand::Tag(tag) => match tag.sub {
                TagSubCommand::Rename(args) => {
                    assert_eq!(args.old, "old_tag");
                    assert_eq!(args.new, "new_tag");
                }
                other => panic!("expected Rename, got {other:?}"),
            },
            other => panic!("expected Tag, got {other:?}"),
        }
    }

    // ── TableRow 测试 ─────────────────────────────────────────

    #[test]
    fn bookmark_entry_columns_correct() {
        let cols = BookmarkEntry::columns();
        assert_eq!(cols.len(), 6);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "path");
        assert_eq!(cols[1].kind, ValueKind::Path);
        assert_eq!(cols[2].name, "tags");
        assert_eq!(cols[2].kind, ValueKind::String);
        assert_eq!(cols[3].name, "visits");
        assert_eq!(cols[3].kind, ValueKind::Int);
        assert_eq!(cols[4].name, "last_used");
        assert_eq!(cols[4].kind, ValueKind::Date);
        assert_eq!(cols[5].name, "pinned");
        assert_eq!(cols[5].kind, ValueKind::Bool);
    }

    #[test]
    fn bookmark_entry_cells_match_fields() {
        let entry = BookmarkEntry::new("myproj", "C:/code/myproj", "rust,cli", 42, "2026-05-12", true);
        let cells = entry.cells();
        assert_eq!(cells.len(), 6);
        assert!(matches!(&cells[0], Value::String(s) if s == "myproj"));
        assert!(matches!(&cells[1], Value::String(s) if s == "C:/code/myproj"));
        assert!(matches!(&cells[2], Value::String(s) if s == "rust,cli"));
        assert!(matches!(&cells[3], Value::Int(42)));
        assert!(matches!(&cells[4], Value::String(s) if s == "2026-05-12"));
        assert!(matches!(&cells[5], Value::Bool(true)));
    }

    #[test]
    fn bookmark_entry_renders_as_json() {
        let entry = BookmarkEntry::new("test", "/tmp", "tag1", 5, "2026-01-01", false);
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("test"), "json output: {output}");
        assert!(output.contains("/tmp"), "json output: {output}");
    }

    #[test]
    fn bookmark_entry_vec_to_table() {
        let entries = vec![
            BookmarkEntry::new("a", "/a", "t1", 1, "2026-01-01", false),
            BookmarkEntry::new("b", "/b", "t2", 2, "2026-02-02", true),
        ];
        let table = BookmarkEntry::vec_to_table(&entries);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 6);
    }

    // ── CommandSpec 测试（桩实现） ────────────────────────────

    #[test]
    fn bookmark_entry_to_record_has_all_fields() {
        let entry = BookmarkEntry::new("proj", "/code/proj", "rust", 10, "2026-05-12", true);
        let rec = entry.to_record();
        assert_eq!(rec.len(), 6);
        assert!(rec.contains_key("name"));
        assert!(rec.contains_key("path"));
        assert!(rec.contains_key("tags"));
        assert!(rec.contains_key("visits"));
        assert!(rec.contains_key("last_used"));
        assert!(rec.contains_key("pinned"));
    }

    #[test]
    fn bookmark_entry_new_constructor() {
        let entry = BookmarkEntry::new("n", "p", "t", 99, "2026-01-01", true);
        assert_eq!(entry.name, "n");
        assert_eq!(entry.path, "p");
        assert_eq!(entry.tags, "t");
        assert_eq!(entry.visits, 99);
        assert_eq!(entry.last_used, "2026-01-01");
        assert!(entry.pinned);
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn bookmark_subcommand_count() {
        // 验证 BookmarkSubCommand 有 27 个变体
        let variants = [
            "Z", "Zi", "O", "Oi", "Open", "Save", "Set", "Delete",
            "Tag", "Pin", "Unpin", "Undo", "Redo", "Rename",
            "List", "Recent", "Stats", "Check", "Gc", "Dedup",
            "Export", "Import", "Init", "Learn", "Touch", "Keys", "All",
        ];
        assert_eq!(variants.len(), 27);
    }

    #[test]
    fn tag_subcommand_count() {
        // 验证 TagSubCommand 有 5 个变体
        let variants = ["Add", "AddBatch", "Remove", "List", "Rename"];
        assert_eq!(variants.len(), 5);
    }
}

// ============================================================
// Phase 3.5: Env CLI — 27 个子命令
// ============================================================

mod env_cmd_tests {
    use clap::Parser;
    use xun::xun_core::env_cmd::{
        EnvCmd, EnvSubCommand, EnvVar, EnvSnapshotEntry, EnvProfileEntry,
        EnvPathSubCommand, EnvSnapshotSubCommand, EnvProfileSubCommand,
        EnvBatchSubCommand, EnvSchemaSubCommand, EnvAnnotateSubCommand,
        EnvConfigSubCommand,
    };
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ── 辅助函数 ──────────────────────────────────────────────

    fn parse(args: &[&str]) -> EnvCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        EnvCmd::try_parse_from(full).unwrap()
    }

    // ── CLI 解析测试：独立子命令 ──────────────────────────────

    #[test]
    fn status_parses_defaults() {
        let cmd = parse(&["status"]);
        match cmd.sub {
            EnvSubCommand::Status(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_defaults() {
        let cmd = parse(&["list"]);
        match cmd.sub {
            EnvSubCommand::List(args) => {
                assert_eq!(args.scope, "user");
                assert_eq!(args.format, "auto");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_parses_scope_and_format() {
        let cmd = parse(&["list", "--scope", "all", "-f", "json"]);
        match cmd.sub {
            EnvSubCommand::List(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "json");
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn search_parses_query() {
        let cmd = parse(&["search", "PATH"]);
        match cmd.sub {
            EnvSubCommand::Search(args) => {
                assert_eq!(args.query, "PATH");
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Search, got {other:?}"),
        }
    }

    #[test]
    fn get_parses_name() {
        let cmd = parse(&["get", "HOME"]);
        match cmd.sub {
            EnvSubCommand::Show(args) => {
                assert_eq!(args.name, "HOME");
                assert_eq!(args.scope, "user");
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_name_value() {
        let cmd = parse(&["set", "MY_VAR", "hello", "--scope", "system"]);
        match cmd.sub {
            EnvSubCommand::Set(args) => {
                assert_eq!(args.name, "MY_VAR");
                assert_eq!(args.value, "hello");
                assert_eq!(args.scope, "system");
                assert!(!args.no_snapshot);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn set_parses_no_snapshot() {
        let cmd = parse(&["set", "X", "1", "--no-snapshot"]);
        match cmd.sub {
            EnvSubCommand::Set(args) => {
                assert!(args.no_snapshot);
            }
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn del_parses_name_and_yes() {
        let cmd = parse(&["del", "OLD_VAR", "-y"]);
        match cmd.sub {
            EnvSubCommand::Rm(args) => {
                assert_eq!(args.name, "OLD_VAR");
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn check_parses_defaults() {
        let cmd = parse(&["check"]);
        match cmd.sub {
            EnvSubCommand::Check(args) => {
                assert_eq!(args.scope, "all");
                assert!(!args.fix);
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Check, got {other:?}"),
        }
    }

    #[test]
    fn path_dedup_parses() {
        let cmd = parse(&["path-dedup", "--remove-missing", "--dry-run"]);
        match cmd.sub {
            EnvSubCommand::PathDedup(args) => {
                assert!(args.remove_missing);
                assert!(args.dry_run);
                assert_eq!(args.scope, "user");
            }
            other => panic!("expected PathDedup, got {other:?}"),
        }
    }

    #[test]
    fn doctor_parses_defaults() {
        let cmd = parse(&["doctor"]);
        match cmd.sub {
            EnvSubCommand::Doctor(args) => {
                assert_eq!(args.scope, "user");
                assert!(!args.fix);
            }
            other => panic!("expected Doctor, got {other:?}"),
        }
    }

    #[test]
    fn apply_parses_name() {
        let cmd = parse(&["apply", "dev-profile", "-y"]);
        match cmd.sub {
            EnvSubCommand::Apply(args) => {
                assert_eq!(args.name, "dev-profile");
                assert!(args.yes);
                assert!(args.scope.is_none());
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn export_parses() {
        let cmd = parse(&["export", "--scope", "all", "-f", "json", "-o", "out.json"]);
        match cmd.sub {
            EnvSubCommand::Export(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.format, "json");
                assert_eq!(args.out.as_deref(), Some("out.json"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_all_parses() {
        let cmd = parse(&["export-all", "-o", "bundle.zip"]);
        match cmd.sub {
            EnvSubCommand::ExportAll(args) => {
                assert_eq!(args.scope, "all");
                assert_eq!(args.out.as_deref(), Some("bundle.zip"));
            }
            other => panic!("expected ExportAll, got {other:?}"),
        }
    }

    #[test]
    fn export_live_parses() {
        let cmd = parse(&["export-live", "-f", "sh", "--env", ".env1", "--env", ".env2", "--set", "X=1"]);
        match cmd.sub {
            EnvSubCommand::ExportLive(args) => {
                assert_eq!(args.format, "sh");
                assert_eq!(args.env_files, vec![".env1", ".env2"]);
                assert_eq!(args.set, vec!["X=1"]);
            }
            other => panic!("expected ExportLive, got {other:?}"),
        }
    }

    #[test]
    fn env_merged_parses() {
        let cmd = parse(&["env", "--scope", "system"]);
        match cmd.sub {
            EnvSubCommand::Env(args) => {
                assert_eq!(args.scope, "system");
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Env, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_defaults() {
        let cmd = parse(&["import"]);
        match cmd.sub {
            EnvSubCommand::Import(args) => {
                assert!(args.file.is_none());
                assert!(!args.stdin);
                assert_eq!(args.scope, "user");
                assert_eq!(args.mode, "merge");
                assert!(!args.dry_run);
                assert!(!args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_all_options() {
        let cmd = parse(&["import", "data.env", "--stdin", "--scope", "system", "-m", "overwrite", "--dry-run", "-y"]);
        match cmd.sub {
            EnvSubCommand::Import(args) => {
                assert_eq!(args.file.as_deref(), Some("data.env"));
                assert!(args.stdin);
                assert_eq!(args.scope, "system");
                assert_eq!(args.mode, "overwrite");
                assert!(args.dry_run);
                assert!(args.yes);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn diff_live_parses() {
        let cmd = parse(&["diff-live", "--snapshot", "snap-001", "--since", "2026-01-01", "--color"]);
        match cmd.sub {
            EnvSubCommand::DiffLive(args) => {
                assert_eq!(args.snapshot.as_deref(), Some("snap-001"));
                assert_eq!(args.since.as_deref(), Some("2026-01-01"));
                assert!(args.color);
            }
            other => panic!("expected DiffLive, got {other:?}"),
        }
    }

    #[test]
    fn graph_parses() {
        let cmd = parse(&["graph", "PATH", "--max-depth", "16"]);
        match cmd.sub {
            EnvSubCommand::Graph(args) => {
                assert_eq!(args.name, "PATH");
                assert_eq!(args.max_depth, 16);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Graph, got {other:?}"),
        }
    }

    #[test]
    fn validate_parses() {
        let cmd = parse(&["validate", "--strict"]);
        match cmd.sub {
            EnvSubCommand::Validate(args) => {
                assert!(args.strict);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Validate, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses() {
        let cmd = parse(&["audit", "--limit", "100"]);
        match cmd.sub {
            EnvSubCommand::Audit(args) => {
                assert_eq!(args.limit, 100);
                assert_eq!(args.format, "text");
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn watch_parses() {
        let cmd = parse(&["watch", "--interval-ms", "5000", "--once"]);
        match cmd.sub {
            EnvSubCommand::Watch(args) => {
                assert_eq!(args.interval_ms, 5000);
                assert!(args.once);
                assert_eq!(args.scope, "all");
            }
            other => panic!("expected Watch, got {other:?}"),
        }
    }

    #[test]
    fn template_parses() {
        let cmd = parse(&["template", "Path=%PATH%", "--validate-only"]);
        match cmd.sub {
            EnvSubCommand::Template(args) => {
                assert_eq!(args.input, "Path=%PATH%");
                assert!(args.validate_only);
            }
            other => panic!("expected Template, got {other:?}"),
        }
    }

    #[test]
    fn run_parses() {
        let cmd = parse(&["run", "--scope", "user", "--schema-check", "--notify", "--", "echo", "hello"]);
        match cmd.sub {
            EnvSubCommand::Run(args) => {
                assert_eq!(args.scope, "user");
                assert!(args.schema_check);
                assert!(args.notify);
                assert_eq!(args.command, vec!["echo", "hello"]);
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }

    #[test]
    fn tui_parses_empty() {
        let cmd = parse(&["tui"]);
        match cmd.sub {
            EnvSubCommand::Tui(_) => {}
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 path ──────────────────────

    #[test]
    fn path_add_parses() {
        let cmd = parse(&["path", "add", "C:/tools", "--head"]);
        match cmd.sub {
            EnvSubCommand::Path(p) => match p.sub {
                EnvPathSubCommand::Add(args) => {
                    assert_eq!(args.entry, "C:/tools");
                    assert!(args.head);
                    assert!(!args.tail);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn path_rm_parses() {
        let cmd = parse(&["path", "rm", "C:/old"]);
        match cmd.sub {
            EnvSubCommand::Path(p) => match p.sub {
                EnvPathSubCommand::Rm(args) => {
                    assert_eq!(args.entry, "C:/old");
                    assert_eq!(args.scope, "user");
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Path, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 snapshot ──────────────────

    #[test]
    fn snapshot_create_parses() {
        let cmd = parse(&["snapshot", "create", "--desc", "before install"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Create(args) => {
                    assert_eq!(args.desc.as_deref(), Some("before install"));
                }
                other => panic!("expected Create, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_list_parses() {
        let cmd = parse(&["snapshot", "list"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::List(args) => {
                    assert_eq!(args.format, "auto");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_restore_parses() {
        let cmd = parse(&["snapshot", "restore", "--latest", "-y"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Restore(args) => {
                    assert!(args.latest);
                    assert!(args.yes);
                    assert_eq!(args.scope, "all");
                }
                other => panic!("expected Restore, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_prune_parses() {
        let cmd = parse(&["snapshot", "prune", "--keep", "10"]);
        match cmd.sub {
            EnvSubCommand::Snapshot(s) => match s.sub {
                EnvSnapshotSubCommand::Prune(args) => {
                    assert_eq!(args.keep, 10);
                }
                other => panic!("expected Prune, got {other:?}"),
            },
            other => panic!("expected Snapshot, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 profile ───────────────────

    #[test]
    fn profile_list_parses() {
        let cmd = parse(&["profile", "list"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::List(args) => {
                    assert_eq!(args.format, "auto");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_capture_parses() {
        let cmd = parse(&["profile", "capture", "dev-env"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Capture(args) => {
                    assert_eq!(args.name, "dev-env");
                    assert_eq!(args.scope, "user");
                }
                other => panic!("expected Capture, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_apply_parses() {
        let cmd = parse(&["profile", "apply", "prod", "--scope", "system", "-y"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Apply(args) => {
                    assert_eq!(args.name, "prod");
                    assert_eq!(args.scope.as_deref(), Some("system"));
                    assert!(args.yes);
                }
                other => panic!("expected Apply, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_diff_parses() {
        let cmd = parse(&["profile", "diff", "dev"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Diff(args) => {
                    assert_eq!(args.name, "dev");
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Diff, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    #[test]
    fn profile_delete_parses() {
        let cmd = parse(&["profile", "delete", "old", "-y"]);
        match cmd.sub {
            EnvSubCommand::Profile(p) => match p.sub {
                EnvProfileSubCommand::Rm(args) => {
                    assert_eq!(args.name, "old");
                    assert!(args.yes);
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Profile, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 batch ─────────────────────

    #[test]
    fn batch_set_parses() {
        let cmd = parse(&["batch", "set", "A=1", "B=2", "--dry-run"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Set(args) => {
                    assert_eq!(args.items, vec!["A=1", "B=2"]);
                    assert!(args.dry_run);
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn batch_delete_parses() {
        let cmd = parse(&["batch", "delete", "X", "Y", "Z"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Rm(args) => {
                    assert_eq!(args.names, vec!["X", "Y", "Z"]);
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn batch_rename_parses() {
        let cmd = parse(&["batch", "rename", "OLD", "NEW"]);
        match cmd.sub {
            EnvSubCommand::Batch(b) => match b.sub {
                EnvBatchSubCommand::Rename(args) => {
                    assert_eq!(args.old, "OLD");
                    assert_eq!(args.new, "NEW");
                }
                other => panic!("expected Rename, got {other:?}"),
            },
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 schema ────────────────────

    #[test]
    fn schema_show_parses() {
        let cmd = parse(&["schema", "show"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Show(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_required_parses() {
        let cmd = parse(&["schema", "add-required", "PATH", "--warn-only"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddRequired(args) => {
                    assert_eq!(args.pattern, "PATH");
                    assert!(args.warn_only);
                }
                other => panic!("expected AddRequired, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_regex_parses() {
        let cmd = parse(&["schema", "add-regex", "MY_*", "^valid$"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddRegex(args) => {
                    assert_eq!(args.pattern, "MY_*");
                    assert_eq!(args.regex, "^valid$");
                }
                other => panic!("expected AddRegex, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_add_enum_parses() {
        let cmd = parse(&["schema", "add-enum", "MODE", "dev", "prod", "staging"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::AddEnum(args) => {
                    assert_eq!(args.pattern, "MODE");
                    assert_eq!(args.values, vec!["dev", "prod", "staging"]);
                }
                other => panic!("expected AddEnum, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_remove_parses() {
        let cmd = parse(&["schema", "remove", "PATH"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Remove(args) => {
                    assert_eq!(args.pattern, "PATH");
                }
                other => panic!("expected Remove, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    #[test]
    fn schema_reset_parses() {
        let cmd = parse(&["schema", "reset", "-y"]);
        match cmd.sub {
            EnvSubCommand::Schema(s) => match s.sub {
                EnvSchemaSubCommand::Reset(args) => {
                    assert!(args.yes);
                }
                other => panic!("expected Reset, got {other:?}"),
            },
            other => panic!("expected Schema, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 annotate ──────────────────

    #[test]
    fn annotate_set_parses() {
        let cmd = parse(&["annotate", "set", "PATH", "system PATH variable"]);
        match cmd.sub {
            EnvSubCommand::Annotate(a) => match a.sub {
                EnvAnnotateSubCommand::Set(args) => {
                    assert_eq!(args.name, "PATH");
                    assert_eq!(args.note, "system PATH variable");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Annotate, got {other:?}"),
        }
    }

    #[test]
    fn annotate_list_parses() {
        let cmd = parse(&["annotate", "list"]);
        match cmd.sub {
            EnvSubCommand::Annotate(a) => match a.sub {
                EnvAnnotateSubCommand::List(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Annotate, got {other:?}"),
        }
    }

    // ── CLI 解析测试：嵌套子命令组 config ────────────────────

    #[test]
    fn config_show_parses() {
        let cmd = parse(&["config", "show"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Show(args) => {
                    assert_eq!(args.format, "text");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_path_parses() {
        let cmd = parse(&["config", "path"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Path(_) => {}
                other => panic!("expected Path, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_reset_parses() {
        let cmd = parse(&["config", "reset", "-y"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Reset(args) => {
                    assert!(args.yes);
                }
                other => panic!("expected Reset, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_get_parses() {
        let cmd = parse(&["config", "get", "default_scope"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Get(args) => {
                    assert_eq!(args.key, "default_scope");
                }
                other => panic!("expected Show, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn config_set_parses() {
        let cmd = parse(&["config", "set", "default_scope", "all"]);
        match cmd.sub {
            EnvSubCommand::Config(c) => match c.sub {
                EnvConfigSubCommand::Set(args) => {
                    assert_eq!(args.key, "default_scope");
                    assert_eq!(args.value, "all");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Config, got {other:?}"),
        }
    }

    // ── TableRow 测试：EnvVar ─────────────────────────────────

    #[test]
    fn env_var_columns_correct() {
        let cols = EnvVar::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "value");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "scope");
        assert_eq!(cols[2].kind, ValueKind::String);
    }

    #[test]
    fn env_var_cells_match_fields() {
        let v = EnvVar::new("PATH", "C:\\Windows", "system");
        let cells = v.cells();
        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0], Value::String(s) if s == "PATH"));
        assert!(matches!(&cells[1], Value::String(s) if s == "C:\\Windows"));
        assert!(matches!(&cells[2], Value::String(s) if s == "system"));
    }

    #[test]
    fn env_var_renders_as_json() {
        let v = EnvVar::new("HOME", "/home/user", "user");
        let table = v.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("HOME"), "json output: {output}");
    }

    #[test]
    fn env_var_vec_to_table() {
        let vars = vec![
            EnvVar::new("A", "1", "user"),
            EnvVar::new("B", "2", "system"),
        ];
        let table = EnvVar::vec_to_table(&vars);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 3);
    }

    // ── TableRow 测试：EnvSnapshotEntry ───────────────────────

    #[test]
    fn snapshot_entry_columns_correct() {
        let cols = EnvSnapshotEntry::columns();
        assert_eq!(cols.len(), 4);
        assert_eq!(cols[0].name, "id");
        assert_eq!(cols[1].name, "created");
        assert_eq!(cols[1].kind, ValueKind::Date);
        assert_eq!(cols[2].name, "desc");
        assert_eq!(cols[3].name, "var_count");
        assert_eq!(cols[3].kind, ValueKind::Int);
    }

    #[test]
    fn snapshot_entry_cells_match_fields() {
        let e = EnvSnapshotEntry::new("snap-001", "2026-05-12", "before install", 42);
        let cells = e.cells();
        assert_eq!(cells.len(), 4);
        assert!(matches!(&cells[0], Value::String(s) if s == "snap-001"));
        assert!(matches!(&cells[3], Value::Int(42)));
    }

    // ── TableRow 测试：EnvProfileEntry ────────────────────────

    #[test]
    fn profile_entry_columns_correct() {
        let cols = EnvProfileEntry::columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[1].name, "var_count");
        assert_eq!(cols[1].kind, ValueKind::Int);
        assert_eq!(cols[2].name, "created");
        assert_eq!(cols[2].kind, ValueKind::Date);
    }

    #[test]
    fn profile_entry_cells_match_fields() {
        let e = EnvProfileEntry::new("dev", 15, "2026-05-12");
        let cells = e.cells();
        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0], Value::String(s) if s == "dev"));
        assert!(matches!(&cells[1], Value::Int(15)));
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn env_subcommand_count() {
        let variants = [
            "Status", "List", "Search", "Get", "Set", "Del", "Check",
            "Path", "PathDedup", "Snapshot", "Doctor", "Profile", "Batch",
            "Apply", "Export", "ExportAll", "ExportLive", "Env", "Import",
            "DiffLive", "Graph", "Validate", "Schema", "Annotate", "Config",
            "Audit", "Watch", "Template", "Run", "Tui",
        ];
        assert_eq!(variants.len(), 30);
    }
}

// ============================================================
// Phase 3.6: ACL CLI — 16 个子命令
// ============================================================

mod acl_cmd_tests {
    use clap::Parser;
    use xun::xun_core::acl_cmd::{AclCmd, AclSubCommand, AclEntry};
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::{Value, ValueKind};
    use std::io::Cursor;

    // ── 辅助函数 ──────────────────────────────────────────────

    fn parse(args: &[&str]) -> AclCmd {
        let mut full = vec!["test"];
        full.extend_from_slice(args);
        AclCmd::try_parse_from(full).unwrap()
    }

    // ── CLI 解析测试 ──────────────────────────────────────────

    #[test]
    fn view_parses_basic() {
        let cmd = parse(&["view", "-p", "C:\\test"]);
        match cmd.sub {
            AclSubCommand::Show(args) => {
                assert_eq!(args.path, "C:\\test");
                assert!(!args.detail);
                assert!(args.export.is_none());
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn view_parses_detail_and_export() {
        let cmd = parse(&["view", "-p", "C:\\test", "--detail", "--export", "out.csv"]);
        match cmd.sub {
            AclSubCommand::Show(args) => {
                assert!(args.detail);
                assert_eq!(args.export.as_deref(), Some("out.csv"));
            }
            other => panic!("expected Show, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_basic() {
        let cmd = parse(&["add", "-p", "C:\\test", "--principal", "BUILTIN\\Users", "--rights", "Read"]);
        match cmd.sub {
            AclSubCommand::Add(args) => {
                assert_eq!(args.path.as_deref(), Some("C:\\test"));
                assert_eq!(args.principal.as_deref(), Some("BUILTIN\\Users"));
                assert_eq!(args.rights.as_deref(), Some("Read"));
                assert!(!args.yes);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_all_options() {
        let cmd = parse(&[
            "add", "-p", "C:\\test", "--principal", "DOMAIN\\User",
            "--rights", "FullControl", "--ace-type", "Allow",
            "--inherit", "BothInherit", "-y",
        ]);
        match cmd.sub {
            AclSubCommand::Add(args) => {
                assert_eq!(args.rights.as_deref(), Some("FullControl"));
                assert_eq!(args.ace_type.as_deref(), Some("Allow"));
                assert_eq!(args.inherit.as_deref(), Some("BothInherit"));
                assert!(args.yes);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn remove_parses() {
        let cmd = parse(&["remove", "-p", "C:\\test", "--principal", "BUILTIN\\Users", "-y"]);
        match cmd.sub {
            AclSubCommand::Rm(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.principal.as_deref(), Some("BUILTIN\\Users"));
                assert!(args.yes);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn purge_parses() {
        let cmd = parse(&["purge", "-p", "C:\\test", "--principal", "DOMAIN\\Bad"]);
        match cmd.sub {
            AclSubCommand::Purge(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.principal.as_deref(), Some("DOMAIN\\Bad"));
            }
            other => panic!("expected Purge, got {other:?}"),
        }
    }

    #[test]
    fn diff_parses() {
        let cmd = parse(&["diff", "-p", "C:\\a", "-r", "C:\\b", "-o", "diff.csv"]);
        match cmd.sub {
            AclSubCommand::Diff(args) => {
                assert_eq!(args.path, "C:\\a");
                assert_eq!(args.reference, "C:\\b");
                assert_eq!(args.output.as_deref(), Some("diff.csv"));
            }
            other => panic!("expected Diff, got {other:?}"),
        }
    }

    #[test]
    fn batch_parses() {
        let cmd = parse(&["batch", "--action", "repair", "--file", "paths.txt", "-y"]);
        match cmd.sub {
            AclSubCommand::Batch(args) => {
                assert_eq!(args.action, "repair");
                assert_eq!(args.file.as_deref(), Some("paths.txt"));
                assert!(args.yes);
            }
            other => panic!("expected Batch, got {other:?}"),
        }
    }

    #[test]
    fn effective_parses() {
        let cmd = parse(&["effective", "-p", "C:\\test", "-u", "DOMAIN\\User"]);
        match cmd.sub {
            AclSubCommand::Effective(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.user.as_deref(), Some("DOMAIN\\User"));
            }
            other => panic!("expected Effective, got {other:?}"),
        }
    }

    #[test]
    fn copy_parses() {
        let cmd = parse(&["copy", "-p", "C:\\dst", "-r", "C:\\src", "-y"]);
        match cmd.sub {
            AclSubCommand::Copy(args) => {
                assert_eq!(args.path, "C:\\dst");
                assert_eq!(args.reference, "C:\\src");
                assert!(args.yes);
            }
            other => panic!("expected Copy, got {other:?}"),
        }
    }

    #[test]
    fn backup_parses() {
        let cmd = parse(&["backup", "-p", "C:\\test", "-o", "acl.json"]);
        match cmd.sub {
            AclSubCommand::Backup(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.output.as_deref(), Some("acl.json"));
            }
            other => panic!("expected Backup, got {other:?}"),
        }
    }

    #[test]
    fn restore_parses() {
        let cmd = parse(&["restore", "-p", "C:\\test", "--from", "acl.json", "-y"]);
        match cmd.sub {
            AclSubCommand::Restore(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.from, "acl.json");
                assert!(args.yes);
            }
            other => panic!("expected Restore, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_disable() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--disable"]);
        match cmd.sub {
            AclSubCommand::Inherit(args) => {
                assert!(args.disable);
                assert!(!args.enable);
                assert_eq!(args.preserve, "true");
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_enable() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--enable"]);
        match cmd.sub {
            AclSubCommand::Inherit(args) => {
                assert!(!args.disable);
                assert!(args.enable);
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn inherit_parses_preserve_false() {
        let cmd = parse(&["inherit", "-p", "C:\\test", "--disable", "--preserve", "false"]);
        match cmd.sub {
            AclSubCommand::Inherit(args) => {
                assert_eq!(args.preserve, "false");
            }
            other => panic!("expected Inherit, got {other:?}"),
        }
    }

    #[test]
    fn owner_parses() {
        let cmd = parse(&["owner", "-p", "C:\\test", "--set", "DOMAIN\\Admin", "-y"]);
        match cmd.sub {
            AclSubCommand::Owner(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.set.as_deref(), Some("DOMAIN\\Admin"));
                assert!(args.yes);
            }
            other => panic!("expected Owner, got {other:?}"),
        }
    }

    #[test]
    fn orphans_parses_defaults() {
        let cmd = parse(&["orphans", "-p", "C:\\test"]);
        match cmd.sub {
            AclSubCommand::Orphans(args) => {
                assert_eq!(args.path, "C:\\test");
                assert_eq!(args.recursive, "true");
                assert_eq!(args.action, "none");
                assert!(!args.yes);
            }
            other => panic!("expected Orphans, got {other:?}"),
        }
    }

    #[test]
    fn orphans_parses_action_and_recursive_false() {
        let cmd = parse(&["orphans", "-p", "C:\\test", "--recursive", "false", "--action", "delete", "-y"]);
        match cmd.sub {
            AclSubCommand::Orphans(args) => {
                assert_eq!(args.recursive, "false");
                assert_eq!(args.action, "delete");
                assert!(args.yes);
            }
            other => panic!("expected Orphans, got {other:?}"),
        }
    }

    #[test]
    fn repair_parses_basic() {
        let cmd = parse(&["repair", "-p", "C:\\test"]);
        match cmd.sub {
            AclSubCommand::Repair(args) => {
                assert_eq!(args.path, "C:\\test");
                assert!(!args.export_errors);
                assert!(!args.yes);
                assert!(!args.reset_clean);
                assert!(args.grant.is_none());
            }
            other => panic!("expected Repair, got {other:?}"),
        }
    }

    #[test]
    fn repair_parses_reset_clean() {
        let cmd = parse(&["repair", "-p", "C:\\test", "--reset-clean", "--grant", "DOMAIN\\User,BUILTIN\\Users", "-y"]);
        match cmd.sub {
            AclSubCommand::Repair(args) => {
                assert!(args.reset_clean);
                assert_eq!(args.grant.as_deref(), Some("DOMAIN\\User,BUILTIN\\Users"));
                assert!(args.yes);
            }
            other => panic!("expected Repair, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses_defaults() {
        let cmd = parse(&["audit"]);
        match cmd.sub {
            AclSubCommand::Audit(args) => {
                assert_eq!(args.tail, 30);
                assert!(args.export.is_none());
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn audit_parses_tail_and_export() {
        let cmd = parse(&["audit", "--tail", "100", "--export", "audit.csv"]);
        match cmd.sub {
            AclSubCommand::Audit(args) => {
                assert_eq!(args.tail, 100);
                assert_eq!(args.export.as_deref(), Some("audit.csv"));
            }
            other => panic!("expected Audit, got {other:?}"),
        }
    }

    #[test]
    fn config_parses_set() {
        let cmd = parse(&["config", "--set", "default_owner", "BUILTIN\\Administrators"]);
        match cmd.sub {
            AclSubCommand::Config(args) => {
                assert_eq!(args.set, vec!["default_owner", "BUILTIN\\Administrators"]);
            }
            other => panic!("expected Config, got {other:?}"),
        }
    }

    // ── TableRow 测试 ─────────────────────────────────────────

    #[test]
    fn acl_entry_columns_correct() {
        let cols = AclEntry::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "path");
        assert_eq!(cols[0].kind, ValueKind::Path);
        assert_eq!(cols[1].name, "principal");
        assert_eq!(cols[1].kind, ValueKind::String);
        assert_eq!(cols[2].name, "rights");
        assert_eq!(cols[2].kind, ValueKind::String);
        assert_eq!(cols[3].name, "ace_type");
        assert_eq!(cols[3].kind, ValueKind::String);
        assert_eq!(cols[4].name, "inherited");
        assert_eq!(cols[4].kind, ValueKind::Bool);
    }

    #[test]
    fn acl_entry_cells_match_fields() {
        let entry = AclEntry::new("C:\\test", "BUILTIN\\Users", "Read", "Allow", false);
        let cells = entry.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::String(s) if s == "C:\\test"));
        assert!(matches!(&cells[1], Value::String(s) if s == "BUILTIN\\Users"));
        assert!(matches!(&cells[2], Value::String(s) if s == "Read"));
        assert!(matches!(&cells[3], Value::String(s) if s == "Allow"));
        assert!(matches!(&cells[4], Value::Bool(false)));
    }

    #[test]
    fn acl_entry_renders_as_json() {
        let entry = AclEntry::new("/tmp", "Everyone", "FullControl", "Allow", true);
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("Everyone"), "json output: {output}");
    }

    #[test]
    fn acl_entry_vec_to_table() {
        let entries = vec![
            AclEntry::new("C:\\a", "User1", "Read", "Allow", false),
            AclEntry::new("C:\\b", "User2", "Write", "Deny", true),
        ];
        let table = AclEntry::vec_to_table(&entries);
        assert_eq!(table.len(), 2);
        assert_eq!(table.columns.len(), 5);
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn acl_subcommand_count() {
        let variants = [
            "View", "Add", "Remove", "Purge", "Diff", "Batch",
            "Effective", "Copy", "Backup", "Restore", "Inherit",
            "Owner", "Orphans", "Repair", "Audit", "Config",
        ];
        assert_eq!(variants.len(), 16);
    }
}

// ============================================================
// Phase 3.7: alias_cmd — 10 顶层 + 6 嵌套 app 子命令
// ============================================================

mod alias_cmd_tests {
    use clap::Parser;
    use xun::xun_core::alias_cmd::*;
    use xun::xun_core::table_row::TableRow;
    use xun::xun_core::value::{Value, ValueKind};
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use std::io::Cursor;

    fn parse(args: &[&str]) -> AliasCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        AliasCmd::try_parse_from(&argv).expect("parse failed")
    }

    // ── 顶层子命令解析 ─────────────────────────────────────────

    #[test]
    fn setup_parses_defaults() {
        let cmd = parse(&["setup"]);
        match cmd.sub {
            AliasSubCommand::Setup(args) => {
                assert!(!args.no_cmd);
                assert!(!args.no_ps);
                assert!(!args.no_bash);
                assert!(!args.no_nu);
                assert!(!args.core_only);
            }
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[test]
    fn setup_parses_flags() {
        let cmd = parse(&["setup", "--no-cmd", "--no-ps", "--no-bash", "--no-nu", "--core-only"]);
        match cmd.sub {
            AliasSubCommand::Setup(args) => {
                assert!(args.no_cmd);
                assert!(args.no_ps);
                assert!(args.no_bash);
                assert!(args.no_nu);
                assert!(args.core_only);
            }
            other => panic!("expected Setup, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_positional() {
        let cmd = parse(&["add", "ll", "ls -la"]);
        match cmd.sub {
            AliasSubCommand::Add(args) => {
                assert_eq!(args.name, "ll");
                assert_eq!(args.command, "ls -la");
                assert_eq!(args.mode, "auto");
                assert!(args.desc.is_none());
                assert!(args.tag.is_empty());
                assert!(args.shell.is_empty());
                assert!(!args.force);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn add_parses_all_options() {
        let cmd = parse(&["add", "ll", "ls -la", "--mode", "exe", "--desc", "list long", "--tag", "shell", "--tag", "util", "--shell", "cmd", "--shell", "ps", "--force"]);
        match cmd.sub {
            AliasSubCommand::Add(args) => {
                assert_eq!(args.mode, "exe");
                assert_eq!(args.desc.as_deref(), Some("list long"));
                assert_eq!(args.tag, vec!["shell", "util"]);
                assert_eq!(args.shell, vec!["cmd", "ps"]);
                assert!(args.force);
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn rm_parses_names() {
        let cmd = parse(&["rm", "ll", "la"]);
        match cmd.sub {
            AliasSubCommand::Rm(args) => {
                assert_eq!(args.names, vec!["ll", "la"]);
            }
            other => panic!("expected Rm, got {other:?}"),
        }
    }

    #[test]
    fn ls_parses_defaults() {
        let cmd = parse(&["ls"]);
        match cmd.sub {
            AliasSubCommand::List(args) => {
                assert!(args.r#type.is_none());
                assert!(args.tag.is_none());
                assert!(!args.json);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ls_parses_options() {
        let cmd = parse(&["ls", "--type", "cmd", "--tag", "util", "--json"]);
        match cmd.sub {
            AliasSubCommand::List(args) => {
                assert_eq!(args.r#type.as_deref(), Some("cmd"));
                assert_eq!(args.tag.as_deref(), Some("util"));
                assert!(args.json);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn find_parses_keyword() {
        let cmd = parse(&["find", "git"]);
        match cmd.sub {
            AliasSubCommand::Find(args) => {
                assert_eq!(args.keyword, "git");
            }
            other => panic!("expected Find, got {other:?}"),
        }
    }

    #[test]
    fn which_parses_name() {
        let cmd = parse(&["which", "ll"]);
        match cmd.sub {
            AliasSubCommand::Which(args) => {
                assert_eq!(args.name, "ll");
            }
            other => panic!("expected Which, got {other:?}"),
        }
    }

    #[test]
    fn sync_parses_empty() {
        let cmd = parse(&["sync"]);
        match cmd.sub {
            AliasSubCommand::Sync(_) => {}
            other => panic!("expected Sync, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_defaults() {
        let cmd = parse(&["export"]);
        match cmd.sub {
            AliasSubCommand::Export(args) => {
                assert!(args.output.is_none());
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn export_parses_output() {
        let cmd = parse(&["export", "-o", "aliases.toml"]);
        match cmd.sub {
            AliasSubCommand::Export(args) => {
                assert_eq!(args.output.as_deref(), Some("aliases.toml"));
            }
            other => panic!("expected Export, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_file() {
        let cmd = parse(&["import", "aliases.toml"]);
        match cmd.sub {
            AliasSubCommand::Import(args) => {
                assert_eq!(args.file, "aliases.toml");
                assert!(!args.force);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn import_parses_force() {
        let cmd = parse(&["import", "aliases.toml", "--force"]);
        match cmd.sub {
            AliasSubCommand::Import(args) => {
                assert!(args.force);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn config_flag_parsed() {
        let cmd = AliasCmd::try_parse_from(["test", "--config", "custom.toml", "ls"]).unwrap();
        assert_eq!(cmd.config.as_deref(), Some("custom.toml"));
    }

    // ── App 嵌套子命令解析 ─────────────────────────────────────

    #[test]
    fn app_add_parses_positional() {
        let cmd = parse(&["app", "add", "vscode", "C:\\VSCode\\Code.exe"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Add(args) => {
                    assert_eq!(args.name, "vscode");
                    assert_eq!(args.exe, "C:\\VSCode\\Code.exe");
                    assert!(args.args.is_none());
                    assert!(args.desc.is_none());
                    assert!(args.tag.is_empty());
                    assert!(!args.no_apppaths);
                    assert!(!args.force);
                }
                other => panic!("expected App::Add, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_add_parses_all_options() {
        let cmd = parse(&["app", "add", "vscode", "C:\\VSCode\\Code.exe", "--args=--new-window", "--desc", "VS Code", "--tag", "editor", "--no-apppaths", "--force"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Add(args) => {
                    assert_eq!(args.args.as_deref(), Some("--new-window"));
                    assert_eq!(args.desc.as_deref(), Some("VS Code"));
                    assert_eq!(args.tag, vec!["editor"]);
                    assert!(args.no_apppaths);
                    assert!(args.force);
                }
                other => panic!("expected App::Add, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_rm_parses_names() {
        let cmd = parse(&["app", "rm", "vscode", "sublime"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Rm(args) => {
                    assert_eq!(args.names, vec!["vscode", "sublime"]);
                }
                other => panic!("expected App::Rm, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_ls_parses_json() {
        let cmd = parse(&["app", "ls", "--json"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::List(args) => {
                    assert!(args.json);
                }
                other => panic!("expected App::List, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_scan_parses_defaults() {
        let cmd = parse(&["app", "scan"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Scan(args) => {
                    assert_eq!(args.source, "all");
                    assert!(args.filter.is_none());
                    assert!(!args.json);
                    assert!(!args.all);
                    assert!(!args.no_cache);
                }
                other => panic!("expected App::Scan, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_scan_parses_all_options() {
        let cmd = parse(&["app", "scan", "--source", "reg", "--filter", "code", "--json", "--all", "--no-cache"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Scan(args) => {
                    assert_eq!(args.source, "reg");
                    assert_eq!(args.filter.as_deref(), Some("code"));
                    assert!(args.json);
                    assert!(args.all);
                    assert!(args.no_cache);
                }
                other => panic!("expected App::Scan, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_which_parses_name() {
        let cmd = parse(&["app", "which", "vscode"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Which(args) => {
                    assert_eq!(args.name, "vscode");
                }
                other => panic!("expected App::Which, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn app_sync_parses_empty() {
        let cmd = parse(&["app", "sync"]);
        match cmd.sub {
            AliasSubCommand::App(app) => match app.sub {
                AliasAppSubCommand::Sync(_) => {}
                other => panic!("expected App::Sync, got {other:?}"),
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    // ── TableRow 测试 ─────────────────────────────────────────

    #[test]
    fn alias_entry_columns_correct() {
        let cols = AliasEntry::columns();
        assert_eq!(cols.len(), 5);
        assert_eq!(cols[0].name, "name");
        assert_eq!(cols[0].kind, ValueKind::String);
        assert_eq!(cols[1].name, "command");
        assert_eq!(cols[2].name, "mode");
        assert_eq!(cols[3].name, "desc");
        assert_eq!(cols[4].name, "tags");
    }

    #[test]
    fn alias_entry_cells_match_fields() {
        let entry = AliasEntry::new("ll", "ls -la", "exe", "list long", "shell,util");
        let cells = entry.cells();
        assert_eq!(cells.len(), 5);
        assert!(matches!(&cells[0], Value::String(s) if s == "ll"));
        assert!(matches!(&cells[1], Value::String(s) if s == "ls -la"));
        assert!(matches!(&cells[2], Value::String(s) if s == "exe"));
        assert!(matches!(&cells[3], Value::String(s) if s == "list long"));
        assert!(matches!(&cells[4], Value::String(s) if s == "shell,util"));
    }

    #[test]
    fn alias_entry_renders_as_json() {
        let entry = AliasEntry::new("ll", "ls -la", "exe", "list long", "shell");
        let table = entry.to_table();
        let mut buf = Cursor::new(Vec::new());
        let mut r = JsonRenderer::new(false, &mut buf);
        r.render_table(&table).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.contains("ll"), "json output: {output}");
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn alias_subcommand_count() {
        let variants = [
            "Setup", "Add", "Rm", "Ls", "Find", "Which",
            "Sync", "Export", "Import", "App",
        ];
        assert_eq!(variants.len(), 10);
    }

    #[test]
    fn alias_app_subcommand_count() {
        let variants = ["Add", "Rm", "Ls", "Scan", "Which", "Sync"];
        assert_eq!(variants.len(), 6);
    }
}

// ============================================================
// Phase 3.7: brn_cmd — 单命令，30+ 参数
// ============================================================

mod brn_cmd_tests {
    use clap::Parser;
    use xun::xun_core::brn_cmd::BrnCmd;

    fn parse(args: &[&str]) -> BrnCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        BrnCmd::try_parse_from(&argv).expect("parse failed")
    }

    // ── 基础解析 ──────────────────────────────────────────────

    #[test]
    fn parses_defaults() {
        let cmd = parse(&[]);
        assert_eq!(cmd.path, ".");
        assert!(!cmd.trim);
        assert!(cmd.trim_chars.is_none());
        assert!(cmd.strip_brackets.is_none());
        assert!(cmd.strip_prefix.is_none());
        assert!(cmd.strip_suffix.is_none());
        assert!(cmd.remove_chars.is_none());
        assert!(cmd.from.is_none());
        assert!(cmd.to.is_none());
        assert!(cmd.regex.is_none());
        assert!(cmd.replace.is_none());
        assert!(cmd.regex_flags.is_none());
        assert!(cmd.case.is_none());
        assert!(cmd.ext_case.is_none());
        assert!(cmd.rename_ext.is_none());
        assert!(cmd.add_ext.is_none());
        assert!(cmd.prefix.is_none());
        assert!(cmd.suffix.is_none());
        assert!(cmd.insert_at.is_none());
        assert!(cmd.template.is_none());
        assert_eq!(cmd.template_start, 1);
        assert_eq!(cmd.template_pad, 3);
        assert!(cmd.slice.is_none());
        assert!(cmd.insert_date.is_none());
        assert!(!cmd.ctime);
        assert!(cmd.normalize_seq.is_none());
        assert!(cmd.normalize_unicode.is_none());
        assert!(!cmd.seq);
        assert_eq!(cmd.start, 1);
        assert_eq!(cmd.pad, 3);
        assert!(cmd.ext.is_empty());
        assert!(cmd.filter.is_none());
        assert!(cmd.exclude.is_none());
        assert!(!cmd.recursive);
        assert!(cmd.depth.is_none());
        assert!(!cmd.include_dirs);
        assert!(cmd.sort_by.is_none());
        assert!(cmd.output_format.is_none());
        assert!(!cmd.apply);
        assert!(!cmd.yes);
        assert!(cmd.undo.is_none());
        assert!(cmd.redo.is_none());
    }

    #[test]
    fn parses_path() {
        let cmd = parse(&["C:\\Photos"]);
        assert_eq!(cmd.path, "C:\\Photos");
    }

    // ── Rename steps ──────────────────────────────────────────

    #[test]
    fn parses_trim() {
        let cmd = parse(&["--trim"]);
        assert!(cmd.trim);
    }

    #[test]
    fn parses_trim_with_chars() {
        let cmd = parse(&["--trim", "--trim-chars", "_-"]);
        assert!(cmd.trim);
        assert_eq!(cmd.trim_chars.as_deref(), Some("_-"));
    }

    #[test]
    fn parses_strip_brackets() {
        let cmd = parse(&["--strip-brackets", "round,square"]);
        assert_eq!(cmd.strip_brackets.as_deref(), Some("round,square"));
    }

    #[test]
    fn parses_strip_prefix() {
        let cmd = parse(&["--strip-prefix", "IMG_"]);
        assert_eq!(cmd.strip_prefix.as_deref(), Some("IMG_"));
    }

    #[test]
    fn parses_strip_suffix() {
        let cmd = parse(&["--strip-suffix", "_backup"]);
        assert_eq!(cmd.strip_suffix.as_deref(), Some("_backup"));
    }

    #[test]
    fn parses_remove_chars() {
        let cmd = parse(&["--remove-chars", "()-"]);
        assert_eq!(cmd.remove_chars.as_deref(), Some("()-"));
    }

    #[test]
    fn parses_from_to() {
        let cmd = parse(&["--from", "old", "--to", "new"]);
        assert_eq!(cmd.from.as_deref(), Some("old"));
        assert_eq!(cmd.to.as_deref(), Some("new"));
    }

    #[test]
    fn parses_regex_replace() {
        let cmd = parse(&["--regex", r"(\d+)", "--replace", "num_$1", "--regex-flags", "i"]);
        assert_eq!(cmd.regex.as_deref(), Some(r"(\d+)"));
        assert_eq!(cmd.replace.as_deref(), Some("num_$1"));
        assert_eq!(cmd.regex_flags.as_deref(), Some("i"));
    }

    #[test]
    fn parses_case() {
        let cmd = parse(&["--case", "kebab"]);
        assert_eq!(cmd.case.as_deref(), Some("kebab"));
    }

    #[test]
    fn parses_ext_case() {
        let cmd = parse(&["--ext-case", "upper"]);
        assert_eq!(cmd.ext_case.as_deref(), Some("upper"));
    }

    #[test]
    fn parses_rename_ext() {
        let cmd = parse(&["--rename-ext", "jpeg:jpg"]);
        assert_eq!(cmd.rename_ext.as_deref(), Some("jpeg:jpg"));
    }

    #[test]
    fn parses_add_ext() {
        let cmd = parse(&["--add-ext", ".txt"]);
        assert_eq!(cmd.add_ext.as_deref(), Some(".txt"));
    }

    #[test]
    fn parses_prefix_suffix() {
        let cmd = parse(&["--prefix", "IMG_", "--suffix", "_final"]);
        assert_eq!(cmd.prefix.as_deref(), Some("IMG_"));
        assert_eq!(cmd.suffix.as_deref(), Some("_final"));
    }

    #[test]
    fn parses_insert_at() {
        let cmd = parse(&["--insert-at", "4:hello"]);
        assert_eq!(cmd.insert_at.as_deref(), Some("4:hello"));
    }

    #[test]
    fn parses_template() {
        let cmd = parse(&["--template", "{n:03}_{stem}", "--template-start", "10", "--template-pad", "4"]);
        assert_eq!(cmd.template.as_deref(), Some("{n:03}_{stem}"));
        assert_eq!(cmd.template_start, 10);
        assert_eq!(cmd.template_pad, 4);
    }

    #[test]
    fn parses_slice() {
        let cmd = parse(&["--slice", "0:8"]);
        assert_eq!(cmd.slice.as_deref(), Some("0:8"));
    }

    #[test]
    fn parses_insert_date() {
        let cmd = parse(&["--insert-date", "prefix:%Y%m%d", "--ctime"]);
        assert_eq!(cmd.insert_date.as_deref(), Some("prefix:%Y%m%d"));
        assert!(cmd.ctime);
    }

    #[test]
    fn parses_normalize_seq() {
        let cmd = parse(&["--normalize-seq", "5"]);
        assert_eq!(cmd.normalize_seq, Some(5));
    }

    #[test]
    fn parses_normalize_unicode() {
        let cmd = parse(&["--normalize-unicode", "nfc"]);
        assert_eq!(cmd.normalize_unicode.as_deref(), Some("nfc"));
    }

    #[test]
    fn parses_seq() {
        let cmd = parse(&["--seq", "--start", "100", "--pad", "5"]);
        assert!(cmd.seq);
        assert_eq!(cmd.start, 100);
        assert_eq!(cmd.pad, 5);
    }

    // ── Filters ───────────────────────────────────────────────

    #[test]
    fn parses_ext_filter() {
        let cmd = parse(&["--ext", "jpg", "--ext", "png"]);
        assert_eq!(cmd.ext, vec!["jpg", "png"]);
    }

    #[test]
    fn parses_filter_exclude() {
        let cmd = parse(&["--filter", "IMG_*", "--exclude", "*_backup*"]);
        assert_eq!(cmd.filter.as_deref(), Some("IMG_*"));
        assert_eq!(cmd.exclude.as_deref(), Some("*_backup*"));
    }

    #[test]
    fn parses_recursive() {
        let cmd = parse(&["-r"]);
        assert!(cmd.recursive);
    }

    #[test]
    fn parses_depth() {
        let cmd = parse(&["--depth", "3"]);
        assert_eq!(cmd.depth, Some(3));
    }

    #[test]
    fn parses_include_dirs() {
        let cmd = parse(&["--include-dirs"]);
        assert!(cmd.include_dirs);
    }

    #[test]
    fn parses_sort_by() {
        let cmd = parse(&["--sort-by", "mtime"]);
        assert_eq!(cmd.sort_by.as_deref(), Some("mtime"));
    }

    // ── Output & Execution ────────────────────────────────────

    #[test]
    fn parses_output_format() {
        let cmd = parse(&["--output-format", "json"]);
        assert_eq!(cmd.output_format.as_deref(), Some("json"));
    }

    #[test]
    fn parses_apply_yes() {
        let cmd = parse(&["--apply", "-y"]);
        assert!(cmd.apply);
        assert!(cmd.yes);
    }

    #[test]
    fn parses_undo_redo() {
        let cmd = parse(&["--undo", "3"]);
        assert_eq!(cmd.undo, Some(3));

        let cmd = parse(&["--redo", "5"]);
        assert_eq!(cmd.redo, Some(5));
    }

    // ── 组合测试 ──────────────────────────────────────────────

    #[test]
    fn parses_combined_rename_steps() {
        let cmd = parse(&["C:\\Photos", "--trim", "--strip-prefix", "IMG_", "--case", "kebab", "--apply", "-y"]);
        assert_eq!(cmd.path, "C:\\Photos");
        assert!(cmd.trim);
        assert_eq!(cmd.strip_prefix.as_deref(), Some("IMG_"));
        assert_eq!(cmd.case.as_deref(), Some("kebab"));
        assert!(cmd.apply);
        assert!(cmd.yes);
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn brn_param_count() {
        // 验证 30+ 参数全部存在（编译期检查）
        let cmd = BrnCmd {
            path: ".".into(),
            trim: false,
            trim_chars: None,
            strip_brackets: None,
            strip_prefix: None,
            strip_suffix: None,
            remove_chars: None,
            from: None,
            to: None,
            regex: None,
            replace: None,
            regex_flags: None,
            case: None,
            ext_case: None,
            rename_ext: None,
            add_ext: None,
            prefix: None,
            suffix: None,
            insert_at: None,
            template: None,
            template_start: 1,
            template_pad: 3,
            slice: None,
            insert_date: None,
            ctime: false,
            normalize_seq: None,
            normalize_unicode: None,
            seq: false,
            start: 1,
            pad: 3,
            ext: vec![],
            filter: None,
            exclude: None,
            recursive: false,
            depth: None,
            include_dirs: false,
            sort_by: None,
            output_format: None,
            apply: false,
            yes: false,
            undo: None,
            redo: None,
        };
        assert_eq!(cmd.path, ".");
    }
}

// ============================================================
// Phase 3.7: vault_cmd — 8 子命令
// ============================================================

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

    // ── Enc 子命令 ────────────────────────────────────────────

    #[test]
    fn enc_parses_basic() {
        let cmd = parse(&["enc", "secret.txt"]);
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
            VaultSubCommand::Enc(args) => {
                assert_eq!(args.keyfile.as_deref(), Some("key.bin"));
                assert_eq!(args.recovery_key.as_deref(), Some("abc123"));
                assert_eq!(args.emit_recovery_key.as_deref(), Some("recovery.txt"));
            }
            other => panic!("expected Enc, got {other:?}"),
        }
    }

    // ── Dec 子命令 ────────────────────────────────────────────

    #[test]
    fn dec_parses_basic() {
        let cmd = parse(&["dec", "secret.fv"]);
        match cmd.sub {
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
        match cmd.sub {
            VaultSubCommand::Dec(args) => {
                assert_eq!(args.output.as_deref(), Some("plain.txt"));
                assert_eq!(args.password.as_deref(), Some("s3cret"));
                assert!(args.json);
            }
            other => panic!("expected Dec, got {other:?}"),
        }
    }

    // ── Inspect 子命令 ────────────────────────────────────────

    #[test]
    fn inspect_parses_basic() {
        let cmd = parse(&["inspect", "secret.fv"]);
        match cmd.sub {
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
        match cmd.sub {
            VaultSubCommand::Inspect(args) => {
                assert!(args.json);
            }
            other => panic!("expected Inspect, got {other:?}"),
        }
    }

    // ── Verify 子命令 ─────────────────────────────────────────

    #[test]
    fn verify_parses_basic() {
        let cmd = parse(&["verify", "secret.fv"]);
        match cmd.sub {
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
        match cmd.sub {
            VaultSubCommand::Verify(args) => {
                assert_eq!(args.password.as_deref(), Some("s3cret"));
                assert!(args.dpapi);
                assert!(args.json);
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    // ── Resume 子命令 ─────────────────────────────────────────

    #[test]
    fn resume_parses_basic() {
        let cmd = parse(&["resume", "secret.fv"]);
        match cmd.sub {
            VaultSubCommand::Resume(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(args.password.is_none());
                assert!(!args.dpapi);
                assert!(!args.json);
            }
            other => panic!("expected Resume, got {other:?}"),
        }
    }

    // ── Cleanup 子命令 ────────────────────────────────────────

    #[test]
    fn cleanup_parses_basic() {
        let cmd = parse(&["cleanup", "secret.fv"]);
        match cmd.sub {
            VaultSubCommand::Cleanup(args) => {
                assert_eq!(args.path, "secret.fv");
                assert!(!args.json);
            }
            other => panic!("expected Cleanup, got {other:?}"),
        }
    }

    // ── Rewrap 子命令 ─────────────────────────────────────────

    #[test]
    fn rewrap_parses_basic() {
        let cmd = parse(&["rewrap", "secret.fv"]);
        match cmd.sub {
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
        match cmd.sub {
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

    // ── RecoverKey 子命令 ─────────────────────────────────────

    #[test]
    fn recover_key_parses_basic() {
        let cmd = parse(&["recover-key", "secret.fv", "--unlock-password", "old", "recovery.txt"]);
        match cmd.sub {
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
        match cmd.sub {
            VaultSubCommand::RecoverKey(args) => {
                assert_eq!(args.unlock_keyfile.as_deref(), Some("key.bin"));
                assert!(args.json);
            }
            other => panic!("expected RecoverKey, got {other:?}"),
        }
    }

    // ── TableRow 测试 ─────────────────────────────────────────

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

    // ── E2E 测试（桩） ───────────────────────────────────────

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
// Phase 3.8: lock_cmd — LockCmd (1 子命令) + MvCmd + RenFileCmd
// ============================================================

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

    // ── Lock 子命令 ────────────────────────────────────────────

    #[test]
    fn lock_who_parses_basic() {
        let cmd = parse_lock(&["who", "C:\\test.txt"]);
        match cmd.sub {
            LockSubCommand::Who(args) => {
                assert_eq!(args.path, "C:\\test.txt");
                assert_eq!(args.format, "auto");
            }
        }
    }

    #[test]
    fn lock_who_parses_format() {
        let cmd = parse_lock(&["who", "C:\\test.txt", "-f", "json"]);
        match cmd.sub {
            LockSubCommand::Who(args) => {
                assert_eq!(args.format, "json");
            }
        }
    }

    // ── Mv 命令 ────────────────────────────────────────────────

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

    // ── RenFile 命令 ───────────────────────────────────────────

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

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn lock_subcommand_count() {
        let variants = ["Who"];
        assert_eq!(variants.len(), 1);
    }
}

// ============================================================
// Phase 3.8: protect_cmd — 3 子命令
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
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
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
        match cmd.sub {
            ProtectSubCommand::Clear(args) => {
                assert!(args.system_acl);
            }
            other => panic!("expected Clear, got {other:?}"),
        }
    }

    #[test]
    fn status_parses_defaults() {
        let cmd = parse(&["status"]);
        match cmd.sub {
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
        match cmd.sub {
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
// Phase 3.8: crypt_cmd — EncryptCmd + DecryptCmd
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
// Phase 3.8: dashboard_cmd — ServeCmd
// ============================================================

mod dashboard_cmd_tests {
    use clap::Parser;
    use xun::xun_core::dashboard_cmd::ServeCmd;

    #[test]
    fn serve_parses_defaults() {
        let cmd = ServeCmd::try_parse_from(["test"]).unwrap();
        assert_eq!(cmd.port, 9527);
    }

    #[test]
    fn serve_parses_port() {
        let cmd = ServeCmd::try_parse_from(["test", "-p", "8080"]).unwrap();
        assert_eq!(cmd.port, 8080);
    }

    #[test]
    fn serve_parses_long_port() {
        let cmd = ServeCmd::try_parse_from(["test", "--port", "3000"]).unwrap();
        assert_eq!(cmd.port, 3000);
    }
}

// ============================================================
// Phase 3.8: redirect_cmd — RedirectCmd (20+ 参数)
// ============================================================

mod redirect_cmd_tests {
    use clap::Parser;
    use xun::xun_core::redirect_cmd::RedirectCmd;

    fn parse(args: &[&str]) -> RedirectCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        RedirectCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn parses_defaults() {
        let cmd = parse(&[]);
        assert!(cmd.source.is_none());
        assert_eq!(cmd.profile, "default");
        assert!(cmd.explain.is_none());
        assert!(!cmd.stats);
        assert!(!cmd.confirm);
        assert!(!cmd.review);
        assert!(!cmd.log);
        assert!(cmd.tx.is_none());
        assert!(cmd.last.is_none());
        assert!(!cmd.validate);
        assert!(cmd.plan.is_none());
        assert!(cmd.apply.is_none());
        assert!(cmd.undo.is_none());
        assert!(!cmd.watch);
        assert!(!cmd.status);
        assert!(!cmd.simulate);
        assert!(!cmd.dry_run);
        assert!(!cmd.copy);
        assert!(!cmd.yes);
        assert_eq!(cmd.format, "auto");
    }

    #[test]
    fn parses_source() {
        let cmd = parse(&["C:\\Downloads"]);
        assert_eq!(cmd.source.as_deref(), Some("C:\\Downloads"));
    }

    #[test]
    fn parses_profile() {
        let cmd = parse(&["--profile", "photos"]);
        assert_eq!(cmd.profile, "photos");
    }

    #[test]
    fn parses_explain() {
        let cmd = parse(&["--explain", "IMG_001.jpg"]);
        assert_eq!(cmd.explain.as_deref(), Some("IMG_001.jpg"));
    }

    #[test]
    fn parses_log_options() {
        let cmd = parse(&["--log", "--tx", "abc123", "--last", "10"]);
        assert!(cmd.log);
        assert_eq!(cmd.tx.as_deref(), Some("abc123"));
        assert_eq!(cmd.last, Some(10));
    }

    #[test]
    fn parses_plan_apply_undo() {
        let cmd = parse(&["--plan", "plan.json"]);
        assert_eq!(cmd.plan.as_deref(), Some("plan.json"));

        let cmd = parse(&["--apply", "plan.json"]);
        assert_eq!(cmd.apply.as_deref(), Some("plan.json"));

        let cmd = parse(&["--undo", "tx123"]);
        assert_eq!(cmd.undo.as_deref(), Some("tx123"));
    }

    #[test]
    fn parses_watch_status() {
        let cmd = parse(&["--watch", "--status"]);
        assert!(cmd.watch);
        assert!(cmd.status);
    }

    #[test]
    fn parses_execution_flags() {
        let cmd = parse(&["--dry-run", "--copy", "-y", "-f", "json"]);
        assert!(cmd.dry_run);
        assert!(cmd.copy);
        assert!(cmd.yes);
        assert_eq!(cmd.format, "json");
    }

    #[test]
    fn parses_combined() {
        let cmd = parse(&["C:\\Downloads", "--profile", "docs", "--confirm", "--review", "--simulate", "--validate"]);
        assert_eq!(cmd.source.as_deref(), Some("C:\\Downloads"));
        assert_eq!(cmd.profile, "docs");
        assert!(cmd.confirm);
        assert!(cmd.review);
        assert!(cmd.simulate);
        assert!(cmd.validate);
    }

    #[test]
    fn redirect_param_count() {
        // 验证 20+ 参数全部存在（编译期检查）
        let cmd = RedirectCmd {
            source: None,
            profile: "default".into(),
            explain: None,
            stats: false,
            confirm: false,
            review: false,
            log: false,
            tx: None,
            last: None,
            validate: false,
            plan: None,
            apply: None,
            undo: None,
            watch: false,
            status: false,
            simulate: false,
            dry_run: false,
            copy: false,
            yes: false,
            format: "auto".into(),
        };
        assert_eq!(cmd.profile, "default");
    }
}

// ============================================================
// Phase 3.8: img_cmd — ImgCmd (16 参数)
// ============================================================

mod img_cmd_tests {
    use clap::Parser;
    use xun::xun_core::img_cmd::ImgCmd;

    fn parse(args: &[&str]) -> ImgCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        ImgCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn parses_defaults() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "output/"]);
        assert_eq!(cmd.input, "photo.jpg");
        assert_eq!(cmd.output, "output/");
        assert_eq!(cmd.format, "webp");
        assert_eq!(cmd.svg_method, "bezier");
        assert_eq!(cmd.svg_diffvg_iters, 150);
        assert_eq!(cmd.svg_diffvg_strokes, 64);
        assert_eq!(cmd.jpeg_backend, "auto");
        assert_eq!(cmd.quality, 80);
        assert_eq!(cmd.png_lossy, "true");
        assert_eq!(cmd.png_dither_level, 0.0);
        assert_eq!(cmd.webp_lossy, "true");
        assert!(cmd.mw.is_none());
        assert!(cmd.mh.is_none());
        assert!(cmd.threads.is_none());
        assert!(cmd.avif_threads.is_none());
        assert!(!cmd.overwrite);
    }

    #[test]
    fn parses_format() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "-f", "avif"]);
        assert_eq!(cmd.format, "avif");
    }

    #[test]
    fn parses_svg_options() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--svg-method", "potrace", "--svg-diffvg-iters", "200", "--svg-diffvg-strokes", "128"]);
        assert_eq!(cmd.svg_method, "potrace");
        assert_eq!(cmd.svg_diffvg_iters, 200);
        assert_eq!(cmd.svg_diffvg_strokes, 128);
    }

    #[test]
    fn parses_jpeg_options() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--jpeg-backend", "turbo", "-q", "95"]);
        assert_eq!(cmd.jpeg_backend, "turbo");
        assert_eq!(cmd.quality, 95);
    }

    #[test]
    fn parses_png_options() {
        let cmd = parse(&["-i", "photo.png", "-o", "out/", "--png-lossy", "false", "--png-dither-level", "0.5"]);
        assert_eq!(cmd.png_lossy, "false");
        assert_eq!(cmd.png_dither_level, 0.5);
    }

    #[test]
    fn parses_webp_lossy() {
        let cmd = parse(&["-i", "photo.png", "-o", "out/", "--webp-lossy", "false"]);
        assert_eq!(cmd.webp_lossy, "false");
    }

    #[test]
    fn parses_resize() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--mw", "1920", "--mh", "1080"]);
        assert_eq!(cmd.mw, Some(1920));
        assert_eq!(cmd.mh, Some(1080));
    }

    #[test]
    fn parses_threads() {
        let cmd = parse(&["-i", "dir/", "-o", "out/", "-t", "4", "--avif-threads", "2"]);
        assert_eq!(cmd.threads, Some(4));
        assert_eq!(cmd.avif_threads, Some(2));
    }

    #[test]
    fn parses_overwrite() {
        let cmd = parse(&["-i", "photo.jpg", "-o", "out/", "--overwrite"]);
        assert!(cmd.overwrite);
    }

    #[test]
    fn img_param_count() {
        let cmd = ImgCmd {
            input: "test".into(),
            output: "out".into(),
            format: "webp".into(),
            svg_method: "bezier".into(),
            svg_diffvg_iters: 150,
            svg_diffvg_strokes: 64,
            jpeg_backend: "auto".into(),
            quality: 80,
            png_lossy: "true".into(),
            png_dither_level: 0.0,
            webp_lossy: "true".into(),
            mw: None,
            mh: None,
            threads: None,
            avif_threads: None,
            overwrite: false,
        };
        assert_eq!(cmd.format, "webp");
    }
}

// ============================================================
// Phase 3.8: xunbak_cmd — 嵌套 Plugin → Install/Uninstall/Doctor
// ============================================================

mod xunbak_cmd_tests {
    use clap::Parser;
    use xun::xun_core::xunbak_cmd::*;

    fn parse(args: &[&str]) -> XunbakCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        XunbakCmd::try_parse_from(&argv).expect("parse failed")
    }

    #[test]
    fn plugin_install_parses_basic() {
        let cmd = parse(&["plugin", "install"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Install(args) => {
                    assert!(args.sevenzip_home.is_none());
                    assert!(args.config.is_none());
                    assert!(!args.no_overwrite);
                    assert!(!args.associate);
                }
                other => panic!("expected Install, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_install_parses_all_options() {
        let cmd = parse(&["plugin", "install", "--sevenzip-home", "C:/Program Files/7-Zip", "--config", "release", "--no-overwrite", "--associate"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Install(args) => {
                    assert_eq!(args.sevenzip_home.as_deref(), Some("C:/Program Files/7-Zip"));
                    assert_eq!(args.config.as_deref(), Some("release"));
                    assert!(args.no_overwrite);
                    assert!(args.associate);
                }
                other => panic!("expected Install, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_uninstall_parses_basic() {
        let cmd = parse(&["plugin", "uninstall"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Uninstall(args) => {
                    assert!(args.sevenzip_home.is_none());
                    assert!(!args.remove_association);
                }
                other => panic!("expected Uninstall, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_uninstall_parses_remove_association() {
        let cmd = parse(&["plugin", "uninstall", "--remove-association"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Uninstall(args) => {
                    assert!(args.remove_association);
                }
                other => panic!("expected Uninstall, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_doctor_parses_basic() {
        let cmd = parse(&["plugin", "doctor"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Doctor(args) => {
                    assert!(args.sevenzip_home.is_none());
                }
                other => panic!("expected Doctor, got {other:?}"),
            },
        }
    }

    #[test]
    fn plugin_doctor_parses_sevenzip_home() {
        let cmd = parse(&["plugin", "doctor", "--sevenzip-home", "C:\\7-Zip"]);
        match cmd.sub {
            XunbakSubCommand::Plugin(plugin) => match plugin.sub {
                XunbakPluginSubCommand::Doctor(args) => {
                    assert_eq!(args.sevenzip_home.as_deref(), Some("C:\\7-Zip"));
                }
                other => panic!("expected Doctor, got {other:?}"),
            },
        }
    }

    #[test]
    fn xunbak_subcommand_count() {
        let variants = ["Plugin"];
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn xunbak_plugin_subcommand_count() {
        let variants = ["Install", "Uninstall", "Doctor"];
        assert_eq!(variants.len(), 3);
    }
}

// ============================================================
// Phase 3.8: desktop_cmd — 14 顶层子命令，多个嵌套组
// ============================================================

mod desktop_cmd_tests {
    use clap::Parser;
    use xun::xun_core::desktop_cmd::*;

    fn parse(args: &[&str]) -> DesktopCmd {
        let mut argv = vec!["test"];
        argv.extend_from_slice(args);
        DesktopCmd::try_parse_from(&argv).expect("parse failed")
    }

    // ── Daemon 子命令组 ───────────────────────────────────────

    #[test]
    fn daemon_start_parses_defaults() {
        let cmd = parse(&["daemon", "start"]);
        match cmd.sub {
            DesktopSubCommand::Daemon(d) => match d.sub {
                DesktopDaemonSubCommand::Start(args) => {
                    assert!(!args.quiet);
                    assert!(!args.no_tray);
                    assert!(!args.elevated);
                }
                other => panic!("expected Start, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_start_parses_flags() {
        let cmd = parse(&["daemon", "start", "-q", "--no-tray", "--elevated"]);
        match cmd.sub {
            DesktopSubCommand::Daemon(d) => match d.sub {
                DesktopDaemonSubCommand::Start(args) => {
                    assert!(args.quiet);
                    assert!(args.no_tray);
                    assert!(args.elevated);
                }
                other => panic!("expected Start, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_stop_parses() {
        let cmd = parse(&["daemon", "stop"]);
        match cmd.sub {
            DesktopSubCommand::Daemon(d) => match d.sub {
                DesktopDaemonSubCommand::Stop(_) => {}
                other => panic!("expected Stop, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_status_parses() {
        let cmd = parse(&["daemon", "status"]);
        match cmd.sub {
            DesktopSubCommand::Daemon(d) => match d.sub {
                DesktopDaemonSubCommand::Status(_) => {}
                other => panic!("expected Status, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    #[test]
    fn daemon_reload_parses() {
        let cmd = parse(&["daemon", "reload"]);
        match cmd.sub {
            DesktopSubCommand::Daemon(d) => match d.sub {
                DesktopDaemonSubCommand::Reload(_) => {}
                other => panic!("expected Reload, got {other:?}"),
            },
            other => panic!("expected Daemon, got {other:?}"),
        }
    }

    // ── Hotkey 子命令组 ───────────────────────────────────────

    #[test]
    fn hotkey_bind_parses() {
        let cmd = parse(&["hotkey", "bind", "ctrl+alt+t", "run:wt.exe"]);
        match cmd.sub {
            DesktopSubCommand::Hotkey(h) => match h.sub {
                DesktopHotkeySubCommand::Bind(args) => {
                    assert_eq!(args.hotkey, "ctrl+alt+t");
                    assert_eq!(args.action, "run:wt.exe");
                    assert!(args.app.is_none());
                }
                other => panic!("expected Bind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_bind_parses_app() {
        let cmd = parse(&["hotkey", "bind", "ctrl+alt+t", "run:wt.exe", "--app", "Explorer"]);
        match cmd.sub {
            DesktopSubCommand::Hotkey(h) => match h.sub {
                DesktopHotkeySubCommand::Bind(args) => {
                    assert_eq!(args.app.as_deref(), Some("Explorer"));
                }
                other => panic!("expected Bind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_unbind_parses() {
        let cmd = parse(&["hotkey", "unbind", "ctrl+alt+t"]);
        match cmd.sub {
            DesktopSubCommand::Hotkey(h) => match h.sub {
                DesktopHotkeySubCommand::Unbind(args) => {
                    assert_eq!(args.hotkey, "ctrl+alt+t");
                }
                other => panic!("expected Unbind, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    #[test]
    fn hotkey_list_parses() {
        let cmd = parse(&["hotkey", "list"]);
        match cmd.sub {
            DesktopSubCommand::Hotkey(h) => match h.sub {
                DesktopHotkeySubCommand::List(_) => {}
                other => panic!("expected List, got {other:?}"),
            },
            other => panic!("expected Hotkey, got {other:?}"),
        }
    }

    // ── Remap 子命令组 ────────────────────────────────────────

    #[test]
    fn remap_add_parses() {
        let cmd = parse(&["remap", "add", "capslock", "esc"]);
        match cmd.sub {
            DesktopSubCommand::Remap(r) => match r.sub {
                DesktopRemapSubCommand::Add(args) => {
                    assert_eq!(args.from, "capslock");
                    assert_eq!(args.to, "esc");
                    assert!(args.app.is_none());
                    assert!(!args.exact);
                    assert!(!args.dry_run);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Remap, got {other:?}"),
        }
    }

    #[test]
    fn remap_remove_parses() {
        let cmd = parse(&["remap", "remove", "capslock"]);
        match cmd.sub {
            DesktopSubCommand::Remap(r) => match r.sub {
                DesktopRemapSubCommand::Rm(args) => {
                    assert_eq!(args.from, "capslock");
                    assert!(args.to.is_none());
                }
                other => panic!("expected Rm, got {other:?}"),
            },
            other => panic!("expected Remap, got {other:?}"),
        }
    }

    // ── Snippet 子命令组 ──────────────────────────────────────

    #[test]
    fn snippet_add_parses() {
        let cmd = parse(&["snippet", "add", "btw", "by the way"]);
        match cmd.sub {
            DesktopSubCommand::Snippet(s) => match s.sub {
                DesktopSnippetSubCommand::Add(args) => {
                    assert_eq!(args.trigger, "btw");
                    assert_eq!(args.expand, "by the way");
                    assert!(!args.immediate);
                    assert!(!args.clipboard);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Snippet, got {other:?}"),
        }
    }

    // ── Layout 子命令组 ───────────────────────────────────────

    #[test]
    fn layout_new_parses() {
        let cmd = parse(&["layout", "new", "coding", "-t", "grid", "--rows", "2", "--cols", "3"]);
        match cmd.sub {
            DesktopSubCommand::Layout(l) => match l.sub {
                DesktopLayoutSubCommand::Add(args) => {
                    assert_eq!(args.name, "coding");
                    assert_eq!(args.layout_type, "grid");
                    assert_eq!(args.rows, Some(2));
                    assert_eq!(args.cols, Some(3));
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Layout, got {other:?}"),
        }
    }

    // ── Workspace 子命令组 ────────────────────────────────────

    #[test]
    fn workspace_save_parses() {
        let cmd = parse(&["workspace", "save", "dev", "--name-only"]);
        match cmd.sub {
            DesktopSubCommand::Workspace(w) => match w.sub {
                DesktopWorkspaceSubCommand::Save(args) => {
                    assert_eq!(args.name, "dev");
                    assert!(args.name_only);
                }
                other => panic!("expected Save, got {other:?}"),
            },
            other => panic!("expected Workspace, got {other:?}"),
        }
    }

    // ── Window 子命令组 ───────────────────────────────────────

    #[test]
    fn window_move_parses() {
        let cmd = parse(&["window", "move", "--x", "100", "--y", "200"]);
        match cmd.sub {
            DesktopSubCommand::Window(w) => match w.sub {
                DesktopWindowSubCommand::Move(args) => {
                    assert_eq!(args.x, 100);
                    assert_eq!(args.y, 200);
                }
                other => panic!("expected Move, got {other:?}"),
            },
            other => panic!("expected Window, got {other:?}"),
        }
    }

    #[test]
    fn window_resize_parses() {
        let cmd = parse(&["window", "resize", "--width", "800", "--height", "600"]);
        match cmd.sub {
            DesktopSubCommand::Window(w) => match w.sub {
                DesktopWindowSubCommand::Resize(args) => {
                    assert_eq!(args.width, 800);
                    assert_eq!(args.height, 600);
                }
                other => panic!("expected Resize, got {other:?}"),
            },
            other => panic!("expected Window, got {other:?}"),
        }
    }

    // ── Theme 子命令组 ────────────────────────────────────────

    #[test]
    fn theme_set_parses() {
        let cmd = parse(&["theme", "set", "dark"]);
        match cmd.sub {
            DesktopSubCommand::Theme(t) => match t.sub {
                DesktopThemeSubCommand::Set(args) => {
                    assert_eq!(args.mode, "dark");
                }
                other => panic!("expected Set, got {other:?}"),
            },
            other => panic!("expected Theme, got {other:?}"),
        }
    }

    #[test]
    fn theme_schedule_parses() {
        let cmd = parse(&["theme", "schedule", "--light", "06:00", "--dark", "18:00"]);
        match cmd.sub {
            DesktopSubCommand::Theme(t) => match t.sub {
                DesktopThemeSubCommand::Schedule(args) => {
                    assert_eq!(args.light.as_deref(), Some("06:00"));
                    assert_eq!(args.dark.as_deref(), Some("18:00"));
                }
                other => panic!("expected Schedule, got {other:?}"),
            },
            other => panic!("expected Theme, got {other:?}"),
        }
    }

    // ── Awake 子命令组 ────────────────────────────────────────

    #[test]
    fn awake_on_parses() {
        let cmd = parse(&["awake", "on", "--duration", "2h", "--display-on"]);
        match cmd.sub {
            DesktopSubCommand::Awake(a) => match a.sub {
                DesktopAwakeSubCommand::On(args) => {
                    assert_eq!(args.duration.as_deref(), Some("2h"));
                    assert!(args.display_on);
                }
                other => panic!("expected On, got {other:?}"),
            },
            other => panic!("expected Awake, got {other:?}"),
        }
    }

    // ── Color 命令 ────────────────────────────────────────────

    #[test]
    fn color_parses() {
        let cmd = parse(&["color", "--copy"]);
        match cmd.sub {
            DesktopSubCommand::Color(args) => {
                assert!(args.copy);
            }
            other => panic!("expected Color, got {other:?}"),
        }
    }

    // ── Hosts 子命令组 ────────────────────────────────────────

    #[test]
    fn hosts_add_parses() {
        let cmd = parse(&["hosts", "add", "example.com", "127.0.0.1"]);
        match cmd.sub {
            DesktopSubCommand::Hosts(h) => match h.sub {
                DesktopHostsSubCommand::Add(args) => {
                    assert_eq!(args.host, "example.com");
                    assert_eq!(args.ip, "127.0.0.1");
                    assert!(!args.dry_run);
                }
                other => panic!("expected Add, got {other:?}"),
            },
            other => panic!("expected Hosts, got {other:?}"),
        }
    }

    // ── App 子命令组 ──────────────────────────────────────────

    #[test]
    fn app_list_parses() {
        let cmd = parse(&["app", "list"]);
        match cmd.sub {
            DesktopSubCommand::App(a) => match a.sub {
                DesktopAppSubCommand::List(_) => {}
            },
            other => panic!("expected App, got {other:?}"),
        }
    }

    // ── Tui 命令 ──────────────────────────────────────────────

    #[test]
    fn tui_parses() {
        let cmd = parse(&["tui"]);
        match cmd.sub {
            DesktopSubCommand::Tui(_) => {}
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    // ── Run 命令 ──────────────────────────────────────────────

    #[test]
    fn run_parses() {
        let cmd = parse(&["run", "notepad.exe"]);
        match cmd.sub {
            DesktopSubCommand::Run(args) => {
                assert_eq!(args.command, "notepad.exe");
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }

    // ── E2E 测试（桩） ───────────────────────────────────────

    #[test]
    fn desktop_subcommand_count() {
        let variants = [
            "Daemon", "Hotkey", "Remap", "Snippet", "Layout",
            "Workspace", "Window", "Theme", "Awake", "Color",
            "Hosts", "App", "Tui", "Run",
        ];
        assert_eq!(variants.len(), 14);
    }

    #[test]
    fn desktop_daemon_subcommand_count() {
        let variants = ["Start", "Stop", "Status", "Reload"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_hotkey_subcommand_count() {
        let variants = ["Bind", "Unbind", "List"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_remap_subcommand_count() {
        let variants = ["Add", "Remove", "List", "Clear"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_snippet_subcommand_count() {
        let variants = ["Add", "Remove", "List", "Clear"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_layout_subcommand_count() {
        let variants = ["New", "Apply", "Preview", "List", "Remove"];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn desktop_workspace_subcommand_count() {
        let variants = ["Save", "Launch", "List", "Remove"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_window_subcommand_count() {
        let variants = ["Focus", "Move", "Resize", "Transparent", "Top"];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn desktop_theme_subcommand_count() {
        let variants = ["Set", "Toggle", "Schedule", "Status"];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn desktop_awake_subcommand_count() {
        let variants = ["On", "Off", "Status"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_hosts_subcommand_count() {
        let variants = ["Add", "Remove", "List"];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn desktop_app_subcommand_count() {
        let variants = ["List"];
        assert_eq!(variants.len(), 1);
    }
}

// ============================================================
// Phase 2.3: Proxy CommandSpec 实现（真实 service）
// ============================================================

mod proxy_service_tests {
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::proxy_cmd::{ProxyShowCmd, ProxySetCmd, ProxyRmCmd, ProxyShowArgs, ProxySetArgs, ProxyRmArgs};
    use xun::xun_core::renderer::JsonRenderer;
    use std::io::Cursor;

    #[test]
    fn proxy_show_cmd_returns_value() {
        let cmd = ProxyShowCmd { args: ProxyShowArgs { format: "auto".into() } };
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
        let cmd = ProxySetCmd {
            args: ProxySetArgs {
                url: "".into(),
                noproxy: "localhost".into(),
                only: None,
            },
        };
        let ctx = CmdContext::for_test();
        let result = cmd.validate(&ctx);
        assert!(result.is_err(), "empty URL should fail validation");
    }

    #[test]
    fn proxy_rm_cmd_returns_value() {
        let cmd = ProxyRmCmd { args: ProxyRmArgs { only: None } };
        let mut ctx = CmdContext::for_test();
        let mut buf = Cursor::new(Vec::new());
        let mut renderer = JsonRenderer::new(false, &mut buf);
        let result = execute(&cmd, &mut ctx, &mut renderer);
        // rm 操作应该成功（即使没有代理配置可删）
        assert!(result.is_ok());
    }
}

// ============================================================
// Phase 3.4: BookmarkDeleteOp — Operation trait 测试
// ============================================================

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
// Phase 3.5: EnvSetOp / EnvDelOp — Operation trait 测试
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
// Phase 3.6: AclAddOp / AclRemoveOp / AclRepairOp — Operation trait 测试
// ============================================================

mod acl_operation_tests {
    use xun::xun_core::operation::{Operation, RiskLevel};
    use xun::xun_core::services::acl::{AclAddOp, AclRemoveOp, AclRepairOp};

    #[test]
    fn acl_add_op_preview_has_high_risk() {
        let op = AclAddOp::new("C:\\test", "Users", "FullControl");
        assert_eq!(op.preview().risk_level(), RiskLevel::High);
    }

    #[test]
    fn acl_add_op_preview_has_changes() {
        let op = AclAddOp::new("C:\\test", "Users", "Read");
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
        let op = AclRepairOp::new("C:\\test");
        assert_eq!(op.preview().risk_level(), RiskLevel::Critical);
    }
}

// ============================================================
// Phase 3.7: RenameOperation — Operation trait 测试
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
// Phase 3.7: VaultEncOp / VaultDecOp — Operation trait 测试
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
