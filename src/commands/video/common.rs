use std::path::{Path, PathBuf};

use super::error::VideoError;
use super::types::{OutputContainer, ProbeSummary};

pub(super) fn ensure_input_file(path: &str) -> Result<PathBuf, VideoError> {
    let p = PathBuf::from(path);
    if !p.exists() || !p.is_file() {
        return Err(VideoError::InputNotFound {
            path: path.to_string(),
        });
    }
    Ok(p)
}

pub(super) fn ensure_output_path(path: &str, overwrite: bool) -> Result<PathBuf, VideoError> {
    let p = PathBuf::from(path);
    if p.exists() && !overwrite {
        return Err(VideoError::OutputExists {
            path: path.to_string(),
        });
    }
    Ok(p)
}

pub(super) fn ensure_output_container(path: &Path) -> Result<OutputContainer, VideoError> {
    OutputContainer::from_output_path(path).ok_or_else(|| VideoError::UnsupportedOutputContainer {
        path: path.display().to_string(),
    })
}

pub(super) fn ensure_parent_dir(path: &Path) -> Result<(), VideoError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| VideoError::SpawnFailed {
            detail: format!("创建输出目录失败: {} ({e})", parent.display()),
        })?;
    }
    Ok(())
}

pub(super) fn strict_check_remux_compatibility(
    probe: &ProbeSummary,
    container: OutputContainer,
) -> Result<(), VideoError> {
    for s in &probe.streams {
        let codec = s.codec_name.as_deref().unwrap_or("<unknown>");
        let ok = match container {
            OutputContainer::Mkv => true,
            OutputContainer::Mp4 | OutputContainer::Mov => match s.codec_type.as_str() {
                "video" => matches!(codec, "h264" | "hevc" | "av1" | "mpeg4"),
                "audio" => matches!(codec, "aac" | "mp3" | "ac3" | "eac3" | "alac" | "opus"),
                "subtitle" => matches!(codec, "mov_text"),
                "attachment" => false,
                _ => true,
            },
            OutputContainer::Webm => match s.codec_type.as_str() {
                "video" => matches!(codec, "vp8" | "vp9" | "av1"),
                "audio" => matches!(codec, "vorbis" | "opus"),
                "subtitle" => matches!(codec, "webvtt"),
                "attachment" => false,
                _ => true,
            },
            OutputContainer::MpegTs => match s.codec_type.as_str() {
                "video" => matches!(codec, "h264" | "hevc" | "mpeg2video"),
                "audio" => matches!(codec, "aac" | "mp3" | "ac3" | "eac3"),
                "subtitle" => matches!(codec, "dvb_subtitle"),
                "attachment" => false,
                _ => true,
            },
        };

        if !ok {
            return Err(VideoError::StrictRemuxIncompatible {
                reason: format!(
                    "stream #{} 类型={} codec={} 与目标容器 {} 不兼容",
                    s.index,
                    s.codec_type,
                    codec,
                    container.muxer_name()
                ),
            });
        }
    }
    Ok(())
}
