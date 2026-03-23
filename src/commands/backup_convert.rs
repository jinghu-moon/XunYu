use std::fs;
use std::path::Path;
use std::time::Instant;
#[cfg(feature = "xunbak")]
use std::time::{SystemTime, UNIX_EPOCH};

use dialoguer::Confirm;
use serde::Serialize;
#[cfg(feature = "xunbak")]
use uuid::Uuid;

use crate::backup_export::artifact_source::read_artifact_entries;
use crate::backup_export::dir_writer::write_entries_to_dir;
use crate::backup_export::options::BackupConvertOptions;
use crate::backup_export::output_plan::{
    DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan,
};
use crate::backup_export::progress::{
    ExportProgressEvent, ExportProgressPhase, emit_progress_event, should_emit_progress,
};
#[cfg(feature = "xunbak")]
use crate::backup_export::reader::copy_entry_to_path;
use crate::backup_export::sevenz_io::{
    SevenZMethod, SevenZWriteOptions, write_entries_to_7z,
    write_entries_to_7z_split,
};
use crate::backup_export::selection::select_entries;
use crate::backup_export::sidecar::{
    build_sidecar_bytes, source_info_for_convert, write_sidecar_to_dir,
};
use crate::backup_export::verify::{verify_convert_source, verify_output};
use crate::backup_export::zip_writer::{
    ZipCompressionMethod, ZipWriteOptions, write_entries_to_zip,
};
use crate::backup_formats::{
    BackupAction, BackupArtifactFormat, ExportStatus, OverwriteMode,
};
use crate::cli::BackupConvertCmd;
use crate::output::{CliError, CliResult, can_interact};

#[derive(Serialize)]
struct BackupConvertSelectionSummary {
    action: BackupAction,
    status: ExportStatus,
    mode: String,
    source: String,
    destination: String,
    format: String,
    dry_run: bool,
    selected: usize,
    skipped: usize,
    bytes_in: u64,
    bytes_out: u64,
    overwrite_count: usize,
    verify_source: String,
    verify_output: String,
    duration_ms: u128,
    outputs: Vec<String>,
    entries: Vec<String>,
}

