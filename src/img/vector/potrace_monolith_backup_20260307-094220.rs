//! Potrace 算法 Rust 实现
//! 论文: Peter Selinger, "Potrace: a polygon-based tracing algorithm", 2003
//! https://potrace.sourceforge.net/potrace.pdf
//!
//! 五阶段流程:
//!   §2.1  位图 → 有向边界路径 (minority turn policy)
//!   §2.1.3 Despeckling (面积过滤 turdsize)
//!   §2.2  路径 → 最优多边形 (动态规划: 最少段数, 次选最小 L₂ 惩罚)
//!   §2.3  多边形 → 贝塞尔曲线 (角点检测 alphamax + 对称控制点)
//!   §2.4  曲线优化 (合并相邻可合并贝塞尔段, opttolerance)

use image::DynamicImage;

// ── 公开配置 ──────────────────────────────────────────────────────────────

pub struct PotraceConfig {
    /// 去噪面积阈值 (turdsize), 默认 2
    pub turd_size: f64,
    /// 角点检测阈值 (alphamax), 0=全角点 1.333=无角点, 默认 1.0
    pub alpha_max: f64,
    /// 曲线优化容差 (opttolerance), 默认 0.2
    pub opt_tolerance: f64,
}

impl Default for PotraceConfig {
    fn default() -> Self {
        Self {
            turd_size: 2.0,
            alpha_max: 1.0,
            opt_tolerance: 0.2,
        }
    }
}

// ── 2D 点 ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Pt {
    x: f64,
    y: f64,
}

impl Pt {
    fn new(x: f64, y: f64) -> Self {
        Pt { x, y }
    }
    fn sub(&self, o: &Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
    fn add(&self, o: &Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
    fn scale(&self, s: f64) -> Pt {
        Pt::new(self.x * s, self.y * s)
    }
    fn dot(&self, o: &Pt) -> f64 {
        self.x * o.x + self.y * o.y
    }
    fn len(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    fn dist(&self, o: &Pt) -> f64 {
        self.sub(o).len()
    }
    fn maxdist(&self, o: &Pt) -> f64 {
        (self.x - o.x).abs().max((self.y - o.y).abs())
    }
}

// ── SVG 段类型 ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Seg {
    Curve(Pt, Pt, Pt), // c1, c2, end
    Corner(Pt, Pt),    // vertex, end
}

impl Seg {
    fn end(&self) -> &Pt {
        match self {
            Seg::Curve(_, _, e) | Seg::Corner(_, e) => e,
        }
    }
}

// ── 主入口 ────────────────────────────────────────────────────────────────

pub fn trace(img: &DynamicImage, cfg: &PotraceConfig) -> anyhow::Result<String> {
    let luma = img.to_luma8();
    let w = luma.width() as usize;
    let h = luma.height() as usize;
    let thresh = otsu(luma.as_raw());
    let mut bm: Vec<bool> = luma.as_raw().iter().map(|&v| v < thresh).collect();

    let mut svg_paths = String::new();
    let mut remaining = bm.iter().filter(|&&v| v).count();
    let mut guard_steps = 0usize;
    let guard_limit = (w * h).max(1) * 4;

    while remaining > 0 && guard_steps < guard_limit {
        guard_steps += 1;
        // §2.1: 找下一个黑像素，追踪边界
        let Some((sx, sy)) = find_first_black(&bm, w, h) else {
            break;
        };
        let path = trace_path(&bm, w, h, sx, sy);

        // §2.1.3: Despeckling
        if shoelace(&path) < cfg.turd_size {
            erase_path(&mut bm, w, h, &path);
            let next_remaining = bm.iter().filter(|&&v| v).count();
            if next_remaining >= remaining {
                bm[sy * w + sx] = false;
                remaining = bm.iter().filter(|&&v| v).count();
            } else {
                remaining = next_remaining;
            }
            continue;
        }

        // §2.2: 最优多边形
        let poly = optimal_polygon(&path);

        // §2.3: 贝塞尔拟合
        let segs = fit_bezier(&poly, cfg.alpha_max);

        // §2.4: 曲线优化
        let segs = opt_curves(segs, cfg.opt_tolerance);

        // 生成 path d 属性
        if !segs.is_empty() {
            let start = segs.last().unwrap().end();
            let mut d = format!("M{},{}", f(start.x), f(start.y));
            for seg in &segs {
                match seg {
                    Seg::Curve(c1, c2, e) => d.push_str(&format!(
                        " C{},{} {},{} {},{}",
                        f(c1.x),
                        f(c1.y),
                        f(c2.x),
                        f(c2.y),
                        f(e.x),
                        f(e.y)
                    )),
                    Seg::Corner(v, e) => {
                        d.push_str(&format!(" L{},{} L{},{}", f(v.x), f(v.y), f(e.x), f(e.y)))
                    }
                }
            }
            d.push('Z');
            svg_paths.push_str(&format!(
                "<path d=\"{d}\" fill=\"#000\" fill-rule=\"evenodd\"/>\n"
            ));
        }
        erase_path(&mut bm, w, h, &path);
        let next_remaining = bm.iter().filter(|&&v| v).count();
        if next_remaining >= remaining {
            // 保护措施：当擦除未减少前景像素时，至少清掉当前种子，避免死循环
            bm[sy * w + sx] = false;
            remaining = bm.iter().filter(|&&v| v).count();
        } else {
            remaining = next_remaining;
        }
    }

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!-- Potrace: Selinger 2003 — DP optimal polygon + alphamax Bezier fitting -->\n\
         <svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" \
         width=\"{w}\" height=\"{h}\">\n\
         <rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n\
         {svg_paths}</svg>"
    ))
}

