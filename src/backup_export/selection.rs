use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::backup_export::source::SourceEntry;
use crate::util::{glob_match, normalize_glob_path};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionSpec {
    pub files: Vec<String>,
    pub globs: Vec<String>,
}

impl SelectionSpec {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.globs.is_empty()
    }

    pub fn from_inputs(
        files: &[String],
        globs: &[String],
        patterns_from: &[String],
    ) -> Result<Self, String> {
        let mut spec = Self::default();
        let mut seen_files = HashSet::new();
        let mut seen_globs = HashSet::new();

        for file in files {
            let normalized = normalize_glob_path(file);
            if !normalized.is_empty() && seen_files.insert(normalized) {
                spec.files.push(file.trim().to_string());
            }
        }
        for glob in globs {
            let normalized = normalize_glob_path(glob);
            if !normalized.is_empty() && seen_globs.insert(normalized) {
                spec.globs.push(glob.trim().to_string());
            }
        }
        for patterns_path in patterns_from {
            for pattern in read_patterns_file(Path::new(patterns_path))? {
                let normalized = normalize_glob_path(&pattern);
                if !normalized.is_empty() && seen_globs.insert(normalized) {
                    spec.globs.push(pattern);
                }
            }
        }

        Ok(spec)
    }
}

pub(crate) fn select_entries<'a>(
    entries: &'a [SourceEntry],
    spec: &SelectionSpec,
) -> Vec<&'a SourceEntry> {
    if spec.is_empty() {
        return entries.iter().collect();
    }

    let mut selected = Vec::new();
    for entry in entries {
        let normalized = entry.normalized_path();
        let file_match = spec
            .files
            .iter()
            .any(|file| normalized == normalize_glob_path(file));
        let glob_match_any = spec
            .globs
            .iter()
            .any(|pattern| glob_match(&normalize_glob_path(pattern), &normalized));
        if file_match || glob_match_any {
            selected.push(entry);
        }
    }
    selected
}

fn read_patterns_file(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read patterns file {}: {err}", path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::backup_export::source::{SourceEntry, SourceKind};

    use super::{SelectionSpec, select_entries};

    fn entry(path: &str) -> SourceEntry {
        SourceEntry {
            path: path.to_string(),
            source_path: None,
            size: 0,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::Filesystem,
        }
    }

    #[test]
    fn select_entries_returns_all_when_spec_is_empty() {
        let entries = vec![entry("src/main.rs"), entry("README.md")];
        let selected = select_entries(&entries, &SelectionSpec::default());
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_entries_matches_files_and_globs_as_union() {
        let entries = vec![
            entry("src/main.rs"),
            entry("src/lib.rs"),
            entry("README.md"),
        ];
        let selected = select_entries(
            &entries,
            &SelectionSpec {
                files: vec!["README.md".to_string()],
                globs: vec!["src/*.rs".to_string()],
            },
        );
        let paths: Vec<&str> = selected.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, vec!["src/main.rs", "src/lib.rs", "README.md"]);
    }

    #[test]
    fn selection_spec_loads_patterns_from_files_and_dedupes() {
        let dir = tempdir().unwrap();
        let patterns = dir.path().join("patterns.txt");
        fs::write(
            &patterns,
            r#"
# comment
src/*.rs
README.md
src/*.rs
"#,
        )
        .unwrap();

        let spec = SelectionSpec::from_inputs(
            &["README.md".to_string()],
            &["src/*.rs".to_string()],
            &[patterns.display().to_string()],
        )
        .unwrap();

        assert_eq!(spec.files, vec!["README.md"]);
        assert_eq!(spec.globs, vec!["src/*.rs", "README.md"]);
    }
}