#[derive(Serialize)]
struct BackupConvertExecutionSummary {
    action: BackupAction,
    status: ExportStatus,
    source: String,
    destination: String,
    format: String,
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

#[derive(Serialize)]
struct BackupConvertFailureSummary {
    action: BackupAction,
    status: ExportStatus,
    source: String,
    destination: String,
    format: String,
    error: String,
    dry_run: bool,
    overwrite_count: usize,
    verify_source: String,
    verify_output: String,
    duration_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BackupConvertWriteResult {
    selected: usize,
    written: usize,
    skipped: usize,
    bytes_in: u64,
    bytes_out: u64,
    overwrite_count: usize,
}

pub(crate) fn cmd_backup_convert(args: BackupConvertCmd) -> CliResult {
    let started = Instant::now();
    let options = BackupConvertOptions::try_from(args)?;
    let show_progress = should_emit_progress(options.progress, options.json);
    if options.list || options.dry_run {
        return preview_backup_convert(&options, started);
    }
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::VerifySource,
            selected_files: 0,
            processed_files: 0,
            bytes_in: 0,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let Err(err) = verify_convert_source(&options.artifact, options.verify_source) {
        emit_backup_convert_failure_json(
            &options,
            ExportStatus::PreflightFailed,
            &err,
            started.elapsed().as_millis(),
        );
        return Err(err);
    }

    let result = match options.format {
        BackupArtifactFormat::Dir => execute_backup_convert_to_dir(&options),
        BackupArtifactFormat::Zip => execute_backup_convert_to_zip(&options),
        BackupArtifactFormat::SevenZ => execute_backup_convert_to_7z(&options),
        BackupArtifactFormat::Xunbak => execute_backup_convert_to_xunbak(&options),
    };
    let result = match result {
        Ok(result) => result,
        Err(err) => {
            emit_backup_convert_failure_json(
                &options,
                ExportStatus::WriteFailed,
                &err,
                started.elapsed().as_millis(),
            );
            return Err(err);
        }
    };
    if let Err(err) = maybe_corrupt_output_for_tests(&options) {
        emit_backup_convert_failure_json(
            &options,
            ExportStatus::WriteFailed,
            &err,
            started.elapsed().as_millis(),
        );
        return Err(err);
    }
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::VerifyOutput,
            selected_files: result.selected,
            processed_files: result.written,
            bytes_in: result.bytes_in,
            bytes_out: result.bytes_out,
            throughput: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let Err(err) = verify_output(options.format, &options.output, options.verify_output) {
        emit_backup_convert_failure_json(
            &options,
            ExportStatus::VerifyFailed,
            &err,
            started.elapsed().as_millis(),
        );
        return Err(err);
    }
    if options.json {
        let summary = BackupConvertExecutionSummary {
            action: BackupAction::Convert,
            status: ExportStatus::Ok,
            source: options.artifact.display().to_string(),
            destination: options.output.display().to_string(),
            format: options.format.to_string(),
            dry_run: false,
            selected: result.selected,
            written: result.written,
            skipped: result.skipped,
            bytes_in: result.bytes_in,
            bytes_out: result.bytes_out,
            overwrite_count: result.overwrite_count,
            verify_source: options.verify_source.to_string(),
            verify_output: options.verify_output.to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: collect_output_paths(&options)
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).map_err(|err| {
                CliError::new(1, format!("Serialize backup convert result failed: {err}"))
            })?
        );
    }
    Ok(())
}

fn preview_backup_convert(options: &BackupConvertOptions, started: Instant) -> CliResult {
    let entries = read_artifact_entries(&options.artifact)?;
    let selected = select_entries(&entries, &options.selection);
    let bytes_in = selected.iter().map(|entry| entry.size).sum();
    let paths: Vec<String> = selected.iter().map(|entry| entry.path.clone()).collect();
    let mode = if options.list { "list" } else { "dry_run" };

    if options.json {
        let summary = BackupConvertSelectionSummary {
            action: BackupAction::Convert,
            status: ExportStatus::Ok,
            mode: mode.to_string(),
            source: options.artifact.display().to_string(),
            destination: options.output.display().to_string(),
            format: options.format.to_string(),
            dry_run: options.dry_run,
            selected: paths.len(),
            skipped: 0,
            bytes_in,
            bytes_out: 0,
            overwrite_count: existing_output_count(options.format, &options.output),
            verify_source: options.verify_source.to_string(),
            verify_output: options.verify_output.to_string(),
            duration_ms: started.elapsed().as_millis(),
            outputs: Vec::new(),
            entries: paths,
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).map_err(|err| {
                CliError::new(1, format!("Serialize backup convert preview failed: {err}"))
            })?
        );
        return Ok(());
    }

    eprintln!(
        "Selected {} entry(ies) / {} bytes from {} -> {} ({})",
        paths.len(),
        bytes_in,
        options.artifact.display(),
        options.output.display(),
        options.format
    );
    if options.list {
        for path in &paths {
            out_println!("{path}");
        }
    }
    Ok(())
}

fn execute_backup_convert_to_dir(
    options: &BackupConvertOptions,
) -> CliResult<BackupConvertWriteResult> {
    let phase_started = Instant::now();
    let entries = read_artifact_entries(&options.artifact)?;
    let selected = select_entries(&entries, &options.selection);
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected.len(),
            processed_files: selected.len(),
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Compress,
            selected_files: selected.len(),
            processed_files: 0,
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }
    let plan = prepare_dir_output(options)?;

    let write_result = (|| -> CliResult<_> {
        let summary = write_entries_to_dir(&selected, plan.temp_path())?;
        if !options.no_sidecar {
            let sidecar = build_sidecar_bytes(
                options.format,
                &source_info_for_convert(&options.artifact),
                &selected,
            )?;
            write_sidecar_to_dir(plan.temp_path(), &sidecar)?;
        }
        Ok(summary)
    })();
    let summary = match write_result {
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
    let written = summary.entry_count;
    let bytes_in = summary.bytes_in;
    let bytes_out = compute_output_bytes(BackupArtifactFormat::Dir, &options.output);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected.len(),
            processed_files: written,
            bytes_in,
            bytes_out,
            throughput: throughput(bytes_in, phase_started.elapsed()),
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }

    if !options.json {
        eprintln!(
            "Converted {} entry(ies) / {} bytes from {} -> {} ({})",
            written,
            bytes_in,
            options.artifact.display(),
            options.output.display(),
            options.format
        );
    }
    Ok(BackupConvertWriteResult {
        selected: written,
        written,
        skipped: 0,
        bytes_in,
        bytes_out,
        overwrite_count: 0,
    })
}

