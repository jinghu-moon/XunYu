use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::backup::artifact::dir::write_entries_to_dir;
use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::fs::scan_source_entries;
use crate::backup::artifact::options::BackupCreateOptions;
use crate::backup::artifact::output_plan::{
    DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan,
};
use crate::backup::artifact::progress::{
    ExportProgressEvent, ExportProgressPhase, emit_progress_event, should_emit_progress,
};
use crate::backup::artifact::sevenz::{
    SevenZMethod, SevenZWriteOptions, write_entries_to_7z, write_entries_to_7z_split,
};
use crate::backup::artifact::sidecar::{
    build_sidecar_bytes, source_info_for_create, write_sidecar_to_dir,
};
use crate::backup::artifact::zip::{ZipCompressionMethod, ZipWriteOptions, write_entries_to_zip};
use crate::backup::legacy::{config, util};
use crate::backup_formats::{BackupAction, BackupArtifactFormat, ExportStatus};
use crate::cli::{BackupCmd, BackupCreateCmd};
use crate::output::{CliError, CliResult};
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};

struct BackupScanRules {
    include_roots: Vec<String>,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
}

#[derive(Serialize)]
struct BackupCreateSelectionSummary {
    action: BackupAction,
    status: ExportStatus,
    mode: String,
    source: String,
    destination: Option<String>,
    format: BackupArtifactFormat,
    selected: usize,
    bytes_in: u64,
    entries: Vec<String>,
}

#[derive(Serialize)]
struct BackupCreateExecutionSummary {
    action: BackupAction,
    status: ExportStatus,
    source: String,
    destination: String,
    format: BackupArtifactFormat,
    dry_run: bool,
    selected: usize,
    written: usize,
    skipped: usize,
    bytes_in: u64,
    bytes_out: u64,
    overwrite_count: usize,
    verify_source: String,
    verify_output: String,
    duration_ms: u128,
    outputs: Vec<String>,
}

pub(crate) fn cmd_backup(args: BackupCmd) -> CliResult {
    crate::commands::backup::cmd_backup(args)
}

pub(crate) fn cmd_backup_create(args: BackupCreateCmd) -> CliResult {
    let options = BackupCreateOptions::try_from(args.clone())?;
    if options.list {
        return cmd_backup_create_list(&options);
    }

    match options.format {
        BackupArtifactFormat::Dir => {
            if options.output.is_some() {
                return cmd_backup_create_dir(&options);
            }
            let legacy = BackupCmd {
                cmd: None,
                msg: args.msg,
                dir: args.dir,
                container: None,
                compression: args.compression,
                split_size: args.split_size,
                dry_run: args.dry_run,
                list: false,
                no_compress: true,
                retain: args.retain,
                include: args.include,
                exclude: args.exclude,
                incremental: args.incremental,
                skip_if_unchanged: args.skip_if_unchanged,
            };
            cmd_backup(legacy)
        }
        BackupArtifactFormat::Zip => cmd_backup_create_zip(&options),
        BackupArtifactFormat::SevenZ => cmd_backup_create_7z(&options),
        BackupArtifactFormat::Xunbak => {
            #[cfg(feature = "xunbak")]
            {
                cmd_backup_create_xunbak(&options)
            }
            #[cfg(not(feature = "xunbak"))]
            {
                Err(CliError::with_details(
                    2,
                    "xunbak output is not enabled in this build",
                    &["Fix: Rebuild with `--features xunbak`."],
                ))
            }
        }
    }
}

fn cmd_backup_create_list(options: &BackupCreateOptions) -> CliResult {
    let entries = collect_backup_create_entries(options)?;
    let bytes_in = entries.iter().map(|entry| entry.size).sum();
    let paths: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();

    if options.json {
        let summary = BackupCreateSelectionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            mode: "list".to_string(),
            source: options.source_dir.display().to_string(),
            destination: options
                .output
                .as_ref()
                .map(|path| path.display().to_string()),
            format: options.format,
            selected: paths.len(),
            bytes_in,
            entries: paths,
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary)
                .map_err(|err| CliError::new(1, format!("Serialize list output failed: {err}")))?
        );
        return Ok(());
    }

    emit_selected_entries(&entries);
    eprintln!(
        "Selected {} file(s) / {} bytes from {}",
        entries.len(),
        bytes_in,
        options.source_dir.display()
    );
    Ok(())
}

