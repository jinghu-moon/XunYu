use std::fs;
use std::path::{Path, PathBuf};

use dialoguer::Confirm;

use crate::backup_formats::{BackupArtifactFormat, OverwriteMode};
use crate::output::{CliError, CliResult, can_interact};

pub(crate) fn parse_split_size_bytes(raw: Option<&str>) -> Result<Option<u64>, CliError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let upper = value.to_ascii_uppercase();
    let (number, multiplier) = if let Some(stripped) = upper.strip_suffix('K') {
        (stripped, 1024u64)
    } else if let Some(stripped) = upper.strip_suffix('M') {
        (stripped, 1024u64 * 1024)
    } else if let Some(stripped) = upper.strip_suffix('G') {
        (stripped, 1024u64 * 1024 * 1024)
    } else {
        (upper.as_str(), 1u64)
    };
    let size = number
        .parse::<u64>()
        .map_err(|_| CliError::new(2, format!("Invalid split size: {raw}")))?;
    Ok(Some(size.saturating_mul(multiplier)))
}

pub(crate) fn paths_equal(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

pub(crate) fn collect_file_or_numbered_outputs(output: &Path) -> Vec<PathBuf> {
    if output.exists() {
        return vec![output.to_path_buf()];
    }
    collect_numbered_outputs(output)
}

fn collect_numbered_outputs(output: &Path) -> Vec<PathBuf> {
    let mut outputs = Vec::new();
    if let Some(parent) = output.parent()
        && let Some(prefix) = output.file_name().and_then(|name| name.to_str())
        && let Ok(read_dir) = fs::read_dir(parent)
    {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("{prefix}."))
                && name[prefix.len() + 1..]
                    .chars()
                    .all(|ch| ch.is_ascii_digit())
            {
                outputs.push(entry.path());
            }
        }
    }
    outputs.sort();
    outputs
}

pub(crate) fn is_zip_artifact_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

pub(crate) fn is_7z_artifact_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("7z"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".7z.001"))
}

pub(crate) fn is_xunbak_artifact_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xunbak"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".xunbak.001"))
}

pub(crate) fn collect_artifact_output_paths(
    format: BackupArtifactFormat,
    output: &Path,
) -> Vec<PathBuf> {
    match format {
        BackupArtifactFormat::Dir | BackupArtifactFormat::Zip => {
            if output.exists() {
                vec![output.to_path_buf()]
            } else {
                Vec::new()
            }
        }
        BackupArtifactFormat::SevenZ => collect_file_or_numbered_outputs(output),
        BackupArtifactFormat::Xunbak => {
            let mut outputs = Vec::new();
            if output.exists() {
                outputs.push(output.to_path_buf());
            }
            outputs.extend(collect_numbered_outputs(output));
            outputs.sort();
            outputs.dedup();
            outputs
        }
    }
}

pub(crate) fn compute_artifact_output_bytes(format: BackupArtifactFormat, output: &Path) -> u64 {
    match format {
        BackupArtifactFormat::Dir => dir_size_bytes(output),
        _ => collect_artifact_output_paths(format, output)
            .into_iter()
            .filter_map(|path| fs::metadata(path).ok().map(|meta| meta.len()))
            .sum(),
    }
}

pub(crate) fn throughput_bytes_per_sec(bytes: u64, elapsed: std::time::Duration) -> u64 {
    let millis = elapsed.as_millis();
    if millis == 0 {
        return bytes;
    }
    ((bytes as u128 * 1000) / millis) as u64
}

pub(crate) fn maybe_fail_after_write_for_tests() -> CliResult {
    if std::env::var_os("XUN_TEST_FAIL_AFTER_WRITE").is_none() {
        return Ok(());
    }
    Err(CliError::with_details(
        1,
        "simulated export failure after write",
        &["Fix: Retry the export; resume is not supported yet."],
    ))
}

pub(crate) fn dir_size_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(read_dir) = fs::read_dir(root) else {
        return 0;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += dir_size_bytes(&path);
        } else if let Ok(meta) = entry.metadata() {
            total += meta.len();
        }
    }
    total
}

pub(crate) fn resolve_effective_overwrite(
    output: &Path,
    overwrite: OverwriteMode,
    output_kind: &str,
) -> CliResult<OverwriteMode> {
    resolve_effective_overwrite_with(output, overwrite, output_kind, can_interact(), |prompt| {
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false)
    })
}

