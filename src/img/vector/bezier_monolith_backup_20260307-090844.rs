//! 最小二乘三次贝塞尔曲线拟合器
//! 完整移植自 img2svg (Apache-2.0, Ying Kit WONG)
//! 算法：弦长参数化 → 最小二乘拟合 → Newton-Raphson 重参数化 → G1 连续性强制
//!
//! 与 visioncortex 样条的核心区别：
//!   - visioncortex：Catmull-Rom 样条，各段独立拟合
//!   - 本模块：全局最小二乘 + NR 迭代，G1 连续，子像素精度

/// 2D 点
#[derive(Debug, Clone, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

/// 三次贝塞尔曲线段 (P0, P1, P2, P3)
#[derive(Debug, Clone)]
pub struct Bezier {
    pub p0: Pt, // start
    pub p1: Pt, // control 1
    pub p2: Pt, // control 2
    pub p3: Pt, // end
}

/// 拟合器，可复用
pub struct BezierFitter {
    pub tolerance: f64,
    pub max_iterations: usize,
}

impl BezierFitter {
    pub fn new(tolerance: f64) -> Self {
        Self {
            tolerance,
            max_iterations: 12,
        }
    }

    /// 将点序列拟合为一组三次贝塞尔曲线
    pub fn fit(&self, points: &[Pt], closed: bool) -> Vec<Bezier> {
        if points.len() < 2 {
            return Vec::new();
        }
        if points.len() == 2 {
            return vec![linear_bezier(&points[0], &points[1])];
        }

        let corners = self.detect_corners(points);
        let mut out = Vec::new();

        if corners.is_empty() {
            self.fit_segment(points, &mut out);
        } else {
            let mut splits = vec![0usize];
            for &c in &corners {
                if c > 0 && c < points.len() - 1 {
                    splits.push(c);
                }
            }
            splits.push(points.len() - 1);
            splits.dedup();
            for w in splits.windows(2) {
                let seg = &points[w[0]..=w[1]];
                if seg.len() >= 2 {
                    self.fit_segment(seg, &mut out);
                }
            }
        }

        // G1 连续性：仅在无角点（光滑路径）时强制
        if out.len() > 1 && corners.is_empty() {
            enforce_g1(&mut out);
        }

        // 闭合
        if closed && !out.is_empty() {
            let last = &out.last().unwrap().p3;
            let first = &out[0].p0;
            if dist(last, first) > 0.5 {
                out.push(linear_bezier(last, first));
            }
        }

        // 控制点箝位（防止过冲）
        clamp_controls(&mut out, points);
        out
    }

    // ── 内部：检测锐角（转角 > 30°）──────────────────────────────────────
    fn detect_corners(&self, pts: &[Pt]) -> Vec<usize> {
        let thresh = 30.0_f64.to_radians();
        let mut corners = Vec::new();
        for i in 1..pts.len() - 1 {
            let v1x = pts[i].x - pts[i - 1].x;
            let v1y = pts[i].y - pts[i - 1].y;
            let v2x = pts[i + 1].x - pts[i].x;
            let v2y = pts[i + 1].y - pts[i].y;
            let l1 = (v1x * v1x + v1y * v1y).sqrt();
            let l2 = (v2x * v2x + v2y * v2y).sqrt();
            if l1 < 1e-6 || l2 < 1e-6 {
                continue;
            }
            let cos = ((v1x * v2x + v1y * v2y) / (l1 * l2)).clamp(-1.0, 1.0);
            if cos.acos() > thresh {
                corners.push(i);
            }
        }
        corners
    }

