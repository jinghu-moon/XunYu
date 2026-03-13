use super::*;

pub(super) fn cmd_config(args: AclConfigCmd) -> CliResult {
    let mut cfg = load_config();

    let mut set_parts = args.set.clone();
    if !args.set_value.is_empty() {
        if args.set.is_empty() {
            return Err(CliError::with_details(
                2,
                "--set requires KEY VALUE".to_string(),
                &["Fix: Use `xun acl config --set throttle_limit 8`."],
            ));
        }
        if args.set.len() != 1 {
            return Err(CliError::with_details(
                2,
                "--set requires KEY VALUE".to_string(),
                &[
                    "Fix: Use `xun acl config --set throttle_limit 8`.",
                    "Alt: Use `xun acl config --set throttle_limit --set 8`.",
                ],
            ));
        }
        set_parts.extend(args.set_value);
    }

    if !set_parts.is_empty() {
        if set_parts.len() != 2 {
            return Err(CliError::with_details(
                2,
                "--set requires KEY VALUE".to_string(),
                &[
                    "Fix: Use `xun acl config --set throttle_limit 8`.",
                    "Alt: Use `xun acl config --set throttle_limit --set 8`.",
                ],
            ));
        }
        let key = set_parts[0].as_str();
        let value = set_parts[1].as_str();
        match key {
            "throttle_limit" => {
                cfg.acl.throttle_limit = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid throttle_limit"))?;
            }
            "chunk_size" => {
                cfg.acl.chunk_size = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid chunk_size"))?;
            }
            "audit_log_path" => {
                cfg.acl.audit_log_path = value.to_string();
            }
            "export_path" => {
                cfg.acl.export_path = value.to_string();
            }
            "default_owner" => {
                cfg.acl.default_owner = value.to_string();
            }
            "max_audit_lines" => {
                cfg.acl.max_audit_lines = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid max_audit_lines"))?;
            }
            _ => {
                return Err(CliError::with_details(
                    2,
                    format!("Unknown key: {key}"),
                    &[
                        "Valid keys: throttle_limit, chunk_size, audit_log_path, export_path, default_owner, max_audit_lines",
                    ],
                ));
            }
        }

        save_config(&cfg).map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        ui_println!("ACL config updated.");
        return Ok(());
    }

    let acl_cfg = normalize_acl_config(cfg.acl);

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    table.add_row(vec![
        Cell::new("throttle_limit"),
        Cell::new(acl_cfg.throttle_limit),
    ]);
    table.add_row(vec![Cell::new("chunk_size"), Cell::new(acl_cfg.chunk_size)]);
    table.add_row(vec![
        Cell::new("audit_log_path"),
        Cell::new(acl_cfg.audit_log_path),
    ]);
    table.add_row(vec![
        Cell::new("export_path"),
        Cell::new(acl_cfg.export_path),
    ]);
    table.add_row(vec![
        Cell::new("default_owner"),
        Cell::new(acl_cfg.default_owner),
    ]);
    table.add_row(vec![
        Cell::new("max_audit_lines"),
        Cell::new(acl_cfg.max_audit_lines),
    ]);
    print_table(&table);
    Ok(())
}
