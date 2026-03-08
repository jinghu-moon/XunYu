use std::io::Cursor;
use std::time::Instant;

use image::DynamicImage;

use super::{
    error::ImgError,
    types::{OutputFormat, ProcessParams},
};

mod avif;
mod jpeg;
mod png;
mod svg;
#[cfg(feature = "img-turbo")]
mod turbo_runtime;
mod webp;

#[derive(Debug, Clone, Copy, Default)]
pub struct EncodeOptions {
    pub avif_num_threads: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EncodeTimingsMs {
    pub pixel_convert_ms: u64,
    pub encode_pre_ms: u64,
    pub codec_ms: u64,
    pub png_optimize_ms: u64,
    pub svg_trace_ms: u64,
    pub svg_serialize_ms: u64,
    pub svg_trace_internal_ms: u64,
    pub svg_vc_to_color_ms: u64,
    pub svg_vc_keying_ms: u64,
    pub svg_vc_cluster_ms: u64,
    pub svg_vc_cluster_quantize_ms: u64,
    pub svg_vc_cluster_label_ms: u64,
    pub svg_vc_cluster_stats_ms: u64,
    pub svg_vc_cluster_merge_ms: u64,
    pub svg_vc_cluster_finalize_ms: u64,
    pub svg_vc_path_build_ms: u64,
    pub svg_vc_path_sort_ms: u64,
    pub svg_vc_path_trace_ms: u64,
    pub svg_vc_path_smooth_ms: u64,
    pub svg_vc_path_svg_emit_ms: u64,
    pub svg_vc_path_components_total: u64,
    pub svg_vc_path_components_simplified: u64,
    pub svg_vc_path_components_smoothed: u64,
    pub svg_vc_wrap_ms: u64,
}

pub enum EncodedBytes {
    Vec(Vec<u8>),
}

impl EncodedBytes {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Vec(v) => v.as_slice(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Vec(v) => v.len(),
        }
    }
}

pub struct EncodeResult {
    pub bytes: EncodedBytes,
    pub timings: EncodeTimingsMs,
}

pub(crate) fn encode_png_lossless_bytes(png_bytes: &[u8]) -> Result<Vec<u8>, ImgError> {
    png::encode_png_lossless(png_bytes)
}

