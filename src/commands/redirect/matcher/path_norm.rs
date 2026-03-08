use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

pub(super) fn normalize_ext(s: &str) -> String {
    s.trim().trim_start_matches('.').to_ascii_lowercase()
}

pub(super) fn file_ext_lower(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
}

static REGEX_CACHE: OnceLock<Mutex<HashMap<String, Result<Regex, ()>>>> = OnceLock::new();

pub(super) fn regex_is_match_cached(pattern: &str, text: &str) -> bool {
    let cache = REGEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = guard.get(pattern) {
            return cached.as_ref().map(|rx| rx.is_match(text)).unwrap_or(false);
        }
    }

    let compiled = Regex::new(pattern).map_err(|_| ());
    let ok = compiled
        .as_ref()
        .map(|rx| rx.is_match(text))
        .unwrap_or(false);

    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(pattern.to_string(), compiled);
    ok
}