fn execute_backup_convert_to_zip(
    options: &BackupConvertOptions,
) -> CliResult<BackupConvertWriteResult> {
    if paths_equal(&options.artifact, &options.output) {
        return Err(CliError::with_details(
            2,
            "backup convert source and destination must be different",
            &["Fix: Choose a different `--output` path."],
        ));
    }

    let phase_started = Instant::now();
    let entries = read_artifact_entries(&options.artifact)?;
    let selected = select_entries(&entries, &options.selection);
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected.len(),
            processed_files: selected.len(),
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Compress,
            selected_files: selected.len(),
            processed_files: 0,
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }
    let effective_overwrite =
        resolve_effective_overwrite(&options.output, options.overwrite, "file")?;
    let plan = ZipOutputPlan::prepare(&options.output, effective_overwrite)?;
    let zip_options = ZipWriteOptions {
        method: zip_method_from_options(options.method.as_deref())?,
        sidecar: if options.no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                options.format,
                &source_info_for_convert(&options.artifact),
                &selected,
            )?)
        },
    };

    let write_result = write_entries_to_zip(&selected, plan.temp_path(), zip_options);
    let summary = match write_result {
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
    let bytes_out = compute_output_bytes(BackupArtifactFormat::Zip, &options.output);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected.len(),
            processed_files: summary.entry_count,
            bytes_in: summary.bytes_in,
            bytes_out,
            throughput: throughput(summary.bytes_in, phase_started.elapsed()),
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }

    if !options.json {
        eprintln!(
            "Converted {} entry(ies) / {} bytes from {} -> {} ({})",
            summary.entry_count,
            summary.bytes_in,
            options.artifact.display(),
            options.output.display(),
            options.format
        );
    }
    Ok(BackupConvertWriteResult {
        selected: selected.len(),
        written: summary.entry_count,
        skipped: 0,
        bytes_in: summary.bytes_in,
        bytes_out,
        overwrite_count: 0,
    })
}

fn execute_backup_convert_to_7z(
    options: &BackupConvertOptions,
) -> CliResult<BackupConvertWriteResult> {
    if paths_equal(&options.artifact, &options.output) {
        return Err(CliError::with_details(
            2,
            "backup convert source and destination must be different",
            &["Fix: Choose a different `--output` path."],
        ));
    }

    let phase_started = Instant::now();
    let entries = read_artifact_entries(&options.artifact)?;
    let selected = select_entries(&entries, &options.selection);
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Read,
            selected_files: selected.len(),
            processed_files: selected.len(),
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Compress,
            selected_files: selected.len(),
            processed_files: 0,
            bytes_in: bytes_in_selected,
            bytes_out: 0,
            throughput: 0,
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }
    let effective_overwrite =
        resolve_effective_overwrite(&options.output, options.overwrite, "file")?;
    let sevenz_options = SevenZWriteOptions {
        solid: options.solid,
        method: sevenz_method_from_options(options.method.as_deref())?,
        level: options.level.unwrap_or(1),
        sidecar: if options.no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                options.format,
                &source_info_for_convert(&options.artifact),
                &selected,
            )?)
        },
    };
    let split_size = parse_split_size_bytes(options.split_size.as_deref())?;
    let summary = if let Some(split_size) = split_size {
        let plan = SevenZSplitOutputPlan::prepare(&options.output, effective_overwrite)?;
        let result =
            write_entries_to_7z_split(&selected, plan.temp_base_path(), split_size, &sevenz_options);
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
        let plan = SevenZOutputPlan::prepare(&options.output, effective_overwrite)?;
        let write_result = write_entries_to_7z(&selected, plan.temp_path(), &sevenz_options);
        let summary = match write_result {
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
    };
    let bytes_out = compute_output_bytes(BackupArtifactFormat::SevenZ, &options.output);
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected.len(),
            processed_files: summary.entry_count,
            bytes_in: summary.bytes_in,
            bytes_out,
            throughput: throughput(summary.bytes_in, phase_started.elapsed()),
            elapsed_ms: phase_started.elapsed().as_millis(),
        });
    }

    if !options.json {
        eprintln!(
            "Converted {} entry(ies) / {} bytes from {} -> {} ({})",
            summary.entry_count,
            summary.bytes_in,
            options.artifact.display(),
            options.output.display(),
            options.format
        );
    }
    Ok(BackupConvertWriteResult {
        selected: selected.len(),
        written: summary.entry_count,
        skipped: 0,
        bytes_in: summary.bytes_in,
        bytes_out,
        overwrite_count: 0,
    })
}

