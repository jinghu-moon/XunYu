//! img2svg 风格的矢量化管线（移植自 img2svg, Apache-2.0）
//! 流程：中值切割量化 → Marching Squares 轮廓追踪
//!       → Gaussian 平滑 → RDP 简化 → 最小二乘贝塞尔拟合
//!
//! 与 visioncortex 管线的核心区别：
//!   visioncortex: 颜色聚类 → 逐像素路径 → Catmull-Rom 样条
//!   本模块:       中值切割 → 子像素轮廓 → 最小二乘贝塞尔（Newton-Raphson）

use super::bezier::{BezierFitter, Pt, to_svg_path};
use image::DynamicImage;
use std::collections::HashMap;

// ── 公开 API ─────────────────────────────────────────────────────────────

pub struct TracerConfig {
    /// 量化颜色数（默认 16）
    pub num_colors: usize,
    /// Gaussian 平滑迭代次数（0~3）
    pub smooth: u8,
    /// 贝塞尔拟合容差（像素，默认 1.5）
    pub tolerance: f64,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            num_colors: 16,
            smooth: 2,
            tolerance: 1.5,
        }
    }
}

/// 将 DynamicImage 向量化为 SVG 字符串（img2svg 算法）
pub fn trace(img: &DynamicImage, cfg: &TracerConfig) -> anyhow::Result<String> {
    let rgba = img.to_rgba8();
    let w = rgba.width() as usize;
    let h = rgba.height() as usize;

    // 1. 提取像素 (r,g,b,a)
    let pixels: Vec<(u8, u8, u8, u8)> = rgba.pixels().map(|p| (p[0], p[1], p[2], p[3])).collect();

    // 2. 中值切割量化
    let palette = median_cut(&pixels, cfg.num_colors);
    let quantized = quantize(&pixels, &palette);

    // 3. 按颜色分组像素
    let mut color_map: HashMap<(u8, u8, u8, u8), Vec<usize>> = HashMap::new();
    for (i, &c) in quantized.iter().enumerate() {
        color_map.entry(c).or_default().push(i);
    }

    // 4. 按面积降序（最大面积 = 背景）
    let mut color_list: Vec<_> = color_map.into_iter().collect();
    color_list.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    let bg = color_list
        .first()
        .map(|(c, _)| *c)
        .unwrap_or((255, 255, 255, 255));

    let fitter = BezierFitter::new(cfg.tolerance);
    let mut paths_svg = String::new();

    // 5. 背景矩形
    let (br, bg_, bb, _) = bg;
    paths_svg.push_str(&format!(
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{}\"/>\n",
        hex(br, bg_, bb)
    ));

    // 6. 每种颜色 → marching squares → 平滑 → RDP → 贝塞尔 → SVG path
    for (color, indices) in &color_list {
        if *color == bg {
            continue;
        }
        let mut mask = vec![false; w * h];
        for &idx in indices {
            mask[idx] = true;
        }

        let contours = marching_squares(&mask, w, h);
        let (cr, cg, cb, _) = color;
        let fill = hex(*cr, *cg, *cb);

        let mut path_d_parts: Vec<String> = Vec::new();

        for contour in contours {
            if contour.len() < 4 {
                continue;
            }
            if polygon_area(&contour) < 8.0 {
                continue;
            }

            let smoothed = gaussian_smooth(&contour, cfg.smooth);
            let simplified = rdp_simplify(&smoothed, 2.0);

            // 边缘吸附
            let snap = 4.0;
            let fw = w as f64;
            let fh = h as f64;
            let snapped: Vec<Pt> = simplified
                .into_iter()
                .map(|p| Pt {
                    x: if p.x < snap {
                        0.0
                    } else if p.x > fw - snap {
                        fw
                    } else {
                        p.x
                    },
                    y: if p.y < snap {
                        0.0
                    } else if p.y > fh - snap {
                        fh
                    } else {
                        p.y
                    },
                })
                .collect();

            if snapped.len() < 3 || polygon_area(&snapped) < 8.0 {
                continue;
            }

            let curves = fitter.fit(&snapped, true);
            if !curves.is_empty() {
                path_d_parts.push(to_svg_path(&curves, true));
            }
        }

        if !path_d_parts.is_empty() {
            // 多个子路径合并为一个 <path>（偶奇填充规则）
            let d = path_d_parts.join(" ");
            paths_svg.push_str(&format!(
                "<path d=\"{d}\" fill=\"{fill}\" fill-rule=\"evenodd\"/>\n"
            ));
        }
    }

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\">\n\
         {paths_svg}</svg>\n"
    ))
}

// ── 中值切割量化 ─────────────────────────────────────────────────────────

