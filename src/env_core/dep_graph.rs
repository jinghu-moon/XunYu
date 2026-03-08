use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;
use regex::Regex;

use super::types::{EnvDepTree, EnvError, EnvResult, EnvScope, EnvVar};

lazy_static! {
    static ref VAR_REF_RE: Regex =
        Regex::new(r"%([A-Za-z0-9_]+)%").expect("dependency regex must be valid");
}

pub fn build_tree(
    scope: EnvScope,
    vars: &[EnvVar],
    root: &str,
    max_depth: usize,
) -> EnvResult<EnvDepTree> {
    let root_trimmed = root.trim();
    if root_trimmed.is_empty() {
        return Err(EnvError::InvalidInput(
            "graph root variable name cannot be empty".to_string(),
        ));
    }

    let depth_limit = max_depth.clamp(1, 64);
    let (display_names, adjacency, present) = build_maps(vars);

    let root_norm = normalize_name(root_trimmed);
    let root_name = display_names
        .get(&root_norm)
        .cloned()
        .unwrap_or_else(|| root_trimmed.to_string());

    let mut lines = Vec::new();
    let mut missing = HashSet::<String>::new();
    let mut cycles = HashSet::<String>::new();

    let root_present = present.contains(&root_norm);
    if root_present {
        lines.push(root_name.clone());
    } else {
        lines.push(format!("{} [missing]", root_name));
        missing.insert(root_name.clone());
    }

    if root_present {
        let mut path = vec![root_norm.clone()];
        walk_tree(
            &root_norm,
            "",
            depth_limit,
            0,
            &adjacency,
            &display_names,
            &present,
            &mut path,
            &mut lines,
            &mut missing,
            &mut cycles,
        );
    }

    let mut missing_vec = missing.into_iter().collect::<Vec<_>>();
    missing_vec.sort();
    let mut cycles_vec = cycles.into_iter().collect::<Vec<_>>();
    cycles_vec.sort();

    Ok(EnvDepTree {
        scope,
        root: root_name,
        lines,
        missing: missing_vec,
        cycles: cycles_vec,
    })
}

fn walk_tree(
    current: &str,
    prefix: &str,
    max_depth: usize,
    depth: usize,
    adjacency: &HashMap<String, Vec<String>>,
    display_names: &HashMap<String, String>,
    present: &HashSet<String>,
    path: &mut Vec<String>,
    lines: &mut Vec<String>,
    missing: &mut HashSet<String>,
    cycles: &mut HashSet<String>,
) {
    if depth >= max_depth {
        return;
    }
    let deps = adjacency.get(current).cloned().unwrap_or_default();
    let deps_len = deps.len();
    for (idx, dep) in deps.into_iter().enumerate() {
        let is_last = idx + 1 == deps_len;
        let branch = if is_last { "└─" } else { "├─" };
        let next_prefix = format!("{}{}", prefix, if is_last { "  " } else { "│ " });
        let dep_name = display_names.get(&dep).cloned().unwrap_or(dep.clone());
        let dep_present = present.contains(&dep);

        if let Some(pos) = path.iter().position(|p| *p == dep) {
            let mut cycle_path = path[pos..]
                .iter()
                .map(|n| display_names.get(n).cloned().unwrap_or_else(|| n.clone()))
                .collect::<Vec<_>>();
            cycle_path.push(dep_name.clone());
            cycles.insert(cycle_path.join(" -> "));
            lines.push(format!("{}{} {} [cycle]", prefix, branch, dep_name));
            continue;
        }

        if !dep_present {
            lines.push(format!("{}{} {} [missing]", prefix, branch, dep_name));
            missing.insert(dep_name);
            continue;
        }

        lines.push(format!("{}{} {}", prefix, branch, dep_name));
        let dep_key = dep.clone();
        path.push(dep_key.clone());
        walk_tree(
            &dep_key,
            &next_prefix,
            max_depth,
            depth + 1,
            adjacency,
            display_names,
            present,
            path,
            lines,
            missing,
            cycles,
        );
        let _ = path.pop();
    }
}

fn build_maps(
    vars: &[EnvVar],
) -> (
    HashMap<String, String>,
    HashMap<String, Vec<String>>,
    HashSet<String>,
) {
    let mut display_names = HashMap::<String, String>::new();
    let mut adjacency = HashMap::<String, Vec<String>>::new();
    let mut present = HashSet::<String>::new();

    for var in vars {
        let key = normalize_name(&var.name);
        present.insert(key.clone());
        display_names
            .entry(key.clone())
            .or_insert_with(|| var.name.clone());

        let refs = extract_refs(&var.raw_value);
        adjacency.insert(key, refs.clone());
        for dep in refs {
            display_names.entry(dep.clone()).or_insert(dep);
        }
    }

    (display_names, adjacency, present)
}

fn extract_refs(value: &str) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for caps in VAR_REF_RE.captures_iter(value) {
        let Some(raw) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let key = normalize_name(raw);
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::build_tree;
    use crate::env_core::types::{EnvScope, EnvVar};

    fn mk_var(name: &str, value: &str) -> EnvVar {
        EnvVar {
            scope: EnvScope::User,
            name: name.to_string(),
            raw_value: value.to_string(),
            reg_type: 1,
            inferred_kind: None,
        }
    }

    #[test]
    fn tree_marks_cycle_and_missing() {
        let vars = vec![mk_var("A", "%B%"), mk_var("B", "%A%;%C%")];
        let tree = build_tree(EnvScope::User, &vars, "A", 8).expect("tree");
        assert!(tree.lines.iter().any(|l| l.contains("[cycle]")));
        assert!(tree.lines.iter().any(|l| l.contains("[missing]")));
        assert!(!tree.cycles.is_empty());
        assert!(!tree.missing.is_empty());
    }
}
