use std::collections::HashSet;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};

use crate::cli::{CheckCmd, DedupCmd, GcCmd};
use crate::model::{DedupMode, Entry, ListFormat, parse_dedup_mode, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{
    apply_pretty_table_style, can_interact, format_age, prefer_table_output, print_table,
};
use crate::store::{Lock, db_path, load, now_secs, save_db};
use crate::util::normalize_path;

pub(crate) fn cmd_check(args: CheckCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    if db.is_empty() {
        ui_println!("No bookmarks found.");
        return Ok(());
    }

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    let now = now_secs();
    let stale_secs = args.days.saturating_mul(24 * 60 * 60);
    let stale_before = now.saturating_sub(stale_secs);

    let mut missing: Vec<(String, String)> = Vec::new();
    let mut stale: Vec<(String, String, u64)> = Vec::new(); // (name, path, last_visited)
    let mut by_path: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (name, e) in &db {
        let key = normalize_path(&e.path);
        by_path.entry(key).or_default().push(name.clone());

        if !Path::new(&e.path).exists() {
            missing.push((name.clone(), e.path.clone()));
        }
        if e.last_visited > 0 && e.last_visited < stale_before {
            stale.push((name.clone(), e.path.clone(), e.last_visited));
        }
    }

    let mut duplicates: Vec<(String, Vec<String>)> =
        by_path.into_iter().filter(|(_, v)| v.len() > 1).collect();
    duplicates.sort_by(|a, b| a.0.cmp(&b.0));

    if format == ListFormat::Json {
        let mut items: Vec<serde_json::Value> = Vec::new();
        for (name, path) in &missing {
            items.push(serde_json::json!({
                "kind": "missing",
                "name": name,
                "path": path,
                "fix": "Run `xun delete -bm <name>` or update the bookmark with `xun set <name> <path>`."
            }));
        }
        for (name, path, last_visit) in &stale {
            items.push(serde_json::json!({
                "kind": "stale",
                "name": name,
                "path": path,
                "age_secs": now.saturating_sub(*last_visit),
                "fix": "Run `xun touch <name>` (or use `xun z <pattern>`)."
            }));
        }
        for (path_key, names) in &duplicates {
            items.push(serde_json::json!({
                "kind": "duplicate",
                "path_key": path_key,
                "names": names,
                "fix": "Run `xun dedup -m path` to remove duplicates."
            }));
        }
        out_println!("{}", serde_json::Value::Array(items));
        return Ok(());
    }

    if format == ListFormat::Tsv {
        for (name, path) in &missing {
            out_println!("missing\t{name}\t{path}");
        }
        for (name, path, last_visit) in &stale {
            out_println!("stale\t{name}\t{path}\t{}", now.saturating_sub(*last_visit));
        }
        for (path_key, names) in &duplicates {
            out_println!("duplicate\t{}\t{}", path_key, names.join(","));
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Kind")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Path / Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Detail")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
    ]);

    for (name, path) in &missing {
        table.add_row(vec![
            Cell::new("missing")
                .fg(Color::Red)
                .add_attribute(Attribute::Bold),
            Cell::new(name).fg(Color::Cyan),
            Cell::new(path).fg(Color::Magenta),
            Cell::new("Fix: xun delete -bm <name> or xun set <name> <path>")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim),
        ]);
    }
    for (name, path, last_visit) in &stale {
        table.add_row(vec![
            Cell::new("stale").fg(Color::Yellow),
            Cell::new(name).fg(Color::Cyan),
            Cell::new(path).fg(Color::Magenta),
            Cell::new(format!("last visited: {}", format_age(*last_visit)))
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim),
        ]);
    }
    for (path_key, names) in &duplicates {
        table.add_row(vec![
            Cell::new("duplicate").fg(Color::Yellow),
            Cell::new(names.join(","))
                .fg(Color::Cyan)
                .add_attribute(Attribute::Dim),
            Cell::new(path_key).fg(Color::Magenta),
            Cell::new("Fix: xun dedup -m path")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim),
        ]);
    }

    if table.row_count() == 0 {
        ui_println!("Check OK: no issues found.");
        return Ok(());
    }

    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_gc(args: GcCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    let dead: Vec<String> = db
        .iter()
        .filter(|(_, e)| !Path::new(&e.path).exists())
        .map(|(k, _)| k.clone())
        .collect();

    if dead.is_empty() {
        ui_println!("No dead links found.");
        return Ok(());
    }

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    if args.purge {
        for k in &dead {
            db.remove(k);
        }
        save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
        ui_println!("Purged {} dead bookmarks.", dead.len());
        return Ok(());
    } else {
        if format == ListFormat::Tsv {
            for k in &dead {
                out_println!("{}\t{}", k, db[k].path);
            }
            return Ok(());
        }
        if format == ListFormat::Json {
            let items: Vec<serde_json::Value> = dead
                .iter()
                .map(|k| serde_json::json!({ "name": k, "path": db[k].path }))
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            return Ok(());
        }

        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(vec![
            Cell::new("Dead Bookmark")
                .add_attribute(Attribute::Bold)
                .fg(Color::Red),
            Cell::new("Missing Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
        ]);
        for k in &dead {
            table.add_row(vec![
                Cell::new(k).fg(Color::Red).add_attribute(Attribute::Bold),
                Cell::new(&db[k].path)
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim),
            ]);
        }
        print_table(&table);

        if can_interact() {
            let ans = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Delete all dead bookmarks?")
                .default(false)
                .interact_on(&Term::stderr());

            if matches!(ans, Ok(true)) {
                for k in &dead {
                    db.remove(k);
                }
                save_db(&file, &db)
                    .map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
                ui_println!("Purged.");
            } else {
                return Err(CliError::new(3, "Cancelled."));
            }
        }
    }

    Ok(())
}

pub(crate) fn cmd_dedup(args: DedupCmd) -> CliResult {
    let mode = parse_dedup_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: path | name"],
        )
    })?;
    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

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
