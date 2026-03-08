use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::Sender;
use regex::Regex;

use crate::commands::delete::progress::Progress;

pub(crate) fn scan_tree(
    root: PathBuf,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    tx: &Sender<PathBuf>,
    progress: &Arc<Progress>,
) {
    let mut stack = vec![root];

    while let Some(dir) = stack.pop() {
        if crate::windows::ctrlc::is_cancelled() {
            return;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if crate::windows::ctrlc::is_cancelled() {
                return;
            }

            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if path.is_dir() {
                if exclude_dirs.contains(&name.to_lowercase()) {
                    continue;
                }
                if matches_any_pattern(path.to_str().unwrap_or(""), patterns) {
                    continue;
                }
                stack.push(path);
            } else {
                if !match_all && !target_names.contains(&name.to_lowercase()) {
                    continue;
                }
                if matches_any_pattern(path.to_str().unwrap_or(""), patterns) {
                    continue;
                }
                progress.inc_scanned();
                let _ = tx.send(path);
            }
        }
    }
}

pub(crate) fn compile_patterns(globs: &[String]) -> Vec<Regex> {
    globs
        .iter()
        .filter_map(|g| {
            let pat = "(?i)^".to_string()
                + &regex::escape(g)
                    .replace(r"\*\*", ".*")
                    .replace(r"\*", r"[^\\]*")
                    .replace(r"\?", ".")
                + "$";
            Regex::new(&pat).ok()
        })
        .collect()
}

fn matches_any_pattern(path: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|rx| rx.is_match(path))
}

pub(crate) fn matches_any(path: &str, patterns: &[Regex]) -> bool {
    matches_any_pattern(path, patterns)
}
