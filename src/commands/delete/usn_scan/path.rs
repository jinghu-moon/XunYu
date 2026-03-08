use std::collections::{HashMap, HashSet};

use super::common::frn_index;

pub(super) fn resolve_path(
    frn: u64,
    map: &HashMap<u64, (String, u64, bool)>,
    drive: &str,
) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = frn;
    let mut visited = HashSet::new();

    loop {
        if visited.contains(&current) {
            return None;
        }
        visited.insert(current);

        match map.get(&current) {
            None => break,
            Some((name, parent, _)) => {
                parts.push(name.clone());
                if frn_index(*parent) == current {
                    break;
                }
                current = frn_index(*parent);
            }
        }

        if parts.len() > 64 {
            return None;
        }
    }

    parts.reverse();
    let mut path = format!("{}:\\", drive);
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            path.push('\\');
        }
        path.push_str(part);
    }
    Some(path)
}

pub(super) fn path_excluded(path: &str, exclude_dirs: &HashSet<String>) -> bool {
    let segments = path.split(['\\', '/']).skip(1);
    for seg in segments {
        if exclude_dirs.contains(&seg.to_lowercase()) {
            return true;
        }
    }
    false
}
