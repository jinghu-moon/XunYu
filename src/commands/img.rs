use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    cli::ImgCmd,
    img::{
        ImgError,
        collect::collect_files,
        process::run,
        report::print_summary,
        types::{JpegBackend, OutputFormat, ProcessParams, SvgMethod},
    },
    output::{CliError, CliResult},
    path_guard::{PathIssueKind, PathPolicy, validate_paths},
};

fn parse_bool_flag(value: &str, field_name: &str) -> Result<bool, CliError> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(CliError::new(
            1,
            format!("Invalid boolean value for {field_name}: '{value}' (expected true/false)"),
        )),
    }
}

pub(crate) fn cmd_img(args: ImgCmd) -> CliResult {
    if args.quality == 0 || args.quality > 100 {
        return Err(CliError::new(
            1,
            ImgError::InvalidQuality {
                value: args.quality,
            }
            .to_string(),
        ));
    }

    if matches!(args.avif_threads, Some(0)) {
        return Err(CliError::new(
            1,
            ImgError::InvalidAvifThreads { value: 0 }.to_string(),
        ));
    }

    if args.svg_diffvg_iters == 0 {
        return Err(CliError::new(
            1,
            ImgError::InvalidSvgDiffvgIters { value: 0 }.to_string(),
        ));
    }

    if args.svg_diffvg_strokes == 0 {
        return Err(CliError::new(
            1,
            ImgError::InvalidSvgDiffvgStrokes { value: 0 }.to_string(),
        ));
    }

    if !args.png_dither_level.is_finite() || !(0.0..=1.0).contains(&args.png_dither_level) {
        return Err(CliError::new(
            1,
            ImgError::InvalidPngDitherLevel {
                value: args.png_dither_level,
            }
            .to_string(),
        ));
    }

    let format = OutputFormat::from_str(&args.format).ok_or_else(|| {
        CliError::new(
            1,
            ImgError::UnsupportedFormat {
                format: args.format.clone(),
            }
            .to_string(),
        )
    })?;

    let jpeg_backend = JpegBackend::from_str(&args.jpeg_backend).ok_or_else(|| {
        CliError::new(
            1,
            ImgError::InvalidJpegBackend {
                backend: args.jpeg_backend.clone(),
            }
            .to_string(),
        )
    })?;

    let svg_method = SvgMethod::from_str(&args.svg_method).ok_or_else(|| {
        CliError::new(
            1,
            ImgError::InvalidSvgMethod {
                method: args.svg_method.clone(),
            }
            .to_string(),
        )
    })?;

    if matches!(format, OutputFormat::Jpeg)
        && !cfg!(any(feature = "img-moz", feature = "img-turbo"))
    {
        return Err(CliError::new(1, ImgError::JpegBackendMissing.to_string()));
    }

    if matches!(format, OutputFormat::Jpeg) && !jpeg_backend.is_compiled() {
        return Err(CliError::new(
            1,
            ImgError::JpegBackendUnavailable {
                backend: jpeg_backend.as_str().to_string(),
                available: JpegBackend::available_for_cli().to_string(),
            }
            .to_string(),
        ));
    }

    let mut input_policy = PathPolicy::for_read();
    input_policy.allow_relative = true;
    let input_validation = validate_paths(vec![args.input.clone()], &input_policy);
    if !input_validation.issues.is_empty() {
        let first = &input_validation.issues[0];
        if first.kind == PathIssueKind::NotFound {
            return Err(CliError::new(
                1,
                ImgError::InputNotFound {
                    path: args.input.clone(),
                }
                .to_string(),
            ));
        }
        let details: Vec<String> = input_validation
            .issues
            .iter()
            .map(|issue| format!("Invalid input path: {} ({})", issue.raw, issue.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid input path.".to_string(),
            &details,
        ));
    }

    let input_path = input_validation
        .ok
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(&args.input));
    let input_path = dunce::canonicalize(&input_path).unwrap_or(input_path);

    let mut output_policy = PathPolicy::for_output();
    output_policy.allow_relative = true;
    let output_validation = validate_paths(vec![args.output.clone()], &output_policy);
    if !output_validation.issues.is_empty() {
        let details: Vec<String> = output_validation
            .issues
            .iter()
            .map(|issue| format!("Invalid output path: {} ({})", issue.raw, issue.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid output path.".to_string(),
            &details,
        ));
    }
    let output_path = output_validation
        .ok
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(&args.output));

    let input_root = if input_path.is_dir() {
        input_path.clone()
    } else {
        input_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| input_path.clone())
    };

    let files = collect_files(&input_path, &output_path);
    if files.is_empty() {
        ui_println!("未找到支持的图片文件");
        return Ok(());
    }
    ui_println!("发现 {} 个文件，开始处理...", files.len());

    let png_lossy = parse_bool_flag(&args.png_lossy, "png_lossy")?;
    let webp_lossy = parse_bool_flag(&args.webp_lossy, "webp_lossy")?;

    let params = ProcessParams {
        format,
        svg_method,
        svg_diffvg_iters: args.svg_diffvg_iters,
        svg_diffvg_strokes: args.svg_diffvg_strokes,
        jpeg_backend,
        quality: args.quality,
        png_lossy,
        png_dither_level: args.png_dither_level,
        webp_lossy,
        max_width: args.mw,
        max_height: args.mh,
        avif_threads: args.avif_threads,
    };

    let wall_start = Instant::now();
    let results = run(
        &files,
        &input_root,
        &output_path,
        &params,
        args.threads,
        args.overwrite,
    );
    let wall_ms = wall_start.elapsed().as_millis() as u64;

    print_summary(&results, wall_ms);

    let failed_count = results.iter().filter(|r| r.error.is_some()).count();
    if failed_count > 0 {
        return Err(CliError::new(1, format!("{failed_count} 个文件处理失败")));
    }

    Ok(())
}
