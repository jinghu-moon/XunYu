use super::*;

pub(super) fn cmd_status(manager: &EnvManager, args: EnvStatusCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let summary = manager.status_overview(scope).map_err(map_env_err)?;

    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        return Ok(());
    }
    if !args.format.eq_ignore_ascii_case("text") {
        return Err(CliError::with_details(
            2,
            format!("invalid format '{}'", args.format),
            &["Fix: use --format text|json"],
        ));
    }

    let na = |v: Option<usize>| -> String {
        v.map(|n| n.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    };

    out_println!("env status: scope={}", summary.scope);
    out_println!("  vars(total):   {}", na(summary.total_vars));
    out_println!("  vars(user):    {}", na(summary.user_vars));
    out_println!("  vars(system):  {}", na(summary.system_vars));
    out_println!("  snapshots:     {}", summary.snapshots);
    out_println!(
        "  latest-snap:   {} ({})",
        summary.latest_snapshot_id.as_deref().unwrap_or("none"),
        summary.latest_snapshot_at.as_deref().unwrap_or("n/a")
    );
    out_println!("  profiles:      {}", summary.profiles);
    out_println!("  schema-rules:  {}", summary.schema_rules);
    out_println!("  annotations:   {}", summary.annotations);
    out_println!("  audit-entries: {}", summary.audit_entries);
    out_println!(
        "  last-audit:    {}",
        summary.last_audit_at.as_deref().unwrap_or("none")
    );
    if !summary.notes.is_empty() {
        out_println!("  notes:");
        for note in summary.notes {
            out_println!("    - {}", note);
        }
    }
    Ok(())
}
