use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use console::Style;

use crate::cli::{BackupCmd, BackupSubCommand};
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
mod scan;
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

pub(crate) fn cmd_backup(args: BackupCmd) -> CliResult {
    let t_total = Instant::now();
    let timing = backup_timing_enabled();
    let copy_backend = detect_copy_backend_for_backup();

    let t_config = Instant::now();
    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };
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
            BackupSubCommand::List(cmd) => ("list", list::cmd_backup_list(&root, &cfg, cmd.json)),
            BackupSubCommand::Verify(cmd) => (
                "verify",
                verify::cmd_backup_verify(&root, &cfg, &cmd.name, cmd.json),
            ),
            BackupSubCommand::Find(cmd) => (
                "find",
                find::cmd_backup_find(&root, &cfg, cmd.tag.as_deref(), None, None, cmd.json),
            ),
        };
        if timing {
            emit_backup_timing("subcommand", t_sub.elapsed(), Some(label.to_string()));
            emit_backup_timing("total", t_total.elapsed(), None);
        }
        return result;
    }

    let t_rules = Instant::now();
    let mut include_roots: Vec<String> = Vec::new();
    let mut include_patterns: Vec<String> = Vec::new();
    for inc in &cfg.include {
        if util::is_glob(inc) {
            include_patterns.push(normalize_glob_path(inc));
        } else {
            include_roots.push(inc.clone());
        }
    }
    for inc in split_csv(&args.include) {
        if util::is_glob(&inc) {
            include_patterns.push(normalize_glob_path(&inc));
        } else {
            include_roots.push(inc);
        }
    }

    let mut exclude_patterns: Vec<String> =
        cfg.exclude.iter().map(|e| normalize_glob_path(e)).collect();
    exclude_patterns.extend(
        split_csv(&args.exclude)
            .into_iter()
            .map(|e| normalize_glob_path(&e)),
    );
    if cfg.use_gitignore {
        let ignore = read_ignore_file(&root.join(".gitignore"));
        exclude_patterns.extend(ignore.exclude);
        include_patterns.extend(ignore.include);
    }

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
                include_roots.len(),
                include_patterns.len(),
                exclude_patterns.len()
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
    let current = scan::scan_files(&root, &include_roots, &exclude_patterns, &include_patterns);
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
    let size_bytes = if is_zip {
        fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0)
    } else {
        stats.logical_bytes
    };
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
