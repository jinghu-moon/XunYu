use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::time::Duration;

use crate::bookmark::cache::{SourceFingerprint, load_cache_owner_checked, store_cache_path};
use crate::bookmark::lightweight::{BookmarkArchivedPayloadOwner, BookmarkArchivedRow};
use comfy_table::{Attribute, Cell, Color, Table};

use crate::bookmark::path_probe::{BookmarkPathStatus, path_status};
use crate::bookmark::storage::db_path;
use crate::bookmark_state::{Bookmark, Store};
use crate::cli::{AllCmd, KeysCmd, ListCmd, RecentCmd, StatsCmd};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, format_age, prefer_table_output, print_table};
use crate::store::now_secs;

const TABLE_PATH_PROBE_LIMIT: usize = 128;

pub(crate) fn cmd_list(args: ListCmd) -> CliResult {
    let file = db_path();
    let tag = args.tag.clone().or_else(|| {
        env::var("XUN_DEFAULT_TAG")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });

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

    if let Some(owner) = try_load_lightweight_owner(&file) {
        return cmd_list_borrowed(&args, tag.as_deref(), format, &owner);
    }

    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;

    let mut bookmarks: Vec<&Bookmark> = store
        .bookmarks
        .iter()
        .filter(|bookmark| matches_tag(bookmark, tag.as_deref()))
        .collect();

    match args.sort.to_lowercase().as_str() {
        "name" => bookmarks.sort_by_key(|bookmark| {
            bookmark
                .name
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
        }),
        "last" => bookmarks.sort_by(|a, b| b.last_visited.cmp(&a.last_visited)),
        "visits" => bookmarks.sort_by(|a, b| b.visit_count.cmp(&a.visit_count)),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Invalid sort: {}.", args.sort),
                &["Fix: Use one of: name | last | visits"],
            ))
        }
    }
    if args.reverse {
        bookmarks.reverse();
    }
    if let Some(offset) = args.offset {
        let start = offset.min(bookmarks.len());
        bookmarks = bookmarks[start..].to_vec();
    }
    if let Some(limit) = args.limit {
        let end = limit.min(bookmarks.len());
        bookmarks = bookmarks[..end].to_vec();
    }

    let path_statuses =
        collect_path_probe_states(bookmarks.iter().map(|bookmark| bookmark.path.as_str()), path_status);
    let show_desc = bookmarks.iter().any(|bookmark| !bookmark.desc.trim().is_empty());

    match format {
        ListFormat::Tsv => {
            for bookmark in bookmarks {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    bookmark.name.as_deref().unwrap_or(""),
                    bookmark.path,
                    bookmark.tags.join(","),
                    bookmark.visit_count.unwrap_or(0),
                    bookmark.last_visited.unwrap_or(0),
                    bookmark.desc
                );
            }
            Ok(())
        }
        ListFormat::Json => {
            let items: Vec<serde_json::Value> = bookmarks
                .iter()
                .map(|bookmark| {
                    serde_json::json!({
                        "name": bookmark.name,
                        "path": bookmark.path,
                        "tags": bookmark.tags,
                        "visits": bookmark.visit_count,
                        "last_visited": bookmark.last_visited,
                        "desc": bookmark.desc,
                        "source": format!("{:?}", bookmark.source).to_ascii_lowercase(),
                        "pinned": bookmark.pinned,
                        "workspace": bookmark.workspace
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            Ok(())
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            let mut header = vec![
                Cell::new("Name").add_attribute(Attribute::Bold).fg(Color::Cyan),
                Cell::new("Path")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("Tags")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("Visits")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
            ];
            if show_desc {
                header.push(
                    Cell::new("Desc")
                        .add_attribute(Attribute::Bold)
                        .fg(Color::DarkGrey),
                );
            }
            table.set_header(header);

            for bookmark in bookmarks {
                let mut row = vec![
                    Cell::new(bookmark.name.as_deref().unwrap_or("(unnamed)"))
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan),
                    render_path_cell(
                        &bookmark.path,
                        path_statuses
                            .get(&bookmark.path)
                            .copied()
                            .unwrap_or(BookmarkPathStatus::Unknown),
                    ),
                    if bookmark.tags.is_empty() {
                        Cell::new("-").fg(Color::DarkGrey).add_attribute(Attribute::Dim)
                    } else {
                        Cell::new(bookmark.tags.join(",")).fg(Color::Yellow)
                    },
                    Cell::new(bookmark.visit_count.unwrap_or(0)).fg(Color::Green),
                ];
                if show_desc {
                    row.push(
                        Cell::new(if bookmark.desc.is_empty() {
                            "-"
                        } else {
                            bookmark.desc.as_str()
                        })
                        .fg(Color::DarkGrey),
                    );
                }
                table.add_row(row);
            }

            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

fn cmd_list_borrowed(
    args: &ListCmd,
    tag: Option<&str>,
    format: ListFormat,
    owner: &BookmarkArchivedPayloadOwner,
) -> CliResult {
    let rows = owner
        .rows()
        .map_err(|e| CliError::new(1, format!("Failed to read lightweight rows: {e}")))?;
    let mut bookmarks: Vec<_> = rows
        .iter()
        .filter(|bookmark| matches_tag_row(*bookmark, tag))
        .collect();

    match args.sort.to_lowercase().as_str() {
        "name" => bookmarks.sort_by_key(|bookmark| bookmark.name().unwrap_or("").to_ascii_lowercase()),
        "last" => bookmarks.sort_by(|a, b| b.last_visited().cmp(&a.last_visited())),
        "visits" => bookmarks.sort_by(|a, b| b.visit_count().cmp(&a.visit_count())),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Invalid sort: {}.", args.sort),
                &["Fix: Use one of: name | last | visits"],
            ))
        }
    }
    if args.reverse {
        bookmarks.reverse();
    }
    if let Some(offset) = args.offset {
        let start = offset.min(bookmarks.len());
        bookmarks = bookmarks[start..].to_vec();
    }
    if let Some(limit) = args.limit {
        let end = limit.min(bookmarks.len());
        bookmarks = bookmarks[..end].to_vec();
    }

    let path_statuses =
        collect_path_probe_states(bookmarks.iter().map(|bookmark| bookmark.path()), path_status);
    let show_desc = bookmarks.iter().any(|bookmark| !bookmark.desc().trim().is_empty());

    match format {
        ListFormat::Tsv => {
            for bookmark in bookmarks {
                out_println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    bookmark.name().unwrap_or(""),
                    bookmark.path(),
                    bookmark.tags().collect::<Vec<_>>().join(","),
                    bookmark.visit_count().unwrap_or(0),
                    bookmark.last_visited().unwrap_or(0),
                    bookmark.desc()
                );
            }
            Ok(())
        }
        ListFormat::Json => {
            let items: Vec<serde_json::Value> = bookmarks
                .iter()
                .map(|bookmark| {
                    serde_json::json!({
                        "name": bookmark.name(),
                        "path": bookmark.path(),
                        "tags": bookmark.tags().collect::<Vec<_>>(),
                        "visits": bookmark.visit_count(),
                        "last_visited": bookmark.last_visited(),
                        "desc": bookmark.desc(),
                        "source": format!("{:?}", bookmark.source()).to_ascii_lowercase(),
                        "pinned": bookmark.pinned(),
                        "workspace": bookmark.workspace()
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            Ok(())
        }
        ListFormat::Table => {
            let mut table = Table::new();
            apply_pretty_table_style(&mut table);
            let mut header = vec![
                Cell::new("Name").add_attribute(Attribute::Bold).fg(Color::Cyan),
                Cell::new("Path")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Magenta),
                Cell::new("Tags")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Yellow),
                Cell::new("Visits")
                    .add_attribute(Attribute::Bold)
                    .fg(Color::Green),
            ];
            if show_desc {
                header.push(
                    Cell::new("Desc")
                        .add_attribute(Attribute::Bold)
                        .fg(Color::DarkGrey),
                );
            }
            table.set_header(header);

            for bookmark in bookmarks {
                let tags = bookmark.tags().collect::<Vec<_>>();
                let mut row = vec![
                    Cell::new(bookmark.name().unwrap_or("(unnamed)"))
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan),
                    render_path_cell(
                        bookmark.path(),
                        path_statuses
                            .get(bookmark.path())
                            .copied()
                            .unwrap_or(BookmarkPathStatus::Unknown),
                    ),
                    if tags.is_empty() {
                        Cell::new("-").fg(Color::DarkGrey).add_attribute(Attribute::Dim)
                    } else {
                        Cell::new(tags.join(",")).fg(Color::Yellow)
                    },
                    Cell::new(bookmark.visit_count().unwrap_or(0)).fg(Color::Green),
                ];
                if show_desc {
                    row.push(
                        Cell::new(if bookmark.desc().is_empty() {
                            "-"
                        } else {
                            bookmark.desc()
                        })
                        .fg(Color::DarkGrey),
                    );
                }
                table.add_row(row);
            }

            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

pub(crate) fn cmd_recent(args: RecentCmd) -> CliResult {
    let file = db_path();
    let tag = args.tag.clone().or_else(|| {
        env::var("XUN_DEFAULT_TAG")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });
    let since_secs = match args.since.as_deref() {
        Some(raw) => Some(parse_duration(raw)?.as_secs()),
        None => None,
    };
    let now = now_secs();

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

    if let Some(owner) = try_load_lightweight_owner(&file) {
        return cmd_recent_borrowed(&args, tag.as_deref(), format, now, since_secs, &owner);
    }

    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;

    let mut bookmarks: Vec<&Bookmark> = store
        .bookmarks
        .iter()
        .filter(|bookmark| matches_tag(bookmark, tag.as_deref()))
        .filter(|bookmark| {
            args.workspace.as_deref().is_none_or(|workspace| {
                bookmark.workspace.as_deref() == Some(workspace)
            })
        })
        .filter(|bookmark| {
            if let Some(last) = bookmark.last_visited {
                if let Some(since) = since_secs {
                    return now.saturating_sub(last) <= since;
                }
                return true;
            }
            false
        })
        .collect();

    if bookmarks.is_empty() {
        ui_println!("No recent bookmarks.");
        return Ok(());
    }

    bookmarks.sort_by(|a, b| b.last_visited.cmp(&a.last_visited));
    let limit = args.limit.max(1).min(bookmarks.len());
    let bookmarks = &bookmarks[..limit];

    match format {
        ListFormat::Tsv => {
            for bookmark in bookmarks {
                out_println!(
                    "{}\t{}\t{}\t{}",
                    bookmark.name.as_deref().unwrap_or(""),
                    bookmark.path,
                    bookmark.last_visited.unwrap_or(0),
                    bookmark.visit_count.unwrap_or(0)
                );
            }
            Ok(())
        }
        ListFormat::Json => {
            let items: Vec<serde_json::Value> = bookmarks
                .iter()
                .map(|bookmark| {
                    serde_json::json!({
                        "name": bookmark.name,
                        "path": bookmark.path,
                        "tags": bookmark.tags,
                        "visits": bookmark.visit_count,
                        "last_visited": bookmark.last_visited,
                        "workspace": bookmark.workspace
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            Ok(())
        }
        ListFormat::Table => {
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
            for bookmark in bookmarks {
                table.add_row(vec![
                    Cell::new(bookmark.name.as_deref().unwrap_or("(unnamed)"))
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan),
                    Cell::new(&bookmark.path)
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                    Cell::new(format_age(bookmark.last_visited.unwrap_or(0))).fg(Color::Yellow),
                    Cell::new(bookmark.visit_count.unwrap_or(0)).fg(Color::Green),
                ]);
            }
            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

fn cmd_recent_borrowed(
    args: &RecentCmd,
    tag: Option<&str>,
    format: ListFormat,
    now: u64,
    since_secs: Option<u64>,
    owner: &BookmarkArchivedPayloadOwner,
) -> CliResult {
    let rows = owner
        .rows()
        .map_err(|e| CliError::new(1, format!("Failed to read lightweight rows: {e}")))?;
    let mut bookmarks: Vec<_> = rows
        .iter()
        .filter(|bookmark| matches_tag_row(*bookmark, tag))
        .filter(|bookmark| {
            args.workspace
                .as_deref()
                .is_none_or(|workspace| bookmark.workspace() == Some(workspace))
        })
        .filter(|bookmark| {
            if let Some(last) = bookmark.last_visited() {
                if let Some(since) = since_secs {
                    return now.saturating_sub(last) <= since;
                }
                return true;
            }
            false
        })
        .collect();

    if bookmarks.is_empty() {
        ui_println!("No recent bookmarks.");
        return Ok(());
    }

    bookmarks.sort_by(|a, b| b.last_visited().cmp(&a.last_visited()));
    let limit = args.limit.max(1).min(bookmarks.len());
    let bookmarks = &bookmarks[..limit];

    match format {
        ListFormat::Tsv => {
            for bookmark in bookmarks {
                out_println!(
                    "{}\t{}\t{}\t{}",
                    bookmark.name().unwrap_or(""),
                    bookmark.path(),
                    bookmark.last_visited().unwrap_or(0),
                    bookmark.visit_count().unwrap_or(0)
                );
            }
            Ok(())
        }
        ListFormat::Json => {
            let items: Vec<serde_json::Value> = bookmarks
                .iter()
                .map(|bookmark| {
                    serde_json::json!({
                        "name": bookmark.name(),
                        "path": bookmark.path(),
                        "tags": bookmark.tags().collect::<Vec<_>>(),
                        "visits": bookmark.visit_count(),
                        "last_visited": bookmark.last_visited(),
                        "workspace": bookmark.workspace()
                    })
                })
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            Ok(())
        }
        ListFormat::Table => {
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
            for bookmark in bookmarks {
                table.add_row(vec![
                    Cell::new(bookmark.name().unwrap_or("(unnamed)"))
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan),
                    Cell::new(bookmark.path())
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                    Cell::new(format_age(bookmark.last_visited().unwrap_or(0))).fg(Color::Yellow),
                    Cell::new(bookmark.visit_count().unwrap_or(0)).fg(Color::Green),
                ]);
            }
            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

pub(crate) fn cmd_stats(args: StatsCmd) -> CliResult {
    let file = db_path();
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

    if let Some(owner) = try_load_lightweight_owner(&file) {
        return cmd_stats_borrowed(format, &owner);
    }

    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;

    let total = store.bookmarks.len() as u32;
    let dead = store
        .bookmarks
        .iter()
        .filter(|bookmark| !Path::new(&bookmark.path).exists())
        .count() as u32;
    let visited = store
        .bookmarks
        .iter()
        .filter(|bookmark| bookmark.visit_count.unwrap_or(0) > 0)
        .count() as u32;
    let total_visits: u64 = store
        .bookmarks
        .iter()
        .map(|bookmark| bookmark.visit_count.unwrap_or(0) as u64)
        .sum();
    let last_visit = store
        .bookmarks
        .iter()
        .filter_map(|bookmark| bookmark.last_visited)
        .max()
        .unwrap_or(0);
    let tags = store
        .bookmarks
        .iter()
        .flat_map(|bookmark| bookmark.tags.iter().map(|tag| tag.to_ascii_lowercase()))
        .collect::<HashSet<_>>()
        .len() as u32;

    match format {
        ListFormat::Tsv => {
            out_println!("bookmarks\t{}", total);
            out_println!("dead\t{}", dead);
            out_println!("tags\t{}", tags);
            out_println!("visited\t{}", visited);
            out_println!("total_visits\t{}", total_visits);
            out_println!("last_visit\t{}", last_visit);
            Ok(())
        }
        ListFormat::Json => {
            out_println!(
                "{}",
                serde_json::json!({
                    "bookmarks": total,
                    "dead": dead,
                    "tags": tags,
                    "visited": visited,
                    "total_visits": total_visits,
                    "last_visit": last_visit
                })
            );
            Ok(())
        }
        ListFormat::Table => {
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
            table.add_row(vec![Cell::new("last_visit"), Cell::new(format_age(last_visit))]);
            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

fn cmd_stats_borrowed(format: ListFormat, owner: &BookmarkArchivedPayloadOwner) -> CliResult {
    let rows = owner
        .rows()
        .map_err(|e| CliError::new(1, format!("Failed to read lightweight rows: {e}")))?;
    let bookmarks: Vec<_> = rows.iter().collect();

    let total = bookmarks.len() as u32;
    let dead = bookmarks
        .iter()
        .filter(|bookmark| !Path::new(bookmark.path()).exists())
        .count() as u32;
    let visited = bookmarks
        .iter()
        .filter(|bookmark| bookmark.visit_count().unwrap_or(0) > 0)
        .count() as u32;
    let total_visits: u64 = bookmarks
        .iter()
        .map(|bookmark| bookmark.visit_count().unwrap_or(0) as u64)
        .sum();
    let last_visit = bookmarks
        .iter()
        .filter_map(|bookmark| bookmark.last_visited())
        .max()
        .unwrap_or(0);
    let tags = bookmarks
        .iter()
        .flat_map(|bookmark| bookmark.tags().map(str::to_ascii_lowercase))
        .collect::<HashSet<_>>()
        .len() as u32;

    match format {
        ListFormat::Tsv => {
            out_println!("bookmarks\t{}", total);
            out_println!("dead\t{}", dead);
            out_println!("tags\t{}", tags);
            out_println!("visited\t{}", visited);
            out_println!("total_visits\t{}", total_visits);
            out_println!("last_visit\t{}", last_visit);
            Ok(())
        }
        ListFormat::Json => {
            out_println!(
                "{}",
                serde_json::json!({
                    "bookmarks": total,
                    "dead": dead,
                    "tags": tags,
                    "visited": visited,
                    "total_visits": total_visits,
                    "last_visit": last_visit
                })
            );
            Ok(())
        }
        ListFormat::Table => {
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
            table.add_row(vec![Cell::new("last_visit"), Cell::new(format_age(last_visit))]);
            print_table(&table);
            Ok(())
        }
        ListFormat::Auto => unreachable!(),
    }
}

pub(crate) fn cmd_all(args: AllCmd) -> CliResult {
    let file = db_path();
    if let Some(owner) = try_load_lightweight_owner(&file) {
        return cmd_all_borrowed(&args, &owner);
    }
    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    for bookmark in store
        .bookmarks
        .iter()
        .filter(|bookmark| matches_tag(bookmark, args.tag.as_deref()))
    {
        out_println!(
            "{}\t{}\t{}\t{}\t{}",
            bookmark.name.as_deref().unwrap_or(""),
            bookmark.path,
            bookmark.tags.join(","),
            bookmark.visit_count.unwrap_or(0),
            bookmark.last_visited.unwrap_or(0)
        );
    }
    Ok(())
}

fn cmd_all_borrowed(args: &AllCmd, owner: &BookmarkArchivedPayloadOwner) -> CliResult {
    let rows = owner
        .rows()
        .map_err(|e| CliError::new(1, format!("Failed to read lightweight rows: {e}")))?;
    for bookmark in rows
        .iter()
        .filter(|bookmark| matches_tag_row(*bookmark, args.tag.as_deref()))
    {
        out_println!(
            "{}\t{}\t{}\t{}\t{}",
            bookmark.name().unwrap_or(""),
            bookmark.path(),
            bookmark.tags().collect::<Vec<_>>().join(","),
            bookmark.visit_count().unwrap_or(0),
            bookmark.last_visited().unwrap_or(0)
        );
    }
    Ok(())
}

pub(crate) fn cmd_keys(_args: KeysCmd) -> CliResult {
    let file = db_path();
    if let Some(owner) = try_load_lightweight_owner(&file) {
        return cmd_keys_borrowed(&owner);
    }
    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let mut names: Vec<&str> = store
        .bookmarks
        .iter()
        .filter_map(|bookmark| bookmark.name.as_deref())
        .collect();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    for name in names {
        out_println!("{}", name);
    }
    Ok(())
}

fn cmd_keys_borrowed(owner: &BookmarkArchivedPayloadOwner) -> CliResult {
    let rows = owner
        .rows()
        .map_err(|e| CliError::new(1, format!("Failed to read lightweight rows: {e}")))?;
    let mut names: Vec<String> = rows
        .iter()
        .filter_map(|bookmark| bookmark.name().map(str::to_string))
        .collect();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    for name in names {
        out_println!("{}", name);
    }
    Ok(())
}

fn matches_tag(bookmark: &Bookmark, tag: Option<&str>) -> bool {
    match tag {
        Some(tag) if !tag.trim().is_empty() => bookmark
            .tags
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(tag)),
        _ => true,
    }
}

fn matches_tag_row(bookmark: BookmarkArchivedRow<'_>, tag: Option<&str>) -> bool {
    match tag {
        Some(tag) if !tag.trim().is_empty() => bookmark
            .tags()
            .any(|existing| existing.eq_ignore_ascii_case(tag)),
        _ => true,
    }
}

fn try_load_lightweight_owner(file: &Path) -> Option<BookmarkArchivedPayloadOwner> {
    if std::env::var_os("XUN_BM_DISABLE_LIGHTWEIGHT_VIEW").is_some() {
        return None;
    }
    let fingerprint = SourceFingerprint::from_path(file).ok()?;
    load_cache_owner_checked(
        &store_cache_path(file),
        crate::bookmark::migration::CURRENT_SCHEMA_VERSION,
        &fingerprint,
        None,
    )
    .ok()
    .flatten()
}

fn parse_duration(raw: &str) -> CliResult<Duration> {
    let raw = raw.trim();
    if raw.len() < 2 {
        return Err(CliError::new(2, "Invalid duration format."));
    }
    let (num, unit) = raw.split_at(raw.len() - 1);
    let value = num
        .parse::<u64>()
        .map_err(|_| CliError::new(2, "Invalid duration value."))?;
    match unit {
        "d" => Ok(Duration::from_secs(value * 86_400)),
        "h" => Ok(Duration::from_secs(value * 3_600)),
        "m" => Ok(Duration::from_secs(value * 60)),
        _ => Err(CliError::new(2, "Invalid duration unit.")),
    }
}

fn collect_path_probe_states<'a, I, F>(
    paths: I,
    mut probe: F,
) -> HashMap<String, BookmarkPathStatus>
where
    I: IntoIterator<Item = &'a str>,
    F: FnMut(&Path) -> BookmarkPathStatus,
{
    let mut unique_paths = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        if seen.insert(path.to_string()) {
            unique_paths.push(path.to_string());
        }
    }

    if unique_paths.len() > TABLE_PATH_PROBE_LIMIT {
        return unique_paths
            .into_iter()
            .map(|path| (path, BookmarkPathStatus::Unknown))
            .collect();
    }

    unique_paths
        .into_iter()
        .map(|path| {
            let status = probe(Path::new(&path));
            (path, status)
        })
        .collect()
}

fn render_path_cell(path: &str, status: BookmarkPathStatus) -> Cell {
    match status {
        BookmarkPathStatus::Existing => Cell::new(path)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
        BookmarkPathStatus::Missing => Cell::new(path)
            .fg(Color::Red)
            .add_attribute(Attribute::Bold),
        BookmarkPathStatus::Unknown => Cell::new(path)
            .fg(Color::DarkGrey)
            .add_attribute(Attribute::Dim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn collect_path_probe_states_deduplicates_identical_paths() {
        let paths = [r"C:\a", r"C:\a", r"C:\b"];
        let mut probes = 0usize;

        let statuses = collect_path_probe_states(paths.iter().copied(), |_| {
            probes += 1;
            BookmarkPathStatus::Unknown
        });

        assert_eq!(probes, 2);
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn collect_path_probe_states_skips_large_path_sets() {
        let paths: Vec<String> = (0..=TABLE_PATH_PROBE_LIMIT)
            .map(|idx| format!(r"C:\bulk\{idx}"))
            .collect();
        let statuses = collect_path_probe_states(paths.iter().map(|s| s.as_str()), |_| {
            panic!("probe should not run for large sets")
        });
        assert_eq!(statuses.len(), TABLE_PATH_PROBE_LIMIT + 1);
        assert!(statuses.values().all(|status| *status == BookmarkPathStatus::Unknown));
    }

    #[test]
    fn parse_duration_accepts_common_units() {
        assert_eq!(parse_duration("7d").unwrap().as_secs(), 7 * 86_400);
        assert_eq!(parse_duration("24h").unwrap().as_secs(), 24 * 3_600);
        assert_eq!(parse_duration("30m").unwrap().as_secs(), 30 * 60);
    }

    #[test]
    fn render_path_cell_marks_missing() {
        let cell = render_path_cell("C:/missing", BookmarkPathStatus::Missing);
        assert!(cell.content().contains("C:/missing"));
    }

    #[test]
    fn recent_filter_workspace_smoke() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("bookmark.json");
        let mut store = Store::new();
        store
            .import_entry("C:/work/foo", Path::new("C:/"), None, 50.0, 10)
            .unwrap();
        store.bookmarks[0].workspace = Some("xunyu".to_string());
        store.bookmarks[0].last_visited = Some(now_secs());
        store.save(&file, now_secs()).unwrap();
        let loaded = Store::load(&file).unwrap();
        assert_eq!(loaded.bookmarks[0].workspace.as_deref(), Some("xunyu"));
    }
}