fn median_cut(pixels: &[(u8, u8, u8, u8)], num_colors: usize) -> Vec<(u8, u8, u8)> {
    if num_colors == 0 {
        return vec![];
    }
    let step = (pixels.len() / 50_000).max(1);
    let colors: Vec<(u8, u8, u8)> = pixels
        .iter()
        .step_by(step)
        .map(|&(r, g, b, _)| (r, g, b))
        .collect();
    if colors.is_empty() {
        return vec![(0, 0, 0)];
    }

    let mut boxes: Vec<Vec<(u8, u8, u8)>> = vec![colors];
    while boxes.len() < num_colors {
        let best = boxes
            .iter()
            .enumerate()
            .max_by_key(|(_, b)| box_range(b))
            .map(|(i, _)| i);
        let Some(bi) = best else { break };
        if boxes[bi].len() < 2 {
            break;
        }
        let to_split = boxes.remove(bi);
        let (a, b) = split(to_split);
        if !a.is_empty() {
            boxes.push(a);
        }
        if !b.is_empty() {
            boxes.push(b);
        }
    }
    boxes.iter().map(|b| box_avg(b)).collect()
}

fn box_range(c: &[(u8, u8, u8)]) -> u16 {
    let (mut rn, mut rx, mut gn, mut gx, mut bn, mut bx) = (255, 0u8, 255, 0u8, 255, 0u8);
    for &(r, g, b) in c {
        rn = rn.min(r);
        rx = rx.max(r);
        gn = gn.min(g);
        gx = gx.max(g);
        bn = bn.min(b);
        bx = bx.max(b);
    }
    ((rx - rn) as u16)
        .max((gx - gn) as u16)
        .max((bx - bn) as u16)
}

fn split(mut c: Vec<(u8, u8, u8)>) -> (Vec<(u8, u8, u8)>, Vec<(u8, u8, u8)>) {
    let (mut rn, mut rx, mut gn, mut gx, mut bn, mut bx) = (255, 0u8, 255, 0u8, 255, 0u8);
    for &(r, g, b) in &c {
        rn = rn.min(r);
        rx = rx.max(r);
        gn = gn.min(g);
        gx = gx.max(g);
        bn = bn.min(b);
        bx = bx.max(b);
    }
    let rr = (rx - rn) as u16;
    let gr = (gx - gn) as u16;
    let br = (bx - bn) as u16;
    if rr >= gr && rr >= br {
        c.sort_by_key(|c| c.0);
    } else if gr >= br {
        c.sort_by_key(|c| c.1);
    } else {
        c.sort_by_key(|c| c.2);
    }
    let mid = c.len() / 2;
    let r = c.split_off(mid);
    (c, r)
}

fn box_avg(c: &[(u8, u8, u8)]) -> (u8, u8, u8) {
    if c.is_empty() {
        return (0, 0, 0);
    }
    let (sr, sg, sb) = c
        .iter()
        .fold((0u64, 0u64, 0u64), |(a, b, cc), &(r, g, bl)| {
            (a + r as u64, b + g as u64, cc + bl as u64)
        });
    let n = c.len() as u64;
    ((sr / n) as u8, (sg / n) as u8, (sb / n) as u8)
}

fn quantize(pixels: &[(u8, u8, u8, u8)], palette: &[(u8, u8, u8)]) -> Vec<(u8, u8, u8, u8)> {
    pixels
        .iter()
        .map(|&(r, g, b, a)| {
            let (pr, pg, pb) = palette
                .iter()
                .min_by_key(|&&(pr, pg, pb)| {
                    let dr = pr as i32 - r as i32;
                    let dg = pg as i32 - g as i32;
                    let db = pb as i32 - b as i32;
                    dr * dr + dg * dg + db * db
                })
                .copied()
                .unwrap_or((r, g, b));
            (pr, pg, pb, a)
        })
        .collect()
}

// ── Marching Squares 轮廓追踪 ────────────────────────────────────────────