fn execute_backup_convert_to_xunbak(
    _options: &BackupConvertOptions,
) -> CliResult<BackupConvertWriteResult> {
    #[cfg(feature = "xunbak")]
    {
        use crate::backup_export::output_plan::{XunbakOutputPlan, XunbakSplitOutputPlan};
        let options = _options;
        use crate::xunbak::codec::parse_compression_arg;
        use crate::xunbak::writer::ContainerWriter;

        if paths_equal(&options.artifact, &options.output) {
            return Err(CliError::with_details(
                2,
                "backup convert source and destination must be different",
                &["Fix: Choose a different `--output` path."],
            ));
        }

        let entries = read_artifact_entries(&options.artifact)?;
        let selected = select_entries(&entries, &options.selection);
        let effective_overwrite =
            resolve_effective_overwrite(&options.output, options.overwrite, "file")?;
        if matches!(effective_overwrite, OverwriteMode::Replace) {
            remove_xunbak_outputs(&options.output)?;
        }
        if matches!(effective_overwrite, OverwriteMode::Fail)
            && xunbak_output_exists(&options.output)
        {
            return Err(CliError::with_details(
                2,
                format!(
                    "backup convert output already exists: {}",
                    options.output.display()
                ),
                &["Fix: Remove the destination, or pass `--overwrite replace`."],
            ));
        }

        let staging_dir = create_staging_dir("xunbak-convert")?;
        let copy_result = (|| -> CliResult<(usize, u64)> {
            let mut written = 0usize;
            let mut bytes_in = 0u64;
            for entry in selected {
                let dest = staging_dir.join(entry.path.replace('/', "\\"));
                copy_entry_to_path(entry, &dest)?;
                written += 1;
                bytes_in += entry.size;
            }
            Ok((written, bytes_in))
        })();
        let (written, bytes_in) = match copy_result {
            Ok(result) => result,
            Err(err) => {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(err);
            }
        };

        let backup_options = backup_options_from_convert(options, |raw| {
            parse_compression_arg(raw).map_err(|err| CliError::new(2, err.to_string()))
        })?;
        let result = if backup_options.split_size.is_none() {
            let plan = XunbakOutputPlan::prepare(&options.output, OverwriteMode::Replace)?;
            let result = ContainerWriter::backup(plan.temp_path(), &staging_dir, &backup_options)
                .map_err(|err| CliError::new(2, err.to_string()));
            match result {
                Ok(result) => {
                    if let Err(err) = maybe_fail_after_write_for_tests() {
                        let _ = plan.cleanup();
                        let _ = fs::remove_dir_all(&staging_dir);
                        return Err(err);
                    }
                    if let Err(err) = plan.finalize() {
                        let _ = fs::remove_dir_all(&staging_dir);
                        return Err(err);
                    }
                    Ok(result)
                }
                Err(err) => {
                    let _ = plan.cleanup();
                    Err(err)
                }
            }
        } else {
            let plan = XunbakSplitOutputPlan::prepare(&options.output, OverwriteMode::Replace)?;
            let result =
                ContainerWriter::backup(plan.temp_base_path(), &staging_dir, &backup_options)
                    .map_err(|err| CliError::new(2, err.to_string()));
            match result {
                Ok(result) => {
                    if let Err(err) = maybe_fail_after_write_for_tests() {
                        let _ = plan.cleanup();
                        let _ = fs::remove_dir_all(&staging_dir);
                        return Err(err);
                    }
                    if let Err(err) = plan.finalize() {
                        let _ = fs::remove_dir_all(&staging_dir);
                        return Err(err);
                    }
                    Ok(result)
                }
                Err(err) => {
                    let _ = plan.cleanup();
                    Err(err)
                }
            }
        };
        let _ = fs::remove_dir_all(&staging_dir);
        match result {
            Ok(_) => {}
            Err(err) => {
                let _ = remove_xunbak_outputs(&options.output);
                return Err(err);
            }
        }

        if !options.json {
            eprintln!(
                "Converted {} entry(ies) / {} bytes from {} -> {} ({})",
                written,
                bytes_in,
                options.artifact.display(),
                options.output.display(),
                options.format
            );
        }
        Ok(BackupConvertWriteResult {
            selected: written,
            written,
            skipped: 0,
            bytes_in,
            bytes_out: compute_output_bytes(BackupArtifactFormat::Xunbak, &options.output),
            overwrite_count: 0,
        })
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

fn prepare_dir_output(options: &BackupConvertOptions) -> CliResult<DirOutputPlan> {
    if paths_equal(&options.artifact, &options.output) {
        return Err(CliError::with_details(
            2,
            "backup convert source and destination must be different",
            &["Fix: Choose a different `--output` path."],
        ));
    }

    let effective_overwrite =
        resolve_effective_overwrite(&options.output, options.overwrite, "directory")?;
    DirOutputPlan::prepare(&options.output, effective_overwrite)
}

#[cfg_attr(not(feature = "xunbak"), allow(dead_code))]
fn remove_existing_path(path: &Path) -> CliResult {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|err| {
            CliError::new(
                1,
                format!("Remove output directory failed {}: {err}", path.display()),
            )
        })?;
    } else if path.exists() {
        fs::remove_file(path).map_err(|err| {
            CliError::new(
                1,
                format!("Remove output file failed {}: {err}", path.display()),
            )
        })?;
    }
    Ok(())
}

