use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::backup::app::common::{
    SummaryActionStatus, SummaryDurationOutputs, SummaryExecutionStats, SummaryPaths,
    SummarySelectionStats, SummaryVerifyModes, build_sevenz_write_options, build_zip_write_options,
    ensure_convert_output_distinct, summary_action_status,
};
use crate::backup::artifact::common::{
    collect_artifact_output_paths, compute_artifact_output_bytes, parse_split_size_bytes,
    resolve_effective_overwrite, throughput_bytes_per_sec,
};
use crate::backup::artifact::dir::write_entries_to_dir;
use crate::backup::artifact::options::BackupConvertOptions;
use crate::backup::artifact::output_plan::{
    DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan, commit_output_plan,
};
use crate::backup::artifact::progress::{
    emit_compress_progress, emit_read_progress, emit_verify_output_progress,
    emit_verify_source_progress, emit_write_progress, should_emit_progress,
};
use crate::backup::artifact::reader::sort_entry_refs_for_read_locality;
use crate::backup::artifact::selection::select_entries;
use crate::backup::artifact::sevenz::{write_entries_to_7z, write_entries_to_7z_split};
use crate::backup::artifact::sidecar::{
    SidecarPackingHint, build_sidecar_bytes_with_hashes, source_info_for_convert,
    write_sidecar_to_dir,
};
use crate::backup::artifact::source::read_artifact_entries;
use crate::backup::artifact::verify::{verify_convert_source, verify_output};
use crate::backup::artifact::zip::write_entries_to_zip;
use crate::backup::common::cli::{path_display, path_strings};
use crate::backup_formats::{BackupAction, BackupArtifactFormat, ExportStatus, OverwriteMode};
use crate::cli::BackupConvertCmd;
use crate::output::{CliError, CliResult};

#[derive(Serialize)]
struct BackupConvertSelectionSummary {
    #[serde(flatten)]
    meta: SummaryActionStatus<BackupAction, ExportStatus>,
    mode: String,
    #[serde(flatten)]
    paths: SummaryPaths,
    format: String,
    #[serde(flatten)]
    stats: SummarySelectionStats,
    #[serde(flatten)]
    verify: SummaryVerifyModes,
    #[serde(flatten)]
    timing: SummaryDurationOutputs,
    entries: Vec<String>,
}

#[derive(Serialize)]
struct BackupConvertExecutionSummary {
    #[serde(flatten)]
    meta: SummaryActionStatus<BackupAction, ExportStatus>,
    #[serde(flatten)]
    paths: SummaryPaths,
    format: String,
    #[serde(flatten)]
    stats: SummaryExecutionStats,
    #[serde(flatten)]
    verify: SummaryVerifyModes,
    #[serde(flatten)]
    timing: SummaryDurationOutputs,
}

#[derive(Serialize)]
struct BackupConvertFailureSummary {
    #[serde(flatten)]
    meta: SummaryActionStatus<BackupAction, ExportStatus>,
    #[serde(flatten)]
    paths: SummaryPaths,
    format: String,
    error: String,
    dry_run: bool,
    overwrite_count: usize,
    #[serde(flatten)]
    verify: SummaryVerifyModes,
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
    emit_verify_source_progress(show_progress, started.elapsed().as_millis());
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
    emit_verify_output_progress(
        show_progress,
        result.selected,
        result.written,
        result.bytes_in,
        result.bytes_out,
        started.elapsed().as_millis(),
    );
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
        let summary = build_convert_execution_summary(
            &options,
            &result,
            path_strings(collect_artifact_output_paths(
                options.format,
                &options.output,
            )),
            started.elapsed().as_millis(),
        );
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
        let summary = build_convert_selection_summary(
            options,
            mode,
            paths,
            bytes_in,
            existing_output_count(options.format, &options.output),
            started.elapsed().as_millis(),
        );
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
    let mut selected = select_entries(&entries, &options.selection);
    sort_entry_refs_for_read_locality(&mut selected)?;
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    emit_read_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    emit_compress_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    let plan = prepare_dir_output(options)?;
    let source_info = source_info_for_convert(&options.artifact);
    let summary = commit_output_plan(plan, |plan| {
        let summary = write_entries_to_dir(&selected, plan.temp_path())?;
        if !options.no_sidecar {
            let sidecar = build_sidecar_bytes_with_hashes(
                options.format,
                SidecarPackingHint::Dir,
                &source_info,
                &selected,
                &summary.content_hashes,
            )?;
            write_sidecar_to_dir(plan.temp_path(), &sidecar)?;
        }
        Ok(summary)
    })?;
    let written = summary.entry_count;
    let bytes_in = summary.bytes_in;
    let bytes_out = compute_artifact_output_bytes(BackupArtifactFormat::Dir, &options.output);
    emit_write_progress(
        show_progress,
        selected.len(),
        written,
        bytes_in,
        bytes_out,
        throughput_bytes_per_sec(bytes_in, phase_started.elapsed()),
        phase_started.elapsed().as_millis(),
    );

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
    ensure_convert_output_distinct(&options.artifact, &options.output)?;

