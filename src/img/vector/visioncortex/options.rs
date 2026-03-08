use super::*;

pub(super) fn build_xun_cluster_config(
    pixels: usize,
    complexity: ComplexityProfile,
) -> XunClusterConfig {
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

pub(super) fn build_xun_path_config(default_polygon_area: usize) -> PathBuildConfig {
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

pub(super) fn quantized_color_key(r: u8, g: u8, b: u8, shift: u8) -> u32 {
    (u32::from(r >> shift) << 16) | (u32::from(g >> shift) << 8) | u32::from(b >> shift)
}

pub(super) fn apply_key_to_transparent(img: &mut ColorImage, key: Color) {
    for px in img.pixels.chunks_exact_mut(4) {
        if px[3] == 0 {
            px[0] = key.r;
            px[1] = key.g;
            px[2] = key.b;
            px[3] = 255;
        }
    }
}

pub(super) fn find_key_color(img: &ColorImage) -> Option<Color> {
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

pub(super) fn analyze_complexity(img: &ColorImage) -> ComplexityProfile {
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
