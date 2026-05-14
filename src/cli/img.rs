use clap::Args;

/// image compression and format conversion
#[derive(Args, Debug, Clone)]
pub struct ImgCmd {
    /// input file or directory
    #[arg(short = 'i', long)]
    pub input: String,

    /// output directory (created automatically when missing)
    #[arg(short = 'o', long)]
    pub output: String,

    /// output format [jpeg|png|webp|avif|svg], default webp
    #[arg(short = 'f', long, default_value = "webp")]
    pub format: String,

    /// svg vectorization method [bezier|visioncortex|potrace|skeleton|diffvg], default bezier
    #[arg(long, default_value = "bezier")]
    pub svg_method: String,

    /// diffvg optimizer iterations (only for --svg-method diffvg), default 150
    #[arg(long, default_value_t = 150)]
    pub svg_diffvg_iters: usize,

    /// diffvg stroke count (only for --svg-method diffvg), default 64
    #[arg(long, default_value_t = 64)]
    pub svg_diffvg_strokes: usize,

    /// jpeg backend [auto|moz|turbo], default auto
    #[arg(long, default_value = "auto")]
    pub jpeg_backend: String,

    /// encode quality 1-100 (ignored by lossless modes), default 80
    #[arg(short = 'q', long, default_value_t = 80)]
    pub quality: u8,

    /// png lossy quantization (true=pngquant, false=oxipng)
    #[arg(long, default_value = "true")]
    pub png_lossy: String,

    /// png dithering level in lossy mode [0.0-1.0], default 0.0
    #[arg(long, default_value_t = 0.0)]
    pub png_dither_level: f32,

    /// webp lossy encoding (true=lossy, false=lossless)
    #[arg(long, default_value = "true")]
    pub webp_lossy: String,

    /// max width (keep aspect ratio, never upscale)
    #[arg(long)]
    pub mw: Option<u32>,

    /// max height (keep aspect ratio, never upscale)
    #[arg(long)]
    pub mh: Option<u32>,

    /// worker threads, default cpu core count
    #[arg(short = 't', long)]
    pub threads: Option<usize>,

    /// avif encoder internal threads (default auto)
    #[arg(long)]
    pub avif_threads: Option<usize>,

    /// overwrite existing output files
    #[arg(long)]
    pub overwrite: bool,
}
