use super::error::{is_linear_points, max_error};
use super::geom::{Bezier, Pt, dist};
use super::sample::{chord_param, eval, eval_d1, eval_d2};

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
        if is_linear_points(pts, self.tolerance) {
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
        let (mut best_err, mut best_idx) = max_error(&best, pts);

        if best_err <= self.tolerance {
            out.push(best);
            return;
        }

        for _ in 0..self.max_iterations {
            let t2 = self.nr_reparam(&best, pts, &t);
            t = t2;
            let c2 = self.ls_fit(pts, &t);
            let (e2, idx2) = max_error(&c2, pts);
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
