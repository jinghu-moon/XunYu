use super::*;

pub(super) fn cmd_audit(args: AclAuditCmd) -> CliResult {
    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);

    if let Some(dest) = args.export {
        let dest = PathBuf::from(dest);
        let n = audit.export_csv(&dest).map_err(map_acl_err)?;
        ui_println!("Exported {} rows to {}", n, dest.display());
        return Ok(());
    }

    let entries = audit.tail(args.tail).map_err(map_acl_err)?;
    if entries.is_empty() {
        ui_println!("No audit entries.");
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Time")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Action").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Path").add_attribute(Attribute::Bold),
        Cell::new("Details").add_attribute(Attribute::Bold),
    ]);

    for e in &entries {
        let status = if e.success { "ok" } else { "fail" };
        table.add_row(vec![
            Cell::new(&e.ts),
            Cell::new(&e.action),
            Cell::new(status),
            Cell::new(acl::parse::truncate_left(&e.path, 40)),
            Cell::new(acl::parse::truncate(&e.details, 40)),
        ]);
    }

    print_table(&table);
    Ok(())
}
