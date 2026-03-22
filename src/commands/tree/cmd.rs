use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::cli::TreeCmd;
use crate::config;
use crate::output::{CliError, CliResult, emit_warning};
use crate::util::{normalize_glob_path, read_ignore_file, split_csv};

use super::build::{build_tree_inner, count_tree_inner};
#[cfg(target_os = "windows")]
use super::clipboard::copy_to_clipboard;
use super::constants::{EXCLUDE_EXTS, EXCLUDE_NAMES, EXCLUDE_PATHS};
use super::filters::parse_sort;
use super::stats::print_stats;
use super::types::{TreeFilters, TreeOutput};

pub(crate) fn cmd_tree(args: TreeCmd) -> CliResult {
    let root = match &args.path {
        Some(p) => {
            let mut policy = crate::path_guard::PathPolicy::for_read();
            policy.must_exist = false;
            let validation = crate::path_guard::validate_paths(vec![p.clone()], &policy);
            if !validation.issues.is_empty() {
                let details: Vec<String> = validation
                    .issues
                    .iter()
                    .map(|i| format!("{} ({})", i.raw, i.detail))
                    .collect();
                return Err(CliError::with_details(
                    2,
                    "Invalid path.".to_string(),
                    &details,
                ));
            }
            match validation.ok.into_iter().next() {
                Some(pb) => pb,
                None => PathBuf::from(p),
            }
        }
        None => std::env::current_dir().unwrap(),
    };
    if !root.is_dir() {
        return Err(CliError::with_details(
            1,
            format!("'{}' is not a valid directory.", root.display()),
            &["Fix: Pass a directory path, or omit [path] to use the current directory."],
        ));
    }

    let cfg = config::load_config();
    let depth = args.depth.or(cfg.tree.default_depth).unwrap_or(0);

    let mut exclude_names: Vec<String> = EXCLUDE_NAMES.iter().map(|s| s.to_lowercase()).collect();
    exclude_names.extend(cfg.tree.exclude_names.iter().map(|s| s.to_lowercase()));

    let exclude_paths: Vec<String> = EXCLUDE_PATHS
        .iter()
        .map(|s| normalize_glob_path(s).trim_end_matches('/').to_string())
        .collect();

    let mut exclude_patterns = Vec::new();
    let mut include_patterns = Vec::new();
    let ignore = read_ignore_file(&root.join(".xunignore"));
    exclude_patterns.extend(ignore.exclude);
    include_patterns.extend(ignore.include);

    exclude_patterns.extend(
        split_csv(&args.exclude)
            .into_iter()
            .map(|p| normalize_glob_path(&p)),
    );
    include_patterns.extend(
        split_csv(&args.include)
            .into_iter()
            .map(|p| normalize_glob_path(&p)),
    );

    let filters = TreeFilters {
        hidden: args.hidden,
        exclude_names,
        exclude_paths,
        exclude_exts: EXCLUDE_EXTS.iter().map(|s| s.to_lowercase()).collect(),
        exclude_patterns,
        include_patterns,
    };

    let sort = parse_sort(&args.sort).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid sort: {}.", args.sort),
            &["Fix: Use one of: name | mtime | size"],
        )
    })?;

    if args.stats_only {
        let start = Instant::now();
        let mut count = 1usize;
        count_tree_inner(&root, 1, depth, &root, &filters, &mut count, args.max_items);
        let elapsed = start.elapsed();
        print_stats(&root, count, elapsed, depth);
        return Ok(());
    }

    let need_buffer = args.output.is_some() || !args.no_clip;
    let start = Instant::now();
    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    if need_buffer {
        let mut lines = vec![format!("{root_name}/")];
        let mut count = 1usize;
        let mut output = TreeOutput::Buffer(&mut lines);
        let mut prefix = String::new();
        let mut size_memo: HashMap<PathBuf, u64> = HashMap::new();
        build_tree_inner(
            &root,
            &mut prefix,
            1,
            depth,
            &root,
            &filters,
            sort,
            args.fast,
            args.size,
            args.plain,
            &mut count,
            args.max_items,
            &mut output,
            &mut size_memo,
        );
        let elapsed = start.elapsed();

        for l in &lines {
            out_println!("{l}");
        }
        if let Some(ref out_path) = args.output {
            match fs::File::create(out_path) {
                Ok(mut f) => {
                    for l in &lines {
                        let _ = writeln!(f, "{l}");
                    }
                    ui_println!("\u{2713} Saved to \"{out_path}\"");
                }
                Err(e) => {
                    let detail = format!("Details: {e}");
                    let fix = format!(
                        "Fix: Check that the output path is writable: {}",
                        Path::new(out_path).display()
                    );
                    emit_warning(
                        "Failed to save output file.",
                        &[detail.as_str(), fix.as_str()],
                    );
                }
            }
        }
        #[cfg(target_os = "windows")]
        if !args.no_clip {
            copy_to_clipboard(&lines.join("\n"));
            ui_println!("\u{2713} Copied to clipboard");
        }
        print_stats(&root, lines.len(), elapsed, depth);
    } else {
        out_println!("{root_name}/");
        let mut count = 1usize;
        let mut output = TreeOutput::Stream;
        let mut prefix = String::new();
        let mut size_memo: HashMap<PathBuf, u64> = HashMap::new();
        build_tree_inner(
            &root,
            &mut prefix,
            1,
            depth,
            &root,
            &filters,
            sort,
            args.fast,
            args.size,
            args.plain,
            &mut count,
            args.max_items,
            &mut output,
            &mut size_memo,
        );
        let elapsed = start.elapsed();
        print_stats(&root, count, elapsed, depth);
    }

    Ok(())
}
