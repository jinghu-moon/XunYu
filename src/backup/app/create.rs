use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::backup::app::common::{
    build_sevenz_write_options, build_zip_write_options, ensure_create_output_distinct,
};
use crate::backup::artifact::common::{
    collect_artifact_output_paths, compute_artifact_output_bytes, parse_split_size_bytes,
    throughput_bytes_per_sec,
};
use crate::backup::artifact::dir::write_entries_to_dir;
use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::fs::scan_source_entries;
use crate::backup::artifact::options::BackupCreateOptions;
use crate::backup::artifact::output_plan::{
    DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan, commit_output_plan,
};
use crate::backup::artifact::progress::{
    emit_compress_progress, emit_read_progress, emit_write_progress, should_emit_progress,
};
use crate::backup::artifact::sevenz::{write_entries_to_7z, write_entries_to_7z_split};
use crate::backup::artifact::sidecar::{
    SidecarPackingHint, build_sidecar_bytes, source_info_for_create, write_sidecar_to_dir,
};
use crate::backup::artifact::zip::write_entries_to_zip;
use crate::backup::common::cli::{optional_path_display, path_display, path_strings};
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
                diff_mode: args.diff_mode,
                json: args.json,
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
            source: path_display(&options.source_dir),
            destination: optional_path_display(options.output.as_deref()),
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
    ensure_create_output_distinct(&options.source_dir, &output, options.format)?;
    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let show_progress = should_emit_progress(options.progress, options.json);
    emit_read_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );

    if options.dry_run {
        if options.json {
            let summary = build_create_execution_summary(
                options,
                &output,
                true,
                selected,
                0,
                bytes_in,
                0,
                Vec::new(),
                started.elapsed().as_millis(),
            );
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

    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let source_info = source_info_for_create(&options.source_dir);
    let plan = DirOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
    let written = commit_output_plan(plan, |plan| {
        let summary = write_entries_to_dir(&refs, plan.temp_path())?;
        if !options.no_sidecar {
            let sidecar =
                build_sidecar_bytes(options.format, SidecarPackingHint::Dir, &source_info, &refs)?;
            write_sidecar_to_dir(plan.temp_path(), &sidecar)?;
        }
        Ok(summary.entry_count)
    })?;
    let bytes_out = compute_artifact_output_bytes(options.format, &output);
    emit_write_progress(
        show_progress,
        selected,
        written,
        bytes_in,
        bytes_out,
        throughput_bytes_per_sec(bytes_in, started.elapsed()),
        started.elapsed().as_millis(),
    );

    if options.json {
        let summary = build_create_execution_summary(
            options,
            &output,
            false,
            selected,
            written,
            bytes_in,
            bytes_out,
            vec![path_display(&output)],
            started.elapsed().as_millis(),
        );
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
    emit_read_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );
    emit_compress_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );

    if options.dry_run {
        if options.json {
            let summary = build_create_execution_summary(
                options,
                &output,
                true,
                selected,
                0,
                bytes_in,
                0,
                Vec::new(),
                started.elapsed().as_millis(),
            );
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
    let sevenz_options = build_sevenz_write_options(
        "backup create",
        options.method.as_deref(),
        options.no_compress,
        options.solid,
        options.level,
        options.no_sidecar,
        options.format,
        &source_info_for_create(&options.source_dir),
        &refs,
    )?;
    let summary = if let Some(split_size) = split_size {
        let plan =
            SevenZSplitOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
        commit_output_plan(plan, |plan| {
            write_entries_to_7z_split(&refs, plan.temp_base_path(), split_size, &sevenz_options)
        })?
    } else {
        let plan = SevenZOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
        commit_output_plan(plan, |plan| {
            write_entries_to_7z(&refs, plan.temp_path(), &sevenz_options)
        })?
    };
    let bytes_out = compute_artifact_output_bytes(options.format, &output);
    emit_write_progress(
        show_progress,
        selected,
        summary.entry_count,
        summary.bytes_in,
        bytes_out,
        throughput_bytes_per_sec(summary.bytes_in, started.elapsed()),
        started.elapsed().as_millis(),
    );

    if options.json {
        let result = build_create_execution_summary(
            options,
            &output,
            false,
            selected,
            summary.entry_count,
            summary.bytes_in,
            bytes_out,
            path_strings(collect_artifact_output_paths(options.format, &output)),
            started.elapsed().as_millis(),
        );
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
    emit_read_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );
    emit_compress_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );

    if options.dry_run {
        if options.json {
            let summary = build_create_execution_summary(
                options,
                &output,
                true,
                selected,
                0,
                bytes_in,
                0,
                Vec::new(),
                started.elapsed().as_millis(),
            );
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

    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let zip_options = build_zip_write_options(
        "backup create",
        options.method.as_deref(),
        options.no_compress,
        options.level,
        options.no_sidecar,
        options.format,
        &source_info_for_create(&options.source_dir),
        &refs,
    )?;
    let plan = ZipOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
    let summary = commit_output_plan(plan, |plan| {
        write_entries_to_zip(&refs, plan.temp_path(), zip_options)
    })?;
    let bytes_out = compute_artifact_output_bytes(options.format, &output);
    emit_write_progress(
        show_progress,
        selected,
        summary.entry_count,
        summary.bytes_in,
        bytes_out,
        throughput_bytes_per_sec(summary.bytes_in, started.elapsed()),
        started.elapsed().as_millis(),
    );

    if options.json {
        let result = build_create_execution_summary(
            options,
            &output,
            false,
            selected,
            summary.entry_count,
            summary.bytes_in,
            bytes_out,
            path_strings(collect_artifact_output_paths(options.format, &output)),
            started.elapsed().as_millis(),
        );
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
    ensure_create_output_distinct(&options.source_dir, &output, options.format)?;

    let entries = collect_backup_create_entries(options)?;
    let selected = entries.len();
    let bytes_in: u64 = entries.iter().map(|entry| entry.size).sum();
    let show_progress = should_emit_progress(options.progress, options.json);
    emit_read_progress(
        show_progress,
        selected,
        bytes_in,
        started.elapsed().as_millis(),
    );

    if options.dry_run {
        if options.json {
            let summary = build_create_execution_summary(
                options,
                &output,
                true,
                selected,
                0,
                bytes_in,
                0,
                Vec::new(),
                started.elapsed().as_millis(),
            );
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
    let source_root = path_display(&options.source_dir);
    let summary = crate::backup::artifact::xunbak::write_entries_to_xunbak(
        &refs,
        &output,
        &source_root,
        &backup_options,
        crate::backup_formats::OverwriteMode::Fail,
    )?;
    let bytes_out = compute_artifact_output_bytes(options.format, &summary.destination);
    emit_write_progress(
        show_progress,
        selected,
        summary.entry_count,
        summary.bytes_in,
        bytes_out,
        throughput_bytes_per_sec(summary.bytes_in, started.elapsed()),
        started.elapsed().as_millis(),
    );

    if options.json {
        let summary = build_create_execution_summary(
            options,
            &summary.destination,
            false,
            selected,
            summary.entry_count,
            summary.bytes_in,
            bytes_out,
            path_strings(collect_artifact_output_paths(
                options.format,
                &summary.destination,
            )),
            started.elapsed().as_millis(),
        );
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

fn build_create_execution_summary(
    options: &BackupCreateOptions,
    destination: &Path,
    dry_run: bool,
    selected: usize,
    written: usize,
    bytes_in: u64,
    bytes_out: u64,
    outputs: Vec<String>,
    duration_ms: u128,
) -> BackupCreateExecutionSummary {
    BackupCreateExecutionSummary {
        action: BackupAction::Create,
        status: ExportStatus::Ok,
        source: path_display(&options.source_dir),
        destination: path_display(destination),
        format: options.format,
        dry_run,
        selected,
        written,
        skipped: 0,
        bytes_in,
        bytes_out,
        overwrite_count: 0,
        verify_source: "off".to_string(),
        verify_output: "off".to_string(),
        duration_ms,
        outputs,
    }
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
