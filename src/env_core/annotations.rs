use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::config::{EnvCoreConfig, config_file_path};
use super::types::{AnnotationEntry, EnvResult};

type AnnotationMap = BTreeMap<String, AnnotationEntry>;

pub fn annotations_file_path(_cfg: &EnvCoreConfig) -> PathBuf {
    config_file_path().with_file_name(".xun.env.annotations.json")
}

pub fn list_annotations(cfg: &EnvCoreConfig) -> EnvResult<Vec<AnnotationEntry>> {
    let mut entries: Vec<AnnotationEntry> = load_map(cfg)?.into_values().collect();
    entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    Ok(entries)
}

pub fn set_annotation(cfg: &EnvCoreConfig, name: &str, note: &str) -> EnvResult<AnnotationEntry> {
    let mut map = load_map(cfg)?;
    let entry = AnnotationEntry {
        name: name.to_string(),
        note: note.to_string(),
    };
    map.insert(normalize_key(name), entry.clone());
    save_map(cfg, &map)?;
    Ok(entry)
}

#[cfg(feature = "dashboard")]
pub fn get_annotation(cfg: &EnvCoreConfig, name: &str) -> EnvResult<Option<AnnotationEntry>> {
    let map = load_map(cfg)?;
    Ok(map.get(&normalize_key(name)).cloned())
}

#[cfg(feature = "dashboard")]
pub fn delete_annotation(cfg: &EnvCoreConfig, name: &str) -> EnvResult<bool> {
    let mut map = load_map(cfg)?;
    let removed = map.remove(&normalize_key(name)).is_some();
    if removed {
        save_map(cfg, &map)?;
    }
    Ok(removed)
}

fn load_map(cfg: &EnvCoreConfig) -> EnvResult<AnnotationMap> {
    let path = annotations_file_path(cfg);
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<AnnotationMap>(&content).map_err(Into::into),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AnnotationMap::new()),
        Err(e) => Err(e.into()),
    }
}

fn save_map(cfg: &EnvCoreConfig, map: &AnnotationMap) -> EnvResult<()> {
    let path = annotations_file_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(map)?)?;
    Ok(())
}

fn normalize_key(name: &str) -> String {
    name.to_ascii_uppercase()
}
