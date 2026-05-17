mod tree_cmd_tests {
    use clap::Parser;
    use xun::xun_core::tree_cmd::TreeCmd;
    use xun::xun_core::command::{CommandSpec, execute};
    use xun::xun_core::context::CmdContext;
    use xun::xun_core::error::XunError;
    use xun::xun_core::renderer::{JsonRenderer, Renderer};
    use xun::xun_core::value::Value;
    use std::io::Cursor;

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

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

    // ---- CommandSpec 娴嬭瘯 ----

    struct TreeExecutor {
        path: Option<String>,
        depth: Option<usize>,
    }

    impl CommandSpec for TreeExecutor {
        fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
            let path = self.path.as_deref().unwrap_or(".");
            Ok(Value::String(format!("{path}\n鈹溾攢鈹€ a\n鈹斺攢鈹€ b")))
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

    // ---- E2E 娴嬭瘯 ----

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
// Phase 3.1: Find 鍛戒护锛坈lap derive + CommandSpec锛?
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

    // ---- CLI 瑙ｆ瀽娴嬭瘯 ----

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

    // ---- Output 绫诲瀷娴嬭瘯 ----

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

    // ---- CommandSpec 娴嬭瘯 ----

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

    // ---- E2E 娴嬭瘯 ----

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
// Phase 3.1: Ctx 鍛戒护锛坈lap derive + CommandSpec + TableRow锛?
// ============================================================

