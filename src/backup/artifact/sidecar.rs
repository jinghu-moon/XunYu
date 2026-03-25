use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::reader::copy_entry_to_writer;
use crate::backup::artifact::sevenz::SevenZMethod;
use crate::backup::artifact::zip::{ZipCompressionMethod, resolve_zip_method_for_entry};
use crate::backup::common::hash::encode_hash_hex;
use crate::backup_formats::BackupArtifactFormat;
use crate::output::CliError;

pub(crate) const SIDECAR_PATH: &str = "__xunyu__/export_manifest.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SidecarSourceInfo {
    pub snapshot_id: String,
    pub source_root: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidecarPackingHint {
    Dir,
    Zip(ZipCompressionMethod),
    SevenZ(SevenZMethod),
}

#[derive(Serialize)]
struct SidecarManifest {
    format: String,
    snapshot_id: String,
    source_root: String,
    exported_at: String,
    xunyu_version: String,
    entries: Vec<SidecarEntry>,
}

#[derive(Serialize)]
struct SidecarEntry {
    path: String,
    size: u64,
    mtime_ns: u64,
    content_hash: String,
    created_time_ns: u64,
    win_attributes: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    packed_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    codec: Option<String>,
}

pub(crate) fn source_info_for_create(source_dir: &Path) -> SidecarSourceInfo {
    SidecarSourceInfo {
        snapshot_id: Uuid::new_v4().to_string(),
        source_root: source_dir.display().to_string(),
    }
}

pub(crate) fn source_info_for_convert(artifact: &Path) -> SidecarSourceInfo {
    #[cfg(feature = "xunbak")]
    {
        if artifact
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xunbak"))
            || artifact
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".xunbak.001"))
        {
            if let Ok(reader) = crate::xunbak::reader::ContainerReader::open(artifact)
                && let Ok(manifest) = reader.load_manifest()
            {
                return SidecarSourceInfo {
                    snapshot_id: manifest.snapshot_id,
                    source_root: manifest.source_root,
                };
            }
        }
    }

    SidecarSourceInfo {
        snapshot_id: Uuid::new_v4().to_string(),
        source_root: artifact.display().to_string(),
    }
}

pub(crate) fn build_sidecar_bytes(
    format: BackupArtifactFormat,
    packing_hint: SidecarPackingHint,
    source: &SidecarSourceInfo,
    entries: &[&SourceEntry],
) -> Result<Vec<u8>, CliError> {
    let mut items = Vec::with_capacity(entries.len());
    for entry in entries {
        let effective_codec = sidecar_codec_for_entry(entry, packing_hint);
        items.push(SidecarEntry {
            path: entry.path.clone(),
            size: entry.size,
            mtime_ns: entry.mtime_ns.unwrap_or(0),
            content_hash: resolve_content_hash_hex(entry)?,
            created_time_ns: entry.created_time_ns.unwrap_or(0),
            win_attributes: entry.win_attributes,
            packed_size: sidecar_packed_size_for_entry(entry.size, effective_codec.as_deref()),
            codec: effective_codec.map(str::to_string),
        });
    }

    let manifest = SidecarManifest {
        format: format.to_string(),
        snapshot_id: source.snapshot_id.clone(),
        source_root: source.source_root.clone(),
        exported_at: Utc::now().to_rfc3339(),
        xunyu_version: env!("CARGO_PKG_VERSION").to_string(),
        entries: items,
    };
    serde_json::to_vec_pretty(&manifest)
        .map_err(|err| CliError::new(1, format!("Serialize sidecar failed: {err}")))
}

pub(crate) fn write_sidecar_to_dir(
    output_dir: &Path,
    sidecar_bytes: &[u8],
) -> Result<(), CliError> {
    let sidecar_path = output_dir.join(PathBuf::from(SIDECAR_PATH.replace('/', "\\")));
    if let Some(parent) = sidecar_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Create sidecar directory failed {}: {err}",
                    parent.display()
                ),
            )
        })?;
    }
    std::fs::write(&sidecar_path, sidecar_bytes).map_err(|err| {
        CliError::new(
            1,
            format!("Write sidecar failed {}: {err}", sidecar_path.display()),
        )
    })
}

fn sidecar_codec_for_entry(
    entry: &SourceEntry,
    packing_hint: SidecarPackingHint,
) -> Option<&'static str> {
    match packing_hint {
        SidecarPackingHint::Dir => Some("copy"),
        SidecarPackingHint::SevenZ(SevenZMethod::Copy) => Some("copy"),
        SidecarPackingHint::SevenZ(SevenZMethod::Lzma2) => Some("lzma2"),
        SidecarPackingHint::SevenZ(SevenZMethod::Bzip2) => Some("bzip2"),
        SidecarPackingHint::SevenZ(SevenZMethod::Deflate) => Some("deflate"),
        SidecarPackingHint::SevenZ(SevenZMethod::Ppmd) => Some("ppmd"),
        SidecarPackingHint::SevenZ(SevenZMethod::Zstd) => Some("zstd"),
        SidecarPackingHint::Zip(method) => {
            Some(effective_zip_method_for_entry(entry, method).codec_name())
        }
    }
}

fn sidecar_packed_size_for_entry(size: u64, codec: Option<&str>) -> Option<u64> {
    match codec {
        Some("copy") | Some("stored") => Some(size),
        _ => None,
    }
}

fn effective_zip_method_for_entry(
    entry: &SourceEntry,
    method: ZipCompressionMethod,
) -> ZipCompressionMethod {
    resolve_zip_method_for_entry(entry, method)
}

fn resolve_content_hash_hex(entry: &SourceEntry) -> Result<String, CliError> {
    if let Some(hash) = entry.content_hash {
        return Ok(encode_hash_hex(&hash));
    }
    let mut sink = HashSink::default();
    copy_entry_to_writer(entry, &mut sink)?;
    Ok(encode_hash_hex(sink.hasher.finalize().as_bytes()))
}

#[derive(Default)]
struct HashSink {
    hasher: blake3::Hasher,
}

impl Write for HashSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