fn resolve_effective_overwrite(
    output: &Path,
    overwrite: OverwriteMode,
    output_kind: &str,
) -> CliResult<OverwriteMode> {
    if !output.exists() {
        return Ok(overwrite);
    }
    match overwrite {
        OverwriteMode::Fail => Err(CliError::with_details(
            2,
            format!("backup convert output already exists: {}", output.display()),
            &["Fix: Remove the destination, or pass `--overwrite replace`."],
        )),
        OverwriteMode::Ask => {
            if !can_interact() {
                return Err(CliError::with_details(
                    2,
                    format!(
                        "backup convert output already exists and cannot prompt: {}",
                        output.display()
                    ),
                    &[
                        "Fix: Pass `--overwrite replace` or `--overwrite fail` in non-interactive mode.",
                    ],
                ));
            }
            let confirmed = Confirm::new()
                .with_prompt(format!(
                    "Replace existing output {output_kind} {}?",
                    output.display()
                ))
                .default(false)
                .interact()
                .unwrap_or(false);
            if !confirmed {
                return Err(CliError::new(3, "Cancelled."));
            }
            Ok(OverwriteMode::Replace)
        }
        OverwriteMode::Replace => Ok(OverwriteMode::Replace),
    }
}

fn zip_method_from_options(method: Option<&str>) -> Result<ZipCompressionMethod, CliError> {
    match method.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(ZipCompressionMethod::Auto),
        Some(value) if value == "deflated" => Ok(ZipCompressionMethod::Deflated),
        Some(value) if value == "stored" => Ok(ZipCompressionMethod::Stored),
        Some(value) => Err(CliError::with_details(
            2,
            format!("backup convert --method {value} is invalid for zip"),
            &["Fix: Use `--method stored` or `--method deflated`."],
        )),
    }
}

