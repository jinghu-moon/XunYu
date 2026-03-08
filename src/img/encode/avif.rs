use crate::img::error::ImgError;

fn encoder(quality: u8, num_threads: Option<usize>) -> ravif::Encoder {
    ravif::Encoder::new()
        .with_quality(quality as f32)
        .with_speed(7)
        // 8-bit 输入场景下改为 8-bit 编码，减少无收益的 10-bit 处理开销。
        .with_bit_depth(ravif::BitDepth::Eight)
        .with_num_threads(num_threads)
}

pub fn encode_avif_rgba(
    rgba: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    num_threads: Option<usize>,
) -> Result<Vec<u8>, ImgError> {
    let enc = encoder(quality, num_threads);

    let pixels: &[ravif::RGBA8] = bytemuck::cast_slice(rgba);
    let img = ravif::Img::new(pixels, width, height);

    enc.encode_rgba(img)
        .map(|res| res.avif_file)
        .map_err(|e| ImgError::EncodeFailed {
            encoder: "ravif",
            msg: e.to_string(),
        })
}

pub fn encode_avif_rgb(
    rgb: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    num_threads: Option<usize>,
) -> Result<Vec<u8>, ImgError> {
    let enc = encoder(quality, num_threads);

    let pixels: &[ravif::RGB8] = bytemuck::cast_slice(rgb);
    let img = ravif::Img::new(pixels, width, height);

    enc.encode_rgb(img)
        .map(|res| res.avif_file)
        .map_err(|e| ImgError::EncodeFailed {
            encoder: "ravif",
            msg: e.to_string(),
        })
}
