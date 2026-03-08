use super::super::common::resolve_ctx_list_format;
use super::*;

pub(super) fn cmd_list(args: CtxListCmd) -> CliResult {
    let path = ctx_store_path();
    let store = load_store(&path);
    let active = active_profile_name();

    let mut items: Vec<(&String, &crate::ctx_store::CtxProfile)> = store.profiles.iter().collect();
    items.sort_by_key(|(k, _)| k.to_lowercase());

    let format = resolve_ctx_list_format(&args.format)?;

    if format == ListFormat::Json {
        let arr: Vec<serde_json::Value> = items
            .iter()
            .map(|(name, p)| {
                serde_json::json!({
                    "name": name,
                    "path": p.path,
                    "tags": p.tags,
                    "proxy": p.proxy,
                    "env": p.env,
                    "active": active.as_deref() == Some(name.as_str()),
                })
            })
            .collect();
        out_println!(
            "{}",
            serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
        );
        return Ok(());
    }

    if format == ListFormat::Tsv {
        for (name, p) in items {
            let tags = p.tags.join(",");
            let proxy = proxy_summary(&p.proxy);
            let active_mark = if active.as_deref() == Some(name.as_str()) {
                "yes"
            } else {
                ""
            };
            out_println!("{name}\t{}\t{tags}\t{proxy}\t{active_mark}", p.path);
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Tags")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Proxy")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Active")
            .add_attribute(Attribute::Bold)
            .fg(Color::Blue),
    ]);

    for (name, p) in items {
        let tags = if p.tags.is_empty() {
            Cell::new("-")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(p.tags.join(",")).fg(Color::Yellow)
        };
        let active_mark = if active.as_deref() == Some(name.as_str()) {
            Cell::new("yes").fg(Color::Green)
        } else {
            Cell::new("")
        };
        table.add_row(vec![
            Cell::new(name.as_str())
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new(p.path.as_str()).fg(Color::DarkGrey),
            tags,
            Cell::new(proxy_summary(&p.proxy)).fg(Color::Green),
            active_mark,
        ]);
    }
    print_table(&table);
    Ok(())
}