fn cmd_backup_create_dir(options: &BackupCreateOptions) -> CliResult {
    let started = Instant::now();
    let output = resolve_create_output_path(options)?;
    if paths_equal(&options.source_dir, &output) {
        return Err(CliError::with_details(
            2,
            "backup create --format dir output must differ from source directory",
            &["Fix: Choose a different `--output` path."],
        ));
    }
    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let show_progress = should_emit_progress(options.progress, options.json);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected,
            processed_files: selected,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.dry_run {
        if options.json {
            let summary = BackupCreateExecutionSummary {
                action: BackupAction::Create,
                status: ExportStatus::Ok,
                source: options.source_dir.display().to_string(),
                destination: output.display().to_string(),
                format: options.format,
                dry_run: true,
                selected,
                written: 0,
                skipped: 0,
                bytes_in,
                bytes_out: 0,
                overwrite_count: 0,
                verify_source: "off".to_string(),
                verify_output: "off".to_string(),
                duration_ms: started.elapsed().as_millis(),
                outputs: Vec::new(),
            };
            out_println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|err| {
                    CliError::new(1, format!("Serialize dry-run output failed: {err}"))
                })?
            );
            return Ok(());
        }

        eprintln!(
            "DRY RUN: would create directory backup {} with {} file(s) / {} bytes",
            output.display(),
            selected,
            bytes_in
        );
        return Ok(());
    }

    let plan = DirOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let write_result = (|| -> CliResult<usize> {
        let summary = write_entries_to_dir(&refs, plan.temp_path())?;
        if !options.no_sidecar {
            let sidecar = build_sidecar_bytes(
                options.format,
                &source_info_for_create(&options.source_dir),
                &refs,
            )?;
            write_sidecar_to_dir(plan.temp_path(), &sidecar)?;
        }
        Ok(summary.entry_count)
    })();
    let written = match write_result {
        Ok(written) => written,
        Err(err) => {
            let _ = plan.cleanup();
            return Err(err);
        }
    };
    if let Err(err) = maybe_fail_after_write_for_tests() {
        let _ = plan.cleanup();
        return Err(err);
    }
    plan.finalize()?;
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected,
            processed_files: written,
            bytes_in,
            bytes_out: dir_size_bytes(&output),
            throughput: throughput(bytes_in, started.elapsed()),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.json {
        let summary = BackupCreateExecutionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            source: options.source_dir.display().to_string(),
            destination: output.display().to_string(),
            format: options.format,
            dry_run: false,
            selected,
            written,
            skipped: 0,
            bytes_in,
            bytes_out: dir_size_bytes(&output),
            overwrite_count: 0,
            verify_source: "off".to_string(),
            verify_output: "off".to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: vec![output.display().to_string()],
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).map_err(|err| {
                CliError::new(1, format!("Serialize create dir output failed: {err}"))
            })?
        );
        return Ok(());
    }

    eprintln!(
        "Created directory backup: {}  files={}  bytes={}",
        output.display(),
        written,
        bytes_in
    );
    Ok(())
}

