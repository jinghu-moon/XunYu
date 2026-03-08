pub struct PotraceConfig {
    /// 去噪面积阈值 (turdsize), 默认 2
    pub turd_size: f64,
    /// 角点检测阈值 (alphamax), 0=全角点 1.333=无角点, 默认 1.0
    pub alpha_max: f64,
    /// 曲线优化容差 (opttolerance), 默认 0.2
    pub opt_tolerance: f64,
}

impl Default for PotraceConfig {
    fn default() -> Self {
        Self {
            turd_size: 2.0,
            alpha_max: 1.0,
            opt_tolerance: 0.2,
        }
    }
}
