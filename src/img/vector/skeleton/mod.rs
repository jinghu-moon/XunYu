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

mod common;
mod paths;
mod thin;
mod threshold;

use super::bezier::{BezierFitter, Pt, to_svg_path};
use image::DynamicImage;

use self::paths::extract_paths;
use self::thin::zhang_suen;
use self::threshold::otsu;

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
