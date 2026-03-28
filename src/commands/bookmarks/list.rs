use std::collections::{HashMap, HashSet};
use std::env;
#[cfg(windows)]
use std::ffi::OsString;
use std::path::{Component, Path};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{AllCmd, FuzzyCmd, KeysCmd, ListCmd, RecentCmd, StatsCmd};
use crate::fuzzy::{FuzzyIndex, matches_tag};
use crate::model::{Entry, ListFormat, ListItem, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, format_age, prefer_table_output, print_table};
use crate::store::db_path;

use super::load_bookmark_db;

const TABLE_PATH_PROBE_LIMIT: usize = 128;
#[cfg(windows)]
const DRIVE_FIXED_TYPE: u32 = 3;
#[cfg(windows)]
const DRIVE_RAMDISK_TYPE: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BookmarkPathStatus {
    Existing,
    Missing,
    Unknown,
}

pub(crate) fn cmd_list(args: ListCmd) -> CliResult {
    let file = db_path();
    let db = load_bookmark_db(&file)?;

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

    let path_statuses =
        collect_path_probe_states(entries.iter().map(|(_, e)| e.path.as_str()), default_path_status);
    for (k, e) in entries {
        let tags = if e.tags.is_empty() {
            Cell::new("-")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(e.tags.join(",")).fg(Color::Yellow)
        };

        table.add_row(vec![
            Cell::new(k).add_attribute(Attribute::Bold).fg(Color::Cyan),
            render_path_cell(
                &e.path,
                path_statuses
                    .get(&e.path)
                    .copied()
                    .unwrap_or(BookmarkPathStatus::Unknown),
            ),
            tags,
            Cell::new(e.visit_count).fg(Color::Green),
        ]);
    }

    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_recent(args: RecentCmd) -> CliResult {
    let file = db_path();
    let db = load_bookmark_db(&file)?;
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
    let path_statuses =
        collect_path_probe_states(entries.iter().map(|(_, e)| e.path.as_str()), default_path_status);
    for (k, e) in entries {
        table.add_row(vec![
            Cell::new(k).add_attribute(Attribute::Bold).fg(Color::Cyan),
            render_path_cell(
                &e.path,
                path_statuses
                    .get(&e.path)
                    .copied()
                    .unwrap_or(BookmarkPathStatus::Unknown),
            ),
            Cell::new(format_age(e.last_visited)).fg(Color::Yellow),
            Cell::new(e.visit_count).fg(Color::Green),
        ]);
    }
    print_table(&table);
    Ok(())
}

pub(crate) fn cmd_stats(args: StatsCmd) -> CliResult {
    let file = db_path();
    let db = load_bookmark_db(&file)?;

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
    let db = load_bookmark_db(&file)?;
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
    let db = load_bookmark_db(&file)?;
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
    let db = load_bookmark_db(&file)?;
    for k in db.keys() {
        out_println!("{}", k);
    }
    Ok(())
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

fn default_path_status(path: &Path) -> BookmarkPathStatus {
    if !should_probe_path_exists(path) {
        return BookmarkPathStatus::Unknown;
    }
    if path.exists() {
        BookmarkPathStatus::Existing
    } else {
        BookmarkPathStatus::Missing
    }
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

#[cfg(windows)]
fn should_probe_path_exists(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    let raw = path.to_string_lossy();
    if raw.starts_with(r"\\") || raw.starts_with("//") {
        return false;
    }
    if !path.is_absolute() {
        return true;
    }
    let Some(root) = drive_root(path) else {
        return false;
    };
    let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
    matches!(
        unsafe { GetDriveTypeW(wide.as_ptr()) },
        DRIVE_FIXED_TYPE | DRIVE_RAMDISK_TYPE
    )
}

#[cfg(not(windows))]
fn should_probe_path_exists(path: &Path) -> bool {
    !path.as_os_str().is_empty()
}

#[cfg(windows)]
fn drive_root(path: &Path) -> Option<OsString> {
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Prefix(prefix)), Some(Component::RootDir)) => {
            let mut root = prefix.as_os_str().to_os_string();
            root.push("\\");
            Some(root)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut probes = 0usize;

        let statuses = collect_path_probe_states(paths.iter().map(String::as_str), |_| {
            probes += 1;
            BookmarkPathStatus::Existing
        });

        assert_eq!(probes, 0);
        assert!(statuses
            .values()
            .all(|status| *status == BookmarkPathStatus::Unknown));
    }
}