// ── §2.1 边界追踪 (minority turn policy) ─────────────────────────────────

fn trace_path(bm: &[bool], w: usize, h: usize, sx: usize, sy: usize) -> Vec<Pt> {
    let pix = |x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && bm[y as usize * w + x as usize]
    };
    let mut cx = sx as i32;
    let mut cy = sy as i32;
    let mut dir: u8 = if pix(cx - 1, cy) { 1 } else { 3 }; // 1=下, 3=上
    let (start_x, start_y, start_dir) = (cx, cy, dir);
    let mut pts = Vec::new();

    loop {
        // 记录当前角点
        let (dx, dy) = dd(dir);
        let pt = match dir {
            0 => Pt::new((cx + 1) as f64, cy as f64),
            1 => Pt::new((cx + 1) as f64, (cy + 1) as f64),
            2 => Pt::new(cx as f64, (cy + 1) as f64),
            _ => Pt::new(cx as f64, cy as f64),
        };
        pts.push(pt);

        // minority turn policy (论文§2.1.2)
        // 左侧像素: 相对于前进方向的左边
        let right_black = pix(cx + dx - dy, cy + dy + dx);
        let left_black = pix(cx - dy, cy + dx);
        let next_dir = if right_black {
            (dir + 1) % 4
        } else if !left_black {
            (dir + 3) % 4
        } else {
            dir
        };
        let (ndx, ndy) = dd(next_dir);
        cx += ndx;
        cy += ndy;
        dir = next_dir;

        if cx == start_x && cy == start_y && dir == start_dir {
            break;
        }
        if pts.len() > (w + h) * 8 {
            break;
        }
    }
    pts
}

fn dd(dir: u8) -> (i32, i32) {
    match dir {
        0 => (1, 0),
        1 => (0, 1),
        2 => (-1, 0),
        _ => (0, -1),
    }
}

// ── §2.2 最优多边形 ───────────────────────────────────────────────────────
// 论文: 最少段数多边形，straight subpath 判定用 max-distance ≤ 0.5

fn optimal_polygon(path: &[Pt]) -> Vec<Pt> {
    let n = path.len();
    if n <= 4 {
        return path.to_vec();
    }

    // 预计算 straight[i][j]: path[i..=j] 可用直线逼近
    let mut straight = vec![false; n * n];
    for i in 0..n {
        straight[i * n + i] = true;
        for len in 1..n.min(64) {
            // 论文实践中每段不超过约30个像素
            let j = (i + len) % n;
            let pi = &path[i];
            let pj = &path[j];
            let d = pj.sub(pi);
            let sl = d.len();
            let ok = if sl < 1e-6 {
                true
            } else {
                (1..len).all(|k| {
                    let pk = &path[(i + k) % n];
                    let dp = pk.sub(pi);
                    let t = dp.dot(&d) / (sl * sl);
                    let t = t.clamp(0.0, 1.0);
                    let proj = Pt::new(pi.x + d.x * t, pi.y + d.y * t);
                    pk.maxdist(&proj) <= 0.5
                })
            };
            straight[i * n + j] = ok;
        }
    }

    // DP: dp[j] = (段数, 累积惩罚, 前驱 i)
    let inf = usize::MAX / 2;
    let mut dp: Vec<(usize, f64, usize)> = vec![(inf, f64::INFINITY, 0); n];
    dp[0] = (0, 0.0, 0);

    for i in 0..n {
        let (base_k, base_p, _) = dp[i];
        if base_k == inf {
            continue;
        }
        for len in 1..n.min(64) {
            let j = (i + len) % n;
            if !straight[i * n + j] {
                continue;
            }
            let pen = base_p + path[i].dist(&path[j]);
            let new_k = base_k + 1;
            let (cur_k, cur_p, _) = dp[j];
            if new_k < cur_k || (new_k == cur_k && pen < cur_p) {
                dp[j] = (new_k, pen, i);
            }
        }
    }

    // 回溯
    let mut idx = n - 1;
    let mut result = Vec::new();
    for _ in 0..n {
        result.push(path[idx].clone());
        let (_, _, prev) = dp[idx];
        if prev == idx || result.len() > n {
            break;
        }
        idx = prev;
        if idx == 0 {
            result.push(path[0].clone());
            break;
        }
    }
    if result.len() < 3 {
        return path.to_vec();
    }
    result.reverse();
    result
}

