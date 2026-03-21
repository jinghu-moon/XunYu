//! blake3 完整性校验：生成/验证 .bak-manifest.json
//! 仅在 feature="bak" 时启用 blake3 哈希；否则跳过校验。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub(crate) const MANIFEST_FILE: &str = ".bak-manifest.json";

#[derive(Serialize, Deserialize)]
struct Manifest {
    files: HashMap<String, String>, // rel_path → hex
}

pub(crate) enum VerifyResult {
    Ok,
    /// 损坏的文件列表
    Corrupted(Vec<String>),
    /// manifest 不存在
    NoManifest,
}

/// 计算单个文件的 blake3 哈希；失败返回 None
#[cfg(feature = "bak")]
pub(crate) fn file_blake3(path: &Path) -> Option<[u8; 32]> {
    let data = fs::read(path).ok()?;
    Some(*blake3::hash(&data).as_bytes())
}

#[cfg(not(feature = "bak"))]
pub(crate) fn file_blake3(_path: &Path) -> Option<[u8; 32]> {
    None
}

/// 将文件哈希写入 backup_path/.bak-manifest.json
#[allow(dead_code)]
pub(crate) fn write_manifest(backup_path: &Path, files: &HashMap<String, [u8; 32]>) {
    let hex_map: HashMap<String, String> = files
        .iter()
        .map(|(k, v)| (k.clone(), hex_encode(v)))
        .collect();
    let manifest = Manifest { files: hex_map };
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(backup_path.join(MANIFEST_FILE), json);
    }
}

/// 校验 backup_path 中所有文件是否与 manifest 一致
pub(crate) fn verify_manifest(backup_path: &Path) -> VerifyResult {
    let mf_path = backup_path.join(MANIFEST_FILE);
    let Ok(data) = fs::read_to_string(&mf_path) else {
        return VerifyResult::NoManifest;
    };
    let Ok(manifest) = serde_json::from_str::<Manifest>(&data) else {
        return VerifyResult::NoManifest;
    };

    let mut corrupted: Vec<String> = Vec::new();
    for (rel, expected_hex) in &manifest.files {
        let file_path = backup_path.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        match file_blake3(&file_path) {
            Some(hash) if hex_encode(&hash) == *expected_hex => {}
            _ => corrupted.push(rel.clone()),
        }
    }

    if corrupted.is_empty() {
        VerifyResult::Ok
    } else {
        VerifyResult::Corrupted(corrupted)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
