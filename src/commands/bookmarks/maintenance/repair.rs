use std::collections::HashSet;

use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};

use crate::cli::DedupCmd;
use crate::model::{DedupMode, Entry, ListFormat, parse_dedup_mode};
use crate::output::{CliError, CliResult, can_interact};
use crate::output::{apply_pretty_table_style, format_age, print_table};
use crate::store::{Lock, db_path, load, save_db};
use crate::util::normalize_path;

use super::report::resolve_output_format;

pub(crate) fn cmd_dedup(args: DedupCmd) -> CliResult {
    let mode = parse_dedup_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: path | name"],
        )
    })?;
    let format = resolve_output_format(&args.format)?;

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, entry) in &db {
        let key = match mode {
            DedupMode::Path => normalize_path(&entry.path),
            DedupMode::Name => name.to_lowercase(),
        };
        groups.entry(key).or_default().push(name.clone());
    }

    let groups: Vec<(String, Vec<String>)> =
        groups.into_iter().filter(|(_, v)| v.len() > 1).collect();

    if groups.is_empty() {
        ui_println!("No duplicates found.");
        return Ok(());
    }

    if format == ListFormat::Tsv || !can_interact() || format == ListFormat::Json {
        if format == ListFormat::Json {
            let mut items = Vec::new();
            for (key, names) in &groups {
                for name in names {
                    if let Some(e) = db.get(name) {
                        items.push(serde_json::json!({
                            "key": key,
                            "name": name,
                            "path": e.path,
                            "visits": e.visit_count,
                            "last_visited": e.last_visited,
                            "tags": e.tags,
                        }));
                    }
                }
            }
            out_println!("{}", serde_json::Value::Array(items));
            return Ok(());
        }
        for (key, names) in groups {
            for name in names {
                if let Some(e) = db.get(&name) {
                    out_println!(
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        key,
                        name,
                        e.path,
                        e.visit_count,
                        e.last_visited,
                        e.tags.join(",")
                    );
                }
            }
        }
        return Ok(());
    }

    for (key, names) in groups {
        let items: Vec<(String, Entry)> = names
            .into_iter()
            .filter_map(|n| db.get(&n).cloned().map(|e| (n, e)))
            .collect();
        if items.len() < 2 {
            continue;
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
            Cell::new("Visits")
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new("Last")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);
        for (name, e) in &items {
            let tags = if e.tags.is_empty() {
                Cell::new("-")
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim)
            } else {
                Cell::new(e.tags.join(",")).fg(Color::Yellow)
            };
            table.add_row(vec![
                Cell::new(name)
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Cyan),
                Cell::new(e.path.clone())
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim),
                tags,
                Cell::new(e.visit_count).fg(Color::Green),
                Cell::new(format_age(e.last_visited)).fg(Color::Yellow),
            ]);
        }

        let label = if mode == DedupMode::Path {
            "Duplicate path"
        } else {
            "Duplicate name"
        };
        ui_println!("{}: {}", label, key);
        print_table(&table);

        let mut choices = vec!["skip".to_string()];
        for (name, _) in &items {
            choices.push(format!("keep: {}", name));
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an action")
            .default(0)
            .items(&choices)
            .interact_on(&Term::stderr());

        let Ok(sel) = selection else {
            return Err(CliError::new(3, "Cancelled."));
        };
        if sel == 0 {
            continue;
        }

        let keep_idx = sel - 1;
        let keep_name = items[keep_idx].0.clone();

        if !args.yes {
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Merge duplicates into '{}' and delete others?",
                    keep_name
                ))
                .default(false)
                .interact_on(&Term::stderr());

            if !matches!(confirm, Ok(true)) {
                ui_println!("Skipped.");
                continue;
            }
        }

        let mut merged = items[keep_idx].1.clone();
        let mut tag_set: HashSet<String> = merged.tags.iter().map(|t| t.to_lowercase()).collect();

        for (i, (_name, e)) in items.iter().enumerate() {
            if i == keep_idx {
                continue;
            }
            for t in &e.tags {
                if tag_set.insert(t.to_lowercase()) {
                    merged.tags.push(t.clone());
                }
            }
            merged.visit_count = merged.visit_count.saturating_add(e.visit_count);
            merged.last_visited = merged.last_visited.max(e.last_visited);
        }

        for (name, _) in &items {
            if name != &keep_name {
                db.remove(name);
            }
        }
        db.insert(keep_name.clone(), merged);
        save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
        ui_println!("Merged into '{}'.", keep_name);
    }

    Ok(())
}
