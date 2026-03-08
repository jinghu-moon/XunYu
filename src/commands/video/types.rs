use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoMode {
    Fastest,
    Balanced,
    Smallest,
}

impl VideoMode {
    pub(super) fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "fastest" => Some(Self::Fastest),
            "balanced" => Some(Self::Balanced),
            "smallest" => Some(Self::Smallest),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Fastest => "fastest",
            Self::Balanced => "balanced",
            Self::Smallest => "smallest",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoEngine {
    Auto,
    Cpu,
    Gpu,
}

impl VideoEngine {
    pub(super) fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "cpu" => Some(Self::Cpu),
            "gpu" => Some(Self::Gpu),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputContainer {
    Mp4,
    Mov,
    Mkv,
    Webm,
    MpegTs,
}

impl OutputContainer {
    pub(super) fn from_output_path(path: &Path) -> Option<Self> {
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .map(|s| s.to_ascii_lowercase())?;

        match ext.as_str() {
            "mp4" | "m4v" => Some(Self::Mp4),
            "mov" => Some(Self::Mov),
            "mkv" => Some(Self::Mkv),
            "webm" => Some(Self::Webm),
            "ts" | "mpegts" => Some(Self::MpegTs),
            _ => None,
        }
    }

    pub(super) fn muxer_name(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::Mkv => "matroska",
            Self::Webm => "webm",
            Self::MpegTs => "mpegts",
        }
    }

    pub(super) fn is_mp4_family(self) -> bool {
        matches!(self, Self::Mp4 | Self::Mov)
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ProbeSummary {
    pub(super) format_name: String,
    pub(super) duration_secs: Option<f64>,
    pub(super) bit_rate: Option<u64>,
    pub(super) streams: Vec<ProbeStream>,
}

impl ProbeSummary {
    pub(super) fn has_video_stream(&self) -> bool {
        self.streams.iter().any(|s| s.codec_type == "video")
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ProbeStream {
    pub(super) index: usize,
    pub(super) codec_type: String,
    pub(super) codec_name: Option<String>,
}
