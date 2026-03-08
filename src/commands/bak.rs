use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use console::Style;

use crate::cli::BakCmd;
use crate::output::{CliError, CliResult, can_interact, emit_warning};
use crate::runtime;
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};

#[path = "bak/baseline.rs"]
mod baseline;
#[path = "bak/config.rs"]
mod config;
#[path = "bak/diff.rs"]
mod diff;
#[path = "bak/list.rs"]
mod list;
#[path = "bak/report.rs"]
mod report;
#[path = "bak/restore.rs"]
mod restore;
#[path = "bak/retention.rs"]
mod retention;
#[path = "bak/scan.rs"]
mod scan;
#[path = "bak/time_fmt.rs"]
mod time_fmt;
#[path = "bak/util.rs"]
mod util;
#[path = "bak/version.rs"]
mod version;
#[path = "bak/zip.rs"]
mod zip;

pub(crate) fn cmd_bak(args: BakCmd) -> CliResult {
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
            "restore" => {
                let Some(name) = args.op_args.get(1).map(|s| s.as_str()) else {
                    return Err(CliError::with_details(
                        2,
                        "Missing backup name.".to_string(),
                        &["Fix: Use `xun bak restore <name>` (from `xun bak list`)."],
                    ));
                };
                return restore::cmd_bak_restore(&root, &cfg, name, &args);
            }
            _ => {
                return Err(CliError::with_details(
                    2,
                    format!("Unknown bak operation: {op}"),
                    &[
                        "Fix: Use `xun bak` to create a backup.",
                        "Fix: Use `xun bak list` to list backups.",
                        "Fix: Use `xun bak restore <name>` to restore.",
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
    let max_backups = args.retain.unwrap_or(cfg.retention.max_backups);

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
    } else if args.yes || !can_interact() {
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
    let folder_name = format!(
        "{}{}-{}_{}",
        cfg.naming.prefix, ver.next_version, desc, date_str
    );
    let dest_dir = backups_root.join(&folder_name);
    if !args.dry_run {
        let _ = fs::create_dir_all(&dest_dir);
    }

    // 4. Scan + diff + copy
    eprintln!("\n{}", dim.apply_to("Analysis & Backup..."));
    eprintln!("{}", dim.apply_to("--------------------"));

    let current = scan::scan_files(&root, &include_roots, &exclude_patterns, &include_patterns);
    let mut baseline = match &ver.prev_path {
        Some(p) => baseline::read_baseline(p),
        None => HashMap::new(),
    };
    let copy = !args.dry_run;
    let skip_unchanged = !compress;
    let show_unchanged = runtime::is_verbose() && skip_unchanged;
    let stats = diff::diff_copy_and_print(
        &current,
        &mut baseline,
        &dest_dir,
        copy,
        skip_unchanged,
        show_unchanged,
    );

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
        eprintln!("{}", dim.apply_to("Archiving to .zip..."));
        let zip_path = backups_root.join(format!("{folder_name}.zip"));
        match zip::compress_dir(&dest_dir, &zip_path) {
            Ok(_) => {
                let _ = fs::remove_dir_all(&dest_dir);
                final_path = zip_path;
                is_zip = true;
            }
            Err(e) => {
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
    let cleaned = retention::apply_retention(
        &backups_root,
        &cfg.naming.prefix,
        max_backups,
        cfg.retention.delete_count,
    );

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

    eprintln!("\n{}", green.apply_to("✔ Backup Success"));
    eprintln!();
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
    eprintln!();
    Ok(())
}
