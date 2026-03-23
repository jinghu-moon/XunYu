use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use console::Style;
use serde::Serialize;

use crate::backup_export::dir_writer::write_entries_to_dir;
use crate::backup_export::fs_source::scan_source_entries;
use crate::backup_export::options::BackupCreateOptions;
use crate::backup_export::output_plan::{
    DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan,
};
use crate::backup_export::progress::{
    ExportProgressEvent, ExportProgressPhase, emit_progress_event, should_emit_progress,
};
use crate::backup_export::sevenz_io::{
    SevenZMethod, SevenZWriteOptions, write_entries_to_7z, write_entries_to_7z_split,
};
use crate::backup_export::sidecar::{
    build_sidecar_bytes, source_info_for_create, write_sidecar_to_dir,
};
use crate::backup_export::source::SourceEntry;
use crate::backup_export::zip_writer::{
    ZipCompressionMethod, ZipWriteOptions, write_entries_to_zip,
};
use crate::backup_formats::{BackupAction, BackupArtifactFormat, ExportStatus};
use crate::cli::{BackupCmd, BackupCreateCmd, BackupRestoreCmd, BackupSubCommand, RestoreCmd};
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::runtime;
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};
use crate::windows::file_copy::detect_copy_backend_for_backup;

mod baseline;
mod checksum;
pub(crate) mod config;
mod diff;
mod find;
mod list;
mod meta;
mod report;
mod retention;
pub(crate) mod scan;
mod time_fmt;
mod util;
mod verify;
mod version;
mod zip;

pub(crate) use baseline::read_baseline;

#[doc(hidden)]
pub(crate) fn bench_read_baseline_len(prev: &Path) -> usize {
    read_baseline(prev).len()
}

#[doc(hidden)]
pub(crate) fn bench_scan_and_diff_count(current_root: &Path, prev: &Path) -> usize {
    let current = scan::scan_files(current_root, &[], &[], &[]);
    let mut baseline = read_baseline(prev);
    diff::compute_diff(&current, &mut baseline, false).len()
}

fn backup_timing_enabled() -> bool {
    backup_timing_enabled_with(|name| std::env::var_os(name))
}

fn backup_timing_enabled_with<F>(mut get_env: F) -> bool
where
    F: FnMut(&str) -> Option<OsString>,
{
    ["XUN_CMD_TIMING", "XUN_BACKUP_TIMING", "XUN_BAK_TIMING"]
        .into_iter()
        .any(|name| get_env(name).is_some())
}

fn emit_backup_timing(label: &str, elapsed: std::time::Duration, extra: Option<String>) {
    match extra {
        Some(extra) if !extra.is_empty() => {
            eprintln!("  [{label:<10}] {:>5}ms  {extra}", elapsed.as_millis());
        }
        _ => eprintln!("  [{label:<10}] {:>5}ms", elapsed.as_millis()),
    }
}

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

#[cfg(feature = "xunbak")]
#[derive(Serialize)]
struct BackupCreateXunbakExecutionSummary {
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
    if let Some(subcommand) = args.cmd.clone() {
        match subcommand {
            BackupSubCommand::Create(cmd) => return cmd_backup_create(cmd),
            BackupSubCommand::Restore(cmd) => return super::restore::cmd_restore(cmd.into()),
            BackupSubCommand::Convert(cmd) => {
                return super::backup_convert::cmd_backup_convert(cmd);
            }
            BackupSubCommand::List(_) | BackupSubCommand::Verify(_) | BackupSubCommand::Find(_) => {
            }
        }
    }

    let t_total = Instant::now();
    let timing = backup_timing_enabled();
    let copy_backend = detect_copy_backend_for_backup();