fn sevenz_method_from_options(method: Option<&str>) -> Result<SevenZMethod, CliError> {
    match method.map(|value| value.trim().to_ascii_lowercase()) {
        None => Ok(SevenZMethod::Lzma2),
        Some(value) if value == "lzma2" => Ok(SevenZMethod::Lzma2),
        Some(value) if value == "copy" => Ok(SevenZMethod::Copy),
        Some(value) => Err(CliError::with_details(
            2,
            format!("backup convert --method {value} is invalid for 7z"),
            &["Fix: Use `--method copy` or `--method lzma2`."],
        )),
    }
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

fn emit_backup_convert_failure_json(
    options: &BackupConvertOptions,
    status: ExportStatus,
    err: &CliError,
    duration_ms: u128,
) {
    if !options.json {
        return;
    }
    let summary = BackupConvertFailureSummary {
        action: BackupAction::Convert,
        status,
        source: options.artifact.display().to_string(),
        destination: options.output.display().to_string(),
        format: options.format.to_string(),
        error: err.message.clone(),
        dry_run: options.dry_run,
        overwrite_count: existing_output_count(options.format, &options.output),
        verify_source: options.verify_source.to_string(),
        verify_output: options.verify_output.to_string(),
        duration_ms,
    };
    if let Ok(json) = serde_json::to_string_pretty(&summary) {
        out_println!("{json}");
    }
}

fn compute_output_bytes(format: BackupArtifactFormat, output: &Path) -> u64 {
    match format {
        BackupArtifactFormat::Dir => dir_size_bytes(output),
        BackupArtifactFormat::Zip | BackupArtifactFormat::Xunbak => {
            fs::metadata(output).map(|meta| meta.len()).unwrap_or(0)
        }
        BackupArtifactFormat::SevenZ => sevenz_output_bytes(output),
    }
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

fn existing_output_count(format: BackupArtifactFormat, output: &Path) -> usize {
    match format {
        BackupArtifactFormat::Dir => usize::from(output.exists()),
        BackupArtifactFormat::Zip => usize::from(output.exists()),
        BackupArtifactFormat::SevenZ => collect_file_or_numbered_outputs(output).len(),
        BackupArtifactFormat::Xunbak => collect_output_paths_for_format(format, output).len(),
    }
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

fn collect_output_paths(options: &BackupConvertOptions) -> Vec<std::path::PathBuf> {
    collect_output_paths_for_format(options.format, &options.output)
}

fn collect_output_paths_for_format(
    format: BackupArtifactFormat,
    output: &Path,
) -> Vec<std::path::PathBuf> {
    match format {
        BackupArtifactFormat::Xunbak => {
            let mut outputs = Vec::new();
            if output.exists() {
                outputs.push(output.to_path_buf());
            }
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
            outputs.dedup();
            outputs
        }
        BackupArtifactFormat::SevenZ | BackupArtifactFormat::Zip => {
            collect_file_or_numbered_outputs(output)
        }
        _ => vec![output.to_path_buf()],
    }
}

fn sevenz_output_bytes(output: &Path) -> u64 {
    collect_file_or_numbered_outputs(output)
        .into_iter()
        .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
        .sum()
}

fn collect_file_or_numbered_outputs(output: &Path) -> Vec<std::path::PathBuf> {
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

fn maybe_corrupt_output_for_tests(options: &BackupConvertOptions) -> CliResult {
    let Some(mode) = std::env::var_os("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE") else {
        return Ok(());
    };
    let mode = mode.to_string_lossy().to_ascii_lowercase();
    if mode != "truncate" {
        return Ok(());
    }

    match options.format {
        BackupArtifactFormat::Zip | BackupArtifactFormat::Xunbak | BackupArtifactFormat::SevenZ => {
            let targets = collect_output_paths_for_format(options.format, &options.output);
            let target = targets.last().cloned().unwrap_or_else(|| options.output.clone());
            let metadata = fs::metadata(&target).map_err(|err| {
                CliError::new(
                    1,
                    format!(
                        "test hook failed to read output metadata {}: {err}",
                        target.display()
                    ),
                )
            })?;
            let shrink_by = match options.format {
                BackupArtifactFormat::Zip => 1,
                BackupArtifactFormat::Xunbak => 32,
                BackupArtifactFormat::SevenZ => 1,
                _ => 1,
            };
            if metadata.len() > shrink_by {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&target)
                    .map_err(|err| {
                        CliError::new(
                            1,
                            format!(
                                "test hook failed to open output {}: {err}",
                                target.display()
                            ),
                        )
                    })?
                    .set_len(metadata.len() - shrink_by)
                    .map_err(|err| {
                        CliError::new(
                            1,
                            format!(
                                "test hook failed to truncate output {}: {err}",
                                target.display()
                            ),
                        )
                    })?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(feature = "xunbak")]
fn backup_options_from_convert<F>(
    options: &BackupConvertOptions,
    mut parse_compression: F,
) -> Result<crate::xunbak::writer::BackupOptions, CliError>
where
    F: FnMut(&str) -> Result<crate::xunbak::codec::CompressionMode, CliError>,
{
    use crate::xunbak::codec::CompressionMode;
    use crate::xunbak::constants::Codec;
    use crate::xunbak::writer::BackupOptions;

    let compression_mode = match options.method.as_deref() {
        Some(raw) => parse_compression(raw)?,
        None => CompressionMode::Zstd {
            level: options.level.unwrap_or(1) as i32,
        },
    };

    let mut backup_options = match compression_mode {
        CompressionMode::None => BackupOptions {
            codec: Codec::NONE,
            zstd_level: 1,
            split_size: parse_split_size(options.split_size.as_deref())?,
        },
        CompressionMode::Zstd { level } => BackupOptions {
            codec: Codec::ZSTD,
            zstd_level: options.level.map(|value| value as i32).unwrap_or(level),
            split_size: parse_split_size(options.split_size.as_deref())?,
        },
        CompressionMode::Lz4 => BackupOptions {
            codec: Codec::LZ4,
            zstd_level: 1,
            split_size: parse_split_size(options.split_size.as_deref())?,
        },
        CompressionMode::Lzma => BackupOptions {
            codec: Codec::LZMA,
            zstd_level: 1,
            split_size: parse_split_size(options.split_size.as_deref())?,
        },
        CompressionMode::Auto => BackupOptions {
            codec: Codec::ZSTD,
            zstd_level: options.level.map(|value| value as i32).unwrap_or(1),
            split_size: parse_split_size(options.split_size.as_deref())?,
        },
    };
    if matches!(backup_options.codec, Codec::ZSTD) && backup_options.zstd_level <= 0 {
        backup_options.zstd_level = 1;
    }
    Ok(backup_options)
}

#[cfg(feature = "xunbak")]
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

#[cfg(feature = "xunbak")]
fn create_staging_dir(prefix: &str) -> CliResult<std::path::PathBuf> {
    let mut path = std::env::temp_dir();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    path.push(format!("{prefix}-{}-{millis}", Uuid::new_v4()));
    fs::create_dir_all(&path).map_err(|err| {
        CliError::new(
            1,
            format!("Create staging directory failed {}: {err}", path.display()),
        )
    })?;
    Ok(path)
}

#[cfg(feature = "xunbak")]
fn xunbak_output_exists(path: &Path) -> bool {
    if path.exists() {
        return true;
    }
    path.parent()
        .and_then(|parent| {
            let prefix = path.file_name()?.to_str()?.to_string();
            let entries = fs::read_dir(parent).ok()?;
            Some(entries.flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with(&format!("{prefix}."))
                    && name[prefix.len() + 1..]
                        .chars()
                        .all(|ch| ch.is_ascii_digit())
            }))
        })
        .unwrap_or(false)
}

#[cfg(feature = "xunbak")]
fn remove_xunbak_outputs(path: &Path) -> CliResult {
    if path.exists() {
        remove_existing_path(path)?;
    }
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(prefix) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    let read_dir = match fs::read_dir(parent) {
        Ok(read_dir) => read_dir,
        Err(_) => return Ok(()),
    };
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}."))
            && name[prefix.len() + 1..]
                .chars()
                .all(|ch| ch.is_ascii_digit())
        {
            remove_existing_path(&entry.path())?;
        }
    }
    Ok(())
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
