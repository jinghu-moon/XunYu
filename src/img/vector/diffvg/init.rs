pub(super) fn init_params(rgba: &image::RgbaImage, ns: usize, w: usize, h: usize) -> Vec<f64> {
    let mut p = Vec::with_capacity(ns * super::P);
    let mut rng = super::rng::Rng::new(42);
    let fw = w as f64;
    let fh = h as f64;

    for _ in 0..ns {
        // 在图像上均匀采样颜色
        let px = (rng.next() as usize) % w;
        let py = (rng.next() as usize) % h;
        let c = rgba.get_pixel(px as u32, py as u32);
        let a0 = c[3] as f64 / 255.0;
        let r = if a0 > 0.01 { c[0] as f64 / 255.0 } else { 1.0 };
        let g = if a0 > 0.01 { c[1] as f64 / 255.0 } else { 1.0 };
        let b = if a0 > 0.01 { c[2] as f64 / 255.0 } else { 1.0 };

        // 笔触端点：在图像中随机布局，短笔触
        let x0 = rng.f64() * fw;
        let y0 = rng.f64() * fh;
        let len = (fw.min(fh) * 0.2).max(4.0);
        let angle = rng.f64() * std::f64::consts::TAU;
        let x3 = (x0 + angle.cos() * len).clamp(0.0, fw);
        let y3 = (y0 + angle.sin() * len).clamp(0.0, fh);

        // 控制点初始在直线上加小扰动
        let cx1 = x0 + (x3 - x0) / 3.0 + (rng.f64() - 0.5) * len * 0.3;
        let cy1 = y0 + (y3 - y0) / 3.0 + (rng.f64() - 0.5) * len * 0.3;
        let cx2 = x0 + 2.0 * (x3 - x0) / 3.0 + (rng.f64() - 0.5) * len * 0.3;
        let cy2 = y0 + 2.0 * (y3 - y0) / 3.0 + (rng.f64() - 0.5) * len * 0.3;

        p.extend_from_slice(&[x0, y0, cx1, cy1, cx2, cy2, x3, y3, r, g, b, 0.7]);
    }
    p
}
