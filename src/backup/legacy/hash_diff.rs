use std::collections::HashMap;

use crate::backup::common::hash::{
    build_content_hash_groups, build_lookup_path_index, normalize_path_lookup_key,
};

use super::hash_manifest::{BackupSnapshotEntry, BackupSnapshotManifest};
use super::scan::ScannedFile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HashDiffKind {
    New,
    Modified,
    Reused,
    Unchanged,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HashDiffEntry {
    pub(crate) path: String,
    pub(crate) kind: HashDiffKind,
    pub(crate) reuse_from_path: Option<String>,
}

pub(crate) fn build_current_path_index(
    current: &HashMap<String, ScannedFile>,
) -> HashMap<String, (&str, &ScannedFile)> {
    let mut index = HashMap::with_capacity(current.len());
    for (path, scanned) in current {
        index.insert(normalize_path_lookup_key(path), (path.as_str(), scanned));
    }
    index
}

pub(crate) fn diff_against_hash_manifest(
    current: &HashMap<String, ScannedFile>,
    previous: &BackupSnapshotManifest,
) -> Vec<HashDiffEntry> {
    let previous_entries = &previous.entries;
    let path_index = build_lookup_path_index(previous_entries, |entry| entry.path.as_str());
    let content_groups = build_content_hash_groups(previous_entries, |entry| entry.content_hash);
    let mut matched_paths = std::collections::HashSet::new();
    let mut diff = Vec::new();

    let current_index = build_current_path_index(current);
    let mut current_lookup_keys: Vec<&String> = current_index.keys().collect();
    current_lookup_keys.sort();

    for lookup_key in current_lookup_keys {
        let (path, scanned) = current_index[lookup_key];
        let current_hash = scanned.content_hash.unwrap_or([0; 32]);
        if let Some(old_entry) = path_index.get(lookup_key.as_str()) {
            matched_paths.insert(old_entry.path.clone());
            diff.push(HashDiffEntry {
                path: path.to_string(),
                kind: if old_entry.content_hash == current_hash {
                    HashDiffKind::Unchanged
                } else {
                    HashDiffKind::Modified
                },
                reuse_from_path: None,
            });
            continue;
        }

        if let Some(entries) = content_groups.get(&current_hash)
            && !entries.is_empty()
        {
            diff.push(HashDiffEntry {
                path: path.to_string(),
                kind: HashDiffKind::Reused,
                reuse_from_path: Some(entries[0].path.clone()),
            });
        } else {
            diff.push(HashDiffEntry {
                path: path.to_string(),
                kind: HashDiffKind::New,
                reuse_from_path: None,
            });
        }
    }

    let mut deleted: Vec<&BackupSnapshotEntry> = previous_entries
        .iter()
        .filter(|entry| !matched_paths.contains(&entry.path))
        .collect();
    deleted.sort_by(|a, b| a.path.cmp(&b.path));
    for entry in deleted {
        diff.push(HashDiffEntry {
            path: entry.path.clone(),
            kind: HashDiffKind::Deleted,
            reuse_from_path: None,
        });
    }

    diff
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    use super::{
        HashDiffEntry, HashDiffKind, build_current_path_index, diff_against_hash_manifest,
    };
    use crate::backup::legacy::hash_manifest::{BackupSnapshotEntry, BackupSnapshotManifest};
    use crate::backup::legacy::scan::ScannedFile;

    fn manifest(entries: Vec<BackupSnapshotEntry>) -> BackupSnapshotManifest {
        BackupSnapshotManifest {
            version: 2,
            snapshot_id: "01JQ7J3N4QCG7N2T1DJ7C9ZK4Q".to_string(),
            created_at_ns: 1,
            source_root: ".".to_string(),
            file_count: entries.len() as u64,
            total_raw_bytes: entries.iter().map(|entry| entry.size).sum(),
            entries,
            removed: Vec::new(),
        }
    }

    fn scanned(path: &str, hash: [u8; 32]) -> ScannedFile {
        ScannedFile {
            path: PathBuf::from(path),
            size: 3,
            modified: SystemTime::UNIX_EPOCH,
            modified_ns: 1,
            created_time_ns: None,
            win_attributes: 0,
            file_id: None,
            content_hash: Some(hash),
        }
    }

    #[test]
    fn diff_against_hash_manifest_marks_all_kinds() {
        let previous = manifest(vec![
            BackupSnapshotEntry {
                path: "a.txt".to_string(),
                content_hash: [1; 32],
                size: 3,
                mtime_ns: 1,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
            },
            BackupSnapshotEntry {
                path: "b.txt".to_string(),
                content_hash: [2; 32],
                size: 3,
                mtime_ns: 1,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
            },
            BackupSnapshotEntry {
                path: "c.txt".to_string(),
                content_hash: [3; 32],
                size: 3,
                mtime_ns: 1,
                created_time_ns: None,
                win_attributes: 0,
                file_id: None,
            },
        ]);
        let mut current = HashMap::new();
        current.insert("a.txt".to_string(), scanned("a.txt", [1; 32]));
        current.insert("b.txt".to_string(), scanned("b.txt", [9; 32]));
        current.insert("d.txt".to_string(), scanned("d.txt", [3; 32]));
        current.insert("e.txt".to_string(), scanned("e.txt", [8; 32]));

        let diff = diff_against_hash_manifest(&current, &previous);
        assert_eq!(
            diff,
            vec![
                HashDiffEntry {
                    path: "a.txt".to_string(),
                    kind: HashDiffKind::Unchanged,
                    reuse_from_path: None,
                },
                HashDiffEntry {
                    path: "b.txt".to_string(),
                    kind: HashDiffKind::Modified,
                    reuse_from_path: None,
                },
                HashDiffEntry {
                    path: "d.txt".to_string(),
                    kind: HashDiffKind::Reused,
                    reuse_from_path: Some("c.txt".to_string()),
                },
                HashDiffEntry {
                    path: "e.txt".to_string(),
                    kind: HashDiffKind::New,
                    reuse_from_path: None,
                },
                HashDiffEntry {
                    path: "c.txt".to_string(),
                    kind: HashDiffKind::Deleted,
                    reuse_from_path: None,
                },
            ]
        );
    }

    #[test]
    fn build_current_path_index_normalizes_case_and_separators() {
        let mut current = HashMap::new();
        current.insert("Src\\ReadMe.TXT".to_string(), scanned("Src\\ReadMe.TXT", [1; 32]));

        let index = build_current_path_index(&current);
        let (path, _) = index["src/readme.txt"];
        assert_eq!(path, "Src\\ReadMe.TXT");
    }

    #[test]
    fn diff_against_hash_manifest_matches_paths_case_insensitively() {
        let previous = manifest(vec![BackupSnapshotEntry {
            path: "Src/ReadMe.TXT".to_string(),
            content_hash: [7; 32],
            size: 3,
            mtime_ns: 1,
            created_time_ns: None,
            win_attributes: 0,
            file_id: None,
        }]);
        let mut current = HashMap::new();
        current.insert(
            "src\\readme.txt".to_string(),
            scanned("src\\readme.txt", [7; 32]),
        );

        let diff = diff_against_hash_manifest(&current, &previous);
        assert_eq!(
            diff,
            vec![HashDiffEntry {
                path: "src\\readme.txt".to_string(),
                kind: HashDiffKind::Unchanged,
                reuse_from_path: None,
            }]
        );
    }
}
