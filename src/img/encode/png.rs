use crate::img::error::ImgError;
use std::time::Instant;

pub struct PngLossyEncodeResult {
    pub bytes: Vec<u8>,
    pub pre_ms: u64,
    pub codec_ms: u64,
}

pub fn encode_png_lossy(
    rgba: &[u8],
    width: u32,
    height: u32,
    quality: u8,
    dither_level: f32,
) -> Result<PngLossyEncodeResult, ImgError> {
    let mut liq = imagequant::new();
    let do_encode = |liq: &mut imagequant::Attributes| -> Result<PngLossyEncodeResult, ImgError> {
        liq.set_quality(0, quality)
            .map_err(|e| ImgError::EncodeFailed {
                encoder: "imagequant",
                msg: e.to_string(),
            })?;

        let preprocess_start = Instant::now();
        let pixels: &[imagequant::RGBA] = bytemuck::cast_slice(rgba);
        let mut img = liq
            .new_image_borrowed(pixels, width as usize, height as usize, 0.0)
            .map_err(|e| ImgError::EncodeFailed {
                encoder: "imagequant",
                msg: e.to_string(),
            })?;

        let mut res = liq.quantize(&mut img).map_err(|e| ImgError::EncodeFailed {
            encoder: "imagequant",
            msg: e.to_string(),
        })?;
        res.set_dithering_level(dither_level)
            .map_err(|e| ImgError::EncodeFailed {
                encoder: "imagequant",
                msg: e.to_string(),
            })?;

        let (palette, indexed) = res.remapped(&mut img).map_err(|e| ImgError::EncodeFailed {
            encoder: "lodepng",
            msg: e.to_string(),
        })?;

        let palette: Vec<lodepng::RGBA> = palette
            .iter()
            .map(|c| lodepng::RGBA::new(c.r, c.g, c.b, c.a))
            .collect();
        let pre_ms = preprocess_start.elapsed().as_millis() as u64;

        let codec_start = Instant::now();
        let mut encoder = lodepng::Encoder::new();
        encoder.set_auto_convert(false);
        encoder
            .set_palette(&palette)
            .map_err(|e| ImgError::EncodeFailed {
                encoder: "lodepng",
                msg: e.to_string(),
            })?;

        let bytes = encoder
            .encode(&indexed, width as usize, height as usize)
            .map_err(|e| ImgError::EncodeFailed {
                encoder: "lodepng",
                msg: e.to_string(),
            })?;
        let codec_ms = codec_start.elapsed().as_millis() as u64;

        Ok(PngLossyEncodeResult {
            bytes,
            pre_ms,
            codec_ms,
        })
    };

    do_encode(&mut liq)
}

pub fn encode_png_lossless(png_bytes: &[u8]) -> Result<Vec<u8>, ImgError> {
    oxipng::optimize_from_memory(png_bytes, &oxipng::Options::from_preset(2)).map_err(|e| {
        ImgError::EncodeFailed {
            encoder: "oxipng",
            msg: e.to_string(),
        }
    })
}
