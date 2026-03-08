use super::common::starts_with_ci;
use super::*;

pub(super) fn bookmark_candidates(prefix_lower: &str, cwd: Option<&str>) -> Vec<CompletionItem> {
    let db = cached_db();
    let mut scored: Vec<(f64, String)> = Vec::new();
    for (name, entry) in db.iter() {
        if !starts_with_ci(name, prefix_lower) {
            continue;
        }
        let mut score = frecency(entry);
        if let Some(cwd) = cwd {
            score *= cwd_boost(cwd, &entry.path);
        }
        scored.push((score, name.clone()));
    }
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    scored
        .into_iter()
        .map(|(_, name)| CompletionItem::new(name))
        .collect()
}
