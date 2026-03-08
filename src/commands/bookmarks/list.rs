use std::collections::HashSet;
use std::env;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{AllCmd, FuzzyCmd, KeysCmd, ListCmd, RecentCmd, StatsCmd};
use crate::fuzzy::{FuzzyIndex, matches_tag};
use crate::model::{Entry, ListFormat, ListItem, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, format_age, prefer_table_output, print_table};
use crate::store::{db_path, load};

pub(crate) fn cmd_list(args: ListCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);

    let tag = args.tag.clone().or_else(|| {
        env::var("XUN_DEFAULT_TAG")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });

    let mut entries: Vec<(&String, &Entry)> = db
        .iter()
        .filter(|(_, e)| matches_tag(e, tag.as_deref()))
        .collect();

    match args.sort.to_lowercase().as_str() {
        "name" => entries.sort_by_key(|(k, _)| k.to_lowercase()),
        "last" => entries.sort_by(|a, b| b.1.last_visited.cmp(&a.1.last_visited)),
        "visits" => entries.sort_by(|a, b| b.1.visit_count.cmp(&a.1.visit_count)),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Invalid sort: {}.", args.sort),
                &["Fix: Use one of: name | last | visits"],
            ));
        }
    }
    if args.reverse {
        entries.reverse();
    }
    if let Some(offset) = args.offset {
        let start = offset.min(entries.len());
        entries = entries[start..].to_vec();
    }
    if let Some(limit) = args.limit {
        let end = limit.min(entries.len());
        entries = entries[..end].to_vec();
    }

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;

    if args.tsv && format == ListFormat::Auto {
        format = ListFormat::Tsv;
    }

    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    match format {
        ListFormat::Tsv => {
            for (k, e) in entries {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}",
                    k,
                    e.path,
                    e.tags.join(","),
                    e.visit_count,
                    e.last_visited
                );
            }
            return Ok(());
        }
        ListFormat::Json => {
            let items: Vec<ListItem> = entries
                .iter()
                .map(|(k, e)| ListItem {
                    name: (*k).clone(),
                    path: e.path.clone(),
                    tags: e.tags.clone(),
                    visits: e.visit_count,
                    last_visited: e.last_visited,
                })
                .collect();
            let s = serde_json::to_string(&items)
                .map_err(|e| CliError::new(1, format!("json error: {e}")))?;
            out_println!("{}", s);
            return Ok(());
        }
        ListFormat::Table => {}
        ListFormat::Auto => unreachable!(),
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
    ]);

    for (k, e) in entries {
        let path_exists = Path::new(&e.path).exists();
        let tags = if e.tags.is_empty() {
            Cell::new("-")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(e.tags.join(",")).fg(Color::Yellow)
        };

        table.add_row(vec![
            Cell::new(k).add_attribute(Attribute::Bold).fg(Color::Cyan),
            Cell::new(e.path.clone())
                .fg(if path_exists {
                    Color::DarkGrey
                } else {
                    Color::Red
                })
                .add_attribute(if path_exists {
                    Attribute::Dim
                } else {
                    Attribute::Bold
                }),
            tags,
            Cell::new(e.visit_count).fg(Color::Green),
        ]);
    }

    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_recent(args: RecentCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    let tag = args.tag.clone().or_else(|| {
        env::var("XUN_DEFAULT_TAG")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });
    let mut entries: Vec<(String, Entry)> = db
        .iter()
        .filter(|(_, e)| matches_tag(e, tag.as_deref()))
        .map(|(k, e)| (k.clone(), e.clone()))
        .filter(|(_, e)| e.last_visited > 0)
        .collect();

    if entries.is_empty() {
        ui_println!("No recent bookmarks.");
        return Ok(());
    }

    entries.sort_by(|a, b| b.1.last_visited.cmp(&a.1.last_visited));
    let limit = args.limit.max(1).min(entries.len());
    let entries = &entries[..limit];

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

    if format == ListFormat::Tsv {
        for (k, e) in entries {
            out_println!("{}\t{}\t{}\t{}", k, e.path, e.last_visited, e.visit_count);
        }
        return Ok(());
    }
    if format == ListFormat::Json {
        let items: Vec<ListItem> = entries
            .iter()
            .map(|(k, e)| ListItem {
                name: (*k).clone(),
                path: e.path.clone(),
                tags: e.tags.clone(),
                visits: e.visit_count,
                last_visited: e.last_visited,
            })
            .collect();
        let s = serde_json::to_string(&items)
            .map_err(|e| CliError::new(1, format!("json error: {e}")))?;
        out_println!("{}", s);
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
        Cell::new("Last")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Visits")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    for (k, e) in entries {
        let path_exists = Path::new(&e.path).exists();
        table.add_row(vec![
            Cell::new(k).add_attribute(Attribute::Bold).fg(Color::Cyan),
            Cell::new(e.path.clone())
                .fg(if path_exists {
                    Color::DarkGrey
                } else {
                    Color::Red
                })
                .add_attribute(if path_exists {
                    Attribute::Dim
                } else {
                    Attribute::Bold
                }),
            Cell::new(format_age(e.last_visited)).fg(Color::Yellow),
            Cell::new(e.visit_count).fg(Color::Green),
        ]);
    }
    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_stats(args: StatsCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);

    let total = db.len() as u32;
    let dead = db.values().filter(|e| !Path::new(&e.path).exists()).count() as u32;
    let visited = db.values().filter(|e| e.visit_count > 0).count() as u32;
    let total_visits: u64 = db.values().map(|e| e.visit_count as u64).sum();
    let last_visit = db.values().map(|e| e.last_visited).max().unwrap_or(0);

    let mut tag_set: HashSet<String> = HashSet::new();
    for e in db.values() {
        for t in &e.tags {
            tag_set.insert(t.to_lowercase());
        }
    }
    let tags = tag_set.len() as u32;

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

    if format == ListFormat::Tsv {
        out_println!("bookmarks\t{}", total);
        out_println!("dead\t{}", dead);
        out_println!("tags\t{}", tags);
        out_println!("visited\t{}", visited);
        out_println!("total_visits\t{}", total_visits);
        out_println!("last_visit\t{}", last_visit);
        return Ok(());
    }
    if format == ListFormat::Json {
        let obj = serde_json::json!({
            "bookmarks": total,
            "dead": dead,
            "tags": tags,
            "visited": visited,
            "total_visits": total_visits,
            "last_visit": last_visit
        });
        out_println!("{}", obj);
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Metric")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Value")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    table.add_row(vec![Cell::new("bookmarks"), Cell::new(total)]);
    table.add_row(vec![Cell::new("dead"), Cell::new(dead)]);
    table.add_row(vec![Cell::new("tags"), Cell::new(tags)]);
    table.add_row(vec![Cell::new("visited"), Cell::new(visited)]);
    table.add_row(vec![Cell::new("total_visits"), Cell::new(total_visits)]);
    table.add_row(vec![
        Cell::new("last_visit"),
        Cell::new(format_age(last_visit)),
    ]);
    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_all(args: AllCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    for (k, e) in db
        .iter()
        .filter(|(_, e)| matches_tag(e, args.tag.as_deref()))
    {
        out_println!(
            "{}\t{}\t{}\t{}\t{}",
            k,
            e.path,
            e.tags.join(","),
            e.visit_count,
            e.last_visited
        );
    }
    Ok(())
}

pub(crate) fn cmd_fuzzy(args: FuzzyCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    let index = FuzzyIndex::from_db(&db);
    let mut scored: Vec<(f64, String, Entry)> =
        index.search(&args.pattern, args.tag.as_deref(), None);

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_, k, e) in scored {
        out_println!(
            "{}\t{}\t{}\t{}\t{}",
            k,
            e.path,
            e.tags.join(","),
            e.visit_count,
            e.last_visited
        );
    }
    Ok(())
}

pub(crate) fn cmd_keys(_args: KeysCmd) -> CliResult {
    let file = db_path();
    let db = load(&file);
    for k in db.keys() {
        out_println!("{}", k);
    }
    Ok(())
}
