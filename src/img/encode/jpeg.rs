use super::EncodedBytes;
use crate::img::{error::ImgError, types::JpegBackend};

#[cfg(feature = "img-moz")]
fn encode_jpeg_moz(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<EncodedBytes, ImgError> {
    use mozjpeg::{ColorSpace, Compress};

    let mut comp = Compress::new(ColorSpace::JCS_RGB);
    comp.set_size(width as usize, height as usize);
    comp.set_quality(quality as f32);
    comp.set_progressive_mode();

    let mut started = comp
        .start_compress(Vec::new())
        .map_err(|e| ImgError::EncodeFailed {
            encoder: "mozjpeg",
            msg: e.to_string(),
        })?;

    started
        .write_scanlines(rgb)
        .map_err(|e| ImgError::EncodeFailed {
            encoder: "mozjpeg",
            msg: e.to_string(),
        })?;

    started
        .finish()
        .map(EncodedBytes::Vec)
        .map_err(|e| ImgError::EncodeFailed {
            encoder: "mozjpeg",
            msg: e.to_string(),
        })
}

#[cfg(feature = "img-turbo")]
fn encode_jpeg_turbo(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<EncodedBytes, ImgError> {
    super::turbo_runtime::encode_jpeg_turbo_runtime(rgb, width, height, quality)
        .map(EncodedBytes::Vec)
}

fn encode_jpeg_auto(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<EncodedBytes, ImgError> {
    // 双后端同时启用时，auto 默认优先 turbo（性能优先），失败后回退 moz。
    #[cfg(all(feature = "img-moz", feature = "img-turbo"))]
    {
        if let Ok(bytes) = encode_jpeg_turbo(rgb, width, height, quality) {
            return Ok(bytes);
        }
        encode_jpeg_moz(rgb, width, height, quality)
    }

    #[cfg(all(feature = "img-moz", not(feature = "img-turbo")))]
    {
        return encode_jpeg_moz(rgb, width, height, quality);
    }

    #[cfg(all(feature = "img-turbo", not(feature = "img-moz")))]
    {
        return encode_jpeg_turbo(rgb, width, height, quality);
    }

    #[cfg(not(any(feature = "img-moz", feature = "img-turbo")))]
    {
        Err(ImgError::JpegBackendMissing)
    }
}

fn encode_jpeg_force_moz(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<EncodedBytes, ImgError> {
    let _ = (rgb, width, height, quality);

    #[cfg(feature = "img-moz")]
    {
        encode_jpeg_moz(rgb, width, height, quality)
    }

    #[cfg(not(feature = "img-moz"))]
    {
        Err(ImgError::JpegBackendUnavailable {
            backend: "moz".to_string(),
            available: JpegBackend::available_for_cli().to_string(),
        })
    }
}

fn encode_jpeg_force_turbo(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<EncodedBytes, ImgError> {
    let _ = (rgb, width, height, quality);

    #[cfg(feature = "img-turbo")]
    {
        encode_jpeg_turbo(rgb, width, height, quality)
    }

    #[cfg(not(feature = "img-turbo"))]
    {
        Err(ImgError::JpegBackendUnavailable {
            backend: "turbo".to_string(),
            available: JpegBackend::available_for_cli().to_string(),
        })
    }
}

pub fn encode_jpeg(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
    backend: JpegBackend,
) -> Result<EncodedBytes, ImgError> {
    match backend {
        JpegBackend::Auto => encode_jpeg_auto(rgb, width, height, quality),
        JpegBackend::Moz => encode_jpeg_force_moz(rgb, width, height, quality),
        JpegBackend::Turbo => encode_jpeg_force_turbo(rgb, width, height, quality),
    }
}
