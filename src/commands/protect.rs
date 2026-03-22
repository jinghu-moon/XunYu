use crate::cli::{ProtectClearCmd, ProtectCmd, ProtectSetCmd, ProtectStatusCmd, ProtectSubCommand};
use crate::config::{ProtectRule, load_config, save_config};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, emit_warning, print_table};
use comfy_table::{Attribute, Cell, Color, Table};

fn validate_protect_path(raw: &str) -> CliResult<()> {
    let mut policy = crate::path_guard::PathPolicy::for_read();
    policy.must_exist = false;
    let validation = crate::path_guard::validate_paths(vec![raw.to_string()], &policy);
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
    Ok(())
}

pub(crate) fn cmd_protect(args: ProtectCmd) -> CliResult {
    match args.cmd {
        ProtectSubCommand::Set(a) => cmd_set(a),
        ProtectSubCommand::Clear(a) => cmd_clear(a),
        ProtectSubCommand::Status(a) => cmd_status(a),
    }
}

fn cmd_set(args: ProtectSetCmd) -> CliResult {
    validate_protect_path(&args.path)?;
    let mut cfg = load_config();

    let denys: Vec<String> = args
        .deny
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let reqs: Vec<String> = args
        .require
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let path_str = args.path.replace('\\', "/").to_lowercase();

    if let Some(r) = cfg
        .protect
        .rules
        .iter_mut()
        .find(|r| r.path.to_lowercase() == path_str)
    {
        r.deny = denys;
        r.require = reqs;
    } else {
        cfg.protect.rules.push(ProtectRule {
            path: args.path.clone(),
            deny: denys,
            require: reqs,
        });
    }

    if let Err(e) = save_config(&cfg) {
        return Err(CliError::new(1, format!("Failed to save config: {e}")));
    }
    ui_println!("Protection rule set for: {}", args.path);
    if args.system_acl {
        let p = std::path::Path::new(&args.path);
        if let Err(e) = crate::windows::acl::deny_delete_access(p) {
            let detail = format!("Details: {e:?}");
            emit_warning(
                "Failed to apply system ACL protection.",
                &[
                    detail.as_str(),
                    "Hint: Run as Administrator to apply system ACL rules.",
                ],
            );
        } else {
            ui_println!("Deep Windows (NTFS) ACL protection successfully applied.");
        }
    }

    crate::security::audit::audit_log(
        "protect_set",
        &args.path,
        "cli",
        format!("deny={:?} req={:?}", args.deny, args.require),
        "success",
        "",
    );
    Ok(())
}

fn cmd_clear(args: ProtectClearCmd) -> CliResult {
    validate_protect_path(&args.path)?;
    let mut cfg = load_config();
    let path_str = args.path.replace('\\', "/").to_lowercase();

    let initial_len = cfg.protect.rules.len();
    cfg.protect
        .rules
        .retain(|r| r.path.to_lowercase() != path_str);

    if cfg.protect.rules.len() == initial_len {
        ui_println!("No protection rule found for: {}", args.path);
        return Ok(());
    }

    if let Err(e) = save_config(&cfg) {
        return Err(CliError::new(1, format!("Failed to save config: {e}")));
    }
    ui_println!("Protection rule cleared for: {}", args.path);
    if args.system_acl {
        let p = std::path::Path::new(&args.path);
        if let Err(e) = crate::windows::acl::clear_deny_delete(p) {
            let detail = format!("Details: {e:?}");
            emit_warning(
                "Failed to clear system ACL protection.",
                &[
                    detail.as_str(),
                    "Hint: Run as Administrator to clear system ACL rules.",
                ],
            );
        } else {
            ui_println!("Deep Windows (NTFS) ACL protection successfully cleared.");
        }
    }
    crate::security::audit::audit_log("protect_clear", &args.path, "cli", "", "success", "");
    Ok(())
}

fn cmd_status(args: ProtectStatusCmd) -> CliResult {
    let cfg = load_config();
    let rules = &cfg.protect.rules;

    let filter_path = args
        .path
        .as_ref()
        .map(|p| p.replace('\\', "/").to_lowercase());

    let filtered: Vec<&ProtectRule> = rules
        .iter()
        .filter(|r| {
            if let Some(ref fp) = filter_path {
                r.path.replace('\\', "/").to_lowercase().starts_with(fp)
            } else {
                true
            }
        })
        .collect();

    if args.format == "json" {
        let json_arr: Vec<_> = filtered
            .iter()
            .map(|r| {
                serde_json::json!({
                    "path": r.path,
                    "deny": r.deny,
                    "require": r.require,
                })
            })
            .collect();
        crate::output::ui_println(format_args!(
            "{}",
            serde_json::to_string(&json_arr).unwrap_or_default()
        ));
        return Ok(());
    }

    if args.format == "tsv" {
        for r in &filtered {
            crate::output::ui_println(format_args!(
                "{}\t{}\t{}",
                r.path,
                r.deny.join(","),
                r.require.join(",")
            ));
        }
        return Ok(());
    }

    if filtered.is_empty() {
        if args.path.is_some() {
            ui_println!("No protection rules found matching the prefix.");
        } else {
            ui_println!("No protection rules found.");
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Deny")
            .add_attribute(Attribute::Bold)
            .fg(Color::Red),
        Cell::new("Require")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ]);

    for r in &filtered {
        table.add_row(vec![
            Cell::new(&r.path).fg(Color::Cyan),
            Cell::new(r.deny.join(", ")).fg(Color::Red),
            Cell::new(r.require.join(", ")).fg(Color::Yellow),
        ]);
    }
    print_table(&table);
    Ok(())
}
