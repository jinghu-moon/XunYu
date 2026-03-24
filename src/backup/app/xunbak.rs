use std::path::{Path, PathBuf};

use crate::backup::artifact::output_plan::{
    XunbakOutputPlan, XunbakSingleUpdatePlan, XunbakSplitOutputPlan, XunbakSplitUpdatePlan,
};
use crate::cli::{BackupCmd, VerifyCmd};
use crate::output::{CliError, CliResult};
use crate::xunbak::codec::{CompressionMode, parse_compression_arg};
use crate::xunbak::reader::ContainerReader;
use crate::xunbak::verify::{verify_full_path, verify_paranoid_path, verify_quick_path};
use crate::xunbak::writer::{BackupOptions, ContainerWriter, ProgressEvent};

pub(crate) fn cmd_backup_container(args: &BackupCmd, root: &Path) -> CliResult {
    let container = resolve_container_path(root, args.container.as_deref().unwrap_or_default());
    let options = parse_backup_options(args)?;

    if args.dry_run {
        eprintln!(
            "xunbak dry-run is not implemented yet: {}",
            container.display()
        );
        return Ok(());
    }

    let should_print_progress = estimate_total_files(root) > 100;
    let mut last_reported = 0usize;
    let mut progress = |event: ProgressEvent| {
        if !should_print_progress {
            return;
        }
        if event.processed_files < event.total_files
            && event.processed_files.saturating_sub(last_reported) < 25
        {
            return;
        }
        last_reported = event.processed_files;
        let throughput = if event.elapsed_ms == 0 {
            0.0
        } else {
            event.processed_bytes as f64 / (event.elapsed_ms as f64 / 1000.0)
        };
        eprintln!(
            "xunbak progress: files={}/{} bytes={}/{} throughput={:.0}B/s elapsed={}ms",
            event.processed_files,
            event.total_files,
            event.processed_bytes,
            event.total_bytes,
            throughput,
            event.elapsed_ms
        );
    };

    if container_exists(&container) {
        let result = if options.split_size.is_none() {
            update_single_file_container_with_staging(&container, root, &options, &mut progress)?
        } else {
            update_split_container_with_staging(&container, root, &options, &mut progress)?
        };
        eprintln!(
            "Updated xunbak: {}  files={}  new_blobs={}",
            result.container_path.display(),
            result.file_count,
            result.added_blob_count
        );
    } else {
        let result = if options.split_size.is_none() {
            let plan =
                XunbakOutputPlan::prepare(&container, crate::backup_formats::OverwriteMode::Fail)?;
            let result = ContainerWriter::backup_with_progress(
                plan.temp_path(),
                root,
                &options,
                &mut progress,
            )
            .map_err(|err| CliError::new(2, err.to_string()));
            match result {
                Ok(result) => {
                    plan.finalize()?;
                    result
                }
                Err(err) => {
                    let _ = plan.cleanup();
                    return Err(err);
                }
            }
        } else {
            let plan = XunbakSplitOutputPlan::prepare(
                &container,
                crate::backup_formats::OverwriteMode::Fail,
            )?;
            let result = ContainerWriter::backup_with_progress(
                plan.temp_base_path(),
                root,
                &options,
                &mut progress,
            )
            .map_err(|err| CliError::new(2, err.to_string()));
            match result {
                Ok(result) => {
                    plan.finalize()?;
                    result
                }
                Err(err) => {
                    let _ = plan.cleanup();
                    return Err(err);
                }
            }
        };
        eprintln!(
            "Created xunbak: {}  files={}  blobs={}",
            result.container_path.display(),
            result.file_count,
            result.blob_count
        );
    }
    Ok(())
}

pub(crate) fn cmd_verify(args: VerifyCmd) -> CliResult {
    let path = PathBuf::from(&args.path);
    let level = args
        .level
        .as_deref()
        .unwrap_or("quick")
        .to_ascii_lowercase();
    let report = match level.as_str() {
        "quick" => verify_quick_path(&path),
        "full" => verify_full_path(&path),
        "paranoid" => verify_paranoid_path(&path),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Invalid verify level: {}", args.level.unwrap_or_default()),
                &["Fix: Use quick, full, or paranoid."],
            ));
        }
    };

    if args.json {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|err| CliError::new(2, err.to_string()))?
        );
    } else {
        out_println!("verify: {} ({:?})", path.display(), report.level);
        out_println!("  passed: {}", report.passed);
        out_println!("  blobs: {}", report.stats.blob_count);
        out_println!("  manifest_entries: {}", report.stats.manifest_entries);
        out_println!("  elapsed_ms: {}", report.stats.elapsed_ms);
        if !report.errors.is_empty() {
            for error in &report.errors {
                out_println!("  error: {}", error.message);
            }
        }
    }

    if report.passed {
        Ok(())
    } else {
        Err(CliError::new(1, "xunbak verify failed"))
    }
}

