pub(super) fn render(params: &[f64], w: usize, h: usize, sw: f64) -> Vec<f64> {
    // RGB 缓冲（背景白色）
    let mut buf = vec![1.0f64; w * h * 3];
    let ns = params.len() / super::P;
    let sigma = sw * 0.6;
    let sigma2 = 2.0 * sigma * sigma;
    let radius = (3.0 * sigma).ceil() as i32;
    let n_samp = 16usize;

    for s in 0..ns {
        let b = s * super::P;
        let (x0, y0) = (params[b], params[b + 1]);
        let (cx1, cy1) = (params[b + 2], params[b + 3]);
        let (cx2, cy2) = (params[b + 4], params[b + 5]);
        let (x3, y3) = (params[b + 6], params[b + 7]);
        let (r, g, bc, alpha) = (params[b + 8], params[b + 9], params[b + 10], params[b + 11]);

        for si in 0..=n_samp {
            let t = si as f64 / n_samp as f64;
            let mt = 1.0 - t;
            let bx = mt * mt * mt * x0
                + 3.0 * mt * mt * t * cx1
                + 3.0 * mt * t * t * cx2
                + t * t * t * x3;
            let by = mt * mt * mt * y0
                + 3.0 * mt * mt * t * cy1
                + 3.0 * mt * t * t * cy2
                + t * t * t * y3;

            let px_lo = ((bx - radius as f64).max(0.0)) as usize;
            let px_hi = ((bx + radius as f64).min(w as f64 - 1.0)) as usize;
            let py_lo = ((by - radius as f64).max(0.0)) as usize;
            let py_hi = ((by + radius as f64).min(h as f64 - 1.0)) as usize;

            for py in py_lo..=py_hi {
                for px in px_lo..=px_hi {
                    let dx = px as f64 + 0.5 - bx;
                    let dy = py as f64 + 0.5 - by;
                    let w_g = (-(dx * dx + dy * dy) / sigma2).exp() * alpha;
                    if w_g < 0.002 {
                        continue;
                    }
                    let i = (py * w + px) * 3;
                    // Alpha 合成（论文: 软遮罩合成）
                    buf[i] = w_g * r + (1.0 - w_g) * buf[i];
                    buf[i + 1] = w_g * g + (1.0 - w_g) * buf[i + 1];
                    buf[i + 2] = w_g * bc + (1.0 - w_g) * buf[i + 2];
                }
            }
        }
    }
    buf
}

pub(super) fn loss(rendered: &[f64], target: &[f64]) -> f64 {
    rendered
        .iter()
        .zip(target.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        / rendered.len() as f64
}

pub(super) fn clamp(p: &mut [f64], w: usize, h: usize, ns: usize) {
    let (fw, fh) = (w as f64, h as f64);
    for s in 0..ns {
        let b = s * super::P;
        for i in (0..8).step_by(2) {
            p[b + i] = p[b + i].clamp(-fw * 0.05, fw * 1.05);
            p[b + i + 1] = p[b + i + 1].clamp(-fh * 0.05, fh * 1.05);
        }
        for i in 8..12 {
            p[b + i] = p[b + i].clamp(0.01, 1.0);
        }
    }
}
