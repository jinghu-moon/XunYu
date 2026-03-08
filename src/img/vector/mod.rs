mod bezier;
mod common;
mod diffvg;
mod potrace;
mod skeleton;
mod tracer;
mod visioncortex;

use image::DynamicImage;
use std::time::Instant;

use super::types::SvgMethod;

#[derive(Debug, Clone, Copy, Default)]
pub struct SvgTraceTimingsMs {
    pub trace_total_ms: u64,
    pub vc_to_color_ms: u64,
    pub vc_keying_ms: u64,
    pub vc_cluster_ms: u64,
    pub vc_cluster_quantize_ms: u64,
    pub vc_cluster_label_ms: u64,
    pub vc_cluster_stats_ms: u64,
    pub vc_cluster_merge_ms: u64,
    pub vc_cluster_finalize_ms: u64,
    pub vc_path_build_ms: u64,
    pub vc_path_sort_ms: u64,
    pub vc_path_trace_ms: u64,
    pub vc_path_smooth_ms: u64,
    pub vc_path_svg_emit_ms: u64,
    pub vc_path_components_total: u64,
    pub vc_path_components_simplified: u64,
    pub vc_path_components_smoothed: u64,
    pub vc_svg_wrap_ms: u64,
}

pub struct SvgTraceResult {
    pub svg: String,
    pub timings: SvgTraceTimingsMs,
}

pub fn trace_to_svg_with_timings(
    img: &DynamicImage,
    method: SvgMethod,
    diffvg_iters: usize,
    diffvg_strokes: usize,
) -> anyhow::Result<SvgTraceResult> {
    let start = Instant::now();
    match method {
        SvgMethod::Bezier => {
            let svg = tracer::trace(img, &tracer::TracerConfig::default())?;
            Ok(common::finish_trace(svg, start))
        }
        SvgMethod::Visioncortex => visioncortex::trace_with_timings(img),
        SvgMethod::Potrace => {
            let svg = potrace::trace(img, &potrace::PotraceConfig::default())?;
            Ok(common::finish_trace(svg, start))
        }
        SvgMethod::Skeleton => {
            let svg = skeleton::trace(img, &skeleton::SkeletonConfig::default())?;
            Ok(common::finish_trace(svg, start))
        }
        SvgMethod::Diffvg => {
            let cfg = diffvg::DiffvgConfig {
                iterations: diffvg_iters.max(1),
                num_strokes: diffvg_strokes.max(1),
                ..diffvg::DiffvgConfig::default()
            };
            let svg = diffvg::trace(img, &cfg)?;
            Ok(common::finish_trace(svg, start))
        }
    }
}
