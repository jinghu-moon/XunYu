use std::{
    collections::BTreeSet,
    fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use super::{
    decode::decode_and_scale_from_memory,
    encode::{EncodeOptions, encode, encode_png_lossless_bytes},
    error::ImgError,
    output::{derive_output_path, should_skip},
    types::{OutputFormat, ProcessParams, ProcessResult, StageDurationsMs},
};

fn can_use_png_lossless_fast_path(src: &Path, params: &ProcessParams) -> bool {
    if params.format != OutputFormat::Png
        || params.png_lossy
        || params.max_width.is_some()
        || params.max_height.is_some()
    {
        return false;
    }

    src.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
}

pub fn run(
    files: &[PathBuf],
    input_root: &Path,
    output_dir: &Path,
    params: &ProcessParams,
    threads: Option<usize>,
    overwrite: bool,
) -> Vec<ProcessResult> {
    if let Err(e) = fs::create_dir_all(output_dir) {
        return vec![ProcessResult {
            input_path: input_root.to_path_buf(),
            output_path: output_dir.to_path_buf(),
            input_bytes: 0,
            output_bytes: 0,
            elapsed_ms: 0,
            stage_ms: StageDurationsMs::default(),
            skipped: false,
            error: Some(
                ImgError::OutputDirFailed {
                    path: output_dir.display().to_string(),
                    source: e,
                }
                .to_string(),
            ),
        }];
    }

    let default_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let thread_count = threads.unwrap_or(default_threads);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .expect("rayon ThreadPool 创建失败");
    let encode_options = EncodeOptions {
        avif_num_threads: params.avif_threads,
    };

    let jobs: Vec<(PathBuf, PathBuf)> = files
        .iter()
        .map(|src| {
            (
                src.clone(),
                derive_output_path(src, input_root, output_dir, params.format),
            )
        })
        .collect();

    let mut output_dirs = BTreeSet::new();
    for (_, out) in &jobs {
        if let Some(parent) = out.parent() {
            output_dirs.insert(parent.to_path_buf());
        }
    }

    for dir in output_dirs {
        if let Err(e) = fs::create_dir_all(&dir) {
            return vec![ProcessResult {
                input_path: input_root.to_path_buf(),
                output_path: dir.clone(),
                input_bytes: 0,
                output_bytes: 0,
                elapsed_ms: 0,
                stage_ms: StageDurationsMs::default(),
                skipped: false,
                error: Some(
                    ImgError::OutputDirFailed {
                        path: dir.display().to_string(),
                        source: e,
                    }
                    .to_string(),
                ),
            }];
        }
    }

    let pb = if std::io::stderr().is_terminal() {
        let pb = ProgressBar::new(jobs.len() as u64);
        let style =
            ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("=>-");
        pb.set_style(style);
        pb.enable_steady_tick(Duration::from_millis(120));
        pb
    } else {
        ProgressBar::hidden()
    };
    let pb_worker = pb.clone();

    let results: Vec<ProcessResult> = pool.install(|| {
        jobs.par_iter()
            .map(|(src, out)| {
                let out = out.clone();

                if should_skip(&out, overwrite) {
                    pb_worker.inc(1);
                    return ProcessResult {
                        input_path: src.clone(),
                        output_path: out,
                        input_bytes: 0,
                        output_bytes: 0,
                        elapsed_ms: 0,
                        stage_ms: StageDurationsMs::default(),
                        skipped: true,
                        error: None,
                    };
                }

                let start = Instant::now();
                let mut stage_ms = StageDurationsMs::default();
                let result: Result<(u64, u64), ImgError> =
                    if can_use_png_lossless_fast_path(src, params) {
                        (|| {
                            let read_start = Instant::now();
                            let raw_png = fs::read(src).map_err(ImgError::from)?;
                            stage_ms.read_ms = read_start.elapsed().as_millis() as u64;

                            let input_bytes = raw_png.len() as u64;

                            let optimize_start = Instant::now();
                            let bytes = encode_png_lossless_bytes(&raw_png)?;
                            stage_ms.png_optimize_ms = optimize_start.elapsed().as_millis() as u64;

                            let write_start = Instant::now();
                            fs::write(&out, &bytes)?;
                            stage_ms.write_ms = write_start.elapsed().as_millis() as u64;

                            Ok((input_bytes, bytes.len() as u64))
                        })()
                    } else {
                        (|| {
                            let read_start = Instant::now();
                            let input_buf = fs::read(src).map_err(ImgError::from)?;
                            stage_ms.read_ms = read_start.elapsed().as_millis() as u64;
                            let input_bytes = input_buf.len() as u64;

                            let decoded = decode_and_scale_from_memory(&input_buf, src, params)?;
                            stage_ms.decode_ms = decoded.decode_ms;
                            stage_ms.resize_ms = decoded.resize_ms;

                            let encoded = encode(&decoded.image, params, encode_options)?;
                            stage_ms.pixel_convert_ms = encoded.timings.pixel_convert_ms;
                            stage_ms.encode_pre_ms = encoded.timings.encode_pre_ms;
                            stage_ms.codec_ms = encoded.timings.codec_ms;
                            stage_ms.png_optimize_ms += encoded.timings.png_optimize_ms;
                            stage_ms.svg_trace_ms = encoded.timings.svg_trace_ms;
                            stage_ms.svg_serialize_ms = encoded.timings.svg_serialize_ms;
                            stage_ms.svg_trace_internal_ms = encoded.timings.svg_trace_internal_ms;
                            stage_ms.svg_vc_to_color_ms = encoded.timings.svg_vc_to_color_ms;
                            stage_ms.svg_vc_keying_ms = encoded.timings.svg_vc_keying_ms;
                            stage_ms.svg_vc_cluster_ms = encoded.timings.svg_vc_cluster_ms;
                            stage_ms.svg_vc_cluster_quantize_ms =
                                encoded.timings.svg_vc_cluster_quantize_ms;
                            stage_ms.svg_vc_cluster_label_ms =
                                encoded.timings.svg_vc_cluster_label_ms;
                            stage_ms.svg_vc_cluster_stats_ms =
                                encoded.timings.svg_vc_cluster_stats_ms;
                            stage_ms.svg_vc_cluster_merge_ms =
                                encoded.timings.svg_vc_cluster_merge_ms;
                            stage_ms.svg_vc_cluster_finalize_ms =
                                encoded.timings.svg_vc_cluster_finalize_ms;
                            stage_ms.svg_vc_path_build_ms = encoded.timings.svg_vc_path_build_ms;
                            stage_ms.svg_vc_path_sort_ms = encoded.timings.svg_vc_path_sort_ms;
                            stage_ms.svg_vc_path_trace_ms = encoded.timings.svg_vc_path_trace_ms;
                            stage_ms.svg_vc_path_smooth_ms = encoded.timings.svg_vc_path_smooth_ms;
                            stage_ms.svg_vc_path_svg_emit_ms =
                                encoded.timings.svg_vc_path_svg_emit_ms;
                            stage_ms.svg_vc_path_components_total =
                                encoded.timings.svg_vc_path_components_total;
                            stage_ms.svg_vc_path_components_simplified =
                                encoded.timings.svg_vc_path_components_simplified;
                            stage_ms.svg_vc_path_components_smoothed =
                                encoded.timings.svg_vc_path_components_smoothed;
                            stage_ms.svg_vc_wrap_ms = encoded.timings.svg_vc_wrap_ms;

                            let write_start = Instant::now();
                            fs::write(&out, encoded.bytes.as_slice())?;
                            stage_ms.write_ms = write_start.elapsed().as_millis() as u64;

                            Ok((input_bytes, encoded.bytes.len() as u64))
                        })()
                    };

                let elapsed_ms = start.elapsed().as_millis() as u64;
                pb_worker.inc(1);

                match result {
                    Ok((input_bytes, output_bytes)) => ProcessResult {
                        input_path: src.clone(),
                        output_path: out,
                        input_bytes,
                        output_bytes,
                        elapsed_ms,
                        stage_ms,
                        skipped: false,
                        error: None,
                    },
                    Err(e) => ProcessResult {
                        input_path: src.clone(),
                        output_path: out,
                        input_bytes: fs::metadata(src).map(|m| m.len()).unwrap_or(0),
                        output_bytes: 0,
                        elapsed_ms,
                        stage_ms,
                        skipped: false,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    });

    pb.finish_with_message("完成");
    results
}