    let phase_started = Instant::now();
    let entries = read_artifact_entries(&options.artifact)?;
    let mut selected = select_entries(&entries, &options.selection);
    sort_entry_refs_for_read_locality(&mut selected)?;
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    emit_read_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    emit_compress_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    let effective_overwrite =
        resolve_effective_overwrite(&options.output, options.overwrite, "file")?;
    let plan = ZipOutputPlan::prepare(&options.output, effective_overwrite)?;
    let zip_options = build_zip_write_options(
        "backup convert",
        options.method.as_deref(),
        false,
        options.level,
        options.no_sidecar,
        options.format,
        &source_info_for_convert(&options.artifact),
        &selected,
    )?;

    let summary = commit_output_plan(plan, |plan| {
        write_entries_to_zip(&selected, plan.temp_path(), zip_options)
    })?;
    let bytes_out = compute_artifact_output_bytes(BackupArtifactFormat::Zip, &options.output);
    emit_write_progress(
        show_progress,
        selected.len(),
        summary.entry_count,
        summary.bytes_in,
        bytes_out,
        throughput_bytes_per_sec(summary.bytes_in, phase_started.elapsed()),
        phase_started.elapsed().as_millis(),
    );

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
    ensure_convert_output_distinct(&options.artifact, &options.output)?;

    let phase_started = Instant::now();
    let entries = read_artifact_entries(&options.artifact)?;
    let mut selected = select_entries(&entries, &options.selection);
    sort_entry_refs_for_read_locality(&mut selected)?;
    let show_progress = should_emit_progress(options.progress, options.json);
    let bytes_in_selected: u64 = selected.iter().map(|entry| entry.size).sum();
    emit_read_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    emit_compress_progress(
        show_progress,
        selected.len(),
        bytes_in_selected,
        phase_started.elapsed().as_millis(),
    );
    let effective_overwrite =
        resolve_effective_overwrite(&options.output, options.overwrite, "file")?;
    let sevenz_options = build_sevenz_write_options(
        "backup convert",
        options.method.as_deref(),
        false,
        options.solid,
        options.level,
        options.no_sidecar,
        options.format,
        &source_info_for_convert(&options.artifact),
        &selected,
    )?;
    let split_size = parse_split_size_bytes(options.split_size.as_deref())?;
    let summary = if let Some(split_size) = split_size {
        let plan = SevenZSplitOutputPlan::prepare(&options.output, effective_overwrite)?;
        commit_output_plan(plan, |plan| {
            write_entries_to_7z_split(
                &selected,
                plan.temp_base_path(),
                split_size,
                &sevenz_options,
            )
        })?
    } else {
        let plan = SevenZOutputPlan::prepare(&options.output, effective_overwrite)?;
        commit_output_plan(plan, |plan| {
            write_entries_to_7z(&selected, plan.temp_path(), &sevenz_options)
        })?
    };
    let bytes_out = compute_artifact_output_bytes(BackupArtifactFormat::SevenZ, &options.output);
    emit_write_progress(
        show_progress,
        selected.len(),
        summary.entry_count,
        summary.bytes_in,
        bytes_out,
        throughput_bytes_per_sec(summary.bytes_in, phase_started.elapsed()),
        phase_started.elapsed().as_millis(),
    );

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
        use crate::backup::artifact::output_plan::{XunbakOutputPlan, XunbakSplitOutputPlan};
        let options = _options;

        ensure_convert_output_distinct(&options.artifact, &options.output)?;

        let entries = read_artifact_entries(&options.artifact)?;
        let mut selected = select_entries(&entries, &options.selection);
        sort_entry_refs_for_read_locality(&mut selected)?;
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

        let written = selected.len();
        let bytes_in: u64 = selected.iter().map(|entry| entry.size).sum();
        let source_root = source_info_for_convert(&options.artifact).source_root;

