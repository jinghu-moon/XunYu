use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::backup::common::hash::{decode_hash_hex, encode_hash_hex};

pub(crate) const HASH_CACHE_FILE: &str = ".xun-bak-hash-cache.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HashCacheDocument {
    #[serde(default)]
    pub(crate) files: HashMap<String, HashCacheEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HashCacheEntry {
    pub(crate) size: u64,
    pub(crate) mtime_ns: u64,
    pub(crate) created_time_ns: Option<u64>,
    pub(crate) win_attributes: u32,
    pub(crate) file_id: Option<String>,
    #[serde(
        serialize_with = "serialize_hash32",
        deserialize_with = "deserialize_hash32"
    )]
    pub(crate) content_hash: [u8; 32],
}

pub(crate) fn load_hash_cache(root: &Path) -> HashCacheDocument {
    let path = root.join(HASH_CACHE_FILE);
    let Ok(bytes) = fs::read(&path) else {
        return HashCacheDocument {
            files: HashMap::new(),
        };
    };
    serde_json::from_slice(&bytes).unwrap_or_else(|_| HashCacheDocument {
        files: HashMap::new(),
    })
}

pub(crate) fn save_hash_cache(root: &Path, cache: &HashCacheDocument) {
    let path = root.join(HASH_CACHE_FILE);
    if let Ok(json) = serde_json::to_vec_pretty(cache) {
        let _ = fs::write(path, json);
    }
}

pub(crate) fn cache_hit(
    cache: &HashCacheDocument,
    rel: &str,
    size: u64,
    mtime_ns: u64,
    created_time_ns: Option<u64>,
    win_attributes: u32,
    file_id: Option<&str>,
) -> Option<[u8; 32]> {
    let metadata_matches = |entry: &HashCacheEntry| {
        entry.size == size
            && entry.mtime_ns == mtime_ns
            && entry.created_time_ns == created_time_ns
            && entry.win_attributes == win_attributes
            && entry.file_id.as_deref() == file_id
    };

    if let Some(entry) = cache.files.get(rel) {
        if metadata_matches(entry) {
            return Some(entry.content_hash);
        }
        return None;
    }

    let file_id = file_id?;
    cache.files.values().find_map(|entry| {
        if metadata_matches(entry) && entry.file_id.as_deref() == Some(file_id) {
            Some(entry.content_hash)
        } else {
            None
        }
    })
}

pub(crate) fn update_cache_entry(
    cache: &mut HashCacheDocument,
    rel: String,
    entry: HashCacheEntry,
) {
    cache.files.insert(rel, entry);
}

fn serialize_hash32<S>(value: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&encode_hash_hex(value))
}

fn deserialize_hash32<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    decode_hash_hex(&value).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        HASH_CACHE_FILE, HashCacheDocument, HashCacheEntry, cache_hit, load_hash_cache,
        save_hash_cache, update_cache_entry,
    };

    #[test]
    fn hash_cache_entry_json_roundtrips() {
        let entry = HashCacheEntry {
            size: 12,
            mtime_ns: 34,
            created_time_ns: Some(56),
            win_attributes: 32,
            file_id: Some("file-1".to_string()),
            content_hash: [0xab; 32],
        };
        let json = serde_json::to_vec(&entry).unwrap();
        let roundtrip: HashCacheEntry = serde_json::from_slice(&json).unwrap();
        assert_eq!(roundtrip, entry);
    }

    #[test]
    fn cache_hit_returns_hash_when_metadata_matches() {
        let mut cache = HashCacheDocument {
            files: std::collections::HashMap::new(),
        };
        update_cache_entry(
            &mut cache,
            "a.txt".to_string(),
            HashCacheEntry {
                size: 1,
                mtime_ns: 2,
                created_time_ns: Some(3),
                win_attributes: 32,
                file_id: Some("id-1".to_string()),
                content_hash: [7; 32],
            },
        );
        assert_eq!(
            cache_hit(&cache, "a.txt", 1, 2, Some(3), 32, Some("id-1")),
            Some([7; 32])
        );
    }

    #[test]
    fn cache_hit_returns_none_when_metadata_differs() {
        let mut cache = HashCacheDocument {
            files: std::collections::HashMap::new(),
        };
        update_cache_entry(
            &mut cache,
            "a.txt".to_string(),
            HashCacheEntry {
                size: 1,
                mtime_ns: 2,
                created_time_ns: Some(3),
                win_attributes: 32,
                file_id: None,
                content_hash: [7; 32],
            },
        );
        assert_eq!(cache_hit(&cache, "a.txt", 1, 99, Some(3), 32, None), None);
    }

    #[test]
    fn cache_hit_returns_hash_for_renamed_path_when_file_id_matches() {
        let mut cache = HashCacheDocument {
            files: std::collections::HashMap::new(),
        };
        update_cache_entry(
            &mut cache,
            "old.txt".to_string(),
            HashCacheEntry {
                size: 1,
                mtime_ns: 2,
                created_time_ns: Some(3),
                win_attributes: 32,
                file_id: Some("id-1".to_string()),
                content_hash: [9; 32],
            },
        );
        assert_eq!(
            cache_hit(&cache, "new.txt", 1, 2, Some(3), 32, Some("id-1")),
            Some([9; 32])
        );
    }

    #[test]
    fn cache_hit_returns_none_when_file_id_changes_for_same_path() {
        let mut cache = HashCacheDocument {
            files: std::collections::HashMap::new(),
        };
        update_cache_entry(
            &mut cache,
            "a.txt".to_string(),
            HashCacheEntry {
                size: 1,
                mtime_ns: 2,
                created_time_ns: Some(3),
                win_attributes: 32,
                file_id: Some("id-1".to_string()),
                content_hash: [7; 32],
            },
        );
        assert_eq!(
            cache_hit(&cache, "a.txt", 1, 2, Some(3), 32, Some("id-2")),
            None
        );
    }

    #[test]
    fn load_hash_cache_returns_empty_when_file_is_corrupted() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(HASH_CACHE_FILE), b"{not-json").unwrap();
        let cache = load_hash_cache(dir.path());
        assert!(cache.files.is_empty());
    }

    #[test]
    fn save_and_load_hash_cache_roundtrip() {
        let dir = tempdir().unwrap();
        let mut cache = HashCacheDocument {
            files: std::collections::HashMap::new(),
        };
        update_cache_entry(
            &mut cache,
            "a.txt".to_string(),
            HashCacheEntry {
                size: 1,
                mtime_ns: 2,
                created_time_ns: Some(3),
                win_attributes: 32,
                file_id: Some("id-1".to_string()),
                content_hash: [8; 32],
            },
        );
        save_hash_cache(dir.path(), &cache);
        let loaded = load_hash_cache(dir.path());
        assert_eq!(loaded, cache);
    }
}
