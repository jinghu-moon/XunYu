use crate::cli::DeleteCmd;
use crate::output::{CliError, CliResult, emit_warning};
use crate::util::split_csv;

use super::filters::{
    build_exclude_dirs, build_target_names, filter_direct_files, parse_name_filter, reserved_names,
};
use super::paths::{absolute_path, classify_path};
use super::pipeline::{delete_paths, run_cli_pipeline};
use super::render::{print_summary, render_results, write_csv};
use super::scanner;
use super::types::{DeleteOptions, DeleteRecord, PathKind};
use super::winapi;

#[cfg(feature = "delete_tui")]
use super::tui;

#[path = "cmd/bookmark.rs"]
mod bookmark;
#[path = "cmd/preflight.rs"]
mod preflight;

use bookmark::cmd_delete_bookmark;
use preflight::{maybe_relaunch_elevated, should_use_tui, validate_level};

pub(crate) fn cmd_delete(args: DeleteCmd) -> CliResult {
    if args.bookmark {
        return cmd_delete_bookmark(args);
    }

    if args.any && args.reserved {
        return Err(CliError::with_details(
            2,
            "Conflicting flags: --any and --reserved.".to_string(),
            &["Fix: Use --any to allow non-reserved names, or omit it for reserved-only."],
        ));
    }

    let level = validate_level(args.level)?;
    if maybe_relaunch_elevated()? {
        return Ok(());
    }

    let mut paths = args.paths.clone();
    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let name_filter = parse_name_filter(&args.name);
    let reserved = reserved_names();
    let (target_names, match_all) = build_target_names(&reserved, &name_filter, args.any);
    if !name_filter.is_empty() && !args.any && target_names.is_empty() {
        emit_warning("No reserved names matched by --name filters.", &[]);
    }

    let patterns = scanner::compile_patterns(&split_csv(&args.pattern));
    let exclude_dirs = build_exclude_dirs(&args.exclude, args.no_default_excludes);

    let mut direct_files = Vec::new();
    let mut scan_dirs = Vec::new();
    for raw in paths {
        let abs = absolute_path(&raw);
        match classify_path(&abs) {
            Some(PathKind::Dir) => scan_dirs.push(abs),
            Some(PathKind::File) => direct_files.push(abs),
            None => emit_warning(
                format!("Path not found: {}", raw),
                &["Hint: Check the path exists or use an absolute path."],
            ),
        }
    }

    winapi::enable_delete_privileges();
    crate::windows::ctrlc::install_ctrlc_handler_once();
    crate::windows::ctrlc::reset_cancelled();

    let opts = DeleteOptions {
        level,
        dry_run: args.dry_run || args.what_if,
        collect_info: args.collect_info,
        on_reboot: args.on_reboot,
        force: args.force,
        reason: args.reason.clone(),
    };

    let mut all_results: Vec<DeleteRecord> = Vec::new();

    if !direct_files.is_empty() {
        let filtered = filter_direct_files(direct_files, &target_names, match_all, &patterns);
        let mut res = delete_paths(filtered, &opts, None);
        all_results.append(&mut res);
    }

    let use_tui = should_use_tui(&args);
    for dir in scan_dirs {
        if use_tui {
            #[cfg(feature = "delete_tui")]
            {
                let mut res = tui::run(
                    dir,
                    &target_names,
                    match_all,
                    &exclude_dirs,
                    &patterns,
                    &opts,
                )?;
                all_results.append(&mut res);
            }
            #[cfg(not(feature = "delete_tui"))]
            {
                ui_println!("TUI feature not enabled; running non-TUI pipeline.");
                let mut res = run_cli_pipeline(
                    &dir,
                    &target_names,
                    match_all,
                    &exclude_dirs,
                    &patterns,
                    &opts,
                )?;
                all_results.append(&mut res);
            }
        } else {
            let mut res = run_cli_pipeline(
                &dir,
                &target_names,
                match_all,
                &exclude_dirs,
                &patterns,
                &opts,
            )?;
            all_results.append(&mut res);
        }
    }

    if all_results.is_empty() {
        ui_println!("No matching files found.");
        return Ok(());
    }

    if let Some(log_path) = args.log.as_deref() {
        write_csv(&all_results, log_path)?;
        ui_println!("Log written: {}", log_path);
    }

    render_results(&all_results, &args.format)?;
    print_summary(&all_results);

    Ok(())
}
