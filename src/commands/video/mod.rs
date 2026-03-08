mod common;
mod compress;
mod error;
mod ffmpeg;
mod plan;
mod probe;
mod remux;
mod types;

use crate::cli::{VideoCmd, VideoSubCommand};
use crate::output::CliResult;

pub(crate) fn cmd_video(args: VideoCmd) -> CliResult {
    match args.cmd {
        VideoSubCommand::Probe(a) => probe::cmd_probe(a),
        VideoSubCommand::Compress(a) => compress::cmd_compress(a),
        VideoSubCommand::Remux(a) => remux::cmd_remux(a),
    }
}
