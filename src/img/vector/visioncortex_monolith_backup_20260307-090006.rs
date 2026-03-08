use super::{SvgTraceResult, SvgTraceTimingsMs};
use anyhow::anyhow;
use image::DynamicImage;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::time::{Duration, Instant};
use visioncortex::{
    BinaryImage, Color, ColorImage, ColorName, PathSimplifyMode, PointF64, PointI32,
    clusters::Cluster as BinaryCluster,
    color_clusters::{
        Clusters as VcColorClusters, HIERARCHICAL_MAX, KeyingAction, Runner, RunnerConfig,
    },
};

const KEYING_THRESHOLD: f32 = 0.2;
const ULTRA_LARGE_IMAGE_PX: usize = 4 * 1024 * 1024;
const LARGE_IMAGE_PX: usize = 512 * 512;
const LARGE_BATCH_SIZE: i32 = 65_536;
const DEFAULT_BATCH_SIZE: i32 = 25_600;
const SMALL_CLUSTER_POLYGON_AREA: usize = 24;
const COMPLEX_CLUSTER_MIN_AREA: usize = 5;
const COMPLEX_DEEPEN_DIFF: i32 = 20;
const COMPLEX_IS_SAME_SHIFT: i32 = 2;
const COMPLEX_POLYGON_AREA: usize = 32;
const COMPLEX_SAMPLE_TARGET: usize = 65_536;
const GRAY_DELTA_THRESHOLD: i32 = 6;
const COMPLEX_UNIQUE_5BIT_MIN: usize = 1_024;
const COMPLEX_COLORFUL_RATIO_MAX: f64 = 0.80;

const XUN_CLUSTER_ENV: &str = "XUN_VC_CLUSTER_BACKEND";
const XUN_QUANT_SHIFT_ENV: &str = "XUN_VC_QUANT_SHIFT";
const XUN_QUANT_SHIFT_SIMPLE_ENV: &str = "XUN_VC_QUANT_SHIFT_SIMPLE";
const XUN_QUANT_SHIFT_COMPLEX_ENV: &str = "XUN_VC_QUANT_SHIFT_COMPLEX";
const XUN_MIN_COMPONENT_AREA_ENV: &str = "XUN_VC_MIN_COMPONENT_AREA";
const XUN_MIN_COMPONENT_AREA_SIMPLE_ENV: &str = "XUN_VC_MIN_COMPONENT_AREA_SIMPLE";
const XUN_MIN_COMPONENT_AREA_COMPLEX_ENV: &str = "XUN_VC_MIN_COMPONENT_AREA_COMPLEX";
const XUN_MAX_COMPONENTS_ENV: &str = "XUN_VC_MAX_COMPONENTS";
const XUN_MAX_COMPONENTS_SIMPLE_ENV: &str = "XUN_VC_MAX_COMPONENTS_SIMPLE";
const XUN_MAX_COMPONENTS_COMPLEX_ENV: &str = "XUN_VC_MAX_COMPONENTS_COMPLEX";
const XUN_MERGE_SMALL_AREA_ENV: &str = "XUN_VC_MERGE_SMALL_AREA";
const XUN_MERGE_SMALL_AREA_SIMPLE_ENV: &str = "XUN_VC_MERGE_SMALL_AREA_SIMPLE";
const XUN_MERGE_SMALL_AREA_COMPLEX_ENV: &str = "XUN_VC_MERGE_SMALL_AREA_COMPLEX";
const XUN_MERGE_COLOR_DELTA_ENV: &str = "XUN_VC_MERGE_COLOR_DELTA";
const XUN_MERGE_COLOR_DELTA_SIMPLE_ENV: &str = "XUN_VC_MERGE_COLOR_DELTA_SIMPLE";
const XUN_MERGE_COLOR_DELTA_COMPLEX_ENV: &str = "XUN_VC_MERGE_COLOR_DELTA_COMPLEX";
const XUN_PATH_MODE_ENV: &str = "XUN_VC_PATH_MODE";
const XUN_PATH_POLYGON_AREA_ENV: &str = "XUN_VC_PATH_POLYGON_AREA";
const XUN_CORNER_THRESHOLD_DEG_ENV: &str = "XUN_VC_CORNER_THRESHOLD_DEG";
const XUN_SEGMENT_LENGTH_ENV: &str = "XUN_VC_SEGMENT_LENGTH";
const XUN_MAX_ITERATIONS_ENV: &str = "XUN_VC_MAX_ITERATIONS";
const XUN_SPLICE_THRESHOLD_DEG_ENV: &str = "XUN_VC_SPLICE_THRESHOLD_DEG";
const XUN_PATH_SMOOTH_MIN_AREA_ENV: &str = "XUN_VC_PATH_SMOOTH_MIN_AREA";
const XUN_MIN_COMPONENT_AREA_DEFAULT: usize = 1;
const XUN_MIN_COMPONENT_AREA_COMPLEX: usize = 1;
const XUN_QUANT_SHIFT_SIMPLE: u8 = 1;
const XUN_QUANT_SHIFT_COMPLEX: u8 = 3;
const XUN_MERGE_SMALL_AREA_SIMPLE: usize = 1;
const XUN_MERGE_SMALL_AREA_COMPLEX: usize = 1;
const XUN_MERGE_COLOR_DELTA_SIMPLE: i32 = 18;
const XUN_MERGE_COLOR_DELTA_COMPLEX: i32 = 32;
const XUN_CORNER_THRESHOLD_DEG_DEFAULT: f64 = 60.0;
const XUN_SEGMENT_LENGTH_DEFAULT: f64 = 4.0;
const XUN_MAX_ITERATIONS_DEFAULT: usize = 10;
const XUN_SPLICE_THRESHOLD_DEG_DEFAULT: f64 = 45.0;
const XUN_PATH_SMOOTH_MIN_AREA_DEFAULT: usize = 128;
const XUN_OUTSET_RATIO_DEFAULT: f64 = 8.0;
const LABEL_NONE: usize = 0;

