//! DiffVG-lite: 可微分矢量图优化（纯 CPU 实现）
//!
//! 原论文: Tzu-Mao Li, Michal Lukáč, Michaël Gharbi, Jonathan Ragan-Kelley
//!         "Differentiable Vector Graphics Rasterization for Editing and Learning"
//!         ACM SIGGRAPH Asia 2020, ACM Trans. Graphics 39(6)
//!         https://people.csail.mit.edu/tzumao/diffvg/
//!
//! 论文核心思想:
//!   向量图光栅化在像素预滤波后是可微分的。
//!   通过反向传播可以对贝塞尔曲线参数（控制点、颜色、透明度）做梯度下降。
//!
//! 本实现的 CPU 近似说明:
//!   原始 DiffVG: 解析反传播 + CUDA 并行 + 精确抗锯齿边界梯度
//!   本版本:
//!     - 光栅化: 沿贝塞尔曲线采样 2D Gaussian（近似 DiffVG 的 soft rasterization）
//!     - 梯度估计: SPSA（Simultaneous Perturbation Stochastic Approximation）
//!       每次迭代仅需 2 次前向渲染（而非参数数量×2次），速度提升 ~200×
//!     - 优化器: Adam (β₁=0.9, β₂=0.999)
//!     - 初始化: 从图像颜色采样，智能布局笔触

mod init;
mod render;
mod rng;
mod svg;

use image::DynamicImage;

use self::init::init_params;
use self::render::{clamp, loss, render};
use self::rng::Rng;
use self::svg::to_svg;

pub struct DiffvgConfig {
    /// 贝塞尔笔触数量（越多越精细，越慢）
    pub num_strokes: usize,
    /// 优化迭代次数
    pub iterations: usize,
    /// Adam 学习率
    pub learning_rate: f64,
    /// 笔触宽度（像素）
    pub stroke_width: f64,
    /// SPSA 扰动步长
    pub spsa_c: f64,
}

impl Default for DiffvgConfig {
    fn default() -> Self {
        Self {
            num_strokes: 64,
            iterations: 150,
            learning_rate: 0.8,
            stroke_width: 2.0,
            spsa_c: 1.0,
        }
    }
}

// 每条笔触参数: [x0,y0, cx1,cy1, cx2,cy2, x3,y3, r,g,b, alpha] = 12 个
pub(super) const P: usize = 12;

pub fn trace(img: &DynamicImage, cfg: &DiffvgConfig) -> anyhow::Result<String> {
    let rgba = img.to_rgba8();
    let w = rgba.width() as usize;
    let h = rgba.height() as usize;

    // 目标像素（归一化 RGB，忽略 alpha 通道）
    let target: Vec<f64> = rgba
        .pixels()
        .flat_map(|p| {
            let a = p[3] as f64 / 255.0;
            // 预乘 alpha 以便与白背景合成比较
            let r = p[0] as f64 / 255.0 * a + (1.0 - a);
            let g = p[1] as f64 / 255.0 * a + (1.0 - a);
            let b = p[2] as f64 / 255.0 * a + (1.0 - a);
            [r, g, b]
        })
        .collect();

    // 1. 初始化参数（从图像采样颜色，沿图像重要区域布局）
    let mut params = init_params(&rgba, cfg.num_strokes, w, h);

    // 2. Adam 状态
    let n = params.len();
    let mut m = vec![0.0f64; n]; // 一阶矩
    let mut v_adam = vec![0.0f64; n]; // 二阶矩
    let (b1, b2, eps) = (0.9, 0.999, 1e-8);

    // 3. SPSA + Adam 优化（论文核心思想的 CPU 近似）
    // SPSA: 用随机扰动向量 Δ 估计梯度，每次只需 2 次前向渲染
    // 参考: J. C. Spall, "An Overview of the Simultaneous Perturbation Method for
    //        Efficient Optimization", 1998.
    let mut rng = Rng::new(12345);
    let mut best_params = params.clone();
    let mut best_loss = f64::INFINITY;

    for iter in 0..cfg.iterations {
        let t = iter + 1;

        // SPSA 参数衰减（标准 SPSA 调参）
        let ak = cfg.learning_rate / ((t + 10) as f64).powf(0.602);
        let ck = cfg.spsa_c / (t as f64).powf(0.101);

        // 随机 ±1 扰动向量
        let delta: Vec<f64> = (0..n)
            .map(|_| if rng.next_bool() { 1.0 } else { -1.0 })
            .collect();

        // 正向和负向参数
        let mut pp = params.clone();
        let mut pm = params.clone();
        for i in 0..n {
            pp[i] += ck * delta[i];
            pm[i] -= ck * delta[i];
        }
        clamp(&mut pp, w, h, cfg.num_strokes);
        clamp(&mut pm, w, h, cfg.num_strokes);

        // 两次前向渲染
        let lp = loss(&render(&pp, w, h, cfg.stroke_width), &target);
        let lm = loss(&render(&pm, w, h, cfg.stroke_width), &target);

        // SPSA 梯度估计: gₖ = (L(θ+cΔ) - L(θ-cΔ)) / (2c·Δᵢ)
        // 因 Δᵢ = ±1, 1/Δᵢ = Δᵢ
        let diff = (lp - lm) / (2.0 * ck);
        let grads: Vec<f64> = (0..n).map(|i| diff * delta[i]).collect();

        // Adam 更新
        let tf = t as f64;
        for i in 0..n {
            m[i] = b1 * m[i] + (1.0 - b1) * grads[i];
            v_adam[i] = b2 * v_adam[i] + (1.0 - b2) * grads[i].powi(2);
            let m_hat = m[i] / (1.0 - b1.powf(tf));
            let v_hat = v_adam[i] / (1.0 - b2.powf(tf));
            params[i] -= ak * m_hat / (v_hat.sqrt() + eps);
        }
        clamp(&mut params, w, h, cfg.num_strokes);

        // 记录最优
        let cur_loss = (lp + lm) / 2.0;
        if cur_loss < best_loss {
            best_loss = cur_loss;
            best_params = params.clone();
        }

        if iter % 30 == 0 || iter == cfg.iterations - 1 {
            eprintln!(
                "  DiffVG iter {:>4}/{}: loss={:.6}",
                iter + 1,
                cfg.iterations,
                cur_loss
            );
        }
    }

    Ok(to_svg(
        &best_params,
        w,
        h,
        cfg.num_strokes,
        cfg.stroke_width,
    ))
}
