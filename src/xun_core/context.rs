//! CmdContext — 命令执行上下文
//!
//! 每个命令执行时持有 CmdContext，包含输出格式、安静/详细模式、
//! 交互模式判断、配置延迟加载等运行时状态。

use crate::xun_core::renderer::OutputFormat;

/// 命令执行上下文。
///
/// 持有当前命令运行时的全局状态：输出格式、日志级别、交互模式等。
/// 通过 `for_test()` 构造测试实例，通过 builder 方法链设置参数。
pub struct CmdContext {
    format: OutputFormat,
    quiet: bool,
    verbose: bool,
    non_interactive: bool,
    config_loaded: bool,
    config: Option<serde_json::Value>,
}

impl CmdContext {
    /// 创建默认上下文。
    pub fn new() -> Self {
        Self {
            format: OutputFormat::Auto,
            quiet: false,
            verbose: false,
            non_interactive: false,
            config_loaded: false,
            config: None,
        }
    }

    /// 创建测试用上下文（所有默认值）。
    pub fn for_test() -> Self {
        Self::new()
    }

    // ---- Builder 方法 ----

    /// 设置安静模式。
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// 设置详细模式。
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置非交互模式。
    pub fn with_non_interactive(mut self, non_interactive: bool) -> Self {
        self.non_interactive = non_interactive;
        self
    }

    // ---- Getter 方法 ----

    /// 获取输出格式。
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// 是否安静模式。
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// 是否详细模式。
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// 是否非交互模式。
    pub fn is_non_interactive(&self) -> bool {
        self.non_interactive
    }

    /// 配置是否已加载。
    pub fn config_loaded(&self) -> bool {
        self.config_loaded
    }

    // ---- 配置延迟加载 ----

    /// 获取配置（首次调用时加载，之后缓存）。
    pub fn config(&mut self) -> &serde_json::Value {
        if !self.config_loaded {
            self.config_loaded = true;
            if self.config.is_none() {
                // 默认空配置
                self.config = Some(serde_json::Value::Object(serde_json::Map::new()));
            }
        }
        self.config.as_ref().unwrap()
    }

    // ---- 交互判断 ----

    /// 确认提示。非交互模式下自动返回 true。
    pub fn confirm(&self, _prompt: &str) -> bool {
        if self.non_interactive {
            return true;
        }
        // 交互模式下的实际确认逻辑（后续集成 dialoguer）
        true
    }
}
