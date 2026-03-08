use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use console::Style;

use super::baseline::FileMeta;
use super::util::fmt_size;

pub(crate) struct DiffStats {
    pub(crate) new: u32,
    pub(crate) modified: u32,
    pub(crate) deleted: u32,
}

fn print_colored_path(rel: &str) {
    let dim = Style::new().dim();
    let yellow = Style::new().yellow();
    if let Some(pos) = rel.rfind('\\') {
        eprint!(
            "{}{}",
            dim.apply_to(&rel[..=pos]),
            yellow.apply_to(&rel[pos + 1..])
        );
    } else {
        eprint!("{}", yellow.apply_to(rel));
    }
}

pub(crate) fn diff_copy_and_print(
    current: &HashMap<String, PathBuf>,
    old: &mut HashMap<String, FileMeta>,
    dest: &Path,
    copy: bool,
    skip_unchanged: bool,
    show_unchanged: bool,
) -> DiffStats {
    let green = Style::new().green();
    let yellow = Style::new().yellow();
    let blue = Style::new().blue();
    let dim = Style::new().dim();
    let red = Style::new().red();

    let mut stats = DiffStats {
        new: 0,
        modified: 0,
        deleted: 0,
    };

    // collect changed entries for aligned output
    struct Entry {
        rel: String,
        symbol: String,
        color: u8,
        extra: String,
    }
    let mut changed: Vec<Entry> = Vec::new();
    let mut max_len: usize = 0;

    for (rel, src_path) in current {
        let meta = match fs::metadata(src_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let dst = dest.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));

        if let Some(old_meta) = old.remove(rel) {
            let size_changed = meta.len() != old_meta.size;
            let time_changed = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)
                > old_meta.modified + std::time::Duration::from_secs(2);
            if size_changed || time_changed {
                if copy {
                    if let Some(p) = dst.parent() {
                        let _ = fs::create_dir_all(p);
                    }
                    let _ = fs::copy(src_path, &dst);
                }
                let diff = meta.len() as i64 - old_meta.size as i64;
                let (sym, c, extra) = if diff > 0 {
                    ("↑".into(), 1u8, fmt_size(diff))
                } else if diff < 0 {
                    ("↓".into(), 2u8, fmt_size(diff))
                } else {
                    ("•".into(), 3u8, String::new())
                };
                stats.modified += 1;
                max_len = max_len.max(rel.len());
                changed.push(Entry {
                    rel: rel.clone(),
                    symbol: sym,
                    color: c,
                    extra,
                });
            } else if skip_unchanged {
                if show_unchanged {
                    max_len = max_len.max(rel.len());
                    changed.push(Entry {
                        rel: rel.clone(),
                        symbol: "=".into(),
                        color: 4u8,
                        extra: String::new(),
                    });
                }
                continue;
            } else if copy {
                if let Some(p) = dst.parent() {
                    let _ = fs::create_dir_all(p);
                }
                let _ = fs::copy(src_path, &dst);
            }
        } else {
            if copy {
                if let Some(p) = dst.parent() {
                    let _ = fs::create_dir_all(p);
                }
                let _ = fs::copy(src_path, &dst);
            }
            stats.new += 1;
            max_len = max_len.max(rel.len());
            changed.push(Entry {
                rel: rel.clone(),
                symbol: "+".into(),
                color: 0,
                extra: String::new(),
            });
        }
    }

    // print changed files
    for e in &changed {
        match e.color {
            0 => eprint!("{} ", green.apply_to(&e.symbol)),
            1 => eprint!("{} ", yellow.apply_to(&e.symbol)),
            2 => eprint!("{} ", blue.apply_to(&e.symbol)),
            4 => eprint!("{} ", dim.apply_to(&e.symbol)),
            _ => eprint!("{} ", dim.apply_to(&e.symbol)),
        }
        print_colored_path(&e.rel);
        if !e.extra.is_empty() {
            let pad = max_len.saturating_sub(e.rel.len()) + 4;
            eprint!("{:>width$}", "", width = pad);
            match e.color {
                1 => eprint!("{}", yellow.apply_to(&e.extra)),
                2 => eprint!("{}", blue.apply_to(&e.extra)),
                _ => eprint!("{}", &e.extra),
            }
        }
        eprintln!();
    }

    // deleted files
    for rel in old.keys() {
        eprint!("{} ", red.apply_to("-"));
        print_colored_path(rel);
        eprintln!();
        stats.deleted += 1;
    }

    stats
}
