use super::backend::{build_runner_config, cluster_backend_from_env, should_key, to_color_image};
use super::cluster::build_xun_components;
use super::convert::{build_paths_with_official_clusters, build_paths_with_xun_components};
use super::options::{
    analyze_complexity, apply_key_to_transparent, build_xun_cluster_config, build_xun_path_config,
    find_key_color,
};
use super::*;

pub(super) fn trace_with_timings(img: &DynamicImage) -> anyhow::Result<SvgTraceResult> {
    let mut timings = SvgTraceTimingsMs::default();
    let total_start = Instant::now();

    let to_color_start = Instant::now();
    let mut color_img = to_color_image(img);
    timings.vc_to_color_ms = to_color_start.elapsed().as_millis() as u64;

    let width = color_img.width;
    let height = color_img.height;
    let pixels = width.saturating_mul(height);
    let complexity = analyze_complexity(&color_img);
    let cluster_backend = cluster_backend_from_env();

    let keying_start = Instant::now();
    let key_color = if should_key(&color_img) {
        let kc = find_key_color(&color_img).ok_or_else(|| anyhow!("no unused key color found"))?;
        apply_key_to_transparent(&mut color_img, kc);
        kc
    } else {
        Color::default()
    };
    timings.vc_keying_ms = keying_start.elapsed().as_millis() as u64;

    let small_cluster_polygon_area = if complexity.high_color_complexity && pixels >= LARGE_IMAGE_PX
    {
        COMPLEX_POLYGON_AREA
    } else {
        SMALL_CLUSTER_POLYGON_AREA
    };

    let cluster_start = Instant::now();
    let paths = match cluster_backend {
        ClusterBackend::Official => {
            let runner_cfg = build_runner_config(width, height, key_color, complexity);
            let clusters = Runner::new(runner_cfg, color_img).run();
            timings.vc_cluster_ms = cluster_start.elapsed().as_millis() as u64;

            let path_start = Instant::now();
            let paths = build_paths_with_official_clusters(&clusters, small_cluster_polygon_area);
            timings.vc_path_build_ms = path_start.elapsed().as_millis() as u64;
            paths
        }
        ClusterBackend::Xun => {
            let xun_cfg = build_xun_cluster_config(pixels, complexity);
            let (mut components, xun_cluster_stats) =
                build_xun_components(&color_img, key_color, xun_cfg);
            timings.vc_cluster_ms = cluster_start.elapsed().as_millis() as u64;
            timings.vc_cluster_quantize_ms = xun_cluster_stats.quantize_ms;
            timings.vc_cluster_label_ms = xun_cluster_stats.label_ms;
            timings.vc_cluster_stats_ms = xun_cluster_stats.stats_ms;
            timings.vc_cluster_merge_ms = xun_cluster_stats.merge_ms;
            timings.vc_cluster_finalize_ms = xun_cluster_stats.finalize_ms;

            let path_start = Instant::now();
            let path_cfg = build_xun_path_config(small_cluster_polygon_area);
            let (paths, xun_path_stats) =
                build_paths_with_xun_components(&mut components, path_cfg);
            timings.vc_path_build_ms = path_start.elapsed().as_millis() as u64;
            timings.vc_path_sort_ms = xun_path_stats.sort_ms;
            timings.vc_path_trace_ms = xun_path_stats.trace_ms;
            timings.vc_path_smooth_ms = xun_path_stats.smooth_ms;
            timings.vc_path_svg_emit_ms = xun_path_stats.svg_emit_ms;
            timings.vc_path_components_total = xun_path_stats.components_total;
            timings.vc_path_components_simplified = xun_path_stats.components_simplified;
            timings.vc_path_components_smoothed = xun_path_stats.components_smoothed;
            paths
        }
    };

    let wrap_start = Instant::now();
    let svg = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">\n\
         {paths}</svg>\n"
    );
    timings.vc_svg_wrap_ms = wrap_start.elapsed().as_millis() as u64;
    timings.trace_total_ms = total_start.elapsed().as_millis() as u64;

    Ok(SvgTraceResult { svg, timings })
}