pub(crate) fn restore_container(
    container: &Path,
    target_dir: &Path,
    file: Option<&str>,
    glob: Option<&str>,
) -> CliResult<(usize, usize)> {
    let reader =
        ContainerReader::open(container).map_err(|err| CliError::new(2, err.to_string()))?;
    let result = if let Some(path) = file {
        reader
            .restore_file(path, target_dir)
            .map_err(|err| CliError::new(2, err.to_string()))?
    } else if let Some(pattern) = glob {
        reader
            .restore_glob(pattern, target_dir)
            .map_err(|err| CliError::new(2, err.to_string()))?
    } else {
        reader
            .restore_all(target_dir)
            .map_err(|err| CliError::new(2, err.to_string()))?
    };
    Ok((result.restored_files, 0))
}

fn resolve_container_path(root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn parse_backup_options(args: &BackupCmd) -> Result<BackupOptions, CliError> {
    build_backup_options_from_raw(
        args.compression.as_deref(),
        args.split_size.as_deref(),
        args.no_compress,
    )
}

pub(crate) fn build_backup_options_from_raw(
    compression: Option<&str>,
    split_size: Option<&str>,
    no_compress: bool,
) -> Result<BackupOptions, CliError> {
    if no_compress {
        return Ok(BackupOptions {
            codec: crate::xunbak::constants::Codec::NONE,
            zstd_level: 1,
            split_size: parse_split_size(split_size)?,
        });
    }

    match compression {
        None => Ok(BackupOptions {
            split_size: parse_split_size(split_size)?,
            ..BackupOptions::default()
        }),
        Some(raw) => {
            match parse_compression_arg(raw).map_err(|err| CliError::new(2, err.to_string()))? {
                CompressionMode::None => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::NONE,
                    zstd_level: 1,
                    split_size: parse_split_size(split_size)?,
                }),
                CompressionMode::Zstd { level } => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::ZSTD,
                    zstd_level: level,
                    split_size: parse_split_size(split_size)?,
                }),
                CompressionMode::Lz4 => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::LZ4,
                    zstd_level: 1,
                    split_size: parse_split_size(split_size)?,
                }),
                CompressionMode::Lzma => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::LZMA,
                    zstd_level: 1,
                    split_size: parse_split_size(split_size)?,
                }),
                CompressionMode::Auto => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::ZSTD,
                    zstd_level: 1,
                    split_size: parse_split_size(split_size)?,
                }),
            }
        }
    }
}

fn estimate_total_files(root: &Path) -> usize {
    fn count(dir: &Path) -> usize {
        let mut total = 0usize;
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        total += count(&entry.path());
                    } else if file_type.is_file() {
                        total += 1;
                    }
                }
            }
        }
        total
    }
    count(root)
}

fn update_single_file_container_with_staging(
    container: &Path,
    root: &Path,
    options: &BackupOptions,
    progress: &mut dyn FnMut(ProgressEvent),
) -> CliResult<crate::xunbak::writer::UpdateResult> {
    let plan = XunbakSingleUpdatePlan::prepare(container)?;
    let result = ContainerWriter::update_with_progress(plan.work_path(), root, options, progress)
        .map_err(|err| CliError::new(2, err.to_string()));
    match result {
        Ok(update_result) => {
            plan.commit()?;
            Ok(update_result)
        }
        Err(err) => {
            let _ = plan.rollback();
            Err(err)
        }
    }
}

fn update_split_container_with_staging(
    container: &Path,
    root: &Path,
    options: &BackupOptions,
    progress: &mut dyn FnMut(ProgressEvent),
) -> CliResult<crate::xunbak::writer::UpdateResult> {
    let plan = XunbakSplitUpdatePlan::prepare(container)?;
    let result =
        ContainerWriter::update_with_progress(plan.work_base_path(), root, options, progress)
            .map_err(|err| CliError::new(2, err.to_string()));
    match result {
        Ok(update_result) => {
            plan.commit()?;
            Ok(crate::xunbak::writer::UpdateResult {
                container_path: container.to_path_buf(),
                added_blob_count: update_result.added_blob_count,
                file_count: update_result.file_count,
            })
        }
        Err(err) => {
            let _ = plan.rollback();
            Err(err)
        }
    }
}

fn parse_split_size(raw: Option<&str>) -> Result<Option<u64>, CliError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let upper = value.to_ascii_uppercase();
    let (number, multiplier) = if let Some(stripped) = upper.strip_suffix('K') {
        (stripped, 1024u64)
    } else if let Some(stripped) = upper.strip_suffix('M') {
        (stripped, 1024u64 * 1024)
    } else if let Some(stripped) = upper.strip_suffix('G') {
        (stripped, 1024u64 * 1024 * 1024)
    } else {
        (upper.as_str(), 1u64)
    };
    let size = number
        .parse::<u64>()
        .map_err(|_| CliError::new(2, format!("Invalid split size: {raw}")))?;
    Ok(Some(size.saturating_mul(multiplier)))
}

fn container_exists(path: &Path) -> bool {
    path.exists() || PathBuf::from(format!("{}.001", path.display())).exists()
}
