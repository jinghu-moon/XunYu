use clap::{Args, Parser, Subcommand};

/// video operations: probe / compress / remux
#[derive(Parser, Debug, Clone)]
pub struct VideoCmd {
    #[command(subcommand)]
    pub cmd: VideoSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum VideoSubCommand {
    Probe(VideoProbeCmd),
    Compress(VideoCompressCmd),
    Remux(VideoRemuxCmd),
}

/// inspect media metadata via ffprobe
#[derive(Args, Debug, Clone)]
pub struct VideoProbeCmd {
    /// input media file path
    #[arg(short = 'i', long)]
    pub input: String,

    /// ffprobe executable path override
    #[arg(long)]
    pub ffprobe: Option<String>,
}

/// compress video (lossy transcode)
#[derive(Args, Debug, Clone)]
pub struct VideoCompressCmd {
    /// input media file path
    #[arg(short = 'i', long)]
    pub input: String,

    /// output media file path
    #[arg(short = 'o', long)]
    pub output: String,

    /// mode: fastest|balanced|smallest
    #[arg(long, default_value = "balanced")]
    pub mode: String,

    /// engine: auto|cpu|gpu
    #[arg(long, default_value = "auto")]
    pub engine: String,

    /// overwrite output if exists
    #[arg(long)]
    pub overwrite: bool,

    /// ffmpeg executable path override
    #[arg(long)]
    pub ffmpeg: Option<String>,
}

/// remux container (lossless stream copy)
#[derive(Args, Debug, Clone)]
pub struct VideoRemuxCmd {
    /// input media file path
    #[arg(short = 'i', long)]
    pub input: String,

    /// output media file path
    #[arg(short = 'o', long)]
    pub output: String,

    /// strict mode: true means incompatible streams fail directly
    #[arg(long, default_value_t = true)]
    pub strict: bool,

    /// overwrite output if exists
    #[arg(long)]
    pub overwrite: bool,

    /// ffmpeg executable path override
    #[arg(long)]
    pub ffmpeg: Option<String>,

    /// ffprobe executable path override
    #[arg(long)]
    pub ffprobe: Option<String>,
}
