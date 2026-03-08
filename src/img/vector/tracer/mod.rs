//! img2svg 风格的矢量化管线（移植自 img2svg, Apache-2.0）
//! 流程：中值切割量化 → Marching Squares 轮廓追踪
//!       → Gaussian 平滑 → RDP 简化 → 最小二乘贝塞尔拟合
//!
//! 与 visioncortex 管线的核心区别：
//!   visioncortex: 颜色聚类 → 逐像素路径 → Catmull-Rom 样条
//!   本模块:       中值切割 → 子像素轮廓 → 最小二乘贝塞尔（Newton-Raphson）

mod common;
mod contour;
mod quantize;
mod simplify;

use super::bezier::{BezierFitter, Pt, to_svg_path};
use image::DynamicImage;
use std::collections::HashMap;

use self::common::{hex, polygon_area};
use self::contour::marching_squares;
use self::quantize::{median_cut, quantize};
use self::simplify::{gaussian_smooth, rdp_simplify};

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
