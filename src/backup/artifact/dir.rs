use std::path::Path;

use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::reader::copy_entry_to_path;
use crate::output::CliError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
}

pub(crate) fn write_entries_to_dir<P: AsRef<Path>>(
    entries: &[&SourceEntry],
    output_dir: P,
) -> Result<DirWriteSummary, CliError> {
    let output_dir = output_dir.as_ref();
    let mut bytes_in = 0u64;
    for entry in entries {
        let dest = output_dir.join(entry.path.replace('/', "\\"));
        copy_entry_to_path(entry, &dest)?;
        bytes_in += entry.size;
    }
    Ok(DirWriteSummary {
        entry_count: entries.len(),
        bytes_in,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};

    use super::write_entries_to_dir;

    #[test]
    fn dir_writer_writes_entries_into_output_tree() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("src").join("main.rs");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "fn main() {}").unwrap();

        let entry = SourceEntry {
            path: "src/main.rs".to_string(),
            source_path: Some(source),
            size: 12,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: None,
            kind: SourceKind::DirArtifact,
        };
        let output = dir.path().join("out");

        let summary = write_entries_to_dir(&[&entry], &output).unwrap();

        assert_eq!(summary.entry_count, 1);
        assert_eq!(summary.bytes_in, 12);
        assert_eq!(
            fs::read_to_string(output.join("src").join("main.rs")).unwrap(),
            "fn main() {}"
        );
    }
}
