use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImgError {
    #[error("解码失败 `{path}`: {source}")]
    DecodeFailed {
        path: String,
        #[source]
        source: image::ImageError,
    },

    #[error("不支持的输出格式 `{format}`，可选：jpeg / png / webp / avif / svg")]
    UnsupportedFormat { format: String },

    #[error(
        "不支持的 SVG 算法 `{method}`，可选：bezier / visioncortex / potrace / skeleton / diffvg"
    )]
    InvalidSvgMethod { method: String },

    #[error("JPEG 后端未编译：请使用 --features img-moz 或 --features img-turbo 重新构建")]
    JpegBackendMissing,

    #[error("不支持的 JPEG 后端 `{backend}`，可选：auto / moz / turbo")]
    InvalidJpegBackend { backend: String },

    #[error("JPEG 后端 `{backend}` 未编译，当前可用：{available}")]
    JpegBackendUnavailable { backend: String, available: String },

    #[error("输入文件不存在: {path}")]
    InputNotFound { path: String },

    #[error("无法创建输出目录 `{path}`: {source}")]
    OutputDirFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("质量参数必须在 1-100 之间，当前: {value}")]
    InvalidQuality { value: u8 },

    #[error("AVIF 线程数必须 >= 1，当前: {value}")]
    InvalidAvifThreads { value: usize },

    #[error("SVG diffvg iterations 必须 >= 1，当前: {value}")]
    InvalidSvgDiffvgIters { value: usize },

    #[error("SVG diffvg strokes 必须 >= 1，当前: {value}")]
    InvalidSvgDiffvgStrokes { value: usize },

    #[error("PNG 抖动强度必须在 0.0-1.0 之间，当前: {value}")]
    InvalidPngDitherLevel { value: f32 },

    #[error("编码错误 [{encoder}]: {msg}")]
    EncodeFailed { encoder: &'static str, msg: String },

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}
