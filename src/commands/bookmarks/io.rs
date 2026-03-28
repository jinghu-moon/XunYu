use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};

use console::Term;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::cli::{ExportCmd, ImportCmd};
use crate::model::{Entry, ImportMode, IoFormat, ListItem, parse_import_mode, parse_io_format};
use crate::output::{CliError, CliResult, can_interact};
use crate::store::{Lock, db_path, save_db};
use crate::util::parse_tags;

use super::load_bookmark_db;

pub(crate) fn cmd_export(args: ExportCmd) -> CliResult {
    let format = parse_io_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: json | tsv"],
        )
    })?;

    let file = db_path();
    let db = load_bookmark_db(&file)?;
    let mut items: Vec<ListItem> = db
        .iter()
        .map(|(k, e)| ListItem {
            name: k.clone(),
            path: e.path.clone(),
            tags: e.tags.clone(),
            visits: e.visit_count,
            last_visited: e.last_visited,
        })
        .collect();

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let output = match format {
        IoFormat::Json => serde_json::to_string_pretty(&items).unwrap_or_default(),
        IoFormat::Tsv => {
            let mut s = String::new();
            for i in items {
                s.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    i.name,
                    i.path,
                    i.tags.join(","),
                    i.visits,
                    i.last_visited
                ));
            }
            s
        }
    };

    if let Some(path) = args.out {
        fs::write(&path, output).map_err(|e| CliError::new(1, format!("export failed: {e}")))?;
    } else {
        out_println!("{}", output.trim_end());
    }
    Ok(())
}

pub(crate) fn cmd_import(args: ImportCmd) -> CliResult {
    let format = parse_io_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: json | tsv"],
        )
    })?;
    let mode = parse_import_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: merge | overwrite"],
        )
    })?;

    let content = if let Some(path) = args.input {
        fs::read_to_string(&path).map_err(|e| CliError::new(1, format!("import failed: {e}")))?
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|_| CliError::new(1, "import failed: stdin read error"))?;
        buf
    };

    let mut items: Vec<ListItem> = Vec::new();
    match format {
        IoFormat::Json => {
            let parsed: Vec<ListItem> = serde_json::from_str(&content)
                .map_err(|e| CliError::new(1, format!("import json error: {e}")))?;
            items.extend(parsed);
        }
        IoFormat::Tsv => {
            for line in content.lines() {
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() < 2 {
                    continue;
                }
                let name = cols[0].trim().to_string();
                let path = cols[1].trim().to_string();
                if name.is_empty() || path.is_empty() {
                    continue;
                }
                let tags = if cols.len() > 2 {
                    parse_tags(cols[2])
                } else {
                    Vec::new()
                };
                let visits = if cols.len() > 3 {
                    cols[3].trim().parse::<u32>().unwrap_or(0)
                } else {
                    0
                };
                let last_visited = if cols.len() > 4 {
                    cols[4].trim().parse::<u64>().unwrap_or(0)
                } else {
                    0
                };
                items.push(ListItem {
                    name,
                    path,
                    tags,
                    visits,
                    last_visited,
                });
            }
        }
    }

    if items.is_empty() {
        ui_println!("No items to import.");
        return Ok(());
    }

    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load_bookmark_db(&file)?;

    let conflicts = items.iter().filter(|i| db.contains_key(&i.name)).count();
    if mode == ImportMode::Overwrite && conflicts > 0 && !args.yes {
        if can_interact() {
            let ok = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Overwrite {} existing bookmarks?", conflicts))
                .default(false)
                .interact_on(&Term::stderr());
            if !matches!(ok, Ok(true)) {
                return Err(CliError::new(3, "Cancelled."));
            }
        } else {
            return Err(CliError::with_details(
                2,
                "Conflicts detected. Use --yes to overwrite.".to_string(),
                &["Fix: Add --yes to overwrite existing bookmarks, or use --mode merge."],
            ));
        }
    }

    let mut added = 0usize;
    let mut updated = 0usize;
    for item in items {
        let entry = Entry {
            path: item.path.clone(),
            tags: item.tags.clone(),
            visit_count: item.visits,
            last_visited: item.last_visited,
        };

        if let Some(existing) = db.get_mut(&item.name) {
            match mode {
                ImportMode::Merge => {
                    if !item.path.is_empty() {
                        existing.path = item.path.clone();
                    }
                    let mut seen: HashSet<String> =
                        existing.tags.iter().map(|t| t.to_lowercase()).collect();
                    for t in item.tags {
                        if seen.insert(t.to_lowercase()) {
                            existing.tags.push(t);
                        }
                    }
                    existing.visit_count = existing.visit_count.max(item.visits);
                    existing.last_visited = existing.last_visited.max(item.last_visited);
                }
                ImportMode::Overwrite => {
                    *existing = entry;
                }
            }
            updated += 1;
        } else {
            db.insert(item.name.clone(), entry);
            added += 1;
        }
    }

    save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
    ui_println!("Imported: added {}, updated {}.", added, updated);
    Ok(())
}
