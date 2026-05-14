//! Dashboard CLI 定义（clap derive）
//!
//! 新架构的 dashboard 命令定义，替代 argh 版本。
//! ServeCmd 独立命令。

use clap::Parser;

// ── Serve 命令 ───────────────────────────────────────────────────

/// Start web dashboard server.
#[derive(Parser, Debug, Clone)]
#[command(name = "serve", about = "Start web dashboard server")]
pub struct ServeCmd {
    /// listen port (default: 9527)
    #[arg(short = 'p', long, default_value_t = 9527)]
    pub port: u16,
}
