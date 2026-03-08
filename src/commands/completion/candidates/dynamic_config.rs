use super::common::starts_with_ci;
use super::*;

pub(super) fn dynamic_config_keys(prefix_lower: &str) -> Vec<CompletionItem> {
    let (keys, _) = cached_config_keys_and_profiles();
    keys.iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}