#[derive(Debug, Clone, Copy, Default)]
struct ComplexityProfile {
    high_color_complexity: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClusterBackend {
    Xun,
    Official,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XunPathMode {
    None,
    Smooth,
    HybridSmooth,
    HybridSpline,
    Polygon,
    Spline,
    Adaptive,
}

#[derive(Debug, Clone, Copy)]
struct PathBuildConfig {
    mode: XunPathMode,
    polygon_area: usize,
    smooth_min_area: usize,
    corner_threshold_rad: f64,
    segment_length: f64,
    max_iterations: usize,
    splice_threshold_rad: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct XunClusterBuildStats {
    quantize_ms: u64,
    label_ms: u64,
    stats_ms: u64,
    merge_ms: u64,
    finalize_ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct XunPathBuildStats {
    sort_ms: u64,
    trace_ms: u64,
    smooth_ms: u64,
    svg_emit_ms: u64,
    components_total: u64,
    components_simplified: u64,
    components_smoothed: u64,
}

#[derive(Debug, Clone, Copy)]
struct XunClusterConfig {
    quant_shift: u8,
    min_component_area: usize,
    max_components: Option<usize>,
    merge_small_area: usize,
    merge_color_delta: i32,
}

#[derive(Debug, Clone)]
struct RegionStat {
    area: usize,
    sum_r: u64,
    sum_g: u64,
    sum_b: u64,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

#[derive(Debug)]
struct XunComponent {
    area: usize,
    color: Color,
    origin_x: usize,
    origin_y: usize,
    mask: BinaryImage,
}

#[derive(Debug, Default)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn with_capacity(capacity: usize) -> Self {
        let mut parent = Vec::with_capacity(capacity.max(1024));
        let mut rank = Vec::with_capacity(capacity.max(1024));
        parent.push(0);
        rank.push(0);
        Self { parent, rank }
    }

    fn make_set(&mut self) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.rank.push(0);
        id
    }

    fn find(&mut self, x: usize) -> usize {
        let parent = self.parent[x];
        if parent != x {
            let root = self.find(parent);
            self.parent[x] = root;
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) -> usize {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return ra;
        }
        if self.rank[ra] < self.rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        if self.rank[ra] == self.rank[rb] {
            self.rank[ra] = self.rank[ra].saturating_add(1);
        }
        ra
    }
}

pub fn trace_with_timings(img: &DynamicImage) -> anyhow::Result<SvgTraceResult> {
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

fn to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    ColorImage {
        pixels: rgba.into_raw(),
        width: w,
        height: h,
    }
}

fn should_key(img: &ColorImage) -> bool {
    if img.width == 0 || img.height == 0 {
        return false;
    }
    let thresh = ((img.width * 2) as f32 * KEYING_THRESHOLD) as usize;
    let mut cnt = 0usize;
    for y in [
        0,
        img.height / 4,
        img.height / 2,
        3 * img.height / 4,
        img.height.saturating_sub(1),
    ] {
        let row_start = y * img.width * 4;
        for x in 0..img.width {
            let a = img.pixels[row_start + x * 4 + 3];
            if a == 0 {
                cnt += 1;
                if cnt >= thresh {
                    return true;
                }
            }
        }
    }
    false
}

fn cluster_backend_from_env() -> ClusterBackend {
    let backend = std::env::var(XUN_CLUSTER_ENV)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match backend.as_str() {
        "xun" => ClusterBackend::Xun,
        _ => ClusterBackend::Official,
    }
}

fn build_runner_config(
    width: usize,
    height: usize,
    key_color: Color,
    complexity: ComplexityProfile,
) -> RunnerConfig {
    let mut cfg = RunnerConfig::default();
    cfg.diagonal = false;
    cfg.hierarchical = HIERARCHICAL_MAX;
    let pixels = width.saturating_mul(height);
    cfg.batch_size = if pixels >= LARGE_IMAGE_PX {
        LARGE_BATCH_SIZE
    } else {
        DEFAULT_BATCH_SIZE
    };
    cfg.good_min_area = 4;
    cfg.good_max_area = width.saturating_mul(height);
    cfg.is_same_color_a = 2;
    cfg.is_same_color_b = 1;
    cfg.deepen_diff = 16;
    cfg.hollow_neighbours = 1;
    cfg.key_color = key_color;
    cfg.keying_action = KeyingAction::Discard;

    if complexity.high_color_complexity && pixels >= LARGE_IMAGE_PX {
        cfg.good_min_area = cfg.good_min_area.max(COMPLEX_CLUSTER_MIN_AREA);
        cfg.is_same_color_a = COMPLEX_IS_SAME_SHIFT;
        cfg.deepen_diff = COMPLEX_DEEPEN_DIFF;
    }

    if pixels >= ULTRA_LARGE_IMAGE_PX {
        cfg.good_min_area = 8;
        cfg.is_same_color_a = 3;
        cfg.deepen_diff = 32;
    }
    cfg
}

fn build_xun_cluster_config(pixels: usize, complexity: ComplexityProfile) -> XunClusterConfig {
    let is_complex_large = complexity.high_color_complexity && pixels >= LARGE_IMAGE_PX;
    let mut cfg = XunClusterConfig {
        quant_shift: if is_complex_large {
            XUN_QUANT_SHIFT_COMPLEX
        } else {
            XUN_QUANT_SHIFT_SIMPLE
        },
        min_component_area: if is_complex_large {
            XUN_MIN_COMPONENT_AREA_COMPLEX
        } else {
            XUN_MIN_COMPONENT_AREA_DEFAULT
        },
        max_components: None,
        merge_small_area: if is_complex_large {
            XUN_MERGE_SMALL_AREA_COMPLEX
        } else {
            XUN_MERGE_SMALL_AREA_SIMPLE
        },
        merge_color_delta: if is_complex_large {
            XUN_MERGE_COLOR_DELTA_COMPLEX
        } else {
            XUN_MERGE_COLOR_DELTA_SIMPLE
        },
    };
    apply_xun_env_overrides(&mut cfg, is_complex_large);
    cfg
}

fn apply_xun_env_overrides(cfg: &mut XunClusterConfig, is_complex_large: bool) {
    if let Some(v) = env_u8(XUN_QUANT_SHIFT_ENV) {
        cfg.quant_shift = v.min(7);
    }
    if let Some(v) = env_usize(XUN_MIN_COMPONENT_AREA_ENV) {
        cfg.min_component_area = v;
    }
    if let Some(v) = env_opt_usize(XUN_MAX_COMPONENTS_ENV) {
        cfg.max_components = v;
    }
    if let Some(v) = env_usize(XUN_MERGE_SMALL_AREA_ENV) {
        cfg.merge_small_area = v;
    }
    if let Some(v) = env_i32(XUN_MERGE_COLOR_DELTA_ENV) {
        cfg.merge_color_delta = v.max(0);
    }

    if is_complex_large {
        if let Some(v) = env_u8(XUN_QUANT_SHIFT_COMPLEX_ENV) {
            cfg.quant_shift = v.min(7);
        }
        if let Some(v) = env_usize(XUN_MIN_COMPONENT_AREA_COMPLEX_ENV) {
            cfg.min_component_area = v;
        }
        if let Some(v) = env_opt_usize(XUN_MAX_COMPONENTS_COMPLEX_ENV) {
            cfg.max_components = v;
        }
        if let Some(v) = env_usize(XUN_MERGE_SMALL_AREA_COMPLEX_ENV) {
            cfg.merge_small_area = v;
        }
        if let Some(v) = env_i32(XUN_MERGE_COLOR_DELTA_COMPLEX_ENV) {
            cfg.merge_color_delta = v.max(0);
        }
    } else {
        if let Some(v) = env_u8(XUN_QUANT_SHIFT_SIMPLE_ENV) {
            cfg.quant_shift = v.min(7);
        }
        if let Some(v) = env_usize(XUN_MIN_COMPONENT_AREA_SIMPLE_ENV) {
            cfg.min_component_area = v;
        }
        if let Some(v) = env_opt_usize(XUN_MAX_COMPONENTS_SIMPLE_ENV) {
            cfg.max_components = v;
        }
        if let Some(v) = env_usize(XUN_MERGE_SMALL_AREA_SIMPLE_ENV) {
            cfg.merge_small_area = v;
        }
        if let Some(v) = env_i32(XUN_MERGE_COLOR_DELTA_SIMPLE_ENV) {
            cfg.merge_color_delta = v.max(0);
        }
    }
}

fn env_u8(key: &str) -> Option<u8> {
    std::env::var(key).ok()?.trim().parse::<u8>().ok()
}

fn env_i32(key: &str) -> Option<i32> {
    std::env::var(key).ok()?.trim().parse::<i32>().ok()
}

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok()?.trim().parse::<usize>().ok()
}

fn env_opt_usize(key: &str) -> Option<Option<usize>> {
    let raw = std::env::var(key).ok()?;
    let parsed = raw.trim().parse::<usize>().ok()?;
    if parsed == 0 {
        Some(None)
    } else {
        Some(Some(parsed))
    }
}

fn env_f64(key: &str) -> Option<f64> {
    std::env::var(key).ok()?.trim().parse::<f64>().ok()
}

fn xun_path_mode_from_env() -> XunPathMode {
    match std::env::var(XUN_PATH_MODE_ENV) {
        Ok(v) if v.eq_ignore_ascii_case("none") => XunPathMode::None,
        Ok(v) if v.eq_ignore_ascii_case("smooth") => XunPathMode::Smooth,
        Ok(v) if v.eq_ignore_ascii_case("hybrid_smooth") || v.eq_ignore_ascii_case("hybrid") => {
            XunPathMode::HybridSmooth
        }
        Ok(v) if v.eq_ignore_ascii_case("hybrid_spline") => XunPathMode::HybridSpline,
        Ok(v) if v.eq_ignore_ascii_case("polygon") => XunPathMode::Polygon,
        Ok(v) if v.eq_ignore_ascii_case("spline") => XunPathMode::Spline,
        Ok(v) if v.eq_ignore_ascii_case("adaptive") => XunPathMode::Adaptive,
        _ => XunPathMode::None,
    }
}

fn build_xun_path_config(default_polygon_area: usize) -> PathBuildConfig {
    let polygon_area = env_usize(XUN_PATH_POLYGON_AREA_ENV).unwrap_or(default_polygon_area);
    let smooth_min_area =
        env_usize(XUN_PATH_SMOOTH_MIN_AREA_ENV).unwrap_or(XUN_PATH_SMOOTH_MIN_AREA_DEFAULT);
    let corner_threshold_deg = env_f64(XUN_CORNER_THRESHOLD_DEG_ENV)
        .unwrap_or(XUN_CORNER_THRESHOLD_DEG_DEFAULT)
        .clamp(0.0, 180.0);
    let segment_length = env_f64(XUN_SEGMENT_LENGTH_ENV)
        .unwrap_or(XUN_SEGMENT_LENGTH_DEFAULT)
        .max(0.5);
    let max_iterations = env_usize(XUN_MAX_ITERATIONS_ENV)
        .unwrap_or(XUN_MAX_ITERATIONS_DEFAULT)
        .max(1);
    let splice_threshold_deg = env_f64(XUN_SPLICE_THRESHOLD_DEG_ENV)
        .unwrap_or(XUN_SPLICE_THRESHOLD_DEG_DEFAULT)
        .clamp(0.0, 180.0);

    PathBuildConfig {
        mode: xun_path_mode_from_env(),
        polygon_area,
        smooth_min_area,
        corner_threshold_rad: corner_threshold_deg.to_radians(),
        segment_length,
        max_iterations,
        splice_threshold_rad: splice_threshold_deg.to_radians(),
    }
}

fn build_paths_with_official_clusters(
    clusters: &VcColorClusters,
    small_cluster_polygon_area: usize,
) -> String {
    let view = clusters.view();
    let mut paths = String::with_capacity(view.clusters_output.len() * 64);
    for &ci in view.clusters_output.iter().rev() {
        let cluster = view.get_cluster(ci);
        let simplify_mode = if cluster.area() <= small_cluster_polygon_area {
            PathSimplifyMode::Polygon
        } else {
            PathSimplifyMode::Spline
        };
        let compound = cluster.to_compound_path(
            &view,
            false,
            simplify_mode,
            60.0_f64.to_radians(),
            4.0,
            10,
            45.0_f64.to_radians(),
        );
        let color = cluster.residue_color();
        let (s, offset) = compound.to_svg_string(true, PointF64::default(), Some(3));
        let _ = writeln!(
            &mut paths,
            "<path d=\"{s}\" fill=\"{}\" transform=\"translate({},{})\"/>",
            color.to_hex_string(),
            offset.x,
            offset.y
        );
    }
    paths
}

fn build_paths_with_xun_components(
    components: &mut [XunComponent],
    cfg: PathBuildConfig,
) -> (String, XunPathBuildStats) {
    let mut stats = XunPathBuildStats::default();
    stats.components_total = components.len() as u64;

    let sort_start = Instant::now();
    components.sort_by_key(|c| Reverse(c.area));
    stats.sort_ms = sort_start.elapsed().as_millis() as u64;

    let mut paths = String::with_capacity(components.len() * 64);
    let mut trace_duration = Duration::default();
    let mut smooth_duration = Duration::default();
    let mut emit_duration = Duration::default();
    for comp in components.iter() {
        let (simplify_mode, post_smooth) = match cfg.mode {
            XunPathMode::None => (PathSimplifyMode::None, false),
            XunPathMode::Smooth => (PathSimplifyMode::None, true),
            XunPathMode::HybridSmooth => (PathSimplifyMode::None, comp.area >= cfg.smooth_min_area),
            XunPathMode::HybridSpline => {
                if comp.area >= cfg.smooth_min_area {
                    (PathSimplifyMode::Spline, false)
                } else {
                    (PathSimplifyMode::None, false)
                }
            }
            XunPathMode::Polygon => (PathSimplifyMode::Polygon, false),
            XunPathMode::Spline => (PathSimplifyMode::Spline, false),
            XunPathMode::Adaptive => {
                if comp.area <= cfg.polygon_area {
                    (PathSimplifyMode::Polygon, false)
                } else {
                    (PathSimplifyMode::Spline, false)
                }
            }
        };
        if !matches!(simplify_mode, PathSimplifyMode::None) {
            stats.components_simplified = stats.components_simplified.saturating_add(1);
        }

        let trace_start = Instant::now();
        let mut compound = BinaryCluster::image_to_compound_path(
            &PointI32 {
                x: comp.origin_x as i32,
                y: comp.origin_y as i32,
            },
            &comp.mask,
            simplify_mode,
            cfg.corner_threshold_rad,
            cfg.segment_length,
            cfg.max_iterations,
            cfg.splice_threshold_rad,
        );
        trace_duration += trace_start.elapsed();

        if post_smooth {
            let smooth_start = Instant::now();
            compound = compound.smooth(
                cfg.corner_threshold_rad,
                XUN_OUTSET_RATIO_DEFAULT,
                cfg.segment_length,
            );
            smooth_duration += smooth_start.elapsed();
            stats.components_smoothed = stats.components_smoothed.saturating_add(1);
        }

        let emit_start = Instant::now();
        let (s, offset) = compound.to_svg_string(true, PointF64::default(), Some(3));
        let _ = writeln!(
            &mut paths,
            "<path d=\"{s}\" fill=\"{}\" transform=\"translate({},{})\"/>",
            comp.color.to_hex_string(),
            offset.x,
            offset.y
        );
        emit_duration += emit_start.elapsed();
    }
    stats.trace_ms = (trace_duration.as_micros() / 1_000) as u64;
    stats.smooth_ms = (smooth_duration.as_micros() / 1_000) as u64;
    stats.svg_emit_ms = (emit_duration.as_micros() / 1_000) as u64;
    (paths, stats)
}

fn build_xun_components(
    img: &ColorImage,
    key_color: Color,
    cfg: XunClusterConfig,
) -> (Vec<XunComponent>, XunClusterBuildStats) {
    let mut timings = XunClusterBuildStats::default();
    let width = img.width;
    let height = img.height;
    let pixel_count = width.saturating_mul(height);
    if pixel_count == 0 {
        return (Vec::new(), timings);
    }

    let quantize_start = Instant::now();
    let has_key = key_color != Color::default();
    let mut bins = vec![u32::MAX; pixel_count];
    for (idx, px) in img.pixels.chunks_exact(4).enumerate() {
        let r = px[0];
        let g = px[1];
        let b = px[2];
        if has_key && r == key_color.r && g == key_color.g && b == key_color.b {
            continue;
        }
        bins[idx] = quantized_color_key(r, g, b, cfg.quant_shift);
    }
    timings.quantize_ms = quantize_start.elapsed().as_millis() as u64;

    let label_start = Instant::now();
    let mut labels = vec![LABEL_NONE; pixel_count];
    let mut uf = UnionFind::with_capacity(pixel_count / 8 + 1024);

    for y in 0..height {
        let row_start = y * width;
        for x in 0..width {
            let idx = row_start + x;
            let key = bins[idx];
            if key == u32::MAX {
                continue;
            }

            let mut left = LABEL_NONE;
            if x > 0 {
                let li = idx - 1;
                if bins[li] == key {
                    left = labels[li];
                }
            }

            let mut up = LABEL_NONE;
            if y > 0 {
                let ui = idx - width;
                if bins[ui] == key {
                    up = labels[ui];
                }
            }

            let label = match (left, up) {
                (LABEL_NONE, LABEL_NONE) => uf.make_set(),
                (l, LABEL_NONE) if l != LABEL_NONE => l,
                (LABEL_NONE, u) if u != LABEL_NONE => u,
                (l, u) if l == u => l,
                (l, u) => uf.union(l, u),
            };
            labels[idx] = label;
        }
    }
    timings.label_ms = label_start.elapsed().as_millis() as u64;

    let stats_start = Instant::now();
    let mut root_to_region: HashMap<usize, usize> = HashMap::new();
    let mut stats: Vec<RegionStat> = Vec::new();

    for idx in 0..pixel_count {
        let label = labels[idx];
        if label == LABEL_NONE {
            continue;
        }
        let root = uf.find(label);
        let region_id = *root_to_region.entry(root).or_insert_with(|| {
            stats.push(RegionStat {
                area: 0,
                sum_r: 0,
                sum_g: 0,
                sum_b: 0,
                min_x: usize::MAX,
                min_y: usize::MAX,
                max_x: 0,
                max_y: 0,
            });
            stats.len() - 1
        });

        labels[idx] = region_id + 1;

        let x = idx % width;
        let y = idx / width;
        let px = idx * 4;
        let r = u64::from(img.pixels[px]);
        let g = u64::from(img.pixels[px + 1]);
        let b = u64::from(img.pixels[px + 2]);
        let stat = &mut stats[region_id];
        stat.area += 1;
        stat.sum_r += r;
        stat.sum_g += g;
        stat.sum_b += b;
        stat.min_x = stat.min_x.min(x);
        stat.min_y = stat.min_y.min(y);
        stat.max_x = stat.max_x.max(x);
        stat.max_y = stat.max_y.max(y);
    }
    timings.stats_ms = stats_start.elapsed().as_millis() as u64;

    if stats.is_empty() {
        return (Vec::new(), timings);
    }

    let merge_start = Instant::now();
    if cfg.merge_small_area > 0 && cfg.merge_color_delta >= 0 {
        stats = merge_small_regions(
            &mut labels,
            width,
            height,
            img,
            &stats,
            cfg.merge_small_area,
            cfg.merge_color_delta,
        );
    }
    timings.merge_ms = merge_start.elapsed().as_millis() as u64;

    let finalize_start = Instant::now();
    let mut kept_regions: Vec<usize> = stats
        .iter()
        .enumerate()
        .filter_map(|(idx, s)| (s.area >= cfg.min_component_area).then_some(idx))
        .collect();

    if let Some(max_components) = cfg.max_components {
        if kept_regions.len() > max_components {
            kept_regions.sort_by_key(|&idx| Reverse(stats[idx].area));
            kept_regions.truncate(max_components);
        }
    }

    if kept_regions.is_empty() {
        timings.finalize_ms = finalize_start.elapsed().as_millis() as u64;
        return (Vec::new(), timings);
    }

    let mut region_to_comp = vec![usize::MAX; stats.len()];
    let mut components = Vec::with_capacity(kept_regions.len());
    for &region in &kept_regions {
        let s = &stats[region];
        let area_u64 = s.area as u64;
        let color = Color::new(
            (s.sum_r / area_u64) as u8,
            (s.sum_g / area_u64) as u8,
            (s.sum_b / area_u64) as u8,
        );
        let mask_w = s.max_x - s.min_x + 1;
        let mask_h = s.max_y - s.min_y + 1;
        region_to_comp[region] = components.len();
        components.push(XunComponent {
            area: s.area,
            color,
            origin_x: s.min_x,
            origin_y: s.min_y,
            mask: BinaryImage::new_w_h(mask_w, mask_h),
        });
    }

    for idx in 0..pixel_count {
        let compact = labels[idx];
        if compact == LABEL_NONE {
            continue;
        }
        let region_id = compact - 1;
        let comp_idx = region_to_comp[region_id];
        if comp_idx == usize::MAX {
            continue;
        }
        let x = idx % width;
        let y = idx / width;
        let comp = &mut components[comp_idx];
        comp.mask
            .set_pixel(x - comp.origin_x, y - comp.origin_y, true);
    }
    timings.finalize_ms = finalize_start.elapsed().as_millis() as u64;
    (components, timings)
}

fn merge_small_regions(
    labels: &mut [usize],
    width: usize,
    height: usize,
    img: &ColorImage,
    stats: &[RegionStat],
    merge_small_area: usize,
    merge_color_delta: i32,
) -> Vec<RegionStat> {
    let region_count = stats.len();
    if region_count == 0 || merge_small_area == 0 {
        return stats.to_vec();
    }

    let mut adjacency: Vec<HashSet<usize>> = (0..region_count).map(|_| HashSet::new()).collect();
    for y in 0..height {
        let row_start = y * width;
        for x in 0..width {
            let idx = row_start + x;
            let a = labels[idx];
            if a == LABEL_NONE {
                continue;
            }
            let ra = a - 1;
            if x + 1 < width {
                let b = labels[idx + 1];
                if b != LABEL_NONE && b != a {
                    let rb = b - 1;
                    adjacency[ra].insert(rb);
                    adjacency[rb].insert(ra);
                }
            }
            if y + 1 < height {
                let b = labels[idx + width];
                if b != LABEL_NONE && b != a {
                    let rb = b - 1;
                    adjacency[ra].insert(rb);
                    adjacency[rb].insert(ra);
                }
            }
        }
    }

    let mut parent: Vec<usize> = (0..region_count).collect();
    let mut area: Vec<usize> = stats.iter().map(|s| s.area).collect();
    let mut sum_r: Vec<u64> = stats.iter().map(|s| s.sum_r).collect();
    let mut sum_g: Vec<u64> = stats.iter().map(|s| s.sum_g).collect();
    let mut sum_b: Vec<u64> = stats.iter().map(|s| s.sum_b).collect();

    let mut order: Vec<usize> = (0..region_count).collect();
    order.sort_by_key(|&idx| stats[idx].area);

    for region in order {
        let root = merge_find_root(&mut parent, region);
        if root != region || area[root] == 0 || area[root] > merge_small_area {
            continue;
        }
        if adjacency[region].is_empty() {
            continue;
        }

        let (r0, g0, b0) = region_mean_rgb(area[root], sum_r[root], sum_g[root], sum_b[root]);
        let mut best_target = usize::MAX;
        let mut best_delta = i32::MAX;
        for &nb in &adjacency[region] {
            let nr = merge_find_root(&mut parent, nb);
            if nr == root || area[nr] == 0 {
                continue;
            }
            let (r1, g1, b1) = region_mean_rgb(area[nr], sum_r[nr], sum_g[nr], sum_b[nr]);
            let delta = (r0 - r1).abs() + (g0 - g1).abs() + (b0 - b1).abs();
            if delta < best_delta {
                best_delta = delta;
                best_target = nr;
            }
        }

        if best_target == usize::MAX || best_delta > merge_color_delta {
            continue;
        }

        parent[root] = best_target;
        area[best_target] = area[best_target].saturating_add(area[root]);
        sum_r[best_target] = sum_r[best_target].saturating_add(sum_r[root]);
        sum_g[best_target] = sum_g[best_target].saturating_add(sum_g[root]);
        sum_b[best_target] = sum_b[best_target].saturating_add(sum_b[root]);

        area[root] = 0;
        sum_r[root] = 0;
        sum_g[root] = 0;
        sum_b[root] = 0;
    }

    let mut root_to_new: HashMap<usize, usize> = HashMap::new();
    let mut merged_stats: Vec<RegionStat> = Vec::new();

    for idx in 0..labels.len() {
        let label = labels[idx];
        if label == LABEL_NONE {
            continue;
        }
        let region = label - 1;
        let root = merge_find_root(&mut parent, region);
        let new_id = *root_to_new.entry(root).or_insert_with(|| {
            merged_stats.push(RegionStat {
                area: 0,
                sum_r: 0,
                sum_g: 0,
                sum_b: 0,
                min_x: usize::MAX,
                min_y: usize::MAX,
                max_x: 0,
                max_y: 0,
            });
            merged_stats.len() - 1
        });
        labels[idx] = new_id + 1;

        let x = idx % width;
        let y = idx / width;
        let base = idx * 4;
        let stat = &mut merged_stats[new_id];
        stat.area += 1;
        stat.sum_r += u64::from(img.pixels[base]);
        stat.sum_g += u64::from(img.pixels[base + 1]);
        stat.sum_b += u64::from(img.pixels[base + 2]);
        stat.min_x = stat.min_x.min(x);
        stat.min_y = stat.min_y.min(y);
        stat.max_x = stat.max_x.max(x);
        stat.max_y = stat.max_y.max(y);
    }

    merged_stats
}

fn merge_find_root(parent: &mut [usize], idx: usize) -> usize {
    let p = parent[idx];
    if p != idx {
        let r = merge_find_root(parent, p);
        parent[idx] = r;
        r
    } else {
        idx
    }
}

fn region_mean_rgb(area: usize, sum_r: u64, sum_g: u64, sum_b: u64) -> (i32, i32, i32) {
    if area == 0 {
        return (0, 0, 0);
    }
    let d = area as u64;
    ((sum_r / d) as i32, (sum_g / d) as i32, (sum_b / d) as i32)
}

fn quantized_color_key(r: u8, g: u8, b: u8, shift: u8) -> u32 {
    (u32::from(r >> shift) << 16) | (u32::from(g >> shift) << 8) | u32::from(b >> shift)
}

fn apply_key_to_transparent(img: &mut ColorImage, key: Color) {
    for px in img.pixels.chunks_exact_mut(4) {
        if px[3] == 0 {
            px[0] = key.r;
            px[1] = key.g;
            px[2] = key.b;
            px[3] = 255;
        }
    }
}

fn find_key_color(img: &ColorImage) -> Option<Color> {
    let mut used_colors = HashSet::with_capacity((img.width * img.height).min(1_000_000));
    for px in img.pixels.chunks_exact(4) {
        used_colors.insert(rgb_key(px[0], px[1], px[2]));
    }

    for c in [
        Color::new(255, 0, 0),
        Color::new(0, 255, 0),
        Color::new(0, 0, 255),
        Color::new(255, 255, 0),
        Color::new(0, 255, 255),
        Color::new(255, 0, 255),
        Color::color(&ColorName::White),
    ] {
        if !color_in_set(&used_colors, &c) {
            return Some(c);
        }
    }
    for v in 0u8..=254 {
        let c = Color::new(v, v, v);
        if !color_in_set(&used_colors, &c) {
            return Some(c);
        }
    }
    None
}

fn analyze_complexity(img: &ColorImage) -> ComplexityProfile {
    let pixels = img.width.saturating_mul(img.height);
    if pixels == 0 {
        return ComplexityProfile::default();
    }

    let stride = sampling_stride(pixels, COMPLEX_SAMPLE_TARGET);
    let mut unique_5bit = HashSet::new();
    let mut sampled = 0usize;
    let mut near_gray = 0usize;

    let total_px = img.pixels.len() / 4;
    let mut i = 0usize;
    while i < total_px {
        let base = i * 4;
        let r = img.pixels[base];
        let g = img.pixels[base + 1];
        let b = img.pixels[base + 2];

        unique_5bit.insert(rgb5_key(r, g, b));
        sampled += 1;

        let maxc = i32::from(r.max(g).max(b));
        let minc = i32::from(r.min(g).min(b));
        if maxc - minc <= GRAY_DELTA_THRESHOLD {
            near_gray += 1;
        }

        i = i.saturating_add(stride);
    }

    let gray_ratio = if sampled > 0 {
        near_gray as f64 / sampled as f64
    } else {
        1.0
    };

    ComplexityProfile {
        high_color_complexity: unique_5bit.len() >= COMPLEX_UNIQUE_5BIT_MIN
            && gray_ratio < COMPLEX_COLORFUL_RATIO_MAX,
    }
}

fn sampling_stride(pixels: usize, target_samples: usize) -> usize {
    if pixels <= target_samples {
        1
    } else {
        (pixels / target_samples).max(1)
    }
}

fn rgb5_key(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r >> 3) << 10) | (u32::from(g >> 3) << 5) | u32::from(b >> 3)
}

fn rgb_key(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

fn color_in_set(set: &HashSet<u32>, c: &Color) -> bool {
    set.contains(&rgb_key(c.r, c.g, c.b))
}
