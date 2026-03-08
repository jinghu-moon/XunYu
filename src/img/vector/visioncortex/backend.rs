use super::*;

pub(super) fn to_color_image(img: &DynamicImage) -> ColorImage {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    ColorImage {
        pixels: rgba.into_raw(),
        width: w,
        height: h,
    }
}

pub(super) fn should_key(img: &ColorImage) -> bool {
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

pub(super) fn cluster_backend_from_env() -> ClusterBackend {
    let backend = std::env::var(XUN_CLUSTER_ENV)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match backend.as_str() {
        "xun" => ClusterBackend::Xun,
        _ => ClusterBackend::Official,
    }
}

pub(super) fn build_runner_config(
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
