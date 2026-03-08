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

mod backend;
mod cluster;
mod cluster_merge;
mod convert;
mod options;
mod pipeline;

pub fn trace_with_timings(img: &DynamicImage) -> anyhow::Result<SvgTraceResult> {
    pipeline::trace_with_timings(img)
}
