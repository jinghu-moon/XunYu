use std::collections::BTreeMap;

use super::events::now_iso;
use super::types::{EnvScope, EnvVar, EnvWatchEvent};

pub fn diff_vars(scope: EnvScope, prev: &[EnvVar], next: &[EnvVar]) -> Vec<EnvWatchEvent> {
    let mut prev_map = BTreeMap::<String, (&str, &str)>::new();
    for item in prev {
        prev_map.insert(
            item.name.to_ascii_uppercase(),
            (item.name.as_str(), item.raw_value.as_str()),
        );
    }
    let mut next_map = BTreeMap::<String, (&str, &str)>::new();
    for item in next {
        next_map.insert(
            item.name.to_ascii_uppercase(),
            (item.name.as_str(), item.raw_value.as_str()),
        );
    }

    let mut keys = prev_map.keys().cloned().collect::<Vec<_>>();
    for key in next_map.keys() {
        if !keys.iter().any(|k| k == key) {
            keys.push(key.clone());
        }
    }
    keys.sort();

    let at = now_iso();
    let mut out = Vec::<EnvWatchEvent>::new();
    for key in keys {
        match (prev_map.get(&key), next_map.get(&key)) {
            (None, Some((name, new_value))) => out.push(EnvWatchEvent {
                at: at.clone(),
                op: "added".to_string(),
                scope,
                name: (*name).to_string(),
                old_value: None,
                new_value: Some((*new_value).to_string()),
            }),
            (Some((name, old_value)), None) => out.push(EnvWatchEvent {
                at: at.clone(),
                op: "removed".to_string(),
                scope,
                name: (*name).to_string(),
                old_value: Some((*old_value).to_string()),
                new_value: None,
            }),
            (Some((name, old_value)), Some((_, new_value))) if old_value != new_value => {
                out.push(EnvWatchEvent {
                    at: at.clone(),
                    op: "changed".to_string(),
                    scope,
                    name: (*name).to_string(),
                    old_value: Some((*old_value).to_string()),
                    new_value: Some((*new_value).to_string()),
                });
            }
            _ => {}
        }
    }
    out
}
