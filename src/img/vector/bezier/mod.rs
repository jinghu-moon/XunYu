//! 最小二乘三次贝塞尔曲线拟合器
//! 完整移植自 img2svg (Apache-2.0, Ying Kit WONG)
//! 算法：弦长参数化 → 最小二乘拟合 → Newton-Raphson 重参数化 → G1 连续性强制
//!
//! 与 visioncortex 样条的核心区别：
//!   - visioncortex：Catmull-Rom 样条，各段独立拟合
//!   - 本模块：全局最小二乘 + NR 迭代，G1 连续，子像素精度

mod error;
mod fit;
mod geom;
mod sample;

pub use fit::BezierFitter;
#[allow(unused_imports)]
pub use geom::{Bezier, Pt, to_svg_path};
