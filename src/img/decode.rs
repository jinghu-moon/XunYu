use std::{path::Path, time::Instant};

use image::{DynamicImage, ImageFormat, imageops::FilterType};

use super::{
    avif_runtime::try_decode_avif_via_dll,
    avif_zen_runtime::try_decode_avif_via_zen,
    error::ImgError,
    types::{ProcessParams, calc_scaled_dims},
};

pub struct DecodeResult {
    pub image: DynamicImage,
    pub decode_ms: u64,
    pub resize_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AvifBackend {
    Auto,
    Dll,
    Zen,
    Image,
}

fn avif_debug_enabled() -> bool {
    std::env::var_os("XUN_AVIF_DEBUG").is_some()
}

fn avif_backend_from_env() -> AvifBackend {
    let Ok(raw) = std::env::var("XUN_AVIF_BACKEND") else {
        return AvifBackend::Auto;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "dll" => AvifBackend::Dll,
        "zen" => AvifBackend::Zen,
        "image" => AvifBackend::Image,
        _ => AvifBackend::Auto,
    }
}

fn decode_avif_with_backend(
    bytes: &[u8],
    source_path: &Path,
    decode_with_image: &impl Fn() -> image::ImageResult<DynamicImage>,
) -> Result<DynamicImage, ImgError> {
    let decode_with_fallback = || {
        decode_with_image().map_err(|source| ImgError::DecodeFailed {
            path: source_path.display().to_string(),
            source,
        })
    };

    let backend = avif_backend_from_env();
    if avif_debug_enabled() {
        eprintln!("[img-avif] backend={backend:?}");
    }

    match backend {
        AvifBackend::Image => decode_with_fallback(),
        AvifBackend::Dll => try_decode_avif_via_dll(bytes).map_or_else(decode_with_fallback, Ok),
        AvifBackend::Zen => try_decode_avif_via_zen(bytes)
            .or_else(|| try_decode_avif_via_dll(bytes))
            .map_or_else(decode_with_fallback, Ok),
        AvifBackend::Auto => try_decode_avif_via_dll(bytes)
            .or_else(|| try_decode_avif_via_zen(bytes))
            .map_or_else(decode_with_fallback, Ok),
    }
}

pub fn decode_and_scale_from_memory(
    bytes: &[u8],
    source_path: &Path,
    params: &ProcessParams,
) -> Result<DecodeResult, ImgError> {
    let decode_start = Instant::now();
    let detected_format = ImageFormat::from_path(source_path).ok();
    let decode_with_image = || match detected_format {
        Some(fmt) => image::load_from_memory_with_format(bytes, fmt),
        None => image::load_from_memory(bytes),
    };

    let img = if matches!(detected_format, Some(ImageFormat::Avif)) {
        decode_avif_with_backend(bytes, source_path, &decode_with_image)?
    } else {
        decode_with_image().map_err(|source| ImgError::DecodeFailed {
            path: source_path.display().to_string(),
            source,
        })?
    };
    let decode_ms = decode_start.elapsed().as_millis() as u64;

    let (orig_w, orig_h) = (img.width(), img.height());
    let (new_w, new_h) = calc_scaled_dims(orig_w, orig_h, params.max_width, params.max_height);

    if (new_w, new_h) != (orig_w, orig_h) {
        let resize_start = Instant::now();
        let image = img.resize(new_w, new_h, FilterType::CatmullRom);
        let resize_ms = resize_start.elapsed().as_millis() as u64;
        Ok(DecodeResult {
            image,
            decode_ms,
            resize_ms,
        })
    } else {
        Ok(DecodeResult {
            image: img,
            decode_ms,
            resize_ms: 0,
        })
    }
}
