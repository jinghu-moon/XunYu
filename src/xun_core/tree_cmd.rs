//! Tree CLI 定义（clap derive）
//!
//! 新架构的 tree 命令定义，替代 argh 版本。

use clap::Parser;

/// 生成目录树。
#[derive(Parser, Debug, Clone)]
#[command(name = "tree", about = "Generate directory tree")]
pub struct TreeCmd {
    /// 目标路径（默认当前目录）
    pub path: Option<String>,

    /// 最大深度，0=无限制
    #[arg(short = 'd', long)]
    pub depth: Option<usize>,

    /// 输出文件
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// 包含隐藏文件
    #[arg(long)]
    pub hidden: bool,

    /// 跳过剪贴板复制
    #[arg(long)]
    pub no_clip: bool,

    /// 纯文本输出（无 box drawing）
    #[arg(long)]
    pub plain: bool,

    /// 仅统计（不输出行）
    #[arg(long)]
    pub stats_only: bool,

    /// 快速模式（跳过排序和元数据）
    #[arg(long)]
    pub fast: bool,

    /// 排序方式：name | mtime | size
    #[arg(long, default_value = "name")]
    pub sort: String,

    /// 显示每个项目大小
    #[arg(long)]
    pub size: bool,

    /// 最大输出项目数
    #[arg(long)]
    pub max_items: Option<usize>,

    /// 包含模式（逗号分隔）
    #[arg(long)]
    pub include: Vec<String>,

    /// 排除模式（逗号分隔）
    #[arg(long)]
    pub exclude: Vec<String>,
}
