use super::common::starts_with_ci;
use super::*;

#[cfg(feature = "redirect")]
pub(super) fn dynamic_profiles(prefix_lower: &str) -> Vec<CompletionItem> {
    let (_, profiles) = cached_config_keys_and_profiles();
    profiles
        .iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}

pub(super) fn dynamic_ctx_profiles(prefix_lower: &str) -> Vec<CompletionItem> {
    let profiles = cached_ctx_profiles();
    profiles
        .iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}

#[cfg(not(feature = "redirect"))]
pub(super) fn dynamic_profiles(_prefix_lower: &str) -> Vec<CompletionItem> {
    Vec::new()
}

#[cfg(feature = "redirect")]
pub(super) fn dynamic_txs(prefix_lower: &str) -> Vec<CompletionItem> {
    let txs = cached_audit_txs();
    txs.iter()
        .filter(|tx| starts_with_ci(tx, prefix_lower))
        .map(|tx| CompletionItem::new(tx.clone()))
        .collect()
}

#[cfg(not(feature = "redirect"))]
pub(super) fn dynamic_txs(_prefix_lower: &str) -> Vec<CompletionItem> {
    Vec::new()
}
