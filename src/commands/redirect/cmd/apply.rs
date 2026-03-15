use std::path::PathBuf;

use crate::cli::RedirectCmd;
use crate::config as global_config;
use crate::config::RedirectOnConflict;
use crate::output::CliResult;

use super::super::errors::err2;
use super::super::plan::{PLAN_VERSION, PlanFile, PlanKind};
use super::super::prompt::{confirm_overwrite_for_profile, prompt_conflicts};
use super::super::render::{render_dry_run_summary, render_results, render_stats};
use super::super::{config, engine};
use super::format::resolve_format;

pub(super) fn cmd_redirect_apply(
    args: RedirectCmd,
    cfg: &global_config::GlobalConfig,
) -> CliResult {
    let plan_path = args.apply.as_deref().ok_or_else(|| {
        err2(
            "--apply requires a plan path.",
            &["Fix: Use `--apply <file>`."],
        )
    })?;

    if args.plan.is_some()
        || args.watch
        || args.status
        || args.undo.is_some()
        || args.log
        || args.validate
        || args.explain.is_some()
        || args.simulate
        || args.confirm
        || args.review
    {
        return Err(err2(
            "--apply cannot be combined with other redirect modes.",
            &[
                "Fix: Run `xun redirect --apply <file>` by itself (plus optional --dry-run/--yes/--format/--stats).",
            ],
        ));
    }

    let format = resolve_format(&args.format)?;

    let mut policy = crate::path_guard::PathPolicy::for_read();
    policy.allow_relative = true;
    let validation = crate::path_guard::validate_paths(vec![plan_path.to_string()], &policy);
    if !validation.issues.is_empty() {
        let mut details: Vec<String> = validation
            .issues
            .iter()
            .map(|issue| format!("Invalid plan path: {} ({})", issue.raw, issue.detail))
            .collect();
        details.push("Fix: Provide an existing plan file path.".to_string());
        return Err(crate::output::CliError::with_details(
            2,
            "Invalid plan path.".to_string(),
            &details,
        ));
    }
    let plan_path = validation
        .ok
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(plan_path));
    let raw = std::fs::read_to_string(&plan_path).map_err(|e| {
        err2(
            format!("Failed to read plan file: {e}"),
            &["Fix: Check the plan path and permissions."],
        )
    })?;
    let mut plan_file: PlanFile = serde_json::from_str(&raw).map_err(|e| {
        err2(
            format!("Invalid plan file JSON: {e}"),
            &["Fix: Re-generate with `xun redirect <dir> --plan <file>`."],
        )
    })?;
    if plan_file.version != PLAN_VERSION {
        return Err(err2(
            format!("Unsupported plan version: {}", plan_file.version),
            &["Fix: Re-generate the plan with the current xun version."],
        ));
    }

    let apply_profile = cfg.redirect.profiles.get(&plan_file.profile).ok_or_else(|| {
        err2(
            format!("Redirect profile not found: {}", plan_file.profile),
            &["Fix: Ensure the plan profile exists in `~/.xun.config.json`, or re-generate the plan."],
        )
    })?;
    config::validate_profile(apply_profile).map_err(|msg| {
        err2(
            msg,
            &["Fix: Fix the profile in `~/.xun.config.json` and re-apply."],
        )
    })?;

    if !args.dry_run && !confirm_overwrite_for_profile(apply_profile, args.yes, &plan_file.items)? {
        ui_println!("Cancelled.");
        return Ok(());
    }

    if matches!(apply_profile.on_conflict, RedirectOnConflict::Ask) && !args.dry_run {
        prompt_conflicts(&mut plan_file.items)?;
    }

    let tx = engine::new_tx_id();
    let opts = engine::RedirectOptions {
        dry_run: args.dry_run,
        copy: false,
        yes: args.yes,
        format,
        audit: !args.dry_run,
    };
    let results = engine::apply_plan(&tx, apply_profile, &opts, &plan_file.items);
    render_results(&tx, &results, format);
    if args.stats {
        render_stats(apply_profile, &results);
    }
    if args.dry_run {
        let all_copy = plan_file
            .items
            .iter()
            .all(|it| matches!(it.kind, PlanKind::Copy));
        render_dry_run_summary(&results, all_copy);
    }
    Ok(())
}
