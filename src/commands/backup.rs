use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use crate::cli::{BackupCmd, BackupSubCommand};
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::runtime;
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};
use crate::windows::file_copy::detect_copy_backend_for_backup;
use console::Style;
use serde::Serialize;

pub(crate) use crate::backup::legacy::{
    baseline, config, diff, find, hash_diff, hash_manifest, list, meta, report, retention, scan,
    time_fmt, util, verify, version, zip,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffMode {
    Auto,
    Hash,
    Meta,
}

impl DiffMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hash => "hash",
            Self::Meta => "meta",
        }
    }
}

fn parse_diff_mode(value: Option<&str>) -> Result<DiffMode, CliError> {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(DiffMode::Auto),
        "hash" => Ok(DiffMode::Hash),
        "meta" => Ok(DiffMode::Meta),
        other => Err(CliError::with_details(
            2,
            format!("Invalid --diff-mode: {other}"),
            &["Fix: Use one of `auto`, `hash`, or `meta`."],
        )),
    }
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

struct BackupDerivedStats {
    rename_only_count: u32,
    reused_bytes: u64,
    cache_hit_ratio: f64,
    baseline_source: &'static str,
}

#[derive(Serialize)]
struct BackupRunJsonView {
    action: &'static str,
    status: &'static str,
    backup_name: String,
    result: Option<String>,
    backup_type: &'static str,
    diff_mode: &'static str,
    incremental: bool,
    dry_run: bool,
    baseline_mode: &'static str,
    selected_files: u64,
    baseline_entries: u64,
    new: u32,
    modified: u32,
    reused: u32,
    deleted: u32,
    hash_checked_files: u64,
    hash_cache_hits: u64,
    hash_computed_files: u64,
    hash_failed_files: u64,
    rename_only_count: u32,
    reused_bytes: u64,
    cache_hit_ratio: f64,
    baseline_source: &'static str,
    hardlinked_files: u32,
    logical_bytes: u64,
    copied_bytes: u64,
    cleaned: usize,
    duration_ms: u64,
}

fn emit_backup_json(payload: &BackupRunJsonView) {
    out_println!(
        "{}",
        serde_json::to_string_pretty(payload).unwrap_or_default()
    );
}

fn derive_backup_stats(
    diff_entries: &[diff::DiffEntry],
    scan_hash_stats: &scan::ScanHashStats,
    baseline_mode: &'static str,
) -> BackupDerivedStats {
    let deleted_paths: std::collections::HashSet<&str> = diff_entries
        .iter()
        .filter(|entry| matches!(entry.kind, diff::DiffKind::Deleted))
        .map(|entry| entry.rel.as_str())
        .collect();
    let rename_only_count = diff_entries
        .iter()
        .filter(|entry| {
            matches!(entry.kind, diff::DiffKind::Reused)
                && entry
                    .reuse_from_rel
                    .as_deref()
                    .is_some_and(|path| deleted_paths.contains(path))
        })
        .count() as u32;
    let reused_bytes = diff_entries
        .iter()
        .filter(|entry| matches!(entry.kind, diff::DiffKind::Reused))
        .map(|entry| entry.file_size)
        .sum();
    let cache_hit_ratio = if scan_hash_stats.hash_checked_files == 0 {
        0.0
    } else {
        scan_hash_stats.hash_cache_hits as f64 / scan_hash_stats.hash_checked_files as f64
    };
    BackupDerivedStats {
        rename_only_count,
        reused_bytes,
        cache_hit_ratio,
        baseline_source: baseline_mode,
    }
}

