//! 备份元数据：每次备份在备份目录根写 .bak-meta.json

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::util::dir_size;

pub(crate) const META_FILE: &str = ".bak-meta.json";

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct BackupStats {
    pub(crate) new: u32,
    pub(crate) modified: u32,
    #[serde(default)]
    pub(crate) reused: u32,
    pub(crate) deleted: u32,
    #[serde(default)]
    pub(crate) hash_checked_files: u64,
    #[serde(default)]
    pub(crate) hash_cache_hits: u64,
    #[serde(default)]
    pub(crate) hash_computed_files: u64,
    #[serde(default)]
    pub(crate) rename_only_count: u32,
    #[serde(default)]
    pub(crate) reused_bytes: u64,
    #[serde(default)]
    pub(crate) cache_hit_ratio: f64,
    #[serde(default)]
    pub(crate) baseline_source: String,
    #[serde(default)]
    pub(crate) hardlinked_files: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BackupMeta {
    /// 格式版本，目前固定为 1
    pub(crate) version: u32,
    /// 备份创建时间（Unix 秒）
    pub(crate) ts: u64,
    /// 用户描述
    pub(crate) desc: String,
    /// 用户标签
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    /// 文件统计
    pub(crate) stats: BackupStats,
    /// 是否为增量备份
    pub(crate) incremental: bool,
    /// 备份大小（目录为逻辑文件总大小，zip 为压缩后文件大小）
    #[serde(default)]
    pub(crate) size_bytes: u64,
}

/// 将元数据写入 backup_path/.bak-meta.json
pub(crate) fn write_meta(backup_path: &Path, meta: &BackupMeta) {
    if let Ok(json) = serde_json::to_string_pretty(meta) {
        let _ = fs::write(backup_path.join(META_FILE), json);
    }
}

/// 从 backup_path/.bak-meta.json 读取元数据
pub(crate) fn read_meta(backup_path: &Path) -> Option<BackupMeta> {
    let data = fs::read_to_string(backup_path.join(META_FILE)).ok()?;
    serde_json::from_str(&data).ok()
}

pub(crate) fn read_sidecar_meta(sidecar_path: &Path) -> Option<BackupMeta> {
    let data = fs::read_to_string(sidecar_path).ok()?;
    serde_json::from_str(&data).ok()
}

pub(crate) struct BackupRecord {
    pub(crate) entry_name: String,
    pub(crate) display_name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_zip: bool,
    pub(crate) mtime: u64,
    pub(crate) size_bytes: u64,
    pub(crate) meta: Option<BackupMeta>,
}

pub(crate) fn collect_backup_records(backups_root: &Path, prefix: &str) -> Vec<BackupRecord> {
    let mut records = Vec::new();

    if let Ok(rd) = fs::read_dir(backups_root) {
        for entry in rd.flatten() {
            let path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            if !entry_name.starts_with(prefix) || entry_name.ends_with(".meta.json") {
                continue;
            }

            let metadata = entry.metadata().ok();
            let mtime = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let is_zip = path.extension().is_some_and(|ext| ext == "zip");
            if !path.is_dir() && !is_zip {
                continue;
            }

            let meta = if path.is_dir() {
                read_meta(&path)
            } else {
                let stem = entry_name.strip_suffix(".zip").unwrap_or(&entry_name);
                read_sidecar_meta(&backups_root.join(format!("{stem}.meta.json")))
            };

            let size_bytes = meta
                .as_ref()
                .map(|meta| meta.size_bytes)
                .filter(|size| *size > 0)
                .unwrap_or_else(|| {
                    if path.is_dir() {
                        dir_size(&path)
                    } else {
                        metadata.as_ref().map(|m| m.len()).unwrap_or(0)
                    }
                });

            let display_name = if is_zip {
                entry_name
                    .strip_suffix(".zip")
                    .unwrap_or(&entry_name)
                    .to_string()
            } else {
                entry_name.clone()
            };

            records.push(BackupRecord {
                entry_name,
                display_name,
                path,
                is_zip,
                mtime,
                size_bytes,
                meta,
            });
        }
    }

    records.sort_by(|a, b| {
        a.mtime
            .cmp(&b.mtime)
            .then_with(|| a.entry_name.cmp(&b.entry_name))
    });
    records
}

/// 获取当前 Unix 时间戳（秒）
pub(crate) fn now_unix_secs() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
