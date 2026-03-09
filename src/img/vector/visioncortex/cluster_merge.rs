use super::*;

pub(super) fn merge_small_regions(
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

    for (idx, label_slot) in labels.iter_mut().enumerate() {
        let label = *label_slot;
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
        *label_slot = new_id + 1;

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
