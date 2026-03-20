// batch_rename/cycle_break.rs
//
// Detects rename cycles and injects temporary intermediate ops to break them.
// A cycle a→b→…→a is broken by: a→tmp, (rest of cycle unchanged), last→a.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::batch_rename::types::RenameOp;

/// Break all rename cycles by injecting temporary intermediate ops.
///
/// `existing_names`: set of filenames already present on disk (used to avoid
/// tmp name collisions).
pub fn break_cycles(ops: Vec<RenameOp>, existing: &[PathBuf]) -> Vec<RenameOp> {
    // Build from→to map (string keys for easy lookup)
    let map: HashMap<String, String> = ops
        .iter()
        .map(|o| (
            o.from.to_string_lossy().into_owned(),
            o.to.to_string_lossy().into_owned(),
        ))
        .collect();

    // Detect all cycles using DFS
    let cycles = find_all_cycles(&map);

    if cycles.is_empty() {
        return ops;
    }

    // Build a set of names to avoid for tmp (existing files + all op targets + sources)
    let mut avoid: HashSet<String> = existing
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    for op in &ops {
        avoid.insert(op.to.to_string_lossy().into_owned());
        avoid.insert(op.from.to_string_lossy().into_owned());
    }

    // Track which nodes are part of any cycle
    let cycle_nodes: HashSet<String> = cycles.iter().flat_map(|c| c.iter().cloned()).collect();

    let mut result: Vec<RenameOp> = Vec::new();

    // Non-cycle ops first (they have no ordering constraint with cycles)
    for op in &ops {
        let from_s = op.from.to_string_lossy().into_owned();
        if !cycle_nodes.contains(&from_s) {
            result.push(RenameOp { from: op.from.clone(), to: op.to.clone() });
        }
    }

    // Break each cycle by injecting a tmp node and emitting ops in safe order.
    //
    // For cycle [a, b, c] representing a→b, b→c, c→a:
    //   1. a → tmp          (save a's content; a's slot is now free)
    //   2. c → a            (fill a's slot with c's content)
    //   3. b → c            (fill c's slot with b's content)
    //   4. tmp → b          (fill b's slot with saved a content)
    //
    // General pattern:
    //   - first → tmp
    //   - cycle[n-1] → cycle[n-2], ..., cycle[2] → cycle[1]  (reverse inner pairs)
    //   - tmp → first_target (= cycle[1])
    for (cycle_idx, cycle) in cycles.iter().enumerate() {
        let first = &cycle[0];
        let tmp = gen_tmp_name(first, &avoid, cycle_idx);
        avoid.insert(tmp.clone());

        let first_target = map.get(first).cloned().unwrap_or_default();
        let n = cycle.len();

        // Step 1: first → tmp
        result.push(RenameOp {
            from: PathBuf::from(first),
            to: PathBuf::from(&tmp),
        });

        // Steps 2..n: execute remaining cycle ops in safe order.
        // Original cycle: cycle[0]→cycle[1]→...→cycle[n-1]→cycle[0]
        // After saving cycle[0] to tmp, we execute:
        //   cycle[n-1] → cycle[0]    (fills cycle[0]'s slot)
        //   cycle[n-2] → cycle[n-1]  (fills cycle[n-1]'s slot)
        //   ...until cycle[1] → cycle[2]
        // This is the original edges traversed in reverse order (last edge first).
        for k in (1..n).rev() {
            // Original edge: cycle[k-1] → cycle[k]
            // But we execute them back-to-front so each target slot is already free:
            // edge index k-1 (0-based): from=cycle[k-1], to=cycle[k]
            // Reversed traversal means we pick edge (n-1), (n-2), ..., (1):
            //   edge n-1: cycle[n-1] → cycle[0]  (using original map target)
            let from_node = &cycle[k];
            let to_node = map.get(from_node).map(|s| s.as_str()).unwrap_or("");
            result.push(RenameOp {
                from: PathBuf::from(from_node),
                to: PathBuf::from(to_node),
            });
        }

        // Final step: tmp → first_target
        result.push(RenameOp {
            from: PathBuf::from(&tmp),
            to: PathBuf::from(&first_target),
        });
    }

    result
}


// ─── Cycle detection ─────────────────────────────────────────────────────────

/// Returns each detected cycle as a Vec of node strings (the cycle path).
fn find_all_cycles(graph: &HashMap<String, String>) -> Vec<Vec<String>> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut reported: HashSet<String> = HashSet::new();
    let mut cycles: Vec<Vec<String>> = Vec::new();

    for start in graph.keys() {
        if visited.contains(start) {
            continue;
        }
        let mut path: Vec<String> = Vec::new();
        let mut path_set: HashSet<String> = HashSet::new();
        let mut cur = start.clone();

        loop {
            if path_set.contains(&cur) {
                let idx = path.iter().position(|n| n == &cur).unwrap_or(0);
                let cycle: Vec<String> = path[idx..].to_vec();
                let mut key = cycle.clone();
                key.sort();
                let key_str = key.join("|");
                if reported.insert(key_str) {
                    cycles.push(cycle);
                }
                break;
            }
            visited.insert(cur.clone());
            path_set.insert(cur.clone());
            path.push(cur.clone());
            match graph.get(&cur) {
                Some(next) => cur = next.clone(),
                None => break,
            }
        }
    }
    cycles
}

// ─── Tmp name generation ─────────────────────────────────────────────────────

fn gen_tmp_name(base: &str, avoid: &HashSet<String>, cycle_idx: usize) -> String {
    // Use cycle index for deterministic, collision-free naming.
    // Format: __xun_brn_tmp_{cycle_idx}__ with counter fallback.
    let mut candidate = format!("__xun_brn_tmp_{}__", cycle_idx);
    let mut counter = 0u32;
    while avoid.contains(&candidate) {
        counter += 1;
        candidate = format!("__xun_brn_tmp_{}_{}__", cycle_idx, counter);
    }
    // Preserve the directory part of base if present
    let base_path = PathBuf::from(base);
    if let Some(parent) = base_path.parent() {
        if parent != std::path::Path::new("") {
            return parent.join(&candidate).to_string_lossy().into_owned();
        }
    }
    candidate
}
