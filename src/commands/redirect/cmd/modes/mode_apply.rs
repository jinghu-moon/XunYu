use std::path::Path;

use crate::cli::RedirectCmd;
use crate::config::RedirectOnConflict;
use crate::model::ListFormat;
use crate::output::{CliResult, can_interact};

use super::super::super::errors::err2;
use super::super::super::render::{
    render_dry_run_summary, render_results, render_stats, render_undo_results,
};
use super::super::super::{engine, undo, watcher};
use super::super::format::resolve_format;

pub(in super::super) fn run_undo(
    profile: &crate::config::RedirectProfile,
    args: &RedirectCmd,
    tx: &str,
) -> CliResult {
    if args.watch {
        return Err(err2(
            "--undo cannot be used with --watch.",
            &["Fix: Run undo as a one-shot command without --watch."],
        ));
    }
    if args.copy {
        return Err(err2(
            "--undo cannot be used with --copy.",
            &["Fix: Undo applies to previous runs; remove --copy."],
        ));
    }

    let format = resolve_format(&args.format)?;

    let selected_tx;
    let tx = if tx.eq_ignore_ascii_case("select") || tx == "-" {
        if !can_interact() {
            return Err(err2(
                "--undo select requires interactive mode.",
                &[
                    "Fix: Run `xun redirect --log --last 10` to find a tx id, then pass `--undo <tx>`.",
                ],
            ));
        }
        let items = super::super::super::redirect_log::query_tx_summaries(None, Some(20));
        if items.is_empty() {
            return Err(err2(
                "No redirect transactions found in audit log.",
                &["Hint: Run `xun redirect <dir>` first to generate tx entries."],
            ));
        }
        let labels: Vec<String> = items
            .iter()
            .map(|it| {
                format!(
                    "{} (total={}, ok={}, fail={})",
                    it.tx, it.total, it.ok, it.fail
                )
            })
            .collect();
        let idx = dialoguer::Select::new()
            .with_prompt("Select transaction to undo")
            .items(&labels)
            .default(0)
            .interact()
            .unwrap_or(usize::MAX);
        if idx >= items.len() {
            ui_println!("Cancelled.");
            return Ok(());
        }
        selected_tx = items[idx].tx.clone();
        selected_tx.as_str()
    } else {
        tx
    };

    let results = undo::run_undo(tx, &profile.on_conflict, args.dry_run, args.yes);
    render_undo_results(tx, &results, format);
    Ok(())
}

pub(in super::super) fn run_watch(
    args: &RedirectCmd,
    tx: &str,
    source: &Path,
    profile: &crate::config::RedirectProfile,
    opts: &engine::RedirectOptions,
) -> CliResult {
    if matches!(profile.on_conflict, RedirectOnConflict::Ask) {
        return Err(err2(
            "on_conflict=ask cannot be used with --watch.",
            &[
                "Hint: Watch mode is non-interactive by nature.",
                "Fix: Change on_conflict to rename_new/skip/trash, or run one-shot without --watch.",
            ],
        ));
    }
    if args.status {
        let format = resolve_format(&args.format)?;
        let status = watcher::read_watch_status(source)
            .map_err(|msg| err2(msg, &["Hint: Start watch first to generate status file."]))?;
        watcher::render_watch_status(&status, format);
        return Ok(());
    }
    if matches!(profile.on_conflict, RedirectOnConflict::Overwrite) && !args.yes {
        if !can_interact() {
            return Err(err2(
                "on_conflict=overwrite requires explicit confirmation.",
                &["Fix: Add --yes, or change on_conflict to rename_new/skip/trash in config."],
            ));
        }
        let ok = dialoguer::Confirm::new()
            .with_prompt("Overwrite existing destination files?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !ok {
            ui_println!("Cancelled.");
            return Ok(());
        }
    }
    watcher::watch_loop(tx, source, &args.profile, profile.clone(), opts)?;
    Ok(())
}

pub(in super::super) fn run_simple(
    args: &RedirectCmd,
    source: &Path,
    profile: &crate::config::RedirectProfile,
    format: ListFormat,
    tx: &str,
    opts: &engine::RedirectOptions,
) -> CliResult {
    let results = engine::run_redirect(tx, source, profile, opts);
    render_results(tx, &results, format);
    if args.stats {
        render_stats(profile, &results);
    }
    if args.dry_run {
        render_dry_run_summary(&results, opts.copy);
    }
    Ok(())
}
