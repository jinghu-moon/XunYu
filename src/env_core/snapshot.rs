use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::config::{EnvCoreConfig, ensure_dir};
use super::registry;
use super::types::{EnvError, EnvResult, EnvScope, Snapshot, SnapshotEntry, SnapshotMeta};

pub fn snapshot_dir(cfg: &EnvCoreConfig) -> EnvResult<PathBuf> {
    let dir = cfg.snapshot_dir();
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn create_snapshot(cfg: &EnvCoreConfig, description: &str) -> EnvResult<SnapshotMeta> {
    let dir = snapshot_dir(cfg)?;
    let now = Utc::now();
    let id = now.format("%Y%m%d-%H%M%S").to_string();
    let created_at = now.to_rfc3339();

    let user_vars = registry::list_scope(EnvScope::User)?
        .into_iter()
        .map(|v| SnapshotEntry {
            name: v.name,
            raw_value: v.raw_value,
            reg_type: v.reg_type,
        })
        .collect();

    let system_vars = match registry::list_scope(EnvScope::System) {
        Ok(vars) => vars
            .into_iter()
            .map(|v| SnapshotEntry {
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            })
            .collect(),
        Err(EnvError::PermissionDenied(_)) => Vec::new(),
        Err(e) => return Err(e),
    };

    let snapshot = Snapshot {
        id: id.clone(),
        description: description.to_string(),
        created_at: created_at.clone(),
        user_vars,
        system_vars,
    };

    let path = dir.join(format!("{id}.json"));
    let body = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&path, body)?;

    prune_old_snapshots(cfg, &dir)?;

    Ok(SnapshotMeta {
        id,
        description: description.to_string(),
        created_at,
        path,
    })
}

pub fn list_snapshots(cfg: &EnvCoreConfig) -> EnvResult<Vec<SnapshotMeta>> {
    let dir = snapshot_dir(cfg)?;
    let mut metas = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if !is_snapshot_file(&path) {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let snap: Snapshot = serde_json::from_str(&raw)?;
        metas.push(SnapshotMeta {
            id: snap.id,
            description: snap.description,
            created_at: snap.created_at,
            path,
        });
    }
    metas.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(metas)
}

pub fn prune_snapshots(cfg: &EnvCoreConfig, keep: usize) -> EnvResult<usize> {
    let dir = snapshot_dir(cfg)?;
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| is_snapshot_file(p))
        .collect();
    files.sort();
    let remove_count = files.len().saturating_sub(keep);
    let mut removed = 0usize;
    for path in files.into_iter().take(remove_count) {
        fs::remove_file(path)?;
        removed += 1;
    }
    Ok(removed)
}

pub fn load_snapshot_by_id(cfg: &EnvCoreConfig, id: &str) -> EnvResult<Snapshot> {
    let dir = snapshot_dir(cfg)?;
    let path = dir.join(format!("{id}.json"));
    if !path.exists() {
        return Err(EnvError::NotFound(format!("snapshot '{}' not found", id)));
    }
    load_snapshot_from_path(&path)
}

pub fn load_latest_snapshot(cfg: &EnvCoreConfig) -> EnvResult<Snapshot> {
    let metas = list_snapshots(cfg)?;
    let Some(meta) = metas.first() else {
        return Err(EnvError::NotFound("no snapshots found".to_string()));
    };
    load_snapshot_from_path(&meta.path)
}

pub fn restore_snapshot(snapshot: &Snapshot, scope: EnvScope) -> EnvResult<()> {
    match scope {
        EnvScope::User => registry::replace_scope(EnvScope::User, &snapshot.user_vars),
        EnvScope::System => registry::replace_scope(EnvScope::System, &snapshot.system_vars),
        EnvScope::All => {
            registry::replace_scope(EnvScope::User, &snapshot.user_vars)?;
            if !snapshot.system_vars.is_empty() {
                registry::replace_scope(EnvScope::System, &snapshot.system_vars)?;
            }
            Ok(())
        }
    }
}

fn load_snapshot_from_path(path: &Path) -> EnvResult<Snapshot> {
    let raw = fs::read_to_string(path)?;
    let snapshot: Snapshot = serde_json::from_str(&raw)?;
    Ok(snapshot)
}

fn prune_old_snapshots(cfg: &EnvCoreConfig, dir: &Path) -> EnvResult<()> {
    let _ = dir;
    let _ = prune_snapshots(cfg, cfg.max_snapshots)?;
    Ok(())
}

fn is_snapshot_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}
