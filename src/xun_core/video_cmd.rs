//! Video CLI 定义（clap derive）
//!
//! 新架构的 video 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};

/// 视频操作：probe / compress / remux。
#[derive(Parser, Debug, Clone)]
#[command(name = "video", about = "Video operations")]
pub struct VideoCmd {
    #[command(subcommand)]
    pub sub: VideoSubCommand,
}

/// Video 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum VideoSubCommand {
    /// 检查媒体元数据（ffprobe）
    Probe(VideoProbeArgs),
    /// 压缩视频（有损转码）
    Compress(VideoCompressArgs),
    /// 重封装容器（无损流拷贝）
    Remux(VideoRemuxArgs),
}

/// video probe 参数。
#[derive(Parser, Debug, Clone)]
pub struct VideoProbeArgs {
    /// 输入媒体文件路径
    #[arg(short = 'i', long)]
    pub input: String,
    /// ffprobe 可执行文件路径覆盖
    #[arg(long)]
    pub ffprobe: Option<String>,
}

/// video compress 参数。
#[derive(Parser, Debug, Clone)]
pub struct VideoCompressArgs {
    /// 输入媒体文件路径
    #[arg(short = 'i', long)]
    pub input: String,
    /// 输出媒体文件路径
    #[arg(short = 'o', long)]
    pub output: String,
    /// 模式：fastest | balanced | smallest
    #[arg(long, default_value = "balanced")]
    pub mode: String,
    /// 引擎：auto | cpu | gpu
    #[arg(long, default_value = "auto")]
    pub engine: String,
    /// 覆盖已存在的输出文件
    #[arg(long)]
    pub overwrite: bool,
    /// ffmpeg 可执行文件路径覆盖
    #[arg(long)]
    pub ffmpeg: Option<String>,
}

/// video remux 参数。
#[derive(Parser, Debug, Clone)]
pub struct VideoRemuxArgs {
    /// 输入媒体文件路径
    #[arg(short = 'i', long)]
    pub input: String,
    /// 输出媒体文件路径
    #[arg(short = 'o', long)]
    pub output: String,
    /// 严格模式：true 表示不兼容流直接失败
    #[arg(long, default_value = "true")]
    pub strict: String,
    /// 覆盖已存在的输出文件
    #[arg(long)]
    pub overwrite: bool,
    /// ffmpeg 可执行文件路径覆盖
    #[arg(long)]
    pub ffmpeg: Option<String>,
    /// ffprobe 可执行文件路径覆盖
    #[arg(long)]
    pub ffprobe: Option<String>,
}
