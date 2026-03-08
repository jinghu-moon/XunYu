//! Zhang-Suen 骨架化算法
//! 论文: T. Y. Zhang & C. Y. Suen,
//!      "A Fast Parallel Algorithm for Thinning Digital Patterns"
//!      Communications of the ACM, 27(3):236–239, 1984.
//!      DOI: 10.1145/357994.358023
//!
//! 算法核心 (论文图1 邻域定义):
//!   P9 P2 P3
//!   P8 P1 P4
//!   P7 P6 P5
//!
//!   B(P1) = Σ(P2..P9)           — 黑色邻居数
//!   A(P1) = 0→1 跳变次数 (顺时针)
//!
//!   子迭代1删除条件 (东南边界 + 西北角点):
//!     (a) 2 ≤ B(P1) ≤ 6
//!     (b) A(P1) = 1
//!     (c) P2 · P4 · P6 = 0
//!     (d) P4 · P6 · P8 = 0
//!
//!   子迭代2删除条件 (西北边界 + 东南角点):
//!     (a)(b) 同上
//!     (c') P2 · P4 · P8 = 0
//!     (d') P2 · P6 · P8 = 0
//!
//! 后处理:
//!   骨架路径提取 → 最小二乘贝塞尔拟合 (复用 bezier.rs)

use super::bezier::{BezierFitter, Pt, to_svg_path};
use image::DynamicImage;

pub struct SkeletonConfig {
    /// 骨架路径最短点数（过滤毛刺）
    pub min_path_len: usize,
    /// 贝塞尔拟合容差（像素）
    pub bezier_tolerance: f64,
    /// SVG 描边颜色
    pub stroke_color: String,
    /// SVG 描边宽度
    pub stroke_width: f64,
}

impl Default for SkeletonConfig {
    fn default() -> Self {
        Self {
            min_path_len: 4,
            bezier_tolerance: 1.5,
            stroke_color: "#000000".into(),
            stroke_width: 1.0,
        }
    }
}

pub fn trace(img: &DynamicImage, cfg: &SkeletonConfig) -> anyhow::Result<String> {
    let luma = img.to_luma8();
    let w = luma.width() as usize;
    let h = luma.height() as usize;

    // 1. Otsu 二值化 (前景=1)
    let thresh = otsu(luma.as_raw());
    let mut bin: Vec<u8> = luma
        .as_raw()
        .iter()
        .map(|&v| if v < thresh { 1 } else { 0 })
        .collect();

    // 2. Zhang-Suen 并行细化
    zhang_suen(&mut bin, w, h);

    // 3. 骨架路径提取
    let paths = extract_paths(&bin, w, h, cfg.min_path_len);

    // 4. 贝塞尔拟合 → SVG
    let fitter = BezierFitter::new(cfg.bezier_tolerance);
    let mut path_elements = String::new();
    for pts in &paths {
        let curves = fitter.fit(pts, false);
        if curves.is_empty() {
            continue;
        }
        let d = to_svg_path(&curves, false);
        path_elements.push_str(&format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\"/>\n",
            cfg.stroke_color, cfg.stroke_width
        ));
    }

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!-- Zhang-Suen 1984: parallel thinning → bezier stroke fitting -->\n\
         <svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" \
         width=\"{w}\" height=\"{h}\">\n\
         <rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n\
         {path_elements}</svg>"
    ))
}

// ── Zhang-Suen 并行细化 ───────────────────────────────────────────────────
// 论文: 两步并行删除，迭代至收敛

fn zhang_suen(bin: &mut [u8], w: usize, h: usize) {
    loop {
        let mut changed = false;

        // 子迭代 1：删除东南边界 + 西北角点
        let mut del: Vec<usize> = Vec::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let idx = y * w + x;
                if bin[idx] == 0 {
                    continue;
                }
                // 按论文图1顺序读取8邻域: P2(北),P3(东北),P4(东),P5(东南),P6(南),P7(西南),P8(西),P9(西北)
                let [p2, p3, p4, p5, p6, p7, p8, p9] = nb(bin, w, x, y);
                let b = p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9;
                let a = trans([p2, p3, p4, p5, p6, p7, p8, p9, p2]);
                if b >= 2 && b <= 6 && a == 1 && p2 * p4 * p6 == 0 && p4 * p6 * p8 == 0 {
                    del.push(idx);
                }
            }
        }
        for idx in &del {
            bin[*idx] = 0;
        }
        changed |= !del.is_empty();

        // 子迭代 2：删除西北边界 + 东南角点
        let mut del2: Vec<usize> = Vec::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let idx = y * w + x;
                if bin[idx] == 0 {
                    continue;
                }
                let [p2, p3, p4, p5, p6, p7, p8, p9] = nb(bin, w, x, y);
                let b = p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9;
                let a = trans([p2, p3, p4, p5, p6, p7, p8, p9, p2]);
                if b >= 2 && b <= 6 && a == 1 && p2 * p4 * p8 == 0 && p2 * p6 * p8 == 0 {
                    del2.push(idx);
                }
            }
        }
        for idx in &del2 {
            bin[*idx] = 0;
        }
        changed |= !del2.is_empty();

        if !changed {
            break;
        }
    }
}

