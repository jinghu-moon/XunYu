use crate::cli::RedirectCmd;
use crate::output::{CliError, CliResult};

use super::super::super::debug_tools;
use super::super::super::render::render_simulate_results;
use super::super::format::resolve_format;

pub(in super::super) fn run_log(args: &RedirectCmd) -> CliResult {
    let format = resolve_format(&args.format)?;
    let items =
        super::super::super::redirect_log::query_tx_summaries(args.tx.as_deref(), args.last);
    super::super::super::redirect_log::render_tx_summaries(&items, format);
    Ok(())
}

pub(in super::super) fn run_explain(
    profile: &crate::config::RedirectProfile,
    name: &str,
) -> CliResult {
    let out = debug_tools::explain_one(profile, name);
    for line in out.lines {
        ui_println!(
            "Rule \"{}\": {} {} \u{2192} dest_dir={}",
            line.rule_name,
            if line.ok { "✓" } else { "✗" },
            line.details,
            line.rendered_dest_dir
        );
    }
    match (
        out.matched_rule.as_deref(),
        out.rendered_dest_file.as_deref(),
    ) {
        (Some(rule), Some(dest)) => {
            ui_println!("Result: would match \"{}\" \u{2192} {}", rule, dest);
        }
        _ => {
            ui_println!(
                "Result: no match ({})",
                out.note.unwrap_or_else(|| "unknown".to_string())
            );
        }
    }
    Ok(())
}

pub(in super::super) fn run_simulate(
    profile: &crate::config::RedirectProfile,
    args: &RedirectCmd,
) -> CliResult {
    let format = resolve_format(&args.format)?;
    let items = debug_tools::read_simulate_input_lines()
        .map_err(|e| CliError::new(2, format!("Failed to read stdin: {e}")))?;
    render_simulate_results(profile, &items, format);
    Ok(())
}
