use crate::cli::VideoCompressCmd;
use crate::output::{CliError, CliResult};

use super::common::{
    ensure_input_file, ensure_output_container, ensure_output_path, ensure_parent_dir,
};
use super::error::VideoError;
use super::ffmpeg::{
    list_encoders, probe_media, resolve_ffmpeg_path, resolve_ffprobe_path, run_ffmpeg,
};
use super::plan::build_compress_plan;
use super::types::{VideoEngine, VideoMode};

pub(super) fn cmd_compress(args: VideoCompressCmd) -> CliResult {
    let mode = VideoMode::parse(&args.mode).ok_or_else(|| {
        to_cli_err(VideoError::InvalidMode {
            value: args.mode.clone(),
        })
    })?;
    let engine = VideoEngine::parse(&args.engine).ok_or_else(|| {
        to_cli_err(VideoError::InvalidEngine {
            value: args.engine.clone(),
        })
    })?;

    let input = ensure_input_file(&args.input).map_err(to_cli_err)?;
    let output = ensure_output_path(&args.output, args.overwrite).map_err(to_cli_err)?;
    ensure_parent_dir(&output).map_err(to_cli_err)?;
    let container = ensure_output_container(&output).map_err(to_cli_err)?;

    let ffmpeg = resolve_ffmpeg_path(args.ffmpeg.as_deref()).map_err(to_cli_err)?;
    let ffprobe = resolve_ffprobe_path(None).map_err(to_cli_err)?;

    let probe = probe_media(&ffprobe, &input).map_err(to_cli_err)?;
    if !probe.has_video_stream() {
        return Err(to_cli_err(VideoError::MissingVideoStream {
            path: input.display().to_string(),
        }));
    }

    let encoders = list_encoders(&ffmpeg).map_err(to_cli_err)?;
    let (attempts, audio_args) =
        build_compress_plan(mode, engine, container, &encoders).map_err(to_cli_err)?;

    let mut last_error: Option<CliError> = None;
    for attempt in attempts {
        let mut ff_args = vec![
            "-hide_banner".to_string(),
            if args.overwrite {
                "-y".to_string()
            } else {
                "-n".to_string()
            },
            "-i".to_string(),
            input.display().to_string(),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-map".to_string(),
            "0:s?".to_string(),
        ];
        ff_args.extend(attempt.video_args.clone());
        ff_args.extend(audio_args.clone());
        ff_args.push("-c:s".to_string());
        ff_args.push("copy".to_string());
        if container.is_mp4_family() {
            ff_args.push("-movflags".to_string());
            ff_args.push("+faststart".to_string());
        }
        ff_args.push(output.display().to_string());

        ui_println!(
            "compress 尝试编码器={} (mode={}, engine={})",
            attempt.encoder,
            mode.as_str(),
            engine.as_str()
        );

        match run_ffmpeg(&ffmpeg, &ff_args) {
            Ok(()) => {
                ui_println!("compress 完成: {}", output.display());
                return Ok(());
            }
            Err(e) => {
                let cli_err = to_cli_err(e);
                if attempt.is_gpu {
                    ui_println!("编码器 {} 失败，尝试回退...", attempt.encoder);
                }
                last_error = Some(cli_err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| CliError::new(1, "compress 失败：没有可用编码器".to_string())))
}

fn to_cli_err(err: impl ToString) -> CliError {
    CliError::new(1, err.to_string())
}
