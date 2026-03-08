use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum VideoError {
    #[error("输入文件不存在: {path}")]
    InputNotFound { path: String },

    #[error("输出文件已存在（可使用 --overwrite 覆盖）: {path}")]
    OutputExists { path: String },

    #[error("不支持的输出容器，请使用扩展名 mp4/mov/mkv/webm/ts: {path}")]
    UnsupportedOutputContainer { path: String },

    #[error("无效的 mode: {value}（可选 fastest|balanced|smallest）")]
    InvalidMode { value: String },

    #[error("无效的 engine: {value}（可选 auto|cpu|gpu）")]
    InvalidEngine { value: String },

    #[error("探测结果没有视频流: {path}")]
    MissingVideoStream { path: String },

    #[error("strict remux 检测到容器/编码不兼容: {reason}")]
    StrictRemuxIncompatible { reason: String },

    #[error("未找到 ffmpeg，可通过 --ffmpeg 或环境变量 XUN_FFMPEG 指定")]
    FfmpegNotFound,

    #[error("未找到 ffprobe，可通过 --ffprobe 或环境变量 XUN_FFPROBE 指定")]
    FfprobeNotFound,

    #[error("执行外部命令失败: {detail}")]
    SpawnFailed { detail: String },

    #[error("ffprobe 执行失败: {detail}")]
    FfprobeFailed { detail: String },

    #[error("ffmpeg 执行失败: {detail}")]
    FfmpegFailed { detail: String },

    #[error("未找到可用的 GPU 编码器（engine=gpu）")]
    NoUsableGpuEncoder,

    #[error("未找到可用编码器，候选: {candidates}")]
    NoUsableEncoder { candidates: String },
}
