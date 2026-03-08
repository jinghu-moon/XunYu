use argh::FromArgs;

/// video operations: probe / compress / remux
#[derive(FromArgs)]
#[argh(subcommand, name = "video")]
pub struct VideoCmd {
    #[argh(subcommand)]
    pub cmd: VideoSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum VideoSubCommand {
    Probe(VideoProbeCmd),
    Compress(VideoCompressCmd),
    Remux(VideoRemuxCmd),
}

/// inspect media metadata via ffprobe
#[derive(FromArgs)]
#[argh(subcommand, name = "probe")]
pub struct VideoProbeCmd {
    /// input media file path
    #[argh(option, short = 'i')]
    pub input: String,

    /// ffprobe executable path override
    #[argh(option)]
    pub ffprobe: Option<String>,
}

/// compress video (lossy transcode)
#[derive(FromArgs)]
#[argh(subcommand, name = "compress")]
pub struct VideoCompressCmd {
    /// input media file path
    #[argh(option, short = 'i')]
    pub input: String,

    /// output media file path
    #[argh(option, short = 'o')]
    pub output: String,

    /// mode: fastest|balanced|smallest
    #[argh(option, default = "String::from(\"balanced\")")]
    pub mode: String,

    /// engine: auto|cpu|gpu
    #[argh(option, default = "String::from(\"auto\")")]
    pub engine: String,

    /// overwrite output if exists
    #[argh(switch)]
    pub overwrite: bool,

    /// ffmpeg executable path override
    #[argh(option)]
    pub ffmpeg: Option<String>,
}

/// remux container (lossless stream copy)
#[derive(FromArgs)]
#[argh(subcommand, name = "remux")]
pub struct VideoRemuxCmd {
    /// input media file path
    #[argh(option, short = 'i')]
    pub input: String,

    /// output media file path
    #[argh(option, short = 'o')]
    pub output: String,

    /// strict mode: true means incompatible streams fail directly
    #[argh(option, default = "true")]
    pub strict: bool,

    /// overwrite output if exists
    #[argh(switch)]
    pub overwrite: bool,

    /// ffmpeg executable path override
    #[argh(option)]
    pub ffmpeg: Option<String>,

    /// ffprobe executable path override
    #[argh(option)]
    pub ffprobe: Option<String>,
}
