//! Args — 统一参数组
//!
//! 可复用的 clap 参数组，通过 `#[command(flatten)]` 嵌入各命令。
//! derive Parser 使每个 struct 可独立测试。

/// 列表类命令的公共参数。
#[derive(clap::Parser, Clone, Debug)]
pub struct ListArgs {
    /// 最大返回条目数
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// 跳过前 N 条
    #[arg(long, default_value_t = 0)]
    pub offset: usize,

    /// 排序字段
    #[arg(long)]
    pub sort: Option<String>,

    /// 反转排序
    #[arg(long)]
    pub reverse: bool,
}

/// 模糊搜索命令的公共参数。
#[derive(clap::Parser, Clone, Debug)]
pub struct FuzzyArgs {
    /// 搜索模式（可多个）
    #[arg(required = true)]
    pub patterns: Vec<String>,

    /// 仅列出候选，不执行跳转
    #[arg(long)]
    pub list: bool,

    /// 按标签过滤
    #[arg(long, short = 't')]
    pub tag: Option<String>,

    /// 输出格式
    #[arg(long, short = 'f', default_value = "auto")]
    pub format: String,
}

/// 作用域参数（bookmark 等模块使用）。
#[derive(clap::Parser, Clone, Debug)]
pub struct ScopeArgs {
    /// 全局作用域
    #[arg(long, short = 'g')]
    pub global: bool,

    /// 子作用域
    #[arg(long, short = 'c')]
    pub child: bool,

    /// 工作区名称
    #[arg(long, short = 'w')]
    pub workspace: Option<String>,

    /// 预设名称
    #[arg(long)]
    pub preset: Option<String>,

    /// 基目录限制
    #[arg(long)]
    pub base: Option<String>,
}

/// 确认/干运行参数。
#[derive(clap::Parser, Clone, Debug)]
pub struct ConfirmArgs {
    /// 跳过确认提示
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// 干运行，仅预览不执行
    #[arg(long)]
    pub dry_run: bool,
}
