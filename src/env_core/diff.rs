use std::collections::{HashMap, HashSet};

use super::types::{DiffChangeKind, DiffEntry, EnvDiff, EnvVar, PathSegmentDiff};

pub fn diff_var_lists(before: &[EnvVar], after: &[EnvVar]) -> EnvDiff {
    let before_map: HashMap<String, String> = before
        .iter()
        .map(|v| (v.name.to_uppercase(), v.raw_value.clone()))
        .collect();
    let after_map: HashMap<String, String> = after
        .iter()
        .map(|v| (v.name.to_uppercase(), v.raw_value.clone()))
        .collect();
    diff_maps(&before_map, &after_map)
}

pub fn diff_maps(before: &HashMap<String, String>, after: &HashMap<String, String>) -> EnvDiff {
    let mut result = EnvDiff::default();

    for (name, value) in after {
        if !before.contains_key(name) {
            result.added.push(DiffEntry {
                name: name.clone(),
                kind: DiffChangeKind::Added,
                old_value: None,
                new_value: Some(value.clone()),
                path_diff: Vec::new(),
            });
        }
    }

    for (name, value) in before {
        if !after.contains_key(name) {
            result.removed.push(DiffEntry {
                name: name.clone(),
                kind: DiffChangeKind::Removed,
                old_value: Some(value.clone()),
                new_value: None,
                path_diff: Vec::new(),
            });
        }
    }

    for (name, old_value) in before {
        if let Some(new_value) = after.get(name)
            && old_value != new_value
        {
            result.changed.push(DiffEntry {
                name: name.clone(),
                kind: DiffChangeKind::Changed,
                old_value: Some(old_value.clone()),
                new_value: Some(new_value.clone()),
                path_diff: diff_path_segments(old_value, new_value),
            });
        }
    }

    sort_diff(&mut result);
    result
}

pub fn format_diff(diff: &EnvDiff, color: bool) -> String {
    let mut lines = Vec::new();

    for e in &diff.added {
        lines.push(colorize(
            &format!("+ {}={}", e.name, e.new_value.as_deref().unwrap_or("")),
            color,
            "32",
        ));
    }
    for e in &diff.removed {
        lines.push(colorize(
            &format!("- {}={}", e.name, e.old_value.as_deref().unwrap_or("")),
            color,
            "31",
        ));
    }
    for e in &diff.changed {
        lines.push(colorize(&format!("~ {}", e.name), color, "33"));
        lines.push(colorize(
            &format!("  < {}", e.old_value.as_deref().unwrap_or("")),
            color,
            "31",
        ));
        lines.push(colorize(
            &format!("  > {}", e.new_value.as_deref().unwrap_or("")),
            color,
            "32",
        ));
        for seg in &e.path_diff {
            let (prefix, code) = match seg.kind {
                DiffChangeKind::Added => ("+", "32"),
                DiffChangeKind::Removed => ("-", "31"),
                DiffChangeKind::Changed => ("~", "33"),
            };
            lines.push(colorize(
                &format!("    {} {}", prefix, seg.segment),
                color,
                code,
            ));
        }
    }

    if lines.is_empty() {
        "(no changes)".to_string()
    } else {
        lines.join("\n")
    }
}

fn diff_path_segments(old: &str, new: &str) -> Vec<PathSegmentDiff> {
    let old_items = split_path(old);
    let new_items = split_path(new);
    if old_items.is_empty() && new_items.is_empty() {
        return Vec::new();
    }

    let old_set: HashSet<String> = old_items.into_iter().map(|s| s.to_lowercase()).collect();
    let new_set: HashSet<String> = new_items.into_iter().map(|s| s.to_lowercase()).collect();
    let mut diffs = Vec::new();
    for seg in new_set.difference(&old_set) {
        diffs.push(PathSegmentDiff {
            segment: seg.clone(),
            kind: DiffChangeKind::Added,
        });
    }
    for seg in old_set.difference(&new_set) {
        diffs.push(PathSegmentDiff {
            segment: seg.clone(),
            kind: DiffChangeKind::Removed,
        });
    }
    diffs.sort_by(|a, b| a.segment.cmp(&b.segment));
    diffs
}

fn split_path(value: &str) -> Vec<String> {
    if !value.contains(';') {
        return Vec::new();
    }
    value
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn sort_diff(diff: &mut EnvDiff) {
    diff.added
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    diff.removed
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    diff.changed
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
}

fn colorize(line: &str, enabled: bool, code: &str) -> String {
    if enabled {
        format!("\x1b[{}m{}\x1b[0m", code, line)
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_basic() {
        let before = HashMap::from([
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "2".to_string()),
        ]);
        let after = HashMap::from([
            ("A".to_string(), "1".to_string()),
            ("C".to_string(), "3".to_string()),
        ]);
        let diff = diff_maps(&before, &after);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
    }
}
