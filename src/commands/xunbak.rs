use std::path::{Path, PathBuf};

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
        if event.processed_files < event.total_files && event.processed_files.saturating_sub(last_reported) < 25 {
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

    if container.exists() {
        let result = ContainerWriter::update_with_progress(&container, root, &options, &mut progress)
            .map_err(|err| CliError::new(2, err.to_string()))?;
        eprintln!(
            "Updated xunbak: {}  files={}  new_blobs={}",
            result.container_path.display(),
            result.file_count,
            result.added_blob_count
        );
    } else {
        let result = ContainerWriter::backup_with_progress(&container, root, &options, &mut progress)
            .map_err(|err| CliError::new(2, err.to_string()))?;
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
    if args.no_compress {
        return Ok(BackupOptions {
            codec: crate::xunbak::constants::Codec::NONE,
            zstd_level: 1,
        });
    }

    match args.compression.as_deref() {
        None => Ok(BackupOptions::default()),
        Some(raw) => {
            match parse_compression_arg(raw).map_err(|err| CliError::new(2, err.to_string()))? {
                CompressionMode::None => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::NONE,
                    zstd_level: 1,
                }),
                CompressionMode::Zstd { level } => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::ZSTD,
                    zstd_level: level,
                }),
                CompressionMode::Lz4 => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::LZ4,
                    zstd_level: 1,
                }),
                CompressionMode::Lzma => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::LZMA,
                    zstd_level: 1,
                }),
                CompressionMode::Auto => Ok(BackupOptions {
                    codec: crate::xunbak::constants::Codec::ZSTD,
                    zstd_level: 1,
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