        let backup_options = backup_options_from_convert(options, |raw| {
            crate::backup::app::xunbak::build_backup_options_from_mode(
                crate::backup::app::xunbak::parse_xunbak_compression_arg(raw)?,
                options.split_size.as_deref(),
                options.level.map(|value| value as i32),
            )
        })?;
        let result = if backup_options.split_size.is_none() {
            let plan = XunbakOutputPlan::prepare(&options.output, OverwriteMode::Replace)?;
            commit_output_plan(plan, |plan| {
                crate::backup::artifact::xunbak::write_entries_to_xunbak(
                    &selected,
                    plan.temp_path(),
                    &source_root,
                    &backup_options,
                    OverwriteMode::Replace,
                )
                .map(|_| ())
            })
        } else {
            let plan = XunbakSplitOutputPlan::prepare(&options.output, OverwriteMode::Replace)?;
            commit_output_plan(plan, |plan| {
                crate::backup::artifact::xunbak::write_entries_to_xunbak(
                    &selected,
                    plan.temp_base_path(),
                    &source_root,
                    &backup_options,
                    OverwriteMode::Replace,
                )
                .map(|_| ())
            })
        };
        match result {
            Ok(()) => {}
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
            bytes_out: compute_artifact_output_bytes(BackupArtifactFormat::Xunbak, &options.output),
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
    ensure_convert_output_distinct(&options.artifact, &options.output)?;

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

fn emit_backup_convert_failure_json(
    options: &BackupConvertOptions,
    status: ExportStatus,
    err: &CliError,
    duration_ms: u128,
) {
    if !options.json {
        return;
    }
    let summary = build_convert_failure_summary(options, status, err, duration_ms);
    if let Ok(json) = serde_json::to_string_pretty(&summary) {
        out_println!("{json}");
    }
}

fn build_convert_selection_summary(
    options: &BackupConvertOptions,
    mode: &str,
    entries: Vec<String>,
    bytes_in: u64,
    overwrite_count: usize,
    duration_ms: u128,
) -> BackupConvertSelectionSummary {
    BackupConvertSelectionSummary {
        meta: summary_action_status(BackupAction::Convert, ExportStatus::Ok),
        mode: mode.to_string(),
        paths: SummaryPaths {
            source: path_display(&options.artifact),
            destination: path_display(&options.output),
        },
        format: options.format.to_string(),
        stats: SummarySelectionStats {
            dry_run: options.dry_run,
            selected: entries.len(),
            skipped: 0,
            bytes_in,
            bytes_out: 0,
            overwrite_count,
        },
        verify: SummaryVerifyModes {
            verify_source: options.verify_source.to_string(),
            verify_output: options.verify_output.to_string(),
        },
        timing: SummaryDurationOutputs {
            duration_ms,
            outputs: Vec::new(),
        },
        entries,
    }
}

fn build_convert_execution_summary(
    options: &BackupConvertOptions,
    result: &BackupConvertWriteResult,
    outputs: Vec<String>,
    duration_ms: u128,
) -> BackupConvertExecutionSummary {
    BackupConvertExecutionSummary {
        meta: summary_action_status(BackupAction::Convert, ExportStatus::Ok),
        paths: SummaryPaths {
            source: path_display(&options.artifact),
            destination: path_display(&options.output),
        },
        format: options.format.to_string(),
        stats: SummaryExecutionStats {
            dry_run: false,
            selected: result.selected,
            written: result.written,
            skipped: result.skipped,
            bytes_in: result.bytes_in,
            bytes_out: result.bytes_out,
            overwrite_count: result.overwrite_count,
        },
        verify: SummaryVerifyModes {
            verify_source: options.verify_source.to_string(),
            verify_output: options.verify_output.to_string(),
        },
        timing: SummaryDurationOutputs {
            duration_ms,
            outputs,
        },
    }
}

fn build_convert_failure_summary(
    options: &BackupConvertOptions,
    status: ExportStatus,
    err: &CliError,
    duration_ms: u128,
) -> BackupConvertFailureSummary {
    BackupConvertFailureSummary {
        meta: summary_action_status(BackupAction::Convert, status),
        paths: SummaryPaths {
            source: path_display(&options.artifact),
            destination: path_display(&options.output),
        },
        format: options.format.to_string(),
        error: err.message.clone(),
        dry_run: options.dry_run,
        overwrite_count: existing_output_count(options.format, &options.output),
        verify: SummaryVerifyModes {
            verify_source: options.verify_source.to_string(),
            verify_output: options.verify_output.to_string(),
        },
        duration_ms,
    }
}

fn existing_output_count(format: BackupArtifactFormat, output: &Path) -> usize {
    collect_artifact_output_paths(format, output).len()
}

fn maybe_corrupt_output_for_tests(options: &BackupConvertOptions) -> CliResult {
    let Some(mode) = std::env::var_os("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE") else {
        return Ok(());
    };
    let mode = mode.to_string_lossy().to_ascii_lowercase();

    match options.format {
        BackupArtifactFormat::Zip | BackupArtifactFormat::Xunbak | BackupArtifactFormat::SevenZ => {
            let targets = collect_artifact_output_paths(options.format, &options.output);
            let target = targets
                .last()
                .cloned()
                .unwrap_or_else(|| options.output.clone());
            match mode.as_str() {
                "truncate" => {
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
                }
                "flip-data-byte" => match options.format {
                    BackupArtifactFormat::Zip => corrupt_zip_payload_byte(&target)?,
                    BackupArtifactFormat::SevenZ => corrupt_7z_payload_byte(&target)?,
                    _ => {}
                },
                _ => {}
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn corrupt_zip_payload_byte(path: &Path) -> CliResult {
    let data_offset = {
        let file = fs::File::open(path).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "test hook failed to open zip output {}: {err}",
                    path.display()
                ),
            )
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "test hook failed to parse zip output {}: {err}",
                    path.display()
                ),
            )
        })?;
        let entry = archive.by_index(0).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "test hook failed to open first zip entry {}: {err}",
                    path.display()
                ),
            )
        })?;
        entry.data_start()
    };
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| {
            CliError::new(
                1,
                format!(
                    "test hook failed to open zip output {}: {err}",
                    path.display()
                ),
            )
        })?;
    flip_byte_at(path, &mut file, data_offset)
}

