use image::DynamicImage;

use super::approx::{fit_bezier, opt_curves, optimal_polygon};
use super::options::PotraceConfig;
use super::path::{Seg, erase_path, f, find_first_black, otsu, shoelace, trace_path};

pub(crate) fn trace(img: &DynamicImage, cfg: &PotraceConfig) -> anyhow::Result<String> {
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
