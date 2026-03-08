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

mod approx;
mod options;
mod path;
mod trace;

pub(crate) use options::PotraceConfig;
pub(crate) use trace::trace;
