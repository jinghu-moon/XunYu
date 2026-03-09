use std::path::{Path, PathBuf};

use crate::cli::RedirectCmd;
use crate::config::RedirectOnConflict;
use crate::model::ListFormat;
use crate::output::{CliResult, can_interact};

use super::super::super::engine;
use super::super::super::errors::err2;
use super::super::super::plan::{PlanItem, PlanKind};
use super::super::super::prompt::{confirm_overwrite_for_profile, prompt_conflicts};
use super::super::super::render::{
    render_dry_run_summary, render_preview_summary, render_results, render_stats,
};

pub(in super::super) fn run_review(
    args: &RedirectCmd,
    source: &Path,
    profile: &crate::config::RedirectProfile,
    format: ListFormat,
) -> CliResult {
    if !can_interact() {
        return Err(err2(
            "--review requires interactive mode.",
            &[
                "Fix: Remove --review, or run in a terminal (not piped) and ensure `--non-interactive` is not set.",
            ],
        ));
    }

    let planned = engine::plan_redirect(source, profile, args.copy);
    if planned.items.is_empty() {
        ui_println!("Nothing to do.");
        return Ok(());
    }

    let mut selected_items: Vec<PlanItem> = Vec::new();
    let mut apply_all = false;

    for (i, it) in planned.items.iter().enumerate() {
        if apply_all {
            selected_items.push(it.clone());
            continue;
        }

        let name = Path::new(&it.src)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&it.src);
        ui_println!(
            "[{}/{}] {} {} \u{2192} {} (rule: {})",
            i + 1,
            planned.items.len(),
            match it.kind {
                PlanKind::Move => "move",
                PlanKind::Copy => "copy",
            },
            name,
            Path::new(&it.dst).to_string_lossy(),
            it.rule
        );

        let prompt = "[y] apply  [n] skip  [a] apply all remaining  [q] quit";
        let c = loop {
            let choice: String = dialoguer::Input::new()
                .with_prompt(prompt)
                .default("y".to_string())
                .interact_text()
                .unwrap_or_else(|_| "q".to_string());
            let c = choice.trim().to_ascii_lowercase();
            if ["y", "n", "a", "q"].contains(&c.as_str()) {
                break c;
            }
            ui_println!("Hint: Use y/n/a/q.");
        };
        match c.as_str() {
            "y" => {
                selected_items.push(it.clone());
            }
            "n" => {}
            "a" => {
                apply_all = true;
                selected_items.push(it.clone());
            }
            "q" => {
                ui_println!("Cancelled.");
                return Ok(());
            }
            _ => unreachable!(),
        }
    }

    if selected_items.is_empty() {
        ui_println!("No actions selected.");
        return Ok(());
    }

    if args.dry_run {
        let preview_opts = engine::RedirectOptions {
            dry_run: true,
            copy: args.copy,
            yes: true,
            format,
            audit: false,
        };
        let preview_tx = "redirect_preview";
        let preview = engine::apply_plan(preview_tx, profile, &preview_opts, &selected_items);
        render_results(preview_tx, &preview, format);
        render_dry_run_summary(&preview, args.copy);
        return Ok(());
    }

    if matches!(profile.on_conflict, RedirectOnConflict::Ask) {
        prompt_conflicts(&mut selected_items)?;
    }
    if !confirm_overwrite_for_profile(profile, args.yes, &selected_items)? {
        ui_println!("Cancelled.");
        return Ok(());
    }

    let exec_tx = engine::new_tx_id();
    let exec_opts = engine::RedirectOptions {
        dry_run: false,
        copy: args.copy,
        yes: args.yes,
        format,
        audit: true,
    };
    let results = engine::apply_plan(&exec_tx, profile, &exec_opts, &selected_items);
    render_results(&exec_tx, &results, format);
    if args.stats {
        render_stats(profile, &results);
    }
    Ok(())
}

