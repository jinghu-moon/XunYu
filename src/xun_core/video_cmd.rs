//! Video CLI 定义（clap derive）

use clap::{Args, Parser, Subcommand};

/// 视频操作：probe / compress / remux。
#[derive(Parser, Debug, Clone)]
#[command(name = "video", about = "Video operations")]
pub struct VideoCmd {
    #[command(subcommand)]
    pub cmd: VideoSubCommand,
}

/// Video 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum VideoSubCommand {
    /// 检查媒体元数据（ffprobe）
    Probe(VideoProbeCmd),
    /// 压缩视频（有损转码）
    Compress(VideoCompressCmd),
    /// 重封装容器（无损流拷贝）
    Remux(VideoRemuxCmd),
}

/// video probe 参数。
#[derive(Args, Debug, Clone)]
pub struct VideoProbeCmd {
    /// 输入媒体文件路径
    #[arg(short = 'i', long)]
    pub input: String,
    /// ffprobe 可执行文件路径覆盖
    #[arg(long)]
    pub ffprobe: Option<String>,
}

/// video compress 参数。
#[derive(Args, Debug, Clone)]
pub struct VideoCompressCmd {
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
#[derive(Args, Debug, Clone)]
pub struct VideoRemuxCmd {
    /// 输入媒体文件路径
    #[arg(short = 'i', long)]
    pub input: String,
    /// 输出媒体文件路径
    #[arg(short = 'o', long)]
    pub output: String,
    /// 严格模式：true 表示不兼容流直接失败
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub strict: bool,
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

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::value::Value;

/// video probe 命令。
pub struct VideoProbeCmdSpec {
    pub args: VideoProbeCmd,
}

impl CommandSpec for VideoProbeCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let video_cmd = VideoCmd {
            cmd: VideoSubCommand::Probe(self.args.clone()),
        };
        crate::commands::video::cmd_video(video_cmd)?;
        Ok(Value::Null)
    }
}

/// video compress 命令。
pub struct VideoCompressCmdSpec {
    pub args: VideoCompressCmd,
}

impl CommandSpec for VideoCompressCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let video_cmd = VideoCmd {
            cmd: VideoSubCommand::Compress(self.args.clone()),
        };
        crate::commands::video::cmd_video(video_cmd)?;
        Ok(Value::Null)
    }
}

/// video remux 命令。
pub struct VideoRemuxCmdSpec {
    pub args: VideoRemuxCmd,
}

impl CommandSpec for VideoRemuxCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let video_cmd = VideoCmd {
            cmd: VideoSubCommand::Remux(self.args.clone()),
        };
        crate::commands::video::cmd_video(video_cmd)?;
        Ok(Value::Null)
    }
}
