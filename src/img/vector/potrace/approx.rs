use super::path::{Pt, Seg};

pub(super) fn optimal_polygon(path: &[Pt]) -> Vec<Pt> {
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

pub(super) fn fit_bezier(poly: &[Pt], alpha_max: f64) -> Vec<Seg> {
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

pub(super) fn opt_curves(segs: Vec<Seg>, tol: f64) -> Vec<Seg> {
    if segs.len() < 2 || tol <= 0.0 {
        return segs;
    }
    let mut result = Vec::new();
    let mut i = 0;
    while i < segs.len() {
        if i + 1 < segs.len()
            && let (Seg::Curve(c1a, c1b, mid), Seg::Curve(_c2a, c2b, end)) =
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
        result.push(segs[i].clone());
        i += 1;
    }
    result
}
