use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LegacyEntry {
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) visit_count: u32,
    #[serde(default)]
    pub(crate) last_visited: u64,
}

pub(crate) fn detect_schema_version(value: &Value) -> Option<u32> {
    value
        .as_object()
        .and_then(|map| map.get("schema_version"))
        .and_then(|value| value.as_u64())
        .map(|value| value as u32)
}

pub(crate) fn parse_legacy_entries(value: Value) -> Option<Vec<(String, LegacyEntry)>> {
    if let Some(entries) = parse_legacy_root_map(&value) {
        return Some(entries);
    }
    let bookmarks = value
        .as_object()
        .and_then(|map| map.get("bookmarks"))?
        .clone();
    parse_legacy_root_map(&bookmarks)
}

fn parse_legacy_root_map(value: &Value) -> Option<Vec<(String, LegacyEntry)>> {
    let raw: BTreeMap<String, LegacyEntry> = serde_json::from_value(value.clone()).ok()?;
    (!raw.is_empty()).then(|| raw.into_iter().collect())
}
