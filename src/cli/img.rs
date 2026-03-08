use argh::FromArgs;

/// image compression and format conversion
#[derive(FromArgs)]
#[argh(subcommand, name = "img")]
pub struct ImgCmd {
    /// input file or directory
    #[argh(option, short = 'i')]
    pub input: String,

    /// output directory (created automatically when missing)
    #[argh(option, short = 'o')]
    pub output: String,

    /// output format [jpeg|png|webp|avif|svg], default webp
    #[argh(option, short = 'f', default = "String::from(\"webp\")")]
    pub format: String,

    /// svg vectorization method [bezier|visioncortex|potrace|skeleton|diffvg], default bezier
    #[argh(option, default = "String::from(\"bezier\")")]
    pub svg_method: String,

    /// diffvg optimizer iterations (only for --svg-method diffvg), default 150
    #[argh(option, default = "150")]
    pub svg_diffvg_iters: usize,

    /// diffvg stroke count (only for --svg-method diffvg), default 64
    #[argh(option, default = "64")]
    pub svg_diffvg_strokes: usize,

    /// jpeg backend [auto|moz|turbo], default auto
    #[argh(option, default = "String::from(\"auto\")")]
    pub jpeg_backend: String,

    /// encode quality 1-100 (ignored by lossless modes), default 80
    #[argh(option, short = 'q', default = "80")]
    pub quality: u8,

    /// png lossy quantization (true=pngquant, false=oxipng)
    #[argh(option, default = "true")]
    pub png_lossy: bool,

    /// png dithering level in lossy mode [0.0-1.0], default 0.0
    #[argh(option, default = "0.0")]
    pub png_dither_level: f32,

    /// webp lossy encoding (true=lossy, false=lossless)
    #[argh(option, default = "true")]
    pub webp_lossy: bool,

    /// max width (keep aspect ratio, never upscale)
    #[argh(option)]
    pub mw: Option<u32>,

    /// max height (keep aspect ratio, never upscale)
    #[argh(option)]
    pub mh: Option<u32>,

    /// worker threads, default cpu core count
    #[argh(option, short = 't')]
    pub threads: Option<usize>,

    /// avif encoder internal threads (default auto)
    #[argh(option)]
    pub avif_threads: Option<usize>,

    /// overwrite existing output files
    #[argh(switch)]
    pub overwrite: bool,
}
