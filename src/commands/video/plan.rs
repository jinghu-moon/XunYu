use std::collections::HashSet;

use super::error::VideoError;
use super::types::{OutputContainer, VideoEngine, VideoMode};

#[derive(Debug, Clone)]
pub(super) struct EncodeAttempt {
    pub(super) encoder: String,
    pub(super) is_gpu: bool,
    pub(super) video_args: Vec<String>,
}

pub(super) fn build_compress_plan(
    mode: VideoMode,
    engine: VideoEngine,
    container: OutputContainer,
    encoders: &HashSet<String>,
) -> Result<(Vec<EncodeAttempt>, Vec<String>), VideoError> {
    let attempts = if matches!(container, OutputContainer::Webm) {
        build_webm_attempts(mode, engine, encoders)?
    } else {
        build_general_attempts(mode, engine, encoders)?
    };
    let audio_args = build_audio_args(mode, container);
    Ok((attempts, audio_args))
}

fn build_general_attempts(
    mode: VideoMode,
    engine: VideoEngine,
    encoders: &HashSet<String>,
) -> Result<Vec<EncodeAttempt>, VideoError> {
    let (gpu, cpu, prefer_cpu_in_auto) = match mode {
        VideoMode::Fastest => (
            vec!["h264_nvenc", "h264_qsv", "h264_amf"],
            vec!["libx264"],
            false,
        ),
        VideoMode::Balanced => (
            vec!["hevc_nvenc", "hevc_qsv", "hevc_amf"],
            vec!["libx265"],
            false,
        ),
        VideoMode::Smallest => (
            vec!["hevc_nvenc", "hevc_qsv", "hevc_amf"],
            vec!["libx265", "libsvtav1"],
            true,
        ),
    };

    let mut ordered = Vec::new();
    match engine {
        VideoEngine::Auto => {
            if prefer_cpu_in_auto {
                ordered.extend(cpu);
                ordered.extend(gpu);
            } else {
                ordered.extend(gpu);
                ordered.extend(cpu);
            }
        }
        VideoEngine::Cpu => ordered.extend(cpu),
        VideoEngine::Gpu => ordered.extend(gpu),
    }

    let filtered = ordered
        .into_iter()
        .filter(|name| encoders.contains(*name))
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        return if matches!(engine, VideoEngine::Gpu) {
            Err(VideoError::NoUsableGpuEncoder)
        } else {
            Err(VideoError::NoUsableEncoder {
                candidates: "h264/hevc 硬件编码器, libx264, libx265, libsvtav1".to_string(),
            })
        };
    }

    Ok(filtered
        .into_iter()
        .map(|encoder| EncodeAttempt {
            encoder: encoder.to_string(),
            is_gpu: is_gpu_encoder(encoder),
            video_args: build_video_args(mode, encoder),
        })
        .collect())
}

fn build_webm_attempts(
    mode: VideoMode,
    engine: VideoEngine,
    encoders: &HashSet<String>,
) -> Result<Vec<EncodeAttempt>, VideoError> {
    if matches!(engine, VideoEngine::Gpu) {
        return Err(VideoError::NoUsableGpuEncoder);
    }

    let mut ordered = match mode {
        VideoMode::Fastest => vec!["libvpx-vp9"],
        VideoMode::Balanced => vec!["libvpx-vp9"],
        VideoMode::Smallest => vec!["libsvtav1", "libvpx-vp9"],
    };

    ordered.retain(|name| encoders.contains(*name));
    if ordered.is_empty() {
        return Err(VideoError::NoUsableEncoder {
            candidates: "libvpx-vp9, libsvtav1".to_string(),
        });
    }

    Ok(ordered
        .into_iter()
        .map(|encoder| EncodeAttempt {
            encoder: encoder.to_string(),
            is_gpu: false,
            video_args: build_video_args(mode, encoder),
        })
        .collect())
}

fn build_video_args(mode: VideoMode, encoder: &str) -> Vec<String> {
    match encoder {
        "libx264" => vec![
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-crf".into(),
            "24".into(),
        ],
        "libx265" => {
            let preset = match mode {
                VideoMode::Fastest => "fast",
                VideoMode::Balanced => "medium",
                VideoMode::Smallest => "slow",
            };
            let crf = match mode {
                VideoMode::Fastest => "28",
                VideoMode::Balanced => "26",
                VideoMode::Smallest => "28",
            };
            vec![
                "-c:v".into(),
                "libx265".into(),
                "-preset".into(),
                preset.into(),
                "-crf".into(),
                crf.into(),
            ]
        }
        "libvpx-vp9" => match mode {
            VideoMode::Fastest => vec![
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-deadline".into(),
                "realtime".into(),
                "-cpu-used".into(),
                "8".into(),
                "-crf".into(),
                "36".into(),
                "-b:v".into(),
                "0".into(),
            ],
            VideoMode::Balanced => vec![
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-deadline".into(),
                "good".into(),
                "-cpu-used".into(),
                "4".into(),
                "-crf".into(),
                "33".into(),
                "-b:v".into(),
                "0".into(),
            ],
            VideoMode::Smallest => vec![
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-deadline".into(),
                "good".into(),
                "-cpu-used".into(),
                "1".into(),
                "-crf".into(),
                "30".into(),
                "-b:v".into(),
                "0".into(),
            ],
        },
        "libsvtav1" => vec![
            "-c:v".into(),
            "libsvtav1".into(),
            "-preset".into(),
            "6".into(),
            "-crf".into(),
            "35".into(),
        ],
        "h264_nvenc" | "h264_qsv" | "h264_amf" => vec![
            "-c:v".into(),
            encoder.into(),
            "-cq".into(),
            "28".into(),
            "-b:v".into(),
            "0".into(),
        ],
        "hevc_nvenc" | "hevc_qsv" | "hevc_amf" => vec![
            "-c:v".into(),
            encoder.into(),
            "-cq".into(),
            "29".into(),
            "-b:v".into(),
            "0".into(),
        ],
        other => vec!["-c:v".into(), other.into()],
    }
}

fn build_audio_args(mode: VideoMode, container: OutputContainer) -> Vec<String> {
    match container {
        OutputContainer::Webm => {
            let br = if matches!(mode, VideoMode::Smallest) {
                "80k"
            } else {
                "96k"
            };
            vec!["-c:a".into(), "libopus".into(), "-b:a".into(), br.into()]
        }
        _ => {
            let br = if matches!(mode, VideoMode::Smallest) {
                "96k"
            } else {
                "128k"
            };
            vec!["-c:a".into(), "aac".into(), "-b:a".into(), br.into()]
        }
    }
}

fn is_gpu_encoder(name: &str) -> bool {
    name.ends_with("_nvenc") || name.ends_with("_qsv") || name.ends_with("_amf")
}