pub(crate) fn cmd_backup(args: BackupCmd) -> CliResult {
    if let Some(subcommand) = args.cmd.clone() {
        match subcommand {
            BackupSubCommand::Create(cmd) => {
                return crate::backup::app::create::cmd_backup_create(cmd);
            }
            BackupSubCommand::Restore(cmd) => {
                return crate::backup::app::restore::cmd_restore(cmd);
            }
            BackupSubCommand::Convert(cmd) => {
                return crate::backup::app::convert::cmd_backup_convert(cmd);
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
            return crate::backup::app::xunbak::cmd_backup_container(&args, &root);
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

    let resolved_diff_mode = parse_diff_mode(args.diff_mode.as_deref())?;

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

    // 增量模式只追踪变更文件；全量模式（含目录备份）需追踪所有文件
    let skip_unchanged = args.incremental;
    let show_unchanged = runtime::is_verbose() && !skip_unchanged;
    let (
        mut current,
        mut scan_hash_stats,
        baseline_mode,
        baseline_entry_count,
        diff_entries,
        elapsed_scan,
        elapsed_baseline,
    ) = match resolved_diff_mode {
        DiffMode::Meta => {
            let t_scan = Instant::now();
            let current = scan::scan_files(
                &root,
                &scan_rules.include_roots,
                &scan_rules.exclude_patterns,
                &scan_rules.include_patterns,
            );
            let elapsed_scan = t_scan.elapsed();
            let t_baseline = Instant::now();
            let mut baseline = match &ver.prev_path {
                Some(prev) => baseline::read_metadata_only_baseline(prev),
                None => HashMap::new(),
            };
            let entry_count = baseline.len();
            let mode = if ver.prev_path.is_some() {
                "metadata"
            } else {
                "fresh_full"
            };
            let diff_entries = diff::compute_diff(&current, &mut baseline, skip_unchanged);
            (
                current,
                scan::ScanHashStats::default(),
                mode,
                entry_count,
                diff_entries,
                elapsed_scan,
                t_baseline.elapsed(),
            )
        }
        DiffMode::Auto => {
            let t_scan = Instant::now();
            let scan_result = scan::scan_files_with_hash_details(
                &root,
                &scan_rules.include_roots,
                &scan_rules.exclude_patterns,
                &scan_rules.include_patterns,
            );
            let current = scan_result.files;
            let scan_hash_stats = scan_result.stats;
            let elapsed_scan = t_scan.elapsed();
            let t_baseline = Instant::now();

            let previous_hash_manifest = match &ver.prev_path {
                Some(prev) => hash_manifest::read_backup_snapshot_manifest(prev).ok(),
                None => None,
            };
            if let Some(ref manifest) = previous_hash_manifest {
                let entry_count = manifest.entries.len();
                let diff_entries = convert_hash_diff_entries(
                    &current,
                    hash_diff::diff_against_hash_manifest(&current, manifest),
                    skip_unchanged,
                );
                (
                    current,
                    scan_hash_stats,
                    "hash_manifest",
                    entry_count,
                    diff_entries,
                    elapsed_scan,
                    t_baseline.elapsed(),
                )
            } else {
                if ver.prev_path.is_some() {
                    emit_warning(
                        "Previous backup is missing .bak-manifest.json; running a fresh full comparison.",
                        &[
                            "Legacy size/mtime metadata is no longer used as incremental truth.",
                            "Fix: Re-create the previous backup if you want hash-based incremental reuse.",
                        ],
                    );
                }
                let mut baseline = HashMap::new();
                let diff_entries = diff::compute_diff(&current, &mut baseline, skip_unchanged);
                (
                    current,
                    scan_hash_stats,
                    "fresh_full",
                    0,
                    diff_entries,
                    elapsed_scan,
                    t_baseline.elapsed(),
                )
            }
        }
        DiffMode::Hash => {
            let t_scan = Instant::now();
            let scan_result = scan::scan_files_with_hash_details(
                &root,
                &scan_rules.include_roots,
                &scan_rules.exclude_patterns,
                &scan_rules.include_patterns,
            );
            let current = scan_result.files;
            let scan_hash_stats = scan_result.stats;
            let elapsed_scan = t_scan.elapsed();
            let t_baseline = Instant::now();

            match &ver.prev_path {
                Some(prev) => {
                    let manifest = hash_manifest::read_backup_snapshot_manifest(prev).map_err(|_| {
                        CliError::with_details(
                            2,
                            "Hash diff mode requires previous backup .bak-manifest.json",
                            &[
                                "Fix: Re-create the previous backup with the new hash manifest.",
                                "Fix: Or rerun with `--diff-mode auto` / `--diff-mode meta`.",
                            ],
                        )
                    })?;
                    let entry_count = manifest.entries.len();
                    let diff_entries = convert_hash_diff_entries(
                        &current,
                        hash_diff::diff_against_hash_manifest(&current, &manifest),
                        skip_unchanged,
                    );
                    (
                        current,
                        scan_hash_stats,
                        "hash_manifest",
                        entry_count,
                        diff_entries,
                        elapsed_scan,
                        t_baseline.elapsed(),
                    )
                }
                None => {
                    let mut baseline = HashMap::new();
                    let diff_entries = diff::compute_diff(&current, &mut baseline, skip_unchanged);
                    (
                        current,
                        scan_hash_stats,
                        "fresh_full",
                        0,
                        diff_entries,
                        elapsed_scan,
                        t_baseline.elapsed(),
                    )
                }
            }
        }
    };
    if let Some(ref sp) = scan_spinner {
        sp.finish_and_clear();
    }
    if timing {
        emit_backup_timing(
            "scan",
            elapsed_scan,
            Some(format!(
                "files={} diff_mode={}",
                current.len(),
                resolved_diff_mode.as_str()
            )),
        );
    }
    if timing {
        emit_backup_timing(
            "baseline",
            elapsed_baseline,
            Some(format!(
                "entries={baseline_entry_count} mode={baseline_mode}"
            )),
        );
    }
    let t_diff = Instant::now();
    if timing {
        emit_backup_timing(
            "diff",
            t_diff.elapsed(),
            Some(format!("entries={}", diff_entries.len())),
        );
    }
    let derived_stats = derive_backup_stats(&diff_entries, &scan_hash_stats, baseline_mode);
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
        if args.json {
            emit_backup_json(&BackupRunJsonView {
                action: "backup",
                status: "skipped",
                backup_name: folder_name.clone(),
                result: None,
                backup_type: "dir",
                diff_mode: resolved_diff_mode.as_str(),
                incremental: args.incremental,
                dry_run: false,
                baseline_mode,
                selected_files: current.len() as u64,
                baseline_entries: baseline_entry_count as u64,
                new: 0,
                modified: 0,
                reused: 0,
                deleted: 0,
                hash_checked_files: scan_hash_stats.hash_checked_files,
                hash_cache_hits: scan_hash_stats.hash_cache_hits,
                hash_computed_files: scan_hash_stats.hash_computed_files,
                hash_failed_files: scan_hash_stats.hash_failed_files,
                rename_only_count: derived_stats.rename_only_count,
                reused_bytes: derived_stats.reused_bytes,
                cache_hit_ratio: derived_stats.cache_hit_ratio,
                baseline_source: derived_stats.baseline_source,
                hardlinked_files: 0,
                logical_bytes: 0,
                copied_bytes: 0,
                cleaned: 0,
                duration_ms: t_total.elapsed().as_millis() as u64,
            });
        }
        if timing {
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return Ok(());
    }

    if matches!(resolved_diff_mode, DiffMode::Meta) {
        let hash_scan_result = scan::scan_files_with_hash_details(
            &root,
            &scan_rules.include_roots,
            &scan_rules.exclude_patterns,
            &scan_rules.include_patterns,
        );
        current = hash_scan_result.files;
        scan_hash_stats = hash_scan_result.stats;
    }

    let removed_paths: Vec<String> = diff_entries
        .iter()
        .filter(|entry| matches!(entry.kind, diff::DiffKind::Deleted))
        .map(|entry| entry.rel.replace('\\', "/"))
        .collect();
    let snapshot_entries = current
        .iter()
        .filter_map(|(rel, scanned)| {
            scanned.content_hash.map(|content_hash| {
                crate::backup::legacy::hash_manifest::BackupSnapshotEntry {
                    path: rel.replace('\\', "/"),
                    content_hash,
                    size: scanned.size,
                    mtime_ns: scanned.modified_ns,
                    created_time_ns: scanned.created_time_ns,
                    win_attributes: scanned.win_attributes,
                    file_id: scanned.file_id.clone(),
                }
            })
        })
        .collect();
    let snapshot_manifest = crate::backup::legacy::hash_manifest::BackupSnapshotManifest::new(
        root.display().to_string(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0),
        snapshot_entries,
        removed_paths,
    );

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
                        diff::DiffKind::New
                            | diff::DiffKind::Modified
                            | diff::DiffKind::Reused
                            | diff::DiffKind::Unchanged
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
            reused: 0,
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
                "+{} ~{} ↺{} -{} linked={} copied={}B backend={:?}",
                stats.new,
                stats.modified,
                stats.reused,
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
            &format!(
                "+{}  ~{}  ↺{}  -{}",
                stats.new, stats.modified, stats.reused, stats.deleted
            ),
            &white,
        );
        eprintln!();
        if args.json {
            emit_backup_json(&BackupRunJsonView {
                action: "backup",
                status: "dry_run",
                backup_name: folder_name.clone(),
                result: None,
                backup_type: "dir",
                diff_mode: resolved_diff_mode.as_str(),
                incremental: args.incremental,
                dry_run: true,
                baseline_mode,
                selected_files: current.len() as u64,
                baseline_entries: baseline_entry_count as u64,
                new: stats.new,
                modified: stats.modified,
                reused: stats.reused,
                deleted: stats.deleted,
                hash_checked_files: scan_hash_stats.hash_checked_files,
                hash_cache_hits: scan_hash_stats.hash_cache_hits,
                hash_computed_files: scan_hash_stats.hash_computed_files,
                hash_failed_files: scan_hash_stats.hash_failed_files,
                rename_only_count: derived_stats.rename_only_count,
                reused_bytes: derived_stats.reused_bytes,
                cache_hit_ratio: derived_stats.cache_hit_ratio,
                baseline_source: derived_stats.baseline_source,
                hardlinked_files: stats.hardlinked_files,
                logical_bytes: stats.logical_bytes,
                copied_bytes: stats.copied_bytes,
                cleaned: 0,
                duration_ms: t_total.elapsed().as_millis() as u64,
            });
        }
        if timing {
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return Ok(());
    }

    let _ = crate::backup::legacy::hash_manifest::write_backup_snapshot_manifest(
        &dest_dir,
        &snapshot_manifest,
    );

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
            reused: stats.reused,
            deleted: stats.deleted,
            hash_checked_files: scan_hash_stats.hash_checked_files,
            hash_cache_hits: scan_hash_stats.hash_cache_hits,
            hash_computed_files: scan_hash_stats.hash_computed_files,
            rename_only_count: derived_stats.rename_only_count,
            reused_bytes: derived_stats.reused_bytes,
            cache_hit_ratio: derived_stats.cache_hit_ratio,
            baseline_source: derived_stats.baseline_source.to_string(),
            hardlinked_files: stats.hardlinked_files,
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

    if !args.json {
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
            &format!(
                "+{}  ~{}  ↺{}  -{}",
                stats.new, stats.modified, stats.reused, stats.deleted
            ),
            &white,
        );
        report::report(
            "Hash",
            &format!(
                "{} checked  {} cache-hit  {} recomputed",
                scan_hash_stats.hash_checked_files,
                scan_hash_stats.hash_cache_hits,
                scan_hash_stats.hash_computed_files
            ),
            &white,
        );
        report::report(
            "Cache Hit",
            &format!("{:.2}%", derived_stats.cache_hit_ratio * 100.0),
            &white,
        );
        report::report(
            "Reuse",
            &format!(
                "{} hardlinks  {} copied",
                stats.hardlinked_files, stats.copied_bytes
            ),
            &white,
        );
        report::report(
            "Reuse Stats",
            &format!(
                "rename-only={}  reused-bytes={}",
                derived_stats.rename_only_count, derived_stats.reused_bytes
            ),
            &white,
        );
        report::report("Baseline Source", derived_stats.baseline_source, &dim);
        if cleaned > 0 {
            report::report("Cleaned", &format!("{cleaned} old backups"), &yellow);
        }
    }
    let elapsed_total = t_total.elapsed();
    if !args.json {
        report::report(
            "Time",
            &format!("{:.2}s", elapsed_total.as_secs_f64()),
            &dim,
        );
        println!();
    }
    if args.json {
        emit_backup_json(&BackupRunJsonView {
            action: "backup",
            status: "ok",
            backup_name: folder_name.clone(),
            result: Some(final_path.display().to_string()),
            backup_type: if is_zip { "zip" } else { "dir" },
            diff_mode: resolved_diff_mode.as_str(),
            incremental: args.incremental,
            dry_run: false,
            baseline_mode,
            selected_files: current.len() as u64,
            baseline_entries: baseline_entry_count as u64,
            new: stats.new,
            modified: stats.modified,
            reused: stats.reused,
            deleted: stats.deleted,
            hash_checked_files: scan_hash_stats.hash_checked_files,
            hash_cache_hits: scan_hash_stats.hash_cache_hits,
            hash_computed_files: scan_hash_stats.hash_computed_files,
            hash_failed_files: scan_hash_stats.hash_failed_files,
            rename_only_count: derived_stats.rename_only_count,
            reused_bytes: derived_stats.reused_bytes,
            cache_hit_ratio: derived_stats.cache_hit_ratio,
            baseline_source: derived_stats.baseline_source,
            hardlinked_files: stats.hardlinked_files,
            logical_bytes: stats.logical_bytes,
            copied_bytes: stats.copied_bytes,
            cleaned,
            duration_ms: elapsed_total.as_millis() as u64,
        });
    }
    if timing {
        emit_backup_timing("report", t_report.elapsed(), Some(size_display));
        emit_backup_timing("total", t_total.elapsed(), None);
    }
    Ok(())
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

fn convert_hash_diff_entries(
    current: &HashMap<String, scan::ScannedFile>,
    entries: Vec<hash_diff::HashDiffEntry>,
    skip_unchanged: bool,
) -> Vec<diff::DiffEntry> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let rel = entry.path.replace('/', "\\");
        match entry.kind {
            hash_diff::HashDiffKind::Unchanged => {
                if skip_unchanged {
                    continue;
                }
                if let Some(scanned) = current.get(&entry.path).or_else(|| current.get(&rel)) {
                    out.push(diff::DiffEntry {
                        rel,
                        src_path: Some(scanned.path.clone()),
                        kind: diff::DiffKind::Unchanged,
                        size_delta: 0,
                        file_size: scanned.size,
                        reuse_from_rel: None,
                    });
                }
            }
            hash_diff::HashDiffKind::Modified => {
                if let Some(scanned) = current.get(&entry.path).or_else(|| current.get(&rel)) {
                    out.push(diff::DiffEntry {
                        rel,
                        src_path: Some(scanned.path.clone()),
                        kind: diff::DiffKind::Modified,
                        size_delta: 0,
                        file_size: scanned.size,
                        reuse_from_rel: None,
                    });
                }
            }
            hash_diff::HashDiffKind::Reused => {
                if let Some(scanned) = current.get(&entry.path).or_else(|| current.get(&rel)) {
                    out.push(diff::DiffEntry {
                        rel,
                        src_path: Some(scanned.path.clone()),
                        kind: diff::DiffKind::Reused,
                        size_delta: 0,
                        file_size: scanned.size,
                        reuse_from_rel: entry.reuse_from_path.map(|value| value.replace('/', "\\")),
                    });
                }
            }
            hash_diff::HashDiffKind::New => {
                if let Some(scanned) = current.get(&entry.path).or_else(|| current.get(&rel)) {
                    out.push(diff::DiffEntry {
                        rel,
                        src_path: Some(scanned.path.clone()),
                        kind: diff::DiffKind::New,
                        size_delta: scanned.size as i64,
                        file_size: scanned.size,
                        reuse_from_rel: None,
                    });
                }
            }
            hash_diff::HashDiffKind::Deleted => {
                out.push(diff::DiffEntry {
                    rel,
                    src_path: None,
                    kind: diff::DiffKind::Deleted,
                    size_delta: 0,
                    file_size: 0,
                    reuse_from_rel: None,
                });
            }
        }
    }
    out
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