// ── §2.3 多边形 → 贝塞尔曲线 ─────────────────────────────────────────────
// 论文§2.3: 检查每个顶点的 α 值，决定角点或贝塞尔
// 控制点 = 端点沿切向延伸 α·min(l₁,l₂)/2

fn fit_bezier(poly: &[Pt], alpha_max: f64) -> Vec<Seg> {
    let n = poly.len();
    if n < 2 {
        return Vec::new();
    }
    let mut segs = Vec::new();

    for i in 0..n {
        let p0 = &poly[(i + n - 1) % n];
        let p1 = &poly[i];
        let p2 = &poly[(i + 1) % n];

        let d1 = p1.sub(p0);
        let l1 = d1.len().max(1e-10);
        let d2 = p2.sub(p1);
        let l2 = d2.len().max(1e-10);

        let cos_a = d1.dot(&d2) / (l1 * l2);
        // α = 4γ/3 where γ = ratio of segment lengths (论文§2.3)
        let gamma = (l1 / l2).min(l2 / l1);
        let alpha = (4.0 * gamma / 3.0).min(4.0 / 3.0);
        let is_corner = alpha >= alpha_max || cos_a < 0.0;

        if is_corner {
            segs.push(Seg::Corner(p1.clone(), p2.clone()));
        } else {
            let arm = (l1.min(l2) * alpha) / 2.0;
            let c1 = p1.add(&d1.scale(arm / l1));
            let c2 = p2.sub(&d2.scale(arm / l2));
            segs.push(Seg::Curve(c1, c2, p2.clone()));
        }
    }
    segs
}

// ── §2.4 曲线优化 ─────────────────────────────────────────────────────────

fn opt_curves(segs: Vec<Seg>, tol: f64) -> Vec<Seg> {
    if segs.len() < 2 || tol <= 0.0 {
        return segs;
    }
    let mut result = Vec::new();
    let mut i = 0;
    while i < segs.len() {
        if i + 1 < segs.len() {
            if let (Seg::Curve(c1a, c1b, mid), Seg::Curve(_c2a, c2b, end)) =
                (&segs[i], &segs[i + 1])
            {
                // 检查中间点误差
                let err = mid.dist(&Pt::new(
                    0.5 * c1b.x + 0.5 * mid.x,
                    0.5 * c1b.y + 0.5 * mid.y,
                ));
                if err <= tol {
                    // 合并为单段
                    let start = if i > 0 {
                        segs[i - 1].end().clone()
                    } else {
                        segs.last().unwrap().end().clone()
                    };
                    let nc1 = Pt::new((start.x + c1a.x) / 2.0, (start.y + c1a.y) / 2.0);
                    let nc2 = Pt::new((c2b.x + end.x) / 2.0, (c2b.y + end.y) / 2.0);
                    result.push(Seg::Curve(nc1, nc2, end.clone()));
                    i += 2;
                    continue;
                }
            }
        }
        result.push(segs[i].clone());
        i += 1;
    }
    result
}

// ── 工具函数 ──────────────────────────────────────────────────────────────

fn find_first_black(bm: &[bool], w: usize, h: usize) -> Option<(usize, usize)> {
    for y in 0..h {
        for x in 0..w {
            if bm[y * w + x] {
                return Some((x, y));
            }
        }
    }
    None
}

fn shoelace(pts: &[Pt]) -> f64 {
    let mut a = 0.0;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        a += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    a.abs() / 2.0
}

fn erase_path(bm: &mut [bool], w: usize, h: usize, pts: &[Pt]) {
    for y in 0..h {
        let fy = y as f64 + 0.5;
        let mut inside = false;
        for x in 0..w {
            let fx = x as f64 + 0.5;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                let p1 = &pts[i];
                let p2 = &pts[j];
                if (p1.y <= fy && p2.y > fy) || (p2.y <= fy && p1.y > fy) {
                    let t = (fy - p1.y) / (p2.y - p1.y);
                    if p1.x + t * (p2.x - p1.x) > fx {
                        inside = !inside;
                    }
                }
            }
            if inside {
                bm[y * w + x] = false;
            }
        }
    }
}

fn otsu(gray: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in gray {
        hist[v as usize] += 1;
    }
    let total = gray.len() as f64;
    let sum_t: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();
    let (mut wb, mut sum_b, mut best, mut thresh) = (0.0f64, 0.0f64, 0.0f64, 128u8);
    for t in 0..256usize {
        wb += hist[t] as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let between = wb * wf * ((sum_b / wb) - (sum_t - sum_b) / wf).powi(2);
        if between > best {
            best = between;
            thresh = t as u8;
        }
    }
    thresh
}

fn f(v: f64) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.2}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}
