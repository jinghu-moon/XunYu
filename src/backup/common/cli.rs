use std::path::{Path, PathBuf};

use crate::output::CliError;

pub(crate) fn path_display(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn optional_path_display(path: Option<&Path>) -> Option<String> {
    path.map(path_display)
}

pub(crate) fn path_strings<I, P>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    paths
        .into_iter()
        .map(|path| path_display(path.as_ref()))
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn resolve_input_path(root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub(crate) fn backup_named_artifact_path(backups_root: &Path, name: &str) -> Option<PathBuf> {
    let candidate = backups_root.join(name);
    if candidate.is_dir() || candidate.is_file() {
        return Some(candidate);
    }
    let zip = backups_root.join(format!("{name}.zip"));
    if zip.is_file() {
        return Some(zip);
    }
    let sevenz = backups_root.join(format!("{name}.7z"));
    if sevenz.is_file() {
        return Some(sevenz);
    }
    None
}

pub(crate) fn backup_not_found_error(name_or_path: &str) -> CliError {
    CliError::with_details(
        2,
        format!("Backup not found: {name_or_path}"),
        &[
            "Fix: Run `xun backup list` to see available backups.",
            "Fix: Pass a direct path to a backup dir, .zip, .7z, or .xunbak file.",
        ],
    )
}

pub(crate) fn unsafe_restore_path_error(file: &str) -> CliError {
    CliError::with_details(
        2,
        format!("Unsafe restore path: {file}"),
        &["Fix: Use a relative path without '..' components."],
    )
}

pub(crate) fn restore_internal_files_error() -> CliError {
    CliError::new(
        1,
        "Restore failed: backup internal files cannot be restored.",
    )
}

pub(crate) fn restore_path_not_found_message(path: &str) -> String {
    format!("Restore failed: file not found in backup: {path}")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{
        backup_named_artifact_path, path_display, path_strings, resolve_input_path,
        restore_internal_files_error, restore_path_not_found_message,
    };

    #[test]
    fn resolve_input_path_joins_relative_and_preserves_absolute() {
        let root = Path::new("D:/root");
        assert_eq!(resolve_input_path(root, "a/b.txt"), root.join("a/b.txt"));
        assert_eq!(
            resolve_input_path(root, "D:/abs/c.txt"),
            Path::new("D:/abs/c.txt")
        );
    }

    #[test]
    fn backup_named_artifact_path_matches_existing_layout() {
        let dir = tempdir().unwrap();
        let backups_root = dir.path();
        fs::create_dir_all(backups_root.join("v1-test")).unwrap();
        fs::write(backups_root.join("v2-test.zip"), "zip").unwrap();
        fs::write(backups_root.join("v3-test.7z"), "7z").unwrap();

        assert_eq!(
            backup_named_artifact_path(backups_root, "v1-test"),
            Some(backups_root.join("v1-test"))
        );
        assert_eq!(
            backup_named_artifact_path(backups_root, "v2-test"),
            Some(backups_root.join("v2-test.zip"))
        );
        assert_eq!(
            backup_named_artifact_path(backups_root, "v3-test"),
            Some(backups_root.join("v3-test.7z"))
        );
        assert_eq!(backup_named_artifact_path(backups_root, "missing"), None);
    }

    #[test]
    fn restore_message_helpers_keep_expected_text() {
        assert_eq!(
            restore_internal_files_error().message,
            "Restore failed: backup internal files cannot be restored."
        );
        assert_eq!(
            restore_path_not_found_message("a.txt"),
            "Restore failed: file not found in backup: a.txt"
        );
    }

    #[test]
    fn path_helpers_render_expected_strings() {
        assert_eq!(path_display(Path::new("a/b.txt")), "a/b.txt");
        assert_eq!(
            path_strings([Path::new("a.txt"), Path::new("b.txt")]),
            vec!["a.txt".to_string(), "b.txt".to_string()]
        );
    }
}
