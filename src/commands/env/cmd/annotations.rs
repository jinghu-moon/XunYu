use super::*;

pub(super) fn cmd_annotate(manager: &EnvManager, args: EnvAnnotateCmd) -> CliResult {
    match args.cmd {
        EnvAnnotateSubCommand::Set(a) => cmd_annotate_set(manager, a),
        EnvAnnotateSubCommand::List(a) => cmd_annotate_list(manager, a),
    }
}

pub(super) fn cmd_annotate_set(manager: &EnvManager, args: EnvAnnotateSetCmd) -> CliResult {
    let item = manager
        .annotate_set(&args.name, &args.note)
        .map_err(map_env_err)?;
    out_println!("ok\tannotate.set\t{}\t{}", item.name, item.note);
    Ok(())
}

pub(super) fn cmd_annotate_list(manager: &EnvManager, args: EnvAnnotateListCmd) -> CliResult {
    let items = manager.annotate_list().map_err(map_env_err)?;
    if args.format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
        return Ok(());
    }
    if items.is_empty() {
        out_println!("(empty)");
        return Ok(());
    }
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Note")
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
    ]);
    for item in items {
        table.add_row(vec![Cell::new(item.name), Cell::new(item.note)]);
    }
    print_table(&table);
    Ok(())
}
