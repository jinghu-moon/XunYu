use crate::img::error::ImgError;

pub fn encode_webp_lossy(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<Vec<u8>, ImgError> {
    Ok(webp::Encoder::from_rgb(rgb, width, height)
        .encode(quality as f32)
        .to_vec())
}

pub fn encode_webp_lossless(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ImgError> {
    Ok(webp::Encoder::from_rgba(rgba, width, height)
        .encode_lossless()
        .to_vec())
}
