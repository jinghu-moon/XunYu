use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Svg,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "jpeg" | "jpg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Svg => "svg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgMethod {
    Bezier,
    Visioncortex,
    Potrace,
    Skeleton,
    Diffvg,
}

impl SvgMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "bezier" => Some(Self::Bezier),
            "visioncortex" | "vc" => Some(Self::Visioncortex),
            "potrace" => Some(Self::Potrace),
            "skeleton" => Some(Self::Skeleton),
            "diffvg" => Some(Self::Diffvg),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegBackend {
    Auto,
    Moz,
    Turbo,
}

impl JpegBackend {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "moz" | "mozjpeg" => Some(Self::Moz),
            "turbo" | "turbojpeg" => Some(Self::Turbo),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Moz => "moz",
            Self::Turbo => "turbo",
        }
    }

    pub fn is_compiled(self) -> bool {
        match self {
            Self::Auto => cfg!(any(feature = "img-moz", feature = "img-turbo")),
            Self::Moz => cfg!(feature = "img-moz"),
            Self::Turbo => cfg!(feature = "img-turbo"),
        }
    }

    pub fn available_for_cli() -> &'static str {
        if cfg!(all(feature = "img-moz", feature = "img-turbo")) {
            "auto / moz / turbo"
        } else if cfg!(feature = "img-moz") {
            "auto / moz"
        } else if cfg!(feature = "img-turbo") {
            "auto / turbo"
        } else {
            "无（请启用 img-moz 或 img-turbo）"
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessParams {
    pub format: OutputFormat,
    pub svg_method: SvgMethod,
    pub svg_diffvg_iters: usize,
    pub svg_diffvg_strokes: usize,
    pub jpeg_backend: JpegBackend,
    pub quality: u8,
    pub png_lossy: bool,
    pub png_dither_level: f32,
    pub webp_lossy: bool,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub avif_threads: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub elapsed_ms: u64,
    pub stage_ms: StageDurationsMs,
    pub skipped: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StageDurationsMs {
    pub read_ms: u64,
    pub decode_ms: u64,
    pub resize_ms: u64,
    pub pixel_convert_ms: u64,
    pub encode_pre_ms: u64,
    pub codec_ms: u64,
    pub write_ms: u64,
    pub png_optimize_ms: u64,
    pub svg_trace_ms: u64,
    pub svg_serialize_ms: u64,
    pub svg_trace_internal_ms: u64,
    pub svg_vc_to_color_ms: u64,
    pub svg_vc_keying_ms: u64,
    pub svg_vc_cluster_ms: u64,
    pub svg_vc_cluster_quantize_ms: u64,
    pub svg_vc_cluster_label_ms: u64,
    pub svg_vc_cluster_stats_ms: u64,
    pub svg_vc_cluster_merge_ms: u64,
    pub svg_vc_cluster_finalize_ms: u64,
    pub svg_vc_path_build_ms: u64,
    pub svg_vc_path_sort_ms: u64,
    pub svg_vc_path_trace_ms: u64,
    pub svg_vc_path_smooth_ms: u64,
    pub svg_vc_path_svg_emit_ms: u64,
    pub svg_vc_path_components_total: u64,
    pub svg_vc_path_components_simplified: u64,
    pub svg_vc_path_components_smoothed: u64,
    pub svg_vc_wrap_ms: u64,
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];

    let mut val = bytes as f64;
    let mut idx = 0usize;
    while val >= 1024.0 && idx < UNITS.len() - 1 {
        val /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{bytes} B")
    } else {
        format!("{val:.1} {}", UNITS[idx])
    }
}

pub fn calc_scaled_dims(
    orig_w: u32,
    orig_h: u32,
    max_w: Option<u32>,
    max_h: Option<u32>,
) -> (u32, u32) {
    let (mut w, mut h) = (orig_w, orig_h);

    if let Some(mw) = max_w
        && w > mw
    {
        h = (h as f64 * mw as f64 / w as f64).round() as u32;
        w = mw;
    }

    if let Some(mh) = max_h
        && h > mh
    {
        w = (w as f64 * mh as f64 / h as f64).round() as u32;
        h = mh;
    }

    (w.max(1), h.max(1))
}