fn corrupt_7z_payload_byte(path: &Path) -> CliResult {
    let archive = sevenz_rust2::Archive::open(path)
        .map_err(|err| CliError::new(1, format!("test hook failed to inspect 7z: {err}")))?;
    let Some(pack_size) = archive.pack_sizes().first().copied() else {
        return Err(CliError::new(
            1,
            "test hook expected at least one 7z pack stream",
        ));
    };
    if pack_size == 0 {
        return Err(CliError::new(
            1,
            "test hook expected non-empty 7z pack stream",
        ));
    }
    let offset = sevenz_rust2::SIGNATURE_HEADER_SIZE + archive.pack_pos() + ((pack_size - 1) / 2);
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| {
            CliError::new(
                1,
                format!(
                    "test hook failed to open 7z output {}: {err}",
                    path.display()
                ),
            )
        })?;
    flip_byte_at(path, &mut file, offset)
}

fn flip_byte_at(path: &Path, file: &mut fs::File, offset: u64) -> CliResult {
    file.seek(SeekFrom::Start(offset)).map_err(|err| {
        CliError::new(
            1,
            format!(
                "test hook failed to seek output {} at {offset}: {err}",
                path.display()
            ),
        )
    })?;
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).map_err(|err| {
        CliError::new(
            1,
            format!(
                "test hook failed to read output {} at {offset}: {err}",
                path.display()
            ),
        )
    })?;
    byte[0] ^= 0x01;
    file.seek(SeekFrom::Start(offset)).map_err(|err| {
        CliError::new(
            1,
            format!(
                "test hook failed to rewind output {} at {offset}: {err}",
                path.display()
            ),
        )
    })?;
    file.write_all(&byte).map_err(|err| {
        CliError::new(
            1,
            format!(
                "test hook failed to corrupt output {} at {offset}: {err}",
                path.display()
            ),
        )
    })
}

#[cfg(feature = "xunbak")]
fn backup_options_from_convert<F>(
    options: &BackupConvertOptions,
    mut build_options: F,
) -> Result<crate::xunbak::writer::BackupOptions, CliError>
where
    F: FnMut(&str) -> Result<crate::xunbak::writer::BackupOptions, CliError>,
{
    use crate::xunbak::writer::BackupOptions;

    match options.method.as_deref() {
        Some(raw) => build_options(raw),
        None => Ok(BackupOptions {
            split_size: parse_split_size(options.split_size.as_deref())?,
            ..BackupOptions::default()
        }),
    }
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
