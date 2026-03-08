use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::error::VideoError;
use super::types::{ProbeStream, ProbeSummary};

#[derive(Debug, Deserialize)]
struct RawProbe {
    format: Option<RawFormat>,
    streams: Vec<RawStream>,
}

#[derive(Debug, Deserialize)]
struct RawFormat {
    format_name: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawStream {
    index: usize,
    codec_type: Option<String>,
    codec_name: Option<String>,
}

pub(super) fn resolve_ffmpeg_path(override_path: Option<&str>) -> Result<PathBuf, VideoError> {
    resolve_binary_path(
        override_path,
        "XUN_FFMPEG",
        "ffmpeg",
        VideoError::FfmpegNotFound,
    )
}

pub(super) fn resolve_ffprobe_path(override_path: Option<&str>) -> Result<PathBuf, VideoError> {
    resolve_binary_path(
        override_path,
        "XUN_FFPROBE",
        "ffprobe",
        VideoError::FfprobeNotFound,
    )
}

fn resolve_binary_path(
    override_path: Option<&str>,
    env_key: &str,
    fallback_name: &str,
    not_found: VideoError,
) -> Result<PathBuf, VideoError> {
    if let Some(p) = override_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(p) = std::env::var(env_key) {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }

    let status = Command::new(fallback_name).arg("-version").output();
    if status.is_ok() {
        return Ok(PathBuf::from(fallback_name));
    }

    Err(not_found)
}

pub(super) fn probe_media(ffprobe: &Path, input: &Path) -> Result<ProbeSummary, VideoError> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(input)
        .output()
        .map_err(|e| VideoError::SpawnFailed {
            detail: format!("启动 ffprobe 失败: {e}"),
        })?;

    if !output.status.success() {
        return Err(VideoError::FfprobeFailed {
            detail: summarize_stderr(&output.stderr),
        });
    }

    let raw: RawProbe =
        serde_json::from_slice(&output.stdout).map_err(|e| VideoError::FfprobeFailed {
            detail: format!("解析 ffprobe JSON 失败: {e}"),
        })?;

    let format_name = raw
        .format
        .as_ref()
        .and_then(|f| f.format_name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let duration_secs = raw
        .format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(|v| v.parse::<f64>().ok());

    let bit_rate = raw
        .format
        .as_ref()
        .and_then(|f| f.bit_rate.as_deref())
        .and_then(|v| v.parse::<u64>().ok());

    let streams = raw
        .streams
        .into_iter()
        .map(|s| ProbeStream {
            index: s.index,
            codec_type: s.codec_type.unwrap_or_else(|| "unknown".to_string()),
            codec_name: s.codec_name,
        })
        .collect();

    Ok(ProbeSummary {
        format_name,
        duration_secs,
        bit_rate,
        streams,
    })
}

pub(super) fn list_encoders(ffmpeg: &Path) -> Result<HashSet<String>, VideoError> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|e| VideoError::SpawnFailed {
            detail: format!("启动 ffmpeg 失败: {e}"),
        })?;

    if !output.status.success() {
        return Err(VideoError::FfmpegFailed {
            detail: summarize_stderr(&output.stderr),
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut set = HashSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Encoders:")
            || trimmed.starts_with("------")
            || trimmed.starts_with("..")
        {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(flags) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        if flags.len() >= 6 {
            set.insert(name.to_string());
        }
    }
    Ok(set)
}

pub(super) fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<(), VideoError> {
    let output = Command::new(ffmpeg)
        .args(args)
        .output()
        .map_err(|e| VideoError::SpawnFailed {
            detail: format!("启动 ffmpeg 失败: {e}"),
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(VideoError::FfmpegFailed {
        detail: summarize_stderr(&output.stderr),
    })
}

fn summarize_stderr(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return "无错误输出".to_string();
    }
    if text.lines().count() > lines.len() {
        lines.push("...");
    }
    lines.join(" | ")
}
