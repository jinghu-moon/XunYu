//! 备份元数据：每次备份在备份目录根写 .bak-meta.json

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub(crate) const META_FILE: &str = ".bak-meta.json";

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct BakStats {
    pub(crate) new: u32,
    pub(crate) modified: u32,
    pub(crate) deleted: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BakMeta {
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
    pub(crate) stats: BakStats,
    /// 是否为增量备份
    pub(crate) incremental: bool,
}

/// 将元数据写入 backup_path/.bak-meta.json
pub(crate) fn write_meta(backup_path: &Path, meta: &BakMeta) {
    if let Ok(json) = serde_json::to_string_pretty(meta) {
        let _ = fs::write(backup_path.join(META_FILE), json);
    }
}

/// 从 backup_path/.bak-meta.json 读取元数据
pub(crate) fn read_meta(backup_path: &Path) -> Option<BakMeta> {
    let data = fs::read_to_string(backup_path.join(META_FILE)).ok()?;
    serde_json::from_str(&data).ok()
}

/// 获取当前 Unix 时间戳（秒）
pub(crate) fn now_unix_secs() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