/// 按论文图1定义返回8邻域 [P2,P3,P4,P5,P6,P7,P8,P9]
#[inline]
fn nb(bin: &[u8], w: usize, x: usize, y: usize) -> [u8; 8] {
    [
        bin[(y - 1) * w + x],     // P2: 北
        bin[(y - 1) * w + x + 1], // P3: 东北
        bin[y * w + x + 1],       // P4: 东
        bin[(y + 1) * w + x + 1], // P5: 东南
        bin[(y + 1) * w + x],     // P6: 南
        bin[(y + 1) * w + x - 1], // P7: 西南
        bin[y * w + x - 1],       // P8: 西
        bin[(y - 1) * w + x - 1], // P9: 西北
    ]
}

/// A(P1): 顺时针序列中 0→1 跳变次数（论文定义）
#[inline]
fn trans(n: [u8; 9]) -> u8 {
    // n[0]=P2, n[1]=P3, ..., n[7]=P9, n[8]=P2 (论文的循环)
    let mut c = 0u8;
    for i in 0..8 {
        if n[i] == 0 && n[i + 1] == 1 {
            c += 1;
        }
    }
    c
}

// ── 骨架路径提取 ──────────────────────────────────────────────────────────
// 找端点（1邻居）和交叉点（≥3邻居）作为起点，沿连通骨架追踪

fn extract_paths(bin: &[u8], w: usize, h: usize, min_len: usize) -> Vec<Vec<Pt>> {
    let mut visited = vec![false; w * h];
    let mut paths = Vec::new();

    // 收集所有骨架点，优先端点（1邻居）
    let mut endpoints = Vec::new();
    let mut regular = Vec::new();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            if bin[idx] == 0 {
                continue;
            }
            let cnt = nb(bin, w, x, y).iter().map(|&v| v as usize).sum::<usize>();
            if cnt == 1 {
                endpoints.push(idx);
            } else if cnt >= 2 {
                regular.push(idx);
            }
        }
    }
    // 端点优先遍历
    let all: Vec<usize> = endpoints.into_iter().chain(regular).collect();

    for start_idx in all {
        if visited[start_idx] || bin[start_idx] == 0 {
            continue;
        }
        let sx = start_idx % w;
        let sy = start_idx / w;
        let path_pts = follow(bin, w, h, &mut visited, sx, sy);
        if path_pts.len() >= min_len {
            paths.push(path_pts);
        }
    }
    paths
}

/// 从 (sx,sy) 开始沿骨架追踪，返回路径坐标列表
fn follow(bin: &[u8], w: usize, h: usize, visited: &mut [bool], sx: usize, sy: usize) -> Vec<Pt> {
    let mut pts = Vec::new();
    // 用栈式 DFS，但优先选择"直行"方向保持路径连续
    let mut stack = vec![(sx, sy, 0usize, 0usize)]; // x, y, prev_x, prev_y
    let _start_prev = (sx, sy);

    while let Some((cx, cy, px, py)) = stack.pop() {
        if visited[cy * w + cx] {
            continue;
        }
        visited[cy * w + cx] = true;
        pts.push(Pt {
            x: cx as f64,
            y: cy as f64,
        });

        // 找8邻接中未访问的骨架像素
        let mut nexts: Vec<(usize, usize)> = DIRS8
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    return None;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if bin[ny * w + nx] == 1 && !visited[ny * w + nx] {
                    Some((nx, ny))
                } else {
                    None
                }
            })
            .collect();

        if nexts.is_empty() {
            continue;
        }

        // 优先选择与前进方向相同的像素（减少折点）
        nexts.sort_by_key(|&(nx, ny)| {
            let ddx = cx as i32 - px as i32;
            let ddy = cy as i32 - py as i32;
            let tx = nx as i32 - cx as i32;
            let ty = ny as i32 - cy as i32;
            // 用负点积：点积越大（方向越一致）优先级越高
            -(ddx * tx + ddy * ty)
        });
        // 只追踪最优方向（避免分叉导致重复）
        let (nx, ny) = nexts[0];
        stack.push((nx, ny, cx, cy));
    }
    pts
}

const DIRS8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

// ── Otsu 阈值 ─────────────────────────────────────────────────────────────

fn otsu(gray: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = gray.len() as f64;
    let sum_t: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();
    let (mut wb, mut sum_b, mut best, mut thresh) = (0.0f64, 0.0f64, 0.0f64, 128u8);
    for t in 0..256 {
        wb += hist[t] as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let v = wb * wf * ((sum_b / wb) - (sum_t - sum_b) / wf).powi(2);
        if v > best {
            best = v;
            thresh = t as u8;
        }
    }
    thresh
}
