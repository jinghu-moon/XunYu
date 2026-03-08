use image::DynamicImage;
use std::time::Instant;

use crate::img::{
    encode::EncodeTimingsMs, error::ImgError, types::SvgMethod, vector::trace_to_svg_with_timings,
};

pub fn encode_svg_with_timings(
    img: &DynamicImage,
    method: SvgMethod,
    diffvg_iters: usize,
    diffvg_strokes: usize,
) -> Result<(Vec<u8>, EncodeTimingsMs), ImgError> {
    let mut timings = EncodeTimingsMs::default();
    let trace_start = Instant::now();
    let traced =
        trace_to_svg_with_timings(img, method, diffvg_iters, diffvg_strokes).map_err(|e| {
            ImgError::EncodeFailed {
                encoder: "svg",
                msg: e.to_string(),
            }
        })?;
    timings.svg_trace_ms = trace_start.elapsed().as_millis() as u64;
    timings.svg_trace_internal_ms = traced.timings.trace_total_ms;
    timings.svg_vc_to_color_ms = traced.timings.vc_to_color_ms;
    timings.svg_vc_keying_ms = traced.timings.vc_keying_ms;
    timings.svg_vc_cluster_ms = traced.timings.vc_cluster_ms;
    timings.svg_vc_cluster_quantize_ms = traced.timings.vc_cluster_quantize_ms;
    timings.svg_vc_cluster_label_ms = traced.timings.vc_cluster_label_ms;
    timings.svg_vc_cluster_stats_ms = traced.timings.vc_cluster_stats_ms;
    timings.svg_vc_cluster_merge_ms = traced.timings.vc_cluster_merge_ms;
    timings.svg_vc_cluster_finalize_ms = traced.timings.vc_cluster_finalize_ms;
    timings.svg_vc_path_build_ms = traced.timings.vc_path_build_ms;
    timings.svg_vc_path_sort_ms = traced.timings.vc_path_sort_ms;
    timings.svg_vc_path_trace_ms = traced.timings.vc_path_trace_ms;
    timings.svg_vc_path_smooth_ms = traced.timings.vc_path_smooth_ms;
    timings.svg_vc_path_svg_emit_ms = traced.timings.vc_path_svg_emit_ms;
    timings.svg_vc_path_components_total = traced.timings.vc_path_components_total;
    timings.svg_vc_path_components_simplified = traced.timings.vc_path_components_simplified;
    timings.svg_vc_path_components_smoothed = traced.timings.vc_path_components_smoothed;
    timings.svg_vc_wrap_ms = traced.timings.vc_svg_wrap_ms;

    let serialize_start = Instant::now();
    let bytes = traced.svg.into_bytes();
    timings.svg_serialize_ms = serialize_start.elapsed().as_millis() as u64;
    Ok((bytes, timings))
}