fn cmd_backup_create_7z(options: &BackupCreateOptions) -> CliResult {
    let started = Instant::now();
    let output = resolve_create_output_path(options)?;
    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let split_size = parse_split_size_bytes(options.split_size.as_deref())?;
    let show_progress = should_emit_progress(options.progress, options.json);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected,
            processed_files: selected,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Compress,
            selected_files: selected,
            processed_files: 0,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.dry_run {
        if options.json {
            let summary = BackupCreateExecutionSummary {
                action: BackupAction::Create,
                status: ExportStatus::Ok,
                source: options.source_dir.display().to_string(),
                destination: output.display().to_string(),
                format: options.format,
                dry_run: true,
                selected,
                written: 0,
                skipped: 0,
                bytes_in,
                bytes_out: 0,
                overwrite_count: 0,
                verify_source: "off".to_string(),
                verify_output: "off".to_string(),
                duration_ms: started.elapsed().as_millis(),
                outputs: Vec::new(),
            };
            out_println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|err| {
                    CliError::new(1, format!("Serialize dry-run output failed: {err}"))
                })?
            );
            return Ok(());
        }

        eprintln!(
            "DRY RUN: would create 7z {} with {} file(s) / {} bytes",
            output.display(),
            selected,
            bytes_in
        );
        return Ok(());
    }

    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let sevenz_options = SevenZWriteOptions {
        solid: options.solid,
        method: if options.no_compress {
            SevenZMethod::Copy
        } else {
            sevenz_method_for_create(options.method.as_deref())?
        },
        level: options.level.unwrap_or(1),
        sidecar: if options.no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                options.format,
                &source_info_for_create(&options.source_dir),
                &refs,
            )?)
        },
    };
    let summary = if let Some(split_size) = split_size {
        let plan =
            SevenZSplitOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
        let result =
            write_entries_to_7z_split(&refs, plan.temp_base_path(), split_size, &sevenz_options);
        let summary = match result {
            Ok(summary) => summary,
            Err(err) => {
                let _ = plan.cleanup();
                return Err(err);
            }
        };
        if let Err(err) = maybe_fail_after_write_for_tests() {
            let _ = plan.cleanup();
            return Err(err);
        }
        if let Err(err) = plan.finalize() {
            return Err(err);
        }
        summary
    } else {
        let plan = SevenZOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
        let summary = match write_entries_to_7z(&refs, plan.temp_path(), &sevenz_options) {
            Ok(summary) => summary,
            Err(err) => {
                let _ = plan.cleanup();
                return Err(err);
            }
        };
        if let Err(err) = maybe_fail_after_write_for_tests() {
            let _ = plan.cleanup();
            return Err(err);
        }
        plan.finalize()?;
        summary
    };
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected,
            processed_files: summary.entry_count,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &output),
            throughput: throughput(summary.bytes_in, started.elapsed()),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.json {
        let result = BackupCreateExecutionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            source: options.source_dir.display().to_string(),
            destination: output.display().to_string(),
            format: options.format,
            dry_run: false,
            selected,
            written: summary.entry_count,
            skipped: 0,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &output),
            overwrite_count: 0,
            verify_source: "off".to_string(),
            verify_output: "off".to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: collect_created_output_paths(options.format, &output)
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|err| {
                CliError::new(1, format!("Serialize create 7z output failed: {err}"))
            })?
        );
        return Ok(());
    }

    eprintln!(
        "Created 7z: {}  files={}  bytes={}",
        output.display(),
        summary.entry_count,
        summary.bytes_in
    );
    Ok(())
}

fn cmd_backup_create_zip(options: &BackupCreateOptions) -> CliResult {
    let started = Instant::now();
    let output = resolve_create_output_path(options)?;
    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let show_progress = should_emit_progress(options.progress, options.json);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected,
            processed_files: selected,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Compress,
            selected_files: selected,
            processed_files: 0,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.dry_run {
        if options.json {
            let summary = BackupCreateExecutionSummary {
                action: BackupAction::Create,
                status: ExportStatus::Ok,
                source: options.source_dir.display().to_string(),
                destination: output.display().to_string(),
                format: options.format,
                dry_run: true,
                selected,
                written: 0,
                skipped: 0,
                bytes_in,
                bytes_out: 0,
                overwrite_count: 0,
                verify_source: "off".to_string(),
                verify_output: "off".to_string(),
                duration_ms: started.elapsed().as_millis(),
                outputs: Vec::new(),
            };
            out_println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|err| {
                    CliError::new(1, format!("Serialize dry-run output failed: {err}"))
                })?
            );
            return Ok(());
        }

        eprintln!(
            "DRY RUN: would create zip {} with {} file(s) / {} bytes",
            output.display(),
            selected,
            bytes_in
        );
        return Ok(());
    }

    let plan = ZipOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let zip_options = ZipWriteOptions {
        method: if options.no_compress {
            ZipCompressionMethod::Stored
        } else {
            zip_method_for_create(options.method.as_deref())?
        },
        sidecar: if options.no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                options.format,
                &source_info_for_create(&options.source_dir),
                &refs,
            )?)
        },
    };
    let summary = match write_entries_to_zip(&refs, plan.temp_path(), zip_options) {
        Ok(summary) => summary,
        Err(err) => {
            let _ = plan.cleanup();
            return Err(err);
        }
    };
    if let Err(err) = maybe_fail_after_write_for_tests() {
        let _ = plan.cleanup();
        return Err(err);
    }
    plan.finalize()?;
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected,
            processed_files: summary.entry_count,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &output),
            throughput: throughput(summary.bytes_in, started.elapsed()),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.json {
        let result = BackupCreateExecutionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            source: options.source_dir.display().to_string(),
            destination: output.display().to_string(),
            format: options.format,
            dry_run: false,
            selected,
            written: summary.entry_count,
            skipped: 0,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &output),
            overwrite_count: 0,
            verify_source: "off".to_string(),
            verify_output: "off".to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: collect_created_output_paths(options.format, &output)
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|err| {
                CliError::new(1, format!("Serialize create zip output failed: {err}"))
            })?
        );
        return Ok(());
    }

    eprintln!(
        "Created zip: {}  files={}  bytes={}",
        output.display(),
        summary.entry_count,
        summary.bytes_in
    );
    Ok(())
}