    // ── 内部：递归拟合单段 ────────────────────────────────────────────────
    fn fit_segment(&self, pts: &[Pt], out: &mut Vec<Bezier>) {
        if pts.len() < 2 {
            return;
        }
        if pts.len() == 2 {
            out.push(linear_bezier(&pts[0], &pts[1]));
            return;
        }
        if self.is_linear(pts) {
            out.push(linear_bezier(&pts[0], &pts[pts.len() - 1]));
            return;
        }
        if pts.len() == 3 {
            out.push(fit_3pts(pts));
            return;
        }

        const MAX_SEG: usize = 40;
        if pts.len() > MAX_SEG {
            let mid = best_split(pts);
            self.fit_segment(&pts[..=mid], out);
            self.fit_segment(&pts[mid..], out);
            return;
        }

        let mut t = chord_param(pts);
        let mut best = self.ls_fit(pts, &t);
        let (mut best_err, mut best_idx) = self.max_error(&best, pts);

        if best_err <= self.tolerance {
            out.push(best);
            return;
        }

        for _ in 0..self.max_iterations {
            let t2 = self.nr_reparam(&best, pts, &t);
            t = t2;
            let c2 = self.ls_fit(pts, &t);
            let (e2, idx2) = self.max_error(&c2, pts);
            if e2 < best_err {
                best = c2;
                best_err = e2;
                best_idx = idx2;
                if best_err <= self.tolerance {
                    out.push(best);
                    return;
                }
            } else {
                break;
            }
        }

        if pts.len() <= 3 {
            out.push(best);
        } else {
            let split = best_idx.max(2).min(pts.len() - 2);
            self.fit_segment(&pts[..=split], out);
            self.fit_segment(&pts[split..], out);
        }
    }

    // ── Newton-Raphson 重参数化 ───────────────────────────────────────────
    fn nr_reparam(&self, b: &Bezier, pts: &[Pt], t: &[f64]) -> Vec<f64> {
        let mut new_t = t.to_vec();
        for i in 1..pts.len() - 1 {
            let bt = eval(b, t[i]);
            let d1 = eval_d1(b, t[i]);
            let d2 = eval_d2(b, t[i]);
            let dx = bt.x - pts[i].x;
            let dy = bt.y - pts[i].y;
            let num = dx * d1.x + dy * d1.y;
            let den = d1.x * d1.x + d1.y * d1.y + dx * d2.x + dy * d2.y;
            if den.abs() > 1e-12 {
                new_t[i] = (t[i] - num / den).clamp(0.0, 1.0);
            }
        }
        // 保证单调性
        for i in 1..new_t.len() {
            if new_t[i] <= new_t[i - 1] {
                new_t[i] = new_t[i - 1] + 1e-10;
            }
        }
        new_t[0] = 0.0;
        *new_t.last_mut().unwrap() = 1.0;
        new_t
    }

    // ── 最小二乘拟合（核心） ──────────────────────────────────────────────
    fn ls_fit(&self, pts: &[Pt], t: &[f64]) -> Bezier {
        let n = pts.len();
        let p0 = pts[0].clone();
        let p3 = pts[n - 1].clone();
        let (mut a11, mut a12, mut a22) = (0.0, 0.0, 0.0);
        let (mut bx1, mut by1, mut bx2, mut by2) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..n {
            let ti = t[i];
            let mt = 1.0 - ti;
            let b0 = mt * mt * mt;
            let b1 = 3.0 * mt * mt * ti;
            let b2 = 3.0 * mt * ti * ti;
            let b3 = ti * ti * ti;
            a11 += b1 * b1;
            a12 += b1 * b2;
            a22 += b2 * b2;
            let rx = pts[i].x - b0 * p0.x - b3 * p3.x;
            let ry = pts[i].y - b0 * p0.y - b3 * p3.y;
            bx1 += b1 * rx;
            by1 += b1 * ry;
            bx2 += b2 * rx;
            by2 += b2 * ry;
        }
        let det = a11 * a22 - a12 * a12;
        let (p1, p2) = if det.abs() < 1e-12 {
            let dx = p3.x - p0.x;
            let dy = p3.y - p0.y;
            (
                Pt {
                    x: p0.x + dx / 3.0,
                    y: p0.y + dy / 3.0,
                },
                Pt {
                    x: p0.x + 2.0 * dx / 3.0,
                    y: p0.y + 2.0 * dy / 3.0,
                },
            )
        } else {
            let inv = 1.0 / det;
            (
                Pt {
                    x: (a22 * bx1 - a12 * bx2) * inv,
                    y: (a22 * by1 - a12 * by2) * inv,
                },
                Pt {
                    x: (a11 * bx2 - a12 * bx1) * inv,
                    y: (a11 * by2 - a12 * by1) * inv,
                },
            )
        };
        Bezier { p0, p1, p2, p3 }
    }

    fn max_error(&self, b: &Bezier, pts: &[Pt]) -> (f64, usize) {
        let t = chord_param(pts);
        let (mut max_e, mut max_i) = (0.0, 0);
        for i in 1..pts.len() - 1 {
            let q = eval(b, t[i]);
            let e = dist(&pts[i], &q);
            if e > max_e {
                max_e = e;
                max_i = i;
            }
        }
        (max_e, max_i)
    }