fn resolve_effective_overwrite_with<F>(
    output: &Path,
    overwrite: OverwriteMode,
    output_kind: &str,
    interactive: bool,
    confirm: F,
) -> CliResult<OverwriteMode>
where
    F: FnOnce(String) -> bool,
{
    if !output.exists() {
        return Ok(overwrite);
    }
    match overwrite {
        OverwriteMode::Fail => Err(CliError::with_details(
            2,
            format!("backup convert output already exists: {}", output.display()),
            &["Fix: Remove the destination, or pass `--overwrite replace`."],
        )),
        OverwriteMode::Ask => {
            if !interactive {
                return Err(CliError::with_details(
                    2,
                    format!(
                        "backup convert output already exists and cannot prompt: {}",
                        output.display()
                    ),
                    &[
                        "Fix: Pass `--overwrite replace` or `--overwrite fail` in non-interactive mode.",
                    ],
                ));
            }
            let confirmed = confirm(format!(
                "Replace existing output {output_kind} {}?",
                output.display()
            ));
            if !confirmed {
                return Err(CliError::new(3, "Cancelled."));
            }
            Ok(OverwriteMode::Replace)
        }
        OverwriteMode::Replace => Ok(OverwriteMode::Replace),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::backup_formats::{BackupArtifactFormat, OverwriteMode};

    use super::{
        collect_artifact_output_paths, collect_file_or_numbered_outputs,
        compute_artifact_output_bytes, is_7z_artifact_path, is_xunbak_artifact_path,
        is_zip_artifact_path, parse_split_size_bytes, paths_equal,
        resolve_effective_overwrite_with,
    };

    #[test]
    fn parse_split_size_bytes_accepts_expected_units() {
        assert_eq!(parse_split_size_bytes(None).unwrap(), None);
        assert_eq!(parse_split_size_bytes(Some("")).unwrap(), None);
        assert_eq!(
            parse_split_size_bytes(Some("64K")).unwrap(),
            Some(64 * 1024)
        );
        assert_eq!(
            parse_split_size_bytes(Some("2M")).unwrap(),
            Some(2 * 1024 * 1024)
        );
        assert_eq!(
            parse_split_size_bytes(Some("3G")).unwrap(),
            Some(3 * 1024 * 1024 * 1024)
        );
        assert_eq!(parse_split_size_bytes(Some("1024")).unwrap(), Some(1024));
    }

    #[test]
    fn parse_split_size_bytes_rejects_invalid_values() {
        let err = parse_split_size_bytes(Some("abc")).unwrap_err();
        assert!(err.message.contains("Invalid split size"));
    }

    #[test]
    fn paths_equal_recognizes_same_path_after_canonicalize() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, "aaa").unwrap();
        assert!(paths_equal(&file, &file));
        assert!(paths_equal(&file, &dir.path().join(".").join("a.txt")));
    }

    #[test]
    fn collect_file_or_numbered_outputs_prefers_direct_file_when_present() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("archive.7z");
        fs::write(&output, "data").unwrap();
        fs::write(dir.path().join("archive.7z.001"), "vol1").unwrap();

        let outputs = collect_file_or_numbered_outputs(&output);
        assert_eq!(outputs, vec![output]);
    }

    #[test]
    fn collect_file_or_numbered_outputs_lists_numbered_siblings() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("archive.7z");
        fs::write(dir.path().join("archive.7z.001"), "vol1").unwrap();
        fs::write(dir.path().join("archive.7z.002"), "vol2").unwrap();
        fs::write(dir.path().join("archive.7z.tmp"), "tmp").unwrap();

        let outputs = collect_file_or_numbered_outputs(&output);
        assert_eq!(
            outputs,
            vec![
                dir.path().join("archive.7z.001"),
                dir.path().join("archive.7z.002")
            ]
        );
    }

    #[test]
    fn artifact_path_helpers_detect_split_and_regular_artifacts() {
        assert!(is_zip_artifact_path(Path::new("backup.zip")));
        assert!(is_7z_artifact_path(Path::new("backup.7z")));
        assert!(is_7z_artifact_path(Path::new("backup.7z.001")));
        assert!(is_xunbak_artifact_path(Path::new("backup.xunbak")));
        assert!(is_xunbak_artifact_path(Path::new("backup.xunbak.001")));
        assert!(!is_zip_artifact_path(Path::new("backup.bin")));
    }

    #[test]
    fn collect_artifact_output_paths_for_split_xunbak_includes_all_volumes() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("backup.xunbak");
        fs::write(dir.path().join("backup.xunbak.001"), "one").unwrap();
        fs::write(dir.path().join("backup.xunbak.002"), "two").unwrap();

        let outputs = collect_artifact_output_paths(BackupArtifactFormat::Xunbak, &output);
        assert_eq!(
            outputs,
            vec![
                dir.path().join("backup.xunbak.001"),
                dir.path().join("backup.xunbak.002"),
            ]
        );
    }

    #[test]
    fn compute_artifact_output_bytes_sums_split_xunbak_volumes() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("backup.xunbak");
        fs::write(dir.path().join("backup.xunbak.001"), "1234").unwrap();
        fs::write(dir.path().join("backup.xunbak.002"), "12").unwrap();

        assert_eq!(
            compute_artifact_output_bytes(BackupArtifactFormat::Xunbak, &output),
            6
        );
    }

    #[test]
    fn resolve_effective_overwrite_keeps_requested_mode_when_output_missing() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("missing");
        let mode = resolve_effective_overwrite_with(
            &output,
            OverwriteMode::Replace,
            "file",
            false,
            |_| false,
        )
        .unwrap();
        assert_eq!(mode, OverwriteMode::Replace);
    }

    #[test]
    fn resolve_effective_overwrite_rejects_existing_output_in_fail_mode() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("exists");
        fs::write(&output, "data").unwrap();
        let err =
            resolve_effective_overwrite_with(&output, OverwriteMode::Fail, "file", false, |_| {
                false
            })
            .unwrap_err();
        assert!(err.message.contains("output already exists"));
    }

    #[test]
    fn resolve_effective_overwrite_rejects_ask_when_non_interactive() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("exists");
        fs::write(&output, "data").unwrap();
        let err =
            resolve_effective_overwrite_with(&output, OverwriteMode::Ask, "file", false, |_| false)
                .unwrap_err();
        assert!(err.message.contains("cannot prompt"));
    }

    #[test]
    fn resolve_effective_overwrite_promotes_ask_to_replace_on_confirm() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("exists");
        fs::write(&output, "data").unwrap();
        let mode =
            resolve_effective_overwrite_with(&output, OverwriteMode::Ask, "file", true, |_| true)
                .unwrap();
        assert_eq!(mode, OverwriteMode::Replace);
    }

    #[test]
    fn resolve_effective_overwrite_returns_cancelled_when_confirm_rejected() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("exists");
        fs::write(&output, "data").unwrap();
        let err =
            resolve_effective_overwrite_with(&output, OverwriteMode::Ask, "file", true, |_| false)
                .unwrap_err();
        assert_eq!(err.code, 3);
        assert_eq!(err.message, "Cancelled.");
    }
}