    let t_config = Instant::now();
    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };

    if args.container.is_some() {
        #[cfg(feature = "xunbak")]
        {
            return crate::commands::xunbak::cmd_backup_container(&args, &root);
        }
        #[cfg(not(feature = "xunbak"))]
        {
            return Err(CliError::with_details(
                2,
                "xunbak container mode is not enabled in this build",
                &["Fix: Rebuild with `--features xunbak`."],
            ));
        }
    }

    let cfg = config::load_config(&root);
    if timing {
        emit_backup_timing(
            "config",
            t_config.elapsed(),
            Some(root.display().to_string()),
        );
    }

    if let Some(subcommand) = args.cmd {
        let t_sub = Instant::now();
        let (label, result) = match subcommand {
            BackupSubCommand::Create(_)
            | BackupSubCommand::Restore(_)
            | BackupSubCommand::Convert(_) => unreachable!("handled earlier"),
            BackupSubCommand::List(cmd) => ("list", list::cmd_backup_list(&root, &cfg, cmd.json)),
            BackupSubCommand::Verify(cmd) => (
                "verify",
                verify::cmd_backup_verify(&root, &cfg, &cmd.name, cmd.json),
            ),
            BackupSubCommand::Find(cmd) => ("find", {
                let since = find::parse_time_filter_bound(cmd.since.as_deref(), false)?;
                let until = find::parse_time_filter_bound(cmd.until.as_deref(), true)?;
                find::cmd_backup_find(&root, &cfg, cmd.tag.as_deref(), since, until, cmd.json)
            }),
        };
        if timing {
            emit_backup_timing("subcommand", t_sub.elapsed(), Some(label.to_string()));
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return result;
    }

    let t_rules = Instant::now();
    let scan_rules = resolve_scan_rules(&root, &cfg, &args.include, &args.exclude);

    let compress = if args.no_compress {
        false
    } else {
        cfg.storage.compress
    };
    if timing {
        emit_backup_timing(
            "rules",
            t_rules.elapsed(),
            Some(format!(
                "roots={} include_globs={} exclude_globs={} compress={compress}",
                scan_rules.include_roots.len(),
                scan_rules.include_patterns.len(),
                scan_rules.exclude_patterns.len()
            )),
        );
    }

    // 1. Version scanning
    let t_version = Instant::now();
    let backups_root = root.join(&cfg.storage.backups_dir);
    let ver = version::scan_versions(&backups_root, &cfg.naming.prefix);
    if timing {
        emit_backup_timing(
            "version",
            t_version.elapsed(),
            Some(format!("next=v{}", ver.next_version)),
        );
    }

    let dim = Style::new().dim();
    let yellow = Style::new().yellow();
    eprintln!(
        "{}{}",
        dim.apply_to("Next Version: "),
        yellow.apply_to(format!("{}{}", cfg.naming.prefix, ver.next_version))
    );
    if let Some(ref name) = ver.prev_name {
        eprintln!("{}{}", dim.apply_to("Compare With: "), dim.apply_to(name));
    }

    // 2. Get description
    let t_desc = Instant::now();
    let desc = if let Some(ref m) = args.msg {
        m.clone()
    } else if !can_interact() {
        cfg.naming.default_desc.clone()
    } else {
        eprint!("Description [{}] ", cfg.naming.default_desc);
        let _ = io::stderr().flush();
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
        let trimmed = buf.trim();
        if trimmed.is_empty() {
            cfg.naming.default_desc.clone()
        } else {
            trimmed.to_string()
        }
    };
    if timing {
        emit_backup_timing("desc", t_desc.elapsed(), Some(desc.clone()));
    }

    // 3. Build folder name
    let t_prepare = Instant::now();
    let date_str = time_fmt::format_now(&cfg.naming.date_format);
    let incr_suffix = if args.incremental { "-incr" } else { "" };
    let folder_name = format!(
        "{}{}-{}_{}{}",
        cfg.naming.prefix, ver.next_version, desc, date_str, incr_suffix
    );
    let dest_dir = backups_root.join(&folder_name);
    if timing {
        emit_backup_timing("prepare", t_prepare.elapsed(), Some(folder_name.clone()));
    }

    // 4. Scan + diff + copy
    eprintln!("\n{}", dim.apply_to("Analysis & Backup..."));
    eprintln!("{}", dim.apply_to("--------------------"));

    // scan spinner（交互模式下显示）
    let scan_spinner = if can_interact() && !timing {
        let sp = indicatif::ProgressBar::new_spinner();
        sp.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.dim} Scanning {msg}")
                .unwrap(),
        );
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let t_scan = Instant::now();
    let current = scan::scan_files(
        &root,
        &scan_rules.include_roots,
        &scan_rules.exclude_patterns,
        &scan_rules.include_patterns,
    );
    let elapsed_scan = t_scan.elapsed();
    if let Some(ref sp) = scan_spinner {
        sp.finish_and_clear();
    }
    if timing {
        emit_backup_timing(
            "scan",
            elapsed_scan,
            Some(format!("files={}", current.len())),
        );
    }

    let t_baseline = Instant::now();
    let mut baseline = match &ver.prev_path {
        Some(p) => baseline::read_baseline(p),
        None => HashMap::new(),
    };
    if timing {
        emit_backup_timing(
            "baseline",
            t_baseline.elapsed(),
            Some(format!("entries={}", baseline.len())),
        );
    }

    // 增量模式只追踪变更文件；全量模式（含目录备份）需追踪所有文件
    let skip_unchanged = args.incremental;
    let show_unchanged = runtime::is_verbose() && !skip_unchanged;
    let t_diff = Instant::now();
    let diff_entries = diff::compute_diff(&current, &mut baseline, skip_unchanged);
    if timing {
        emit_backup_timing(
            "diff",
            t_diff.elapsed(),
            Some(format!("entries={}", diff_entries.len())),
        );
    }
    let has_changes = diff_entries
        .iter()
        .any(|entry| !matches!(entry.kind, diff::DiffKind::Unchanged));
    let t_diff_print = Instant::now();
    diff::print_diff(&diff_entries, show_unchanged);
    if timing {
        emit_backup_timing(
            "diff-print",
            t_diff_print.elapsed(),
            Some(format!("lines={}", diff_entries.len())),
        );
    }

    let skip_if_unchanged = args.skip_if_unchanged || cfg.skip_if_unchanged;
    if skip_if_unchanged && !has_changes {
        let white = Style::new().white();
        eprintln!();
        report::report("Skipped", "no changes detected", &white);
        if let Some(ref name) = ver.prev_name {
            report::report("Baseline", name, &dim);
        }
        eprintln!();
        if timing {
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return Ok(());
    }

    let t_copy = Instant::now();
    let stats = if !args.dry_run {
        let _ = fs::create_dir_all(&dest_dir);
        // copy 进度：显示文件数
        let copy_bar = if can_interact() && !timing {
            let total = diff_entries
                .iter()
                .filter(|e| {
                    matches!(
                        e.kind,
                        diff::DiffKind::New | diff::DiffKind::Modified | diff::DiffKind::Unchanged
                    )
                })
                .count() as u64;
            if total > 0 {
                let pb = indicatif::ProgressBar::new(total);
                pb.set_style(
                    indicatif::ProgressStyle::default_bar()
                        .template("{spinner:.dim} Copying [{bar:30.cyan/blue}] {pos}/{len} files")
                        .unwrap()
                        .progress_chars("=>-"),
                );
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                Some(pb)
            } else {
                None
            }
        } else {
            None
        };
        let prev_backup_dir = ver.prev_path.as_deref().filter(|path| path.is_dir());
        let result = diff::apply_diff(
            &diff_entries,
            &dest_dir,
            args.incremental,
            prev_backup_dir,
            copy_backend,
        );
        if let Some(ref pb) = copy_bar {
            pb.finish_and_clear();
        }
        result
    } else {
        diff::DiffStats {
            new: 0,
            modified: 0,
            deleted: 0,
            logical_bytes: 0,
            copied_bytes: 0,
            hardlinked_files: 0,
        }
    };
    if timing {
        emit_backup_timing(
            "copy",
            t_copy.elapsed(),
            Some(format!(
                "+{} ~{} -{} linked={} copied={}B backend={:?}",
                stats.new,
                stats.modified,
                stats.deleted,
                stats.hardlinked_files,
                stats.copied_bytes,
                copy_backend
            )),
        );
    }

    eprintln!("{}", dim.apply_to("--------------------"));

    if args.dry_run {
        let white = Style::new().white();
        eprintln!();
        report::report("Dry run", "no files written", &white);
        report::report(
            "Stats",
            &format!("+{}  ~{}  -{}", stats.new, stats.modified, stats.deleted),
            &white,
        );
        eprintln!();
        if timing {
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return Ok(());
    }

    // 5. Optional zip compression
    let mut final_path = dest_dir.clone();
    let is_zip;
    if compress {
        let zip_spinner = if can_interact() && !timing {
            let sp = indicatif::ProgressBar::new_spinner();
            sp.set_style(
                indicatif::ProgressStyle::default_spinner()
                    .template("{spinner:.dim} Archiving to .zip...")
                    .unwrap(),
            );
            sp.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(sp)
        } else {
            eprintln!("{}", dim.apply_to("Archiving to .zip..."));
            None
        };
        let zip_path = backups_root.join(format!("{folder_name}.zip"));
        let t_zip = Instant::now();
        match zip::compress_dir(&dest_dir, &zip_path) {
            Ok(_) => {
                let elapsed_zip = t_zip.elapsed();
                if let Some(ref sp) = zip_spinner {
                    sp.finish_and_clear();
                }
                let _ = fs::remove_dir_all(&dest_dir);
                final_path = zip_path;
                is_zip = true;
                if timing {
                    emit_backup_timing("zip", elapsed_zip, Some("compressed".to_string()));
                }
            }
            Err(e) => {
                if let Some(ref sp) = zip_spinner {
                    sp.finish_and_clear();
                }
                let detail = format!("Details: {e}");
                emit_warning(
                    "Zip failed; keeping folder instead.",
                    &[
                        detail.as_str(),
                        "Hint: Re-run with --no-compress to skip zipping.",
                    ],
                );
                is_zip = false;
            }
        }
    } else {
        eprintln!("{}", dim.apply_to("Keeping as folder (No Zip)..."));
        is_zip = false;
    }

    // 6. Retention
    let mut ret_cfg = cfg.retention.clone();
    if let Some(r) = args.retain {
        ret_cfg.max_backups = r;
    }
    let t_retention = Instant::now();
    let cleaned = retention::apply_retention_policy(&backups_root, &cfg.naming.prefix, &ret_cfg);
    if timing {
        emit_backup_timing(
            "retention",
            t_retention.elapsed(),
            Some(format!("cleaned={cleaned}")),
        );
    }

    // 6b. 写入备份元数据
    let t_meta = Instant::now();
    let backup_meta = meta::BackupMeta {
        version: 1,
        ts: meta::now_unix_secs(),
        desc: desc.clone(),
        tags: Vec::new(),
        stats: meta::BackupStats {
            new: stats.new,
            modified: stats.modified,
            deleted: stats.deleted,
        },
        incremental: args.incremental,
        size_bytes: if is_zip {
            fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0)
        } else {
            stats.logical_bytes
        },
    };
    // 目录备份直接写元数据；zip 备份写入已删除的 dest_dir（zip 前）不可用，改写到 backups_root 旁
    if !is_zip {
        meta::write_meta(&final_path, &backup_meta);
        // 生成 blake3 manifest（目录备份）
        {
            let mut file_hashes = std::collections::HashMap::new();
            for (rel, src) in &current {
                if let Some(hash) = checksum::file_blake3(&src.path) {
                    file_hashes.insert(rel.clone(), hash);
                }
            }
            checksum::write_manifest(&final_path, &file_hashes);
        }
    } else {
        // zip 已完成，元数据写到同名 .meta.json 旁
        let meta_path = backups_root.join(format!("{folder_name}.meta.json"));
        if let Ok(json) = serde_json::to_string_pretty(&backup_meta) {
            let _ = fs::write(&meta_path, json);
        }
    }
    if timing {
        emit_backup_timing("meta", t_meta.elapsed(), Some(format!("zip={is_zip}")));
    }

    // 7. Report
    let t_report = Instant::now();
    let size_bytes = backup_meta.size_bytes;
    let size_display = if size_bytes > 1_048_576 {
        format!("{:.2} MB", size_bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.2} KB", size_bytes as f64 / 1024.0)
    };

    let green = Style::new().green();
    let cyan = Style::new().cyan();
    let white = Style::new().white();

    println!("\n{}", green.apply_to("✔ Backup Success"));
    println!();
    report::report(
        "Version",
        &format!("{}{}", cfg.naming.prefix, ver.next_version),
        &yellow,
    );
    report::report(
        "Result",
        &final_path.file_name().unwrap().to_string_lossy(),
        &cyan,
    );
    report::report("Size", &size_display, &cyan);
    report::report(
        "Stats",
        &format!("+{}  ~{}  -{}", stats.new, stats.modified, stats.deleted),
        &white,
    );
    if cleaned > 0 {
        report::report("Cleaned", &format!("{cleaned} old backups"), &yellow);
    }
    let elapsed_total = t_total.elapsed();
    report::report(
        "Time",
        &format!("{:.2}s", elapsed_total.as_secs_f64()),
        &dim,
    );
    println!();
    if timing {
        emit_backup_timing("report", t_report.elapsed(), Some(size_display));
        emit_backup_timing("total", t_total.elapsed(), None);
    }
    Ok(())
}