pub(in super::super) fn run_confirm(
    args: &RedirectCmd,
    source: &Path,
    profile: &crate::config::RedirectProfile,
    format: ListFormat,
) -> CliResult {
    if !args.yes && !can_interact() {
        return Err(err2(
            "--confirm requires interactive mode unless --yes is set.",
            &["Fix: Add --yes, or run in an interactive terminal."],
        ));
    }
    let planned = engine::plan_redirect(source, profile, args.copy);
    render_preview_summary(&planned.results, args.copy);
    if !args.yes {
        let ok = dialoguer::Confirm::new()
            .with_prompt("Proceed to execute now?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !ok {
            ui_println!("Cancelled.");
            return Ok(());
        }
    }

    let mut items = planned.items;
    if matches!(profile.on_conflict, RedirectOnConflict::Ask) {
        if !can_interact() {
            return Err(err2(
                "on_conflict=ask requires interactive mode.",
                &["Fix: Run in an interactive terminal, or change on_conflict in config."],
            ));
        }
        prompt_conflicts(&mut items)?;
    }
    if !confirm_overwrite_for_profile(profile, args.yes, &items)? {
        ui_println!("Cancelled.");
        return Ok(());
    }

    let exec_tx = engine::new_tx_id();
    let exec_opts = engine::RedirectOptions {
        dry_run: false,
        copy: args.copy,
        yes: args.yes,
        format,
        audit: true,
    };
    let results = engine::apply_plan(&exec_tx, profile, &exec_opts, &items);
    render_results(&exec_tx, &results, format);
    if args.stats {
        render_stats(profile, &results);
    }
    Ok(())
}

pub(in super::super) fn run_ask_conflict(
    args: &RedirectCmd,
    source: &Path,
    profile: &crate::config::RedirectProfile,
    format: ListFormat,
    tx: &str,
) -> CliResult {
    let planned = engine::plan_redirect(source, profile, args.copy);
    if args.dry_run {
        render_results(tx, &planned.results, format);
        if args.stats {
            render_stats(profile, &planned.results);
        }
        render_dry_run_summary(&planned.results, args.copy);
        return Ok(());
    }

    if !can_interact() {
        return Err(err2(
            "on_conflict=ask requires interactive mode.",
            &["Fix: Run in an interactive terminal, or change on_conflict in config."],
        ));
    }
    let mut items = planned.items;
    prompt_conflicts(&mut items)?;

    let exec_opts = engine::RedirectOptions {
        dry_run: false,
        copy: args.copy,
        yes: args.yes,
        format,
        audit: true,
    };
    let results = engine::apply_plan(tx, profile, &exec_opts, &items);
    render_results(tx, &results, format);
    if args.stats {
        render_stats(profile, &results);
    }
    Ok(())
}

pub(in super::super) fn ensure_overwrite_confirm(
    args: &RedirectCmd,
    profile: &crate::config::RedirectProfile,
) -> CliResult {
    if matches!(profile.on_conflict, RedirectOnConflict::Overwrite) {
        if !can_interact() && !args.yes {
            return Err(err2(
                "on_conflict=overwrite requires explicit confirmation.",
                &["Fix: Add --yes, or change on_conflict to rename_new/skip/trash in config."],
            ));
        }
        if can_interact() && !args.yes {
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
    }
    Ok(())
}

pub(in super::super) fn ensure_source_dir(source: &Path, apply_mode: bool) -> CliResult {
    if apply_mode {
        return Ok(());
    }
    if !source.exists() {
        return Err(err2(
            format!("Source directory not found: {}", source.display()),
            &["Hint: Check the path exists, or omit [source] to use the current directory."],
        ));
    }
    if !source.is_dir() {
        return Err(err2(
            format!("Source must be a directory: {}", source.display()),
            &["Fix: Provide a directory path (not a file)."],
        ));
    }
    Ok(())
}

pub(in super::super) fn resolve_source(source: Option<&str>) -> PathBuf {
    source
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}