pub fn encode(
    img: &DynamicImage,
    params: &ProcessParams,
    options: EncodeOptions,
) -> Result<EncodeResult, ImgError> {
    let (w, h) = (img.width(), img.height());
    let mut timings = EncodeTimingsMs::default();

    match params.format {
        OutputFormat::Jpeg => {
            let bytes = if let Some(rgb) = img.as_rgb8() {
                let codec_start = Instant::now();
                let bytes =
                    jpeg::encode_jpeg(rgb.as_raw(), w, h, params.quality, params.jpeg_backend)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            } else {
                let convert_start = Instant::now();
                let rgb = img.to_rgb8();
                timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                let codec_start = Instant::now();
                let bytes =
                    jpeg::encode_jpeg(rgb.as_raw(), w, h, params.quality, params.jpeg_backend)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            };

            Ok(EncodeResult { bytes, timings })
        }
        OutputFormat::Png if params.png_lossy => {
            let encoded = if let Some(rgba) = img.as_rgba8() {
                png::encode_png_lossy(rgba.as_raw(), w, h, params.quality, params.png_dither_level)?
            } else {
                let convert_start = Instant::now();
                let rgba = img.to_rgba8();
                timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                png::encode_png_lossy(rgba.as_raw(), w, h, params.quality, params.png_dither_level)?
            };
            timings.encode_pre_ms = encoded.pre_ms;
            timings.codec_ms = encoded.codec_ms;

            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(encoded.bytes),
                timings,
            })
        }
        OutputFormat::Png => {
            let codec_start = Instant::now();
            let mut cursor = Cursor::new(Vec::<u8>::new());
            img.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| ImgError::EncodeFailed {
                    encoder: "image-png",
                    msg: e.to_string(),
                })?;
            timings.codec_ms = codec_start.elapsed().as_millis() as u64;

            let optimize_start = Instant::now();
            let bytes = encode_png_lossless_bytes(&cursor.into_inner())?;
            timings.png_optimize_ms = optimize_start.elapsed().as_millis() as u64;

            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(bytes),
                timings,
            })
        }
        OutputFormat::WebP if params.webp_lossy => {
            let bytes = if let Some(rgb) = img.as_rgb8() {
                let codec_start = Instant::now();
                let bytes = webp::encode_webp_lossy(rgb.as_raw(), w, h, params.quality)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            } else {
                let convert_start = Instant::now();
                let rgb = img.to_rgb8();
                timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                let codec_start = Instant::now();
                let bytes = webp::encode_webp_lossy(rgb.as_raw(), w, h, params.quality)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            };

            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(bytes),
                timings,
            })
        }
        OutputFormat::WebP => {
            let bytes = if let Some(rgba) = img.as_rgba8() {
                let codec_start = Instant::now();
                let bytes = webp::encode_webp_lossless(rgba.as_raw(), w, h)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            } else {
                let convert_start = Instant::now();
                let rgba = img.to_rgba8();
                timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                let codec_start = Instant::now();
                let bytes = webp::encode_webp_lossless(rgba.as_raw(), w, h)?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            };

            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(bytes),
                timings,
            })
        }
        OutputFormat::Svg => {
            let codec_start = Instant::now();
            let (bytes, svg_timings) = svg::encode_svg_with_timings(
                img,
                params.svg_method,
                params.svg_diffvg_iters,
                params.svg_diffvg_strokes,
            )?;
            timings.codec_ms = codec_start.elapsed().as_millis() as u64;
            timings.svg_trace_ms = svg_timings.svg_trace_ms;
            timings.svg_serialize_ms = svg_timings.svg_serialize_ms;
            timings.svg_trace_internal_ms = svg_timings.svg_trace_internal_ms;
            timings.svg_vc_to_color_ms = svg_timings.svg_vc_to_color_ms;
            timings.svg_vc_keying_ms = svg_timings.svg_vc_keying_ms;
            timings.svg_vc_cluster_ms = svg_timings.svg_vc_cluster_ms;
            timings.svg_vc_cluster_quantize_ms = svg_timings.svg_vc_cluster_quantize_ms;
            timings.svg_vc_cluster_label_ms = svg_timings.svg_vc_cluster_label_ms;
            timings.svg_vc_cluster_stats_ms = svg_timings.svg_vc_cluster_stats_ms;
            timings.svg_vc_cluster_merge_ms = svg_timings.svg_vc_cluster_merge_ms;
            timings.svg_vc_cluster_finalize_ms = svg_timings.svg_vc_cluster_finalize_ms;
            timings.svg_vc_path_build_ms = svg_timings.svg_vc_path_build_ms;
            timings.svg_vc_path_sort_ms = svg_timings.svg_vc_path_sort_ms;
            timings.svg_vc_path_trace_ms = svg_timings.svg_vc_path_trace_ms;
            timings.svg_vc_path_smooth_ms = svg_timings.svg_vc_path_smooth_ms;
            timings.svg_vc_path_svg_emit_ms = svg_timings.svg_vc_path_svg_emit_ms;
            timings.svg_vc_path_components_total = svg_timings.svg_vc_path_components_total;
            timings.svg_vc_path_components_simplified =
                svg_timings.svg_vc_path_components_simplified;
            timings.svg_vc_path_components_smoothed = svg_timings.svg_vc_path_components_smoothed;
            timings.svg_vc_wrap_ms = svg_timings.svg_vc_wrap_ms;
            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(bytes),
                timings,
            })
        }
        OutputFormat::Avif => {
            let has_alpha = img.color().has_alpha();
            let bytes = if has_alpha {
                if let Some(rgba) = img.as_rgba8() {
                    let codec_start = Instant::now();
                    let bytes = avif::encode_avif_rgba(
                        rgba.as_raw(),
                        w as usize,
                        h as usize,
                        params.quality,
                        options.avif_num_threads,
                    )?;
                    timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                    bytes
                } else {
                    let convert_start = Instant::now();
                    let rgba = img.to_rgba8();
                    timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                    let codec_start = Instant::now();
                    let bytes = avif::encode_avif_rgba(
                        rgba.as_raw(),
                        w as usize,
                        h as usize,
                        params.quality,
                        options.avif_num_threads,
                    )?;
                    timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                    bytes
                }
            } else if let Some(rgb) = img.as_rgb8() {
                let codec_start = Instant::now();
                let bytes = avif::encode_avif_rgb(
                    rgb.as_raw(),
                    w as usize,
                    h as usize,
                    params.quality,
                    options.avif_num_threads,
                )?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            } else {
                let convert_start = Instant::now();
                let rgb = img.to_rgb8();
                timings.pixel_convert_ms = convert_start.elapsed().as_millis() as u64;

                let codec_start = Instant::now();
                let bytes = avif::encode_avif_rgb(
                    rgb.as_raw(),
                    w as usize,
                    h as usize,
                    params.quality,
                    options.avif_num_threads,
                )?;
                timings.codec_ms = codec_start.elapsed().as_millis() as u64;
                bytes
            };

            Ok(EncodeResult {
                bytes: EncodedBytes::Vec(bytes),
                timings,
            })
        }
    }
}
