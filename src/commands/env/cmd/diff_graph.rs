use super::*;

pub(super) fn cmd_diff_live(manager: &EnvManager, args: EnvDiffLiveCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    if args.snapshot.is_some() && args.since.is_some() {
        return Err(CliError::with_details(
            2,
            "diff-live does not allow using --snapshot and --since together".to_string(),
            &["Fix: use one baseline selector only."],
        ));
    }
    let diff = if let Some(since) = args.since.as_deref() {
        manager.diff_since(scope, since).map_err(map_env_err)?
    } else {
        manager
            .diff_live(scope, args.snapshot.as_deref())
            .map_err(map_env_err)?
    };
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&diff).unwrap_or_default()
        );
    } else {
        out_println!("{}", diff::format_diff(&diff, args.color));
    }
    Ok(())
}

pub(super) fn cmd_graph(manager: &EnvManager, args: EnvGraphCmd) -> CliResult {
    let scope = parse_scope(&args.scope)?;
    let tree = manager
        .dependency_tree(scope, &args.name, args.max_depth)
        .map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&tree).unwrap_or_default()
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

    out_println!("dependency graph: scope={} root={}", tree.scope, tree.root);
    for line in &tree.lines {
        out_println!("{}", line);
    }
    if !tree.missing.is_empty() {
        out_println!("missing: {}", tree.missing.join(", "));
    }
    if !tree.cycles.is_empty() {
        out_println!("cycles:");
        for item in tree.cycles {
            out_println!("  - {}", item);
        }
    }
    Ok(())
}
