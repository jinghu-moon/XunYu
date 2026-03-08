use crate::cli::VideoProbeCmd;
use crate::output::{CliError, CliResult};

use super::common::ensure_input_file;
use super::ffmpeg::{probe_media, resolve_ffprobe_path};

pub(super) fn cmd_probe(args: VideoProbeCmd) -> CliResult {
    let input = ensure_input_file(&args.input).map_err(to_cli_err)?;
    let ffprobe = resolve_ffprobe_path(args.ffprobe.as_deref()).map_err(to_cli_err)?;
    let summary = probe_media(&ffprobe, &input).map_err(to_cli_err)?;
    let text = serde_json::to_string_pretty(&summary)
        .map_err(|e| CliError::new(1, format!("probe 输出序列化失败: {e}")))?;
    out_println!("{text}");
    Ok(())
}

fn to_cli_err(err: impl ToString) -> CliError {
    CliError::new(1, err.to_string())
}
