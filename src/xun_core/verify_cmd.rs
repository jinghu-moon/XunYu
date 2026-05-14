//! Verify CLI 定义（clap derive）
//!
//! 新架构的 verify 命令定义，替代 argh 版本。

use clap::Parser;

/// 验证 xunbak 容器完整性。
#[derive(Parser, Debug, Clone)]
#[command(name = "verify", about = "Verify xunbak container integrity")]
pub struct VerifyCmd {
    /// xunbak 容器路径
    pub path: String,

    /// 验证级别：quick | full | manifest-only | existence-only | paranoid
    #[arg(long)]
    pub level: Option<String>,

    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
}
