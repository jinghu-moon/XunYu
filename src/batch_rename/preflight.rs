// batch_rename/preflight.rs
//
// Unified preflight checks: ILLEGAL_CHAR, RESERVED_NAME, CYCLE, CONFLICT.
// All checks run to completion — no short-circuit — so the user sees all issues at once.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

// ─── Error types ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum PreflightError {
    /// Target filename contains a Windows-illegal character.
    IllegalChar { target: PathBuf, chars: Vec<char> },
    /// Target stem matches a Windows reserved device name.
    ReservedName { target: PathBuf, name: String },
    /// A set of ops form a rename cycle (a→b→…→a).
    Cycle { files: Vec<PathBuf> },
    /// Two ops share the same target (or target already exists on disk).
    Conflict(String),
}

// ─── Public entry ─────────────────────────────────────────────────────────────

/// Run all preflight checks on the given ops list.
/// Returns every error found; never short-circuits.
pub fn preflight_check(ops: &[RenameOp], check_existing: bool) -> Vec<PreflightError> {
    let mut errors: Vec<PreflightError> = Vec::new();

    check_illegal_chars(ops, &mut errors);
    check_reserved_names(ops, &mut errors);
    check_cycles(ops, &mut errors);
    check_conflicts(ops, check_existing, &mut errors);

    errors
}

// ─── ILLEGAL_CHAR ─────────────────────────────────────────────────────────────

const ILLEGAL: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

fn check_illegal_chars(ops: &[RenameOp], errors: &mut Vec<PreflightError>) {
    for op in ops {
        // Use the raw file_name string; fall back to full path string on Windows
        // where a colon makes Path::file_name() return only the part after ":".
        let name_os = op.to.file_name().unwrap_or(op.to.as_os_str());
        let name = name_os.to_string_lossy();

        // Also check the original target string for characters that Windows
        // path parsing would silently swallow (e.g. colon as drive separator).
        let raw = op.to.to_string_lossy();
        let raw_filename = raw.rsplit(['/', '\\']).next().unwrap_or(&raw);

        let check = if raw_filename.len() > name.len() {
            raw_filename
        } else {
            &name
        };
        let found: Vec<char> = ILLEGAL
            .iter()
            .copied()
            .filter(|&c| check.contains(c))
            .collect();
        if !found.is_empty() {
            errors.push(PreflightError::IllegalChar {
                target: op.to.clone(),
                chars: found,
            });
        }
    }
}

// ─── RESERVED_NAME ────────────────────────────────────────────────────────────

/// Windows reserved device names (case-insensitive, match stem only).
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn check_reserved_names(ops: &[RenameOp], errors: &mut Vec<PreflightError>) {
    for op in ops {
        let stem = match op.to.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_ascii_uppercase(),
            None => continue,
        };
        if RESERVED.contains(&stem.as_str()) {
            errors.push(PreflightError::ReservedName {
                target: op.to.clone(),
                name: stem,
            });
        }
    }
}

// ─── CYCLE ────────────────────────────────────────────────────────────────────

fn check_cycles(ops: &[RenameOp], errors: &mut Vec<PreflightError>) {
    // Build adjacency: from -> to (using canonical string keys)
    let graph: HashMap<String, String> = ops
        .iter()
        .map(|op| {
            (
                op.from.to_string_lossy().into_owned(),
                op.to.to_string_lossy().into_owned(),
            )
        })
        .collect();

    let mut visited: HashSet<String> = HashSet::new();
    let mut reported: HashSet<String> = HashSet::new();

    for start in graph.keys() {
        if visited.contains(start) {
            continue;
        }
        // Walk the chain from `start`, tracking the path
        let mut path: Vec<String> = Vec::new();
        let mut path_set: HashSet<String> = HashSet::new();
        let mut cur = start.clone();

        loop {
            if path_set.contains(&cur) {
                // Found a cycle — collect the cycle nodes
                let cycle_start = path.iter().position(|n| n == &cur).unwrap_or(0);
                let cycle: Vec<String> = path[cycle_start..].to_vec();
                // Only report each unique cycle once
                let mut key = cycle.clone();
                key.sort();
                let key_str = key.join("|");
                if reported.insert(key_str) {
                    errors.push(PreflightError::Cycle {
                        files: cycle.into_iter().map(PathBuf::from).collect(),
                    });
                }
                break;
            }
            visited.insert(cur.clone());
            path_set.insert(cur.clone());
            path.push(cur.clone());

            match graph.get(&cur) {
                Some(next) => cur = next.clone(),
                None => break, // chain ends, no cycle
            }
        }
    }
}

// ─── CONFLICT ─────────────────────────────────────────────────────────────────

fn check_conflicts(ops: &[RenameOp], check_existing: bool, errors: &mut Vec<PreflightError>) {
    let mut seen: HashSet<String> = HashSet::new();
    let sources: HashSet<String> = ops
        .iter()
        .map(|o| o.from.to_string_lossy().into_owned())
        .collect();

    for op in ops {
        let to_key = op.to.to_string_lossy().into_owned();
        if check_existing && op.to.exists() && !sources.contains(&to_key) {
            errors.push(PreflightError::Conflict(format!(
                "Target already exists: {}",
                op.to.display()
            )));
        }
        if !seen.insert(to_key.clone()) {
            errors.push(PreflightError::Conflict(format!(
                "Duplicate target: {}",
                op.to.display()
            )));
        }
    }
}