#[cfg(feature = "xunbak")]
fn cmd_backup_create_xunbak(options: &BackupCreateOptions) -> CliResult {
    let started = Instant::now();
    let output = resolve_create_output_path(options)?;
    if paths_equal(&options.source_dir, &output) {
        return Err(CliError::with_details(
            2,
            "backup create --format xunbak output must differ from source directory",
            &["Fix: Choose a different `--output` path."],
        ));
    }

    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let show_progress = should_emit_progress(options.progress, options.json);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected,
            processed_files: selected,
            bytes_in,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.dry_run {
        if options.json {
            let summary = BackupCreateExecutionSummary {
                action: BackupAction::Create,
                status: ExportStatus::Ok,
                source: options.source_dir.display().to_string(),
                destination: output.display().to_string(),
                format: options.format,
                dry_run: true,
                selected,
                written: 0,
                skipped: 0,
                bytes_in,
                bytes_out: 0,
                overwrite_count: 0,
                verify_source: "off".to_string(),
                verify_output: "off".to_string(),
                duration_ms: started.elapsed().as_millis(),
                outputs: Vec::new(),
            };
            out_println!(
                "{}",
                serde_json::to_string_pretty(&summary).map_err(|err| {
                    CliError::new(1, format!("Serialize dry-run output failed: {err}"))
                })?
            );
            return Ok(());
        }

        eprintln!(
            "DRY RUN: would create xunbak {} with {} file(s) / {} bytes",
            output.display(),
            selected,
            bytes_in
        );
        return Ok(());
    }

    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let backup_options = crate::backup::app::xunbak::build_backup_options_from_raw(
        options.compression.as_deref(),
        options.split_size.as_deref(),
        options.no_compress,
    )?;
    let summary = crate::backup::artifact::xunbak::write_entries_to_xunbak(
        &refs,
        &output,
        &backup_options,
        crate::backup_formats::OverwriteMode::Fail,
    )?;
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected,
            processed_files: summary.entry_count,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &summary.destination),
            throughput: throughput(summary.bytes_in, started.elapsed()),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    if options.json {
        let summary = BackupCreateExecutionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            source: options.source_dir.display().to_string(),
            destination: summary.destination.display().to_string(),
            format: options.format,
            dry_run: false,
            selected,
            written: summary.entry_count,
            skipped: 0,
            bytes_in: summary.bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &summary.destination),
            overwrite_count: 0,
            verify_source: "off".to_string(),
            verify_output: "off".to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: collect_created_output_paths(options.format, &summary.destination)
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).map_err(|err| {
                CliError::new(1, format!("Serialize create xunbak output failed: {err}"))
            })?
        );
        return Ok(());
    }

    eprintln!(
        "Created xunbak: {}  files={}  bytes={}",
        summary.destination.display(),
        summary.entry_count,
        summary.bytes_in
    );
    Ok(())
}

fn parse_split_size_bytes(raw: Option<&str>) -> Result<Option<u64>, CliError> {
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

fn zip_method_for_create(method: Option<&str>) -> Result<ZipCompressionMethod, CliError> {
    match method.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(ZipCompressionMethod::Auto),
        Some(value) if value == "deflated" => Ok(ZipCompressionMethod::Deflated),
        Some(value) if value == "stored" => Ok(ZipCompressionMethod::Stored),
        Some(value) => Err(CliError::with_details(
            2,
            format!("backup create --method {value} is invalid for zip"),
            &["Fix: Use `--method stored` or `--method deflated`."],
        )),
    }
}

fn sevenz_method_for_create(method: Option<&str>) -> Result<SevenZMethod, CliError> {
    match method.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(SevenZMethod::Lzma2),
        Some(value) if value == "lzma2" => Ok(SevenZMethod::Lzma2),
        Some(value) if value == "copy" => Ok(SevenZMethod::Copy),
        Some(value) => Err(CliError::with_details(
            2,
            format!("backup create --method {value} is invalid for 7z"),
            &["Fix: Use `--method copy` or `--method lzma2`."],
        )),
    }
}