fn cmd_backup_create(args: BackupCreateCmd) -> CliResult {
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
        let plan = SevenZSplitOutputPlan::prepare(&output, crate::backup_formats::OverwriteMode::Fail)?;
        let result = write_entries_to_7z_split(&refs, plan.temp_base_path(), split_size, &sevenz_options);
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
    if let Err(err) = plan.finalize() {
        return Err(err);
    }
    if show_progress {
        emit_progress_event(&ExportProgressEvent {
            phase: ExportProgressPhase::Write,
            selected_files: selected,
            processed_files: written,
            bytes_in,
            bytes_out: compute_created_output_bytes(options.format, &output),
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
        return emit_backup_create_xunbak_summary(options, &output, selected, 0, bytes_in);
    }

    let refs: Vec<&SourceEntry> = entries.iter().collect();
    let backup_options = crate::commands::xunbak::build_backup_options_from_raw(
        options.compression.as_deref(),
        options.split_size.as_deref(),
        options.no_compress,
    )?;
    let summary = crate::backup_export::xunbak_writer::write_entries_to_xunbak(
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

    emit_backup_create_xunbak_summary(
        options,
        &summary.destination,
        selected,
        summary.entry_count,
        summary.bytes_in,
    )
}

#[cfg(feature = "xunbak")]
fn emit_backup_create_xunbak_summary(
    options: &BackupCreateOptions,
    output: &Path,
    selected: usize,
    written: usize,
    bytes_in: u64,
) -> CliResult {
    if options.json {
        let summary = BackupCreateXunbakExecutionSummary {
            action: BackupAction::Create,
            status: ExportStatus::Ok,
            source: options.source_dir.display().to_string(),
            destination: output.display().to_string(),
            format: options.format,
            dry_run: options.dry_run,
            selected,
            written,
            skipped: 0,
            bytes_in,
            bytes_out: if options.dry_run {
                0
            } else {
                compute_created_output_bytes(options.format, output)
            },
            overwrite_count: 0,
            verify_source: "off".to_string(),
            verify_output: "off".to_string(),
            duration_ms: 0,
            outputs: if options.dry_run {
                Vec::new()
            } else {
                collect_created_output_paths(options.format, output)
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect()
            },
        };
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).map_err(|err| {
                CliError::new(1, format!("Serialize create xunbak output failed: {err}"))
            })?
        );
        return Ok(());
    }

    if options.dry_run {
        eprintln!(
            "DRY RUN: would create xunbak {} with {} file(s) / {} bytes",
            output.display(),
            selected,
            bytes_in
        );
    } else {
        eprintln!(
            "Created xunbak: {}  files={}  bytes={}",
            output.display(),
            written,
            bytes_in
        );
    }
    Ok(())
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

impl From<BackupRestoreCmd> for RestoreCmd {
    fn from(value: BackupRestoreCmd) -> Self {
        Self {
            name_or_path: value.name_or_path,
            file: value.file,
            glob: value.glob,
            to: value.to,
            snapshot: value.snapshot,
            dir: value.dir,
            dry_run: value.dry_run,
            yes: value.yes,
            json: value.json,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;

    use super::backup_timing_enabled_with;

    #[test]
    fn backup_timing_enabled_accepts_formal_env_name() {
        let env = HashMap::from([("XUN_BACKUP_TIMING", OsString::from("1"))]);
        assert!(backup_timing_enabled_with(|name| env.get(name).cloned()));
    }

    #[test]
    fn backup_timing_enabled_keeps_legacy_env_name_compatible() {
        let env = HashMap::from([("XUN_BAK_TIMING", OsString::from("1"))]);
        assert!(backup_timing_enabled_with(|name| env.get(name).cloned()));
    }
}
