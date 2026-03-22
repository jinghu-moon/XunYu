use std::io::Cursor;

use serde_json::json;
use ulid::Ulid;
use xun::xunbak::constants::Codec;
use xun::xunbak::manifest::{
    ManifestBody, ManifestCodec, ManifestEntry, ManifestError, ManifestPrefix, ManifestReadResult,
    ManifestType, detect_case_conflicts, filetime_to_unix_ns, normalize_path, read_manifest_record,
    unix_ns_to_filetime, write_manifest_record,
};

fn sample_entry() -> ManifestEntry {
    ManifestEntry {
        path: "src/main.rs".to_string(),
        blob_id: [0x11; 32],
        content_hash: [0x22; 32],
        size: 1234,
        mtime_ns: 1_700_000_000_123_456_789,
        created_time_ns: 1_700_000_000_223_456_789,
        win_attributes: 0x21,
        codec: Codec::NONE,
        blob_offset: 64,
        blob_len: 77,
        volume_index: 0,
        parts: None,
        ext: None,
    }
}

fn sample_manifest_body() -> ManifestBody {
    ManifestBody {
        snapshot_id: Ulid::new().to_string(),
        base_snapshot_id: None,
        created_at: 1_700_000_000,
        source_root: "D:/repo".to_string(),
        snapshot_context: json!({"hostname":"devbox","xunyu_version":"0.1.0"}),
        file_count: 1,
        total_raw_bytes: 1234,
        entries: vec![sample_entry()],
        removed: vec![],
    }
}

#[test]
fn manifest_prefix_layout_roundtrips() {
    let prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    let bytes = prefix.to_bytes();
    assert_eq!(bytes, [0x00, 0x00, 0x01, 0x00]);
    assert_eq!(ManifestPrefix::from_bytes(&bytes).unwrap(), prefix);
}

#[test]
fn manifest_body_json_roundtrips() {
    let body = sample_manifest_body();
    let raw = serde_json::to_vec(&body).unwrap();
    let decoded: ManifestBody = serde_json::from_slice(&raw).unwrap();
    assert_eq!(decoded, body);
}

#[test]
fn manifest_entry_contains_required_fields() {
    let value = serde_json::to_value(sample_entry()).unwrap();
    for field in [
        "path",
        "blob_id",
        "content_hash",
        "size",
        "mtime_ns",
        "created_time_ns",
        "win_attributes",
        "codec",
        "blob_offset",
        "blob_len",
        "volume_index",
    ] {
        assert!(value.get(field).is_some(), "missing field {field}");
    }
}

#[test]
fn ext_is_omitted_when_none() {
    let value = serde_json::to_value(sample_entry()).unwrap();
    assert!(value.get("ext").is_none());
}

#[test]
fn parts_is_omitted_when_none() {
    let value = serde_json::to_value(sample_entry()).unwrap();
    assert!(value.get("parts").is_none());
}

#[test]
fn removed_serializes_as_empty_array() {
    let value = serde_json::to_value(sample_manifest_body()).unwrap();
    assert_eq!(value.get("removed").unwrap(), &json!([]));
}

#[test]
fn blob_id_serializes_as_hex() {
    let value = serde_json::to_value(sample_entry()).unwrap();
    let blob_id = value.get("blob_id").unwrap().as_str().unwrap();
    assert_eq!(blob_id.len(), 64);
    assert!(blob_id.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn snapshot_id_serializes_as_ulid_string() {
    let value = serde_json::to_value(sample_manifest_body()).unwrap();
    let snapshot_id = value.get("snapshot_id").unwrap().as_str().unwrap();
    assert_eq!(snapshot_id.len(), 26);
    assert!(Ulid::from_string(snapshot_id).is_ok());
}

#[test]
fn manifest_record_write_sets_prefix_and_crc() {
    let mut out = Vec::new();
    let prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    let body = sample_manifest_body();
    let result = write_manifest_record(&mut out, prefix, &body).unwrap();
    let record_prefix = xun::xunbak::record::RecordPrefix::from_bytes(&out[..13]).unwrap();
    assert_eq!(
        record_prefix.record_type,
        xun::xunbak::constants::RecordType::MANIFEST
    );
    assert_eq!(record_prefix.record_len, (4 + result.body_len) as u64);
}

#[test]
fn manifest_record_read_roundtrips() {
    let mut out = Vec::new();
    let prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    let body = sample_manifest_body();
    write_manifest_record(&mut out, prefix, &body).unwrap();
    let read = read_manifest_record(&mut Cursor::new(out)).unwrap();
    assert_eq!(
        read,
        ManifestReadResult {
            prefix,
            body,
            record_len: read.record_len,
        }
    );
}

#[test]
fn unsupported_manifest_codec_is_rejected() {
    let mut out = Vec::new();
    let prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::MSGPACK,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    assert!(matches!(
        write_manifest_record(&mut out, prefix, &sample_manifest_body()),
        Err(ManifestError::UnsupportedManifestCodec(0x01))
    ));
}

#[test]
fn manifest_crc_mismatch_is_rejected() {
    let mut out = Vec::new();
    let prefix = ManifestPrefix {
        manifest_codec: ManifestCodec::JSON,
        manifest_type: ManifestType::FULL,
        manifest_version: 1,
    };
    write_manifest_record(&mut out, prefix, &sample_manifest_body()).unwrap();
    out[9] ^= 0xFF;
    assert!(matches!(
        read_manifest_record(&mut Cursor::new(out)),
        Err(ManifestError::ManifestCrcMismatch)
    ));
}

#[test]
fn normalize_path_rewrites_windows_separators() {
    assert_eq!(
        normalize_path(r"C:\Users\foo\bar").unwrap(),
        "Users/foo/bar"
    );
}

#[test]
fn detect_case_conflicts_reports_duplicates() {
    let paths = vec!["Foo/Bar.txt".to_string(), "foo/bar.txt".to_string()];
    assert!(matches!(
        detect_case_conflicts(&paths),
        Err(ManifestError::PathCaseConflict(_))
    ));
}

#[test]
fn empty_path_is_rejected() {
    assert_eq!(normalize_path("   "), Err(ManifestError::EmptyPath));
}

#[test]
fn filetime_roundtrip_matches_known_timestamp() {
    let unix_ns = 1_767_225_600_000_000_000i128; // 2026-01-01 00:00:00 UTC
    let filetime = unix_ns_to_filetime(unix_ns);
    assert_eq!(filetime_to_unix_ns(filetime), unix_ns);
}

#[test]
fn zero_filetime_maps_before_unix_epoch() {
    assert!(filetime_to_unix_ns(0) < 0);
}