    fn is_linear(&self, pts: &[Pt]) -> bool {
        let p0 = &pts[0];
        let pn = &pts[pts.len() - 1];
        let dx = pn.x - p0.x;
        let dy = pn.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return true;
        }
        let thresh = (self.tolerance * 0.5).max(len * 0.01);
        pts[1..pts.len() - 1]
            .iter()
            .all(|p| ((p.y - p0.y) * dx - (p.x - p0.x) * dy).abs() / len <= thresh)
    }
}

// ── 自由函数 ──────────────────────────────────────────────────────────────

fn chord_param(pts: &[Pt]) -> Vec<f64> {
    let mut t = vec![0.0; pts.len()];
    for i in 1..pts.len() {
        t[i] = t[i - 1] + dist(&pts[i], &pts[i - 1]);
    }
    let total = t[pts.len() - 1];
    if total > 0.0 {
        for ti in t.iter_mut() {
            *ti /= total;
        }
    }
    *t.last_mut().unwrap() = 1.0;
    t
}

fn eval(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    Pt {
        x: mt3 * b.p0.x + 3.0 * mt2 * t * b.p1.x + 3.0 * mt * t2 * b.p2.x + t3 * b.p3.x,
        y: mt3 * b.p0.y + 3.0 * mt2 * t * b.p1.y + 3.0 * mt * t2 * b.p2.y + t3 * b.p3.y,
    }
}

fn eval_d1(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let ax = b.p1.x - b.p0.x;
    let ay = b.p1.y - b.p0.y;
    let bx = b.p2.x - b.p1.x;
    let by_ = b.p2.y - b.p1.y;
    let cx = b.p3.x - b.p2.x;
    let cy = b.p3.y - b.p2.y;
    Pt {
        x: 3.0 * (mt * mt * ax + 2.0 * mt * t * bx + t * t * cx),
        y: 3.0 * (mt * mt * ay + 2.0 * mt * t * by_ + t * t * cy),
    }
}

fn eval_d2(b: &Bezier, t: f64) -> Pt {
    let mt = 1.0 - t;
    let ax = b.p2.x - 2.0 * b.p1.x + b.p0.x;
    let ay = b.p2.y - 2.0 * b.p1.y + b.p0.y;
    let bx = b.p3.x - 2.0 * b.p2.x + b.p1.x;
    let by_ = b.p3.y - 2.0 * b.p2.y + b.p1.y;
    Pt {
        x: 6.0 * (mt * ax + t * bx),
        y: 6.0 * (mt * ay + t * by_),
    }
}

fn dist(a: &Pt, b: &Pt) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn linear_bezier(a: &Pt, b: &Pt) -> Bezier {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    Bezier {
        p0: a.clone(),
        p1: Pt {
            x: a.x + dx / 3.0,
            y: a.y + dy / 3.0,
        },
        p2: Pt {
            x: a.x + 2.0 * dx / 3.0,
            y: a.y + 2.0 * dy / 3.0,
        },
        p3: b.clone(),
    }
}

fn fit_3pts(pts: &[Pt]) -> Bezier {
    let (p0, p1, p2) = (&pts[0], &pts[1], &pts[2]);
    Bezier {
        p0: p0.clone(),
        p1: Pt {
            x: p0.x + 2.0 / 3.0 * (p1.x - p0.x),
            y: p0.y + 2.0 / 3.0 * (p1.y - p0.y),
        },
        p2: Pt {
            x: p2.x + 2.0 / 3.0 * (p1.x - p2.x),
            y: p2.y + 2.0 / 3.0 * (p1.y - p2.y),
        },
        p3: p2.clone(),
    }
}

fn best_split(pts: &[Pt]) -> usize {
    let n = pts.len();
    let (mut best, mut best_cross) = (n / 2, 0.0f64);
    for i in 2..n - 2 {
        let v1x = pts[i].x - pts[i - 2].x;
        let v1y = pts[i].y - pts[i - 2].y;
        let v2x = pts[i + 2].x - pts[i].x;
        let v2y = pts[i + 2].y - pts[i].y;
        let l1 = (v1x * v1x + v1y * v1y).sqrt();
        let l2 = (v2x * v2x + v2y * v2y).sqrt();
        if l1 > 0.0 && l2 > 0.0 {
            let cross = (v1x * v2y - v1y * v2x).abs() / (l1 * l2);
            if cross > best_cross {
                best_cross = cross;
                best = i;
            }
        }
    }
    best.max(2).min(n - 2)
}

