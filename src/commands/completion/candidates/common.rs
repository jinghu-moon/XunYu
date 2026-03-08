use super::*;

pub(super) fn static_candidates(list: &[&str], prefix_lower: &str) -> Vec<CompletionItem> {
    let mut out: Vec<CompletionItem> = list
        .iter()
        .filter(|s| starts_with_ci(s, prefix_lower))
        .map(|s| CompletionItem::new((*s).to_string()))
        .collect();
    out.sort_by(|a, b| {
        a.value
            .to_ascii_lowercase()
            .cmp(&b.value.to_ascii_lowercase())
    });
    out
}

pub(super) fn starts_with_ci(value: &str, prefix_lower: &str) -> bool {
    if prefix_lower.is_empty() {
        return true;
    }
    value.to_ascii_lowercase().starts_with(prefix_lower)
}
