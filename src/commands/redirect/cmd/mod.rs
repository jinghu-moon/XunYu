use std::path::PathBuf;

use crate::cli::RedirectCmd;
use crate::config as global_config;
use crate::config::RedirectOnConflict;
use crate::output::{CliError, CliResult};
use crate::suggest::did_you_mean;

use super::errors::err2;
use super::{config, engine};

mod apply;
mod format;
mod modes;
mod plan;

use apply::cmd_redirect_apply;
use format::resolve_format;
use modes::{
    ensure_overwrite_confirm, ensure_source_dir, resolve_source, run_ask_conflict, run_confirm,
    run_explain, run_log, run_review, run_simple, run_simulate, run_undo, run_watch,
};
use plan::run_plan;

pub(crate) fn cmd_redirect(args: RedirectCmd) -> CliResult {
    let cfg = global_config::load_config_strict().map_err(|msg| {
        err2(
            msg,
            &[
                "Hint: Config file is usually at `~/.xun.config.json` (or set XUN_CONFIG).",
                "Fix: Run `xun redirect --validate` after editing config.",
            ],
        )
    })?;

    if args.plan.is_some() && args.apply.is_some() {
        return Err(err2(
            "--plan cannot be used with --apply.",
            &["Fix: Use either `--plan <file>` or `--apply <file>`."],
        ));
    }

    if args.apply.is_some() {
        return cmd_redirect_apply(args, &cfg);
    }

    let profile = cfg.redirect.profiles.get(&args.profile).ok_or_else(|| {
        let keys: Vec<&str> = cfg.redirect.profiles.keys().map(|s| s.as_str()).collect();
        let mut details: Vec<String> = Vec::new();
        if let Some(s) = did_you_mean(&args.profile, &keys) {
            details.push(format!("Did you mean: \"{}\"?", s));
        }
        details.push("Hint: Check `~/.xun.config.json` contains `redirect.profiles`.".to_string());
        details.push("Fix: Use `--profile <name>` with an existing profile name.".to_string());
        CliError {
            code: 2,
            message: format!("Redirect profile not found: {}", args.profile),
            details,
        }
    })?;
    config::validate_profile(profile).map_err(|msg| {
        err2(
            msg,
            &[
                "Hint: Run with `--validate` to check config without executing.",
                "Fix: Edit `~/.xun.config.json` (or use Dashboard if enabled).",
            ],
        )
    })?;

    if args.validate {
        ui_println!("redirect validate: OK (profile={})", args.profile);
        return Ok(());
    }

    if args.log {
        return run_log(&args);
    }

    if let Some(name) = args.explain.as_deref() {
        return run_explain(profile, name);
    }

    if args.simulate {
        return run_simulate(profile, &args);
    }

    if let Some(tx) = args.undo.as_deref() {
        return run_undo(profile, &args, tx);
    }

    let source = resolve_source(args.source.as_deref());
    let mut policy = crate::path_guard::PathPolicy::for_read();
    policy.allow_relative = true;
    let validation = crate::path_guard::validate_paths(vec![source], &policy);
    if !validation.issues.is_empty() {
        let mut details: Vec<String> = validation
            .issues
            .iter()
            .map(|issue| format!("Invalid source path: {} ({})", issue.raw, issue.detail))
            .collect();
        details.push("Fix: Provide an existing directory path.".to_string());
        return Err(CliError::with_details(
            2,
            "Invalid source path.".to_string(),
            &details,
        ));
    }
    let source = validation
        .ok
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."));
    ensure_source_dir(&source, args.apply.is_some())?;

    let format = resolve_format(&args.format)?;

    if let Some(path) = args.plan.as_deref() {
        if args.watch {
            return Err(err2(
                "--plan cannot be used with --watch.",
                &["Fix: Generate a plan first, then apply it without --watch."],
            ));
        }
        if args.review {
            return Err(err2(
                "--plan cannot be used with --review.",
                &[
                    "Fix: Use `--review` for interactive per-file confirmation, or `--plan` for plan/apply.",
                ],
            ));
        }
        if args.confirm {
            return Err(err2(
                "--plan cannot be used with --confirm.",
                &["Fix: Use `--confirm` for one-shot preview/execute, or `--plan` for plan/apply."],
            ));
        }
        return run_plan(path, &source, &args.profile, profile, args.copy);
    }

    let tx = engine::new_tx_id();
    let opts = engine::RedirectOptions {
        dry_run: args.dry_run,
        copy: args.copy,
        yes: args.yes,
        format,
        audit: true,
    };

    if args.watch {
        return run_watch(&args, &tx, &source, profile, &opts);
    }

    if args.review {
        return run_review(&args, &source, profile, format);
    }

    if args.confirm && !args.dry_run {
        return run_confirm(&args, &source, profile, format);
    }

    if matches!(profile.on_conflict, RedirectOnConflict::Ask) {
        return run_ask_conflict(&args, &source, profile, format, &tx);
    }

    ensure_overwrite_confirm(&args, profile)?;
    run_simple(&args, &source, profile, format, &tx, &opts)
}
