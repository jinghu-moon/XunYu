use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use console::Style;

use crate::cli::BakCmd;
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::runtime;
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};

#[path = "bak/baseline.rs"]
mod baseline;
#[path = "bak/checksum.rs"]
mod checksum;
#[path = "bak/config.rs"]
pub(crate) mod config;
#[path = "bak/diff.rs"]
mod diff;
#[path = "bak/find.rs"]
mod find;
#[path = "bak/list.rs"]
mod list;
#[path = "bak/meta.rs"]
mod meta;
#[path = "bak/report.rs"]
mod report;
#[path = "bak/retention.rs"]
mod retention;
#[path = "bak/scan.rs"]
mod scan;
#[path = "bak/time_fmt.rs"]
mod time_fmt;
#[path = "bak/util.rs"]
mod util;
#[path = "bak/verify.rs"]
mod verify;
#[path = "bak/version.rs"]
mod version;
#[path = "bak/zip.rs"]
mod zip;

pub(crate) use baseline::{FileMeta, read_baseline};

pub(crate) fn cmd_bak(args: BakCmd) -> CliResult {
    let t_total = Instant::now();
    let timing = std::env::var_os("XUN_BAK_TIMING").is_some();

    let root = match &args.dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()
            .map_err(|e| CliError::new(1, format!("Failed to get current directory: {e}")))?,
    };
    let cfg = config::load_config(&root);

    if !args.op_args.is_empty() {
        let op = args.op_args[0].as_str();
        match op.to_ascii_lowercase().as_str() {
            "list" => return list::cmd_bak_list(&root, &cfg),
            "verify" => {
                let Some(name) = args.op_args.get(1).map(|s| s.as_str()) else {
                    return Err(CliError::with_details(
                        2,
                        "Missing backup name.".to_string(),
                        &["Fix: Use `xun bak verify <name>`."],
                    ));
                };
                return verify::cmd_bak_verify(&root, &cfg, name);
            }
            "find" => {
                let tag = args.op_args.get(1).map(|s| s.as_str());
                return find::cmd_bak_find(&root, &cfg, tag, None, None);
            }
            _ => {
                return Err(CliError::with_details(
                    2,
                    format!("Unknown bak operation: {op}"),
                    &[
                        "Fix: Use `xun bak` to create a backup.",
                        "Fix: Use `xun bak list` to list backups.",
                        "Fix: Use `xun bak verify <name>` to verify integrity.",
                        "Fix: Use `xun bak find [tag]` to search backups.",
                        "Fix: Use `xun restore <name>` to restore a backup.",
                    ],
                ));
            }
        }
    }

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

    // 1. Version scanning
    let backups_root = root.join(&cfg.storage.backups_dir);
    let ver = version::scan_versions(&backups_root, &cfg.naming.prefix);

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

    // 3. Build folder name
    let date_str = time_fmt::format_now(&cfg.naming.date_format);
    let incr_suffix = if args.incremental { "-incr" } else { "" };
    let folder_name = format!(
        "{}{}-{}_{}{}",
        cfg.naming.prefix, ver.next_version, desc, date_str, incr_suffix
    );
    let dest_dir = backups_root.join(&folder_name);
    if !args.dry_run {
        let _ = fs::create_dir_all(&dest_dir);
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
        eprintln!("{}", dim.apply_to(format!("  [scan]  {:>6} files  {:>5}ms", current.len(), elapsed_scan.as_millis())));
    }

    let t_baseline = Instant::now();
    let mut baseline = match &ver.prev_path {
        Some(p) => baseline::read_baseline(p),
        None => HashMap::new(),
    };
    if timing {
        eprintln!("{}", dim.apply_to(format!("  [baseline] {:>5}ms", t_baseline.elapsed().as_millis())));
    }

    // 增量模式只追踪变更文件；全量模式（含目录备份）需追踪所有文件
    let skip_unchanged = args.incremental;
    let show_unchanged = runtime::is_verbose() && !skip_unchanged;
    let t_diff = Instant::now();
    let diff_entries = diff::compute_diff(&current, &mut baseline, skip_unchanged);
    if timing {
        eprintln!("{}", dim.apply_to(format!("  [diff]   {:>6} entries  {:>5}ms", diff_entries.len(), t_diff.elapsed().as_millis())));
    }
    diff::print_diff(&diff_entries, show_unchanged);

    let t_copy = Instant::now();
    let stats = if !args.dry_run {
        // copy 进度：显示文件数
        let copy_bar = if can_interact() && !timing {
            let total = diff_entries.iter()
                .filter(|e| matches!(e.kind, diff::DiffKind::New | diff::DiffKind::Modified | diff::DiffKind::Unchanged))
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
        let result = diff::apply_diff(&diff_entries, &dest_dir, args.incremental);
        if let Some(ref pb) = copy_bar {
            pb.finish_and_clear();
        }
        result
    } else {
        diff::DiffStats { new: 0, modified: 0, deleted: 0 }
    };
    if timing {
        eprintln!("{}", dim.apply_to(format!("  [copy]   +{}~{}-{}  {:>5}ms", stats.new, stats.modified, stats.deleted, t_copy.elapsed().as_millis())));
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
                    eprintln!("{}", dim.apply_to(format!("  [zip]    {:>5}ms", elapsed_zip.as_millis())));
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
    let cleaned = retention::apply_retention_policy(
        &backups_root,
        &cfg.naming.prefix,
        &ret_cfg,
    );
    if timing {
        eprintln!("{}", dim.apply_to(format!("  [retention] cleaned={cleaned}  {:>5}ms", t_retention.elapsed().as_millis())));
    }

    // 6b. 写入备份元数据
    let bak_meta = meta::BakMeta {
        version: 1,
        ts: meta::now_unix_secs(),
        desc: desc.clone(),
        tags: Vec::new(),
        stats: meta::BakStats {
            new: stats.new,
            modified: stats.modified,
            deleted: stats.deleted,
        },
        incremental: args.incremental,
    };
    // 目录备份直接写元数据；zip 备份写入已删除的 dest_dir（zip 前）不可用，改写到 backups_root 旁
    if !is_zip {
        meta::write_meta(&final_path, &bak_meta);
        // 生成 blake3 manifest（目录备份）
        #[cfg(feature = "bak")]
        {
            let mut file_hashes = std::collections::HashMap::new();
            for (rel, src) in &current {
                if let Some(hash) = checksum::file_blake3(src) {
                    file_hashes.insert(rel.clone(), hash);
                }
            }
            checksum::write_manifest(&final_path, &file_hashes);
        }
    } else {
        // zip 已完成，元数据写到同名 .meta.json 旁
        let meta_path = backups_root.join(format!("{folder_name}.meta.json"));
        if let Ok(json) = serde_json::to_string_pretty(&bak_meta) {
            let _ = fs::write(&meta_path, json);
        }
    }

    // 7. Report
    let size_bytes = if is_zip {
        fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0)
    } else {
        util::dir_size(&final_path)
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
    report::report("Time", &format!("{:.2}s", elapsed_total.as_secs_f64()), &dim);
    println!();
    Ok(())
}