fn enforce_g1(curves: &mut [Bezier]) {
    for i in 0..curves.len().saturating_sub(1) {
        let t1x = curves[i].p3.x - curves[i].p2.x;
        let t1y = curves[i].p3.y - curves[i].p2.y;
        let t2x = curves[i + 1].p1.x - curves[i + 1].p0.x;
        let t2y = curves[i + 1].p1.y - curves[i + 1].p0.y;
        let l1 = (t1x * t1x + t1y * t1y).sqrt();
        let l2 = (t2x * t2x + t2y * t2y).sqrt();
        if l1 > 1e-10 && l2 > 1e-10 {
            let scale = l2 / l1;
            let sx = curves[i + 1].p0.x;
            let sy = curves[i + 1].p0.y;
            curves[i + 1].p1 = Pt {
                x: sx + t1x * scale,
                y: sy + t1y * scale,
            };
        }
    }
}

fn clamp_controls(curves: &mut [Bezier], pts: &[Pt]) {
    if curves.is_empty() || pts.is_empty() {
        return;
    }
    let (mut mnx, mut mny, mut mxx, mut mxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in pts {
        mnx = mnx.min(p.x);
        mny = mny.min(p.y);
        mxx = mxx.max(p.x);
        mxy = mxy.max(p.y);
    }
    let margin = ((mxx - mnx).max(mxy - mny) * 0.15).max(2.0);
    let (lx, ly, hx, hy) = (mnx - margin, mny - margin, mxx + margin, mxy + margin);
    for b in curves.iter_mut() {
        b.p1.x = b.p1.x.clamp(lx, hx);
        b.p1.y = b.p1.y.clamp(ly, hy);
        b.p2.x = b.p2.x.clamp(lx, hx);
        b.p2.y = b.p2.y.clamp(ly, hy);
    }
}

// ── SVG 路径序列化 ─────────────────────────────────────────────────────────

/// 将贝塞尔列表转为 SVG path `d` 属性字符串
/// 近直线段用 L，真曲线用 C，连续共线 L 合并
pub fn to_svg_path(curves: &[Bezier], closed: bool) -> String {
    if curves.is_empty() {
        return String::new();
    }
    let mut s = format!("M{},{}", fmt(curves[0].p0.x), fmt(curves[0].p0.y));
    let mut i = 0;
    while i < curves.len() {
        let c = &curves[i];
        if is_linear(c) {
            // 合并连续共线 L 段
            let start = &curves[i].p0;
            let mut end = &c.p3;
            let mut j = i + 1;
            while j < curves.len() {
                let nc = &curves[j];
                if !is_linear(nc) {
                    break;
                }
                let ce = &nc.p3;
                let dx = ce.x - start.x;
                let dy = ce.y - start.y;
                let ll = (dx * dx + dy * dy).sqrt();
                if ll < 0.5 {
                    end = ce;
                    j += 1;
                    continue;
                }
                let d = ((end.y - start.y) * dx - (end.x - start.x) * dy).abs() / ll;
                if d < 1.5 {
                    end = ce;
                    j += 1;
                } else {
                    break;
                }
            }
            s.push_str(&format!("L{},{}", fmt(end.x), fmt(end.y)));
            i = j;
        } else {
            s.push_str(&format!(
                "C{},{} {},{} {},{}",
                fmt(c.p1.x),
                fmt(c.p1.y),
                fmt(c.p2.x),
                fmt(c.p2.y),
                fmt(c.p3.x),
                fmt(c.p3.y)
            ));
            i += 1;
        }
    }
    if closed {
        s.push('Z');
    }
    s
}

fn is_linear(b: &Bezier) -> bool {
    let dx = b.p3.x - b.p0.x;
    let dy = b.p3.y - b.p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return true;
    }
    let d1 = ((b.p1.y - b.p0.y) * dx - (b.p1.x - b.p0.x) * dy).abs() / len;
    let d2 = ((b.p2.y - b.p0.y) * dx - (b.p2.x - b.p0.x) * dy).abs() / len;
    d1 < 1.0 && d2 < 1.0
}

fn fmt(v: f64) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