fn marching_squares(mask: &[bool], width: usize, height: usize) -> Vec<Vec<Pt>> {
    let gw = width + 2;
    let gh = height + 2;
    let inside = |gx: usize, gy: usize| -> bool {
        gx > 0 && gy > 0 && gx <= width && gy <= height && mask[(gy - 1) * width + (gx - 1)]
    };
    let cell_case = |cx: usize, cy: usize| -> u8 {
        (inside(cx, cy) as u8) << 3
            | (inside(cx + 1, cy) as u8) << 2
            | (inside(cx + 1, cy + 1) as u8) << 1
            | (inside(cx, cy + 1) as u8)
    };
    let fw = width as f64;
    let fh = height as f64;
    let edge_pt = move |cx: usize, cy: usize, side: u8| -> Pt {
        let (x, y) = match side {
            0 => (cx as f64 + 0.5, cy as f64),
            1 => ((cx + 1) as f64, cy as f64 + 0.5),
            2 => (cx as f64 + 0.5, (cy + 1) as f64),
            3 => (cx as f64, cy as f64 + 0.5),
            _ => unreachable!(),
        };
        Pt {
            x: (x - 0.5).clamp(0.0, fw),
            y: (y - 0.5).clamp(0.0, fh),
        }
    };
    let case_edges = |case: u8| -> Vec<(u8, u8)> {
        match case {
            0 | 15 => vec![],
            1 => vec![(2, 3)],
            2 => vec![(1, 2)],
            3 => vec![(1, 3)],
            4 => vec![(0, 1)],
            5 => vec![(0, 1), (2, 3)],
            6 => vec![(0, 2)],
            7 => vec![(0, 3)],
            8 => vec![(3, 0)],
            9 => vec![(2, 0)],
            10 => vec![(3, 0), (1, 2)],
            11 => vec![(1, 0)],
            12 => vec![(3, 1)],
            13 => vec![(2, 1)],
            14 => vec![(3, 2)],
            _ => vec![],
        }
    };
    let opp = |s: u8| -> u8 {
        match s {
            0 => 2,
            1 => 3,
            2 => 0,
            3 => 1,
            _ => unreachable!(),
        }
    };
    let neighbor = move |cx: usize, cy: usize, side: u8| -> Option<(usize, usize)> {
        match side {
            0 if cy > 0 => Some((cx, cy - 1)),
            1 if cx + 1 < gw => Some((cx + 1, cy)),
            2 if cy + 1 < gh => Some((cx, cy + 1)),
            3 if cx > 0 => Some((cx - 1, cy)),
            _ => None,
        }
    };

    let mut visited: HashMap<(usize, usize, u8), bool> = HashMap::new();
    let mut contours = Vec::new();

    for cy in 0..gh {
        for cx in 0..gw {
            let case = cell_case(cx, cy);
            for &(entry, exit) in &case_edges(case) {
                if visited.contains_key(&(cx, cy, entry)) {
                    continue;
                }
                let mut contour = Vec::new();
                let (mut ccx, mut ccy, mut cen, mut cex) = (cx, cy, entry, exit);
                let start = (cx, cy, entry);
                loop {
                    visited.insert((ccx, ccy, cen), true);
                    visited.insert((ccx, ccy, cex), true);
                    contour.push(edge_pt(ccx, ccy, cex));
                    let next_entry = opp(cex);
                    if let Some((nx, ny)) = neighbor(ccx, ccy, cex) {
                        let nc = cell_case(nx, ny);
                        if let Some(&(ne, nx_exit)) =
                            case_edges(nc).iter().find(|&&(e, _)| e == next_entry)
                        {
                            if (nx, ny, ne) == start {
                                break;
                            }
                            ccx = nx;
                            ccy = ny;
                            cen = ne;
                            cex = nx_exit;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if contour.len() >= 3 {
                    contours.push(contour);
                }
            }
        }
    }
    contours
}

// ── Gaussian 平滑（保点数，不增点）──────────────────────────────────────

fn gaussian_smooth(pts: &[Pt], level: u8) -> Vec<Pt> {
    if level == 0 || pts.len() < 3 {
        return pts.to_vec();
    }
    let mut cur = pts.to_vec();
    for _ in 0..(level as usize).min(3) {
        let n = cur.len();
        let mut next = Vec::with_capacity(n);
        for i in 0..n {
            let prev = &cur[(i + n - 1) % n];
            let c = &cur[i];
            let nxt = &cur[(i + 1) % n];
            next.push(Pt {
                x: 0.25 * prev.x + 0.5 * c.x + 0.25 * nxt.x,
                y: 0.25 * prev.y + 0.5 * c.y + 0.25 * nxt.y,
            });
        }
        cur = next;
    }
    cur
}

// ── Ramer-Douglas-Peucker 简化 ───────────────────────────────────────────

fn rdp_simplify(pts: &[Pt], eps: f64) -> Vec<Pt> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let (first, last) = (&pts[0], &pts[pts.len() - 1]);
    let (mut max_d, mut max_i) = (0.0f64, 0);
    for i in 1..pts.len() - 1 {
        let d = pt_line_dist(&pts[i], first, last);
        if d > max_d {
            max_d = d;
            max_i = i;
        }
    }
    if max_d > eps {
        let mut l = rdp_simplify(&pts[..=max_i], eps);
        let r = rdp_simplify(&pts[max_i..], eps);
        l.pop();
        l.extend(r);
        l
    } else {
        vec![first.clone(), last.clone()]
    }
}

fn pt_line_dist(p: &Pt, a: &Pt, b: &Pt) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let ls = dx * dx + dy * dy;
    if ls < 1e-10 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / ls;
    let t = t.clamp(0.0, 1.0);
    ((p.x - (a.x + t * dx)).powi(2) + (p.y - (a.y + t * dy)).powi(2)).sqrt()
}

fn polygon_area(pts: &[Pt]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    area.abs() / 2.0
}

// ── 工具 ─────────────────────────────────────────────────────────────────

fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}
