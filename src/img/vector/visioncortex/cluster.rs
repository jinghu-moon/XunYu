use super::cluster_merge::merge_small_regions;
use super::options::quantized_color_key;
use super::*;

pub(super) fn build_xun_components(
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

    for (idx, label_slot) in labels.iter_mut().take(pixel_count).enumerate() {
        let label = *label_slot;
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

        *label_slot = region_id + 1;

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

    if let Some(max_components) = cfg.max_components
        && kept_regions.len() > max_components
    {
        kept_regions.sort_by_key(|&idx| Reverse(stats[idx].area));
        kept_regions.truncate(max_components);
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

    for (idx, &compact) in labels.iter().take(pixel_count).enumerate() {
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
