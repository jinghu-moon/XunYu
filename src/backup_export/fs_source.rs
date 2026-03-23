use std::fs;
use std::path::Path;

use crate::backup_export::source::{
    SourceEntry, SourceKind, file_attributes, metadata_created_time_ns, system_time_to_unix_ns,
};
use crate::commands::backup::scan;

pub(crate) fn scan_source_entries(
    root: &Path,
    includes: &[String],
    exclude_patterns: &[String],
    include_patterns: &[String],
) -> Vec<SourceEntry> {
    let mut entries: Vec<SourceEntry> =
        scan::scan_files(root, includes, exclude_patterns, include_patterns)
            .into_iter()
            .map(|(rel, scanned)| {
                let metadata = fs::metadata(&scanned.path).ok();
                SourceEntry {
                    path: rel.replace('\\', "/"),
                    source_path: Some(scanned.path.clone()),
                    size: scanned.size,
                    mtime_ns: Some(system_time_to_unix_ns(scanned.modified)),
                    created_time_ns: metadata.as_ref().and_then(metadata_created_time_ns),
                    win_attributes: metadata.as_ref().map(file_attributes).unwrap_or_default(),
                    content_hash: None,
                    kind: SourceKind::Filesystem,
                }
            })
            .collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::scan_source_entries;

    #[test]
    fn scan_source_entries_collects_files_with_normalized_paths() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("README.md"), "readme").unwrap();

        let entries = scan_source_entries(root, &[], &[], &[]);
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, vec!["README.md", "src/main.rs"]);
        assert!(entries.iter().all(|entry| entry.source_path.is_some()));
    }

    #[test]
    fn scan_source_entries_reuses_filtered_scan_rules() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("keep.rs"), "keep").unwrap();
        fs::write(root.join("src").join("skip.log"), "skip").unwrap();

        let include_patterns = vec!["src/*.rs".to_string()];
        let exclude_patterns = vec!["*.log".to_string()];
        let entries = scan_source_entries(root, &[], &exclude_patterns, &include_patterns);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "src/keep.rs");
    }
}
