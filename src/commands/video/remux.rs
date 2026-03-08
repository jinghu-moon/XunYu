use crate::cli::VideoRemuxCmd;
use crate::output::{CliError, CliResult};

use super::common::{
    ensure_input_file, ensure_output_container, ensure_output_path, ensure_parent_dir,
    strict_check_remux_compatibility,
};
use super::ffmpeg::{probe_media, resolve_ffmpeg_path, resolve_ffprobe_path, run_ffmpeg};

pub(super) fn cmd_remux(args: VideoRemuxCmd) -> CliResult {
    let input = ensure_input_file(&args.input).map_err(to_cli_err)?;
    let output = ensure_output_path(&args.output, args.overwrite).map_err(to_cli_err)?;
    ensure_parent_dir(&output).map_err(to_cli_err)?;
    let container = ensure_output_container(&output).map_err(to_cli_err)?;

    let ffmpeg = resolve_ffmpeg_path(args.ffmpeg.as_deref()).map_err(to_cli_err)?;
    if args.strict {
        let ffprobe = resolve_ffprobe_path(args.ffprobe.as_deref()).map_err(to_cli_err)?;
        let probe = probe_media(&ffprobe, &input).map_err(to_cli_err)?;
        strict_check_remux_compatibility(&probe, container).map_err(to_cli_err)?;
    }

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
        "0".to_string(),
        "-c".to_string(),
        "copy".to_string(),
    ];

    if container.is_mp4_family() {
        ff_args.push("-movflags".to_string());
        ff_args.push("+faststart".to_string());
    }

    ff_args.push(output.display().to_string());
    run_ffmpeg(&ffmpeg, &ff_args).map_err(to_cli_err)?;
    ui_println!("remux 完成: {}", output.display());
    Ok(())
}

fn to_cli_err(err: impl ToString) -> CliError {
    CliError::new(1, err.to_string())
}
