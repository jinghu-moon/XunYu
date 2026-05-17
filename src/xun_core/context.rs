//! CmdContext — 命令执行上下文
//!
//! 每个命令执行时持有 CmdContext，包含输出格式、安静/详细模式、
//! 交互模式判断、配置延迟加载等运行时状态。
//!
//! 设计：CmdContext 是命令层的**服务定位器**（Service Locator），
//! 负责提供 cwd / data_dir / config 等全局路径与配置，
//! 避免各子命令模块各自调用 `std::env::var("LOCALAPPDATA")`。

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use crate::config::GlobalConfig;
use crate::xun_core::renderer::OutputFormat;

/// 命令执行上下文。
///
/// 持有当前命令运行时的全局状态：输出格式、日志级别、交互模式、
/// 工作目录、数据目录以及延迟加载的配置。
/// 通过 `for_test()` 构造测试实例，通过 builder 方法链设置参数。
pub struct CmdContext {
    format: OutputFormat,
    quiet: bool,
    verbose: bool,
    non_interactive: bool,
    /// 当前工作目录（可被 -C 参数覆盖）
    cwd: PathBuf,
    /// 数据目录（%LOCALAPPDATA%\xun）
    data_dir: PathBuf,
    /// 用户主目录（%USERPROFILE%）
    home_dir: PathBuf,
    /// 延迟加载的全局配置
    config: OnceCell<GlobalConfig>,
}

impl CmdContext {
    /// 创建默认上下文（从环境变量推断路径）。
    pub fn new() -> Self {
        let data_dir = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("xun");
        let home_dir = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            format: OutputFormat::Auto,
            quiet: false,
            verbose: false,
            non_interactive: false,
            cwd,
            data_dir,
            home_dir,
            config: OnceCell::new(),
        }
    }

    /// 创建测试用上下文（注入临时目录，不污染真实文件系统）。
    pub fn for_test() -> Self {
        Self {
            format: OutputFormat::Auto,
            quiet: true,
            verbose: false,
            non_interactive: true,
            cwd: std::env::temp_dir(),
            data_dir: std::env::temp_dir().join("xun_test"),
            home_dir: std::env::temp_dir(),
            config: OnceCell::new(),
        }
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

    /// 覆盖工作目录（对应 -C 参数）。
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = cwd;
        self
    }

    /// 覆盖数据目录（测试注入用）。
    pub fn with_data_dir(mut self, data_dir: PathBuf) -> Self {
        self.data_dir = data_dir;
        self
    }

    // ---- 路径 Getter ----

    /// 获取当前工作目录（尊重 -C 参数）。
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// 获取数据目录（%LOCALAPPDATA%\xun）。
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 获取用户主目录（%USERPROFILE%）。
    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    // ---- 配置延迟加载 ----

    /// 获取全局配置（首次调用时从磁盘加载，失败则使用默认值）。
    pub fn config(&self) -> &GlobalConfig {
        self.config.get_or_init(|| crate::config::load_config())
    }

    // ---- 输出格式 ----

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

    // ---- 交互判断 ----

    /// 确认提示。非交互模式下自动返回 true。
    pub fn confirm(&self, _prompt: &str) -> bool {
        if self.non_interactive {
            return true;
        }
        // 交互模式下的实际确认逻辑（后续集成 dialoguer）
        true
    }

    /// 检查配置是否已加载。
    pub fn config_loaded(&self) -> bool {
        self.config.get().is_some()
    }
}