fn compute_created_output_bytes(format: BackupArtifactFormat, output: &Path) -> u64 {
    match format {
        BackupArtifactFormat::Dir => dir_size_bytes(output),
        BackupArtifactFormat::Zip | BackupArtifactFormat::SevenZ | BackupArtifactFormat::Xunbak => {
            collect_created_output_paths(format, output)
                .into_iter()
                .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
                .sum()
        }
    }
}

fn collect_created_output_paths(format: BackupArtifactFormat, output: &Path) -> Vec<PathBuf> {
    match format {
        BackupArtifactFormat::Xunbak | BackupArtifactFormat::SevenZ => {
            collect_file_or_numbered_outputs(output)
        }
        _ => {
            if output.exists() {
                vec![output.to_path_buf()]
            } else {
                Vec::new()
            }
        }
    }
}

fn collect_file_or_numbered_outputs(output: &Path) -> Vec<PathBuf> {
    if output.exists() {
        return vec![output.to_path_buf()];
    }
    let mut outputs = Vec::new();
    if let Some(parent) = output.parent()
        && let Some(prefix) = output.file_name().and_then(|name| name.to_str())
        && let Ok(read_dir) = fs::read_dir(parent)
    {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("{prefix}."))
                && name[prefix.len() + 1..]
                    .chars()
                    .all(|ch| ch.is_ascii_digit())
            {
                outputs.push(entry.path());
            }
        }
    }
    outputs.sort();
    outputs
}

fn throughput(bytes: u64, elapsed: std::time::Duration) -> u64 {
    let millis = elapsed.as_millis();
    if millis == 0 {
        return bytes;
    }
    ((bytes as u128 * 1000) / millis) as u64
}

fn maybe_fail_after_write_for_tests() -> CliResult {
    if std::env::var_os("XUN_TEST_FAIL_AFTER_WRITE").is_none() {
        return Ok(());
    }
    Err(CliError::with_details(
        1,
        "simulated export failure after write",
        &["Fix: Retry the export; resume is not supported yet."],
    ))
}

fn dir_size_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(read_dir) = fs::read_dir(root) else {
        return 0;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += dir_size_bytes(&path);
        } else if let Ok(meta) = entry.metadata() {
            total += meta.len();
        }
    }
    total
}

fn collect_backup_create_entries(options: &BackupCreateOptions) -> CliResult<Vec<SourceEntry>> {
    let cfg = config::load_config(&options.source_dir);
    let scan_rules = resolve_scan_rules(
        &options.source_dir,
        &cfg,
        &options.include,
        &options.exclude,
    );
    Ok(scan_source_entries(
        &options.source_dir,
        &scan_rules.include_roots,
        &scan_rules.exclude_patterns,
        &scan_rules.include_patterns,
    ))
}

fn resolve_create_output_path(options: &BackupCreateOptions) -> Result<PathBuf, CliError> {
    let output = options.output.as_ref().ok_or_else(|| {
        CliError::with_details(
            2,
            format!(
                "backup create --format {} requires --output",
                options.format
            ),
            &[format!("Fix: Add `-o <target>.{}`.", options.format)],
        )
    })?;
    if output.is_absolute() {
        return Ok(output.clone());
    }
    Ok(options.source_dir.join(output))
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn emit_selected_entries(entries: &[SourceEntry]) {
    for entry in entries {
        out_println!("{}", entry.path);
    }
}

fn resolve_scan_rules(
    root: &Path,
    cfg: &config::BackupConfig,
    cli_include: &[String],
    cli_exclude: &[String],
) -> BackupScanRules {
    let mut include_roots = Vec::new();
    let mut include_patterns = Vec::new();
    for inc in &cfg.include {
        if util::is_glob(inc) {
            include_patterns.push(normalize_glob_path(inc));
        } else {
            include_roots.push(inc.clone());
        }
    }
    for inc in split_csv(cli_include) {
        if util::is_glob(&inc) {
            include_patterns.push(normalize_glob_path(&inc));
        } else {
            include_roots.push(inc);
        }
    }

    let mut exclude_patterns: Vec<String> = cfg
        .exclude
        .iter()
        .map(|entry| normalize_glob_path(entry))
        .collect();
    exclude_patterns.extend(
        split_csv(cli_exclude)
            .into_iter()
            .map(|entry| normalize_glob_path(&entry)),
    );
    if cfg.use_gitignore {
        let ignore = read_ignore_file(&root.join(".gitignore"));
        exclude_patterns.extend(ignore.exclude);
        include_patterns.extend(ignore.include);
    }

    BackupScanRules {
        include_roots,
        include_patterns,
        exclude_patterns,
    }
}
