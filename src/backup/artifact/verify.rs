use std::io::Write;
use std::path::Path;
use std::process::Command;

use rayon::prelude::*;

use crate::backup::artifact::common::is_xunbak_artifact_path;
use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::reader::copy_entry_to_writer;
use crate::backup::artifact::sevenz::{list_7z_entries, list_7z_method_names};
use crate::backup::artifact::source::read_artifact_entries;
use crate::backup::artifact::zip_ppmd::{contains_ppmd_entries, verify_ppmd_zip_entries};
use crate::backup_formats::{BackupArtifactFormat, VerifyOutputMode, VerifySourceMode};
use crate::output::{CliError, CliResult};

#[cfg(feature = "xunbak")]
fn xunbak_verify_error_details(
    path: &Path,
    report: &crate::xunbak::verify::VerifyReport,
) -> Vec<String> {
    let mut details = vec![format!("Source: {}", path.display())];
    if let Some(issue) = report.errors.first() {
        details.push(format!("First error: {}", issue.message));
        if let Some(path) = &issue.path {
            details.push(format!("Path: {path}"));
        }
        if let Some(volume_index) = issue.volume_index {
            details.push(format!("Volume: {volume_index}"));
        }
        if let Some(offset) = issue.offset {
            details.push(format!("Offset: {offset}"));
        }
    }
    details
}

pub(crate) fn verify_convert_source(path: &Path, mode: VerifySourceMode) -> CliResult {
    if matches!(mode, VerifySourceMode::Off) {
        return Ok(());
    }
    if !is_xunbak_artifact_path(path) {
        return Ok(());
    }

    #[cfg(feature = "xunbak")]
    {
        let report = match mode {
            VerifySourceMode::Quick => crate::xunbak::verify::verify_quick_path(path),
            VerifySourceMode::Full => crate::xunbak::verify::verify_full_path(path),
            VerifySourceMode::ManifestOnly => {
                crate::xunbak::verify::verify_manifest_only_path(path)
            }
            VerifySourceMode::ExistenceOnly => {
                crate::xunbak::verify::verify_existence_only_path(path)
            }
            VerifySourceMode::Paranoid => crate::xunbak::verify::verify_paranoid_path(path),
            VerifySourceMode::Off => unreachable!(),
        };
        if report.passed {
            return Ok(());
        }
        let detail = report
            .errors
            .first()
            .map(|issue| issue.message.as_str())
            .unwrap_or("unknown verify error");
        let mut detail_lines = xunbak_verify_error_details(path, &report);
        detail_lines.push(
            "Fix: Re-run with `--verify-source off` only if you accept skipping integrity checks."
                .to_string(),
        );
        let refs: Vec<&str> = detail_lines.iter().map(String::as_str).collect();
        return Err(CliError::with_details(
            1,
            format!("backup convert source verify failed (mode={mode}): {detail}"),
            &refs,
        ));
    }
    #[cfg(not(feature = "xunbak"))]
    {
        Err(CliError::with_details(
            2,
            "xunbak source verify is not enabled in this build",
            &["Fix: Rebuild with `--features xunbak`."],
        ))
    }
}

pub(crate) fn verify_output(
    format: BackupArtifactFormat,
    path: &Path,
    mode: VerifyOutputMode,
) -> CliResult {
    if matches!(mode, VerifyOutputMode::Off) {
        return Ok(());
    }

    match format {
        BackupArtifactFormat::Zip => {
            verify_entries_content_for_bench(path)?;
            if contains_ppmd_entries(path).unwrap_or(false) {
                verify_ppmd_zip_entries(path)?;
            }
            Ok(())
        }
        BackupArtifactFormat::Xunbak => {
            #[cfg(feature = "xunbak")]
            {
                let report = crate::xunbak::verify::verify_quick_path(path);
                if report.passed {
                    return Ok(());
                }
                let detail = report
                    .errors
                    .first()
                    .map(|issue| issue.message.as_str())
                    .unwrap_or("unknown verify error");
                let mut detail_lines = xunbak_verify_error_details(path, &report);
                detail_lines.push("Fix: Re-run export and inspect output integrity.".to_string());
                let refs: Vec<&str> = detail_lines.iter().map(String::as_str).collect();
                Err(CliError::with_details(
                    1,
                    format!("backup convert output verify failed: {detail}"),
                    &refs,
                ))
            }
            #[cfg(not(feature = "xunbak"))]
            {
                Ok(())
            }
        }
        BackupArtifactFormat::SevenZ => {
            list_7z_entries(path).map_err(|err| verify_output_error(path, None, err.message))?;
            verify_entries_content_for_bench(path)?;
            if let Some(cmd) = find_external_7z() {
                let target = external_7z_target(path);
                let output = Command::new(&cmd)
                    .args(["t", target.to_string_lossy().as_ref()])
                    .output()
                    .map_err(|err| {
                        CliError::new(
                            1,
                            format!("backup convert output verify failed: launch 7z: {err}"),
                        )
                    })?;
                if !output.status.success() {
                    let details = build_7z_verify_failure_details(path, &output);
                    let refs: Vec<&str> = details.iter().map(String::as_str).collect();
                    return Err(CliError::with_details(
                        1,
                        "backup convert output verify failed: external 7z test failed",
                        &refs,
                    ));
                }
            }
            Ok(())
        }
        BackupArtifactFormat::Dir => Ok(()),
    }
}

fn find_external_7z() -> Option<String> {
    if std::env::var_os("XUN_TEST_DISABLE_EXTERNAL_7Z").is_some() {
        return None;
    }
    for candidate in ["7z.exe", "7z", "7za.exe", "7za", "7zr.exe", "7zr"] {
        if Command::new(candidate).arg("i").output().is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn external_7z_target(path: &Path) -> std::path::PathBuf {
    let first_volume = std::path::PathBuf::from(format!("{}.001", path.display()));
    if first_volume.exists() {
        first_volume
    } else {
        path.to_path_buf()
    }
}

fn build_7z_verify_failure_details(path: &Path, output: &std::process::Output) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(methods) = read_7z_method_names(path) {
        details.push(format!("Detected 7z methods: {}", methods.join(", ")));
        if methods.iter().any(|method| method == "zstd") {
            details.push(
                "Fix: 7z `zstd` requires extraction-side external codec support (for example 7-Zip-zstd or NanaZip)."
                    .to_string(),
            );
            details.push(
                "Fix: Prefer `--method lzma2|bzip2|deflate` when stock 7-Zip compatibility is required."
                    .to_string(),
            );
        } else if methods.iter().any(|method| method == "ppmd") {
            details.push(
                "Fix: Detected 7z `ppmd` failure. This usually indicates a PPMD bitstream compatibility regression and should be investigated."
                    .to_string(),
            );
            details.push(
                "Fix: Re-run the dedicated PPMD compatibility regression tests before changing method routing."
                    .to_string(),
            );
        } else {
            details.push("Fix: Re-run export and inspect split/archive compatibility.".to_string());
        }
    } else {
        details.push("Fix: Re-run export and inspect split/archive compatibility.".to_string());
    }
    if let Some(line) = first_external_7z_error_line(output) {
        details.push(format!("External 7z: {line}"));
    }
    details
}

fn read_7z_method_names(path: &Path) -> Option<Vec<String>> {
    match list_7z_method_names(path) {
        Ok(methods) if !methods.is_empty() => Some(methods),
        _ => None,
    }
}

fn first_external_7z_error_line(output: &std::process::Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && (line.starts_with("ERROR:")
                    || line.contains("Unsupported Method")
                    || line.contains("Can not open the file as"))
        })
        .map(str::to_string)
}

pub(crate) fn verify_entries_content_for_bench(path: &Path) -> CliResult {
    let entries = read_artifact_entries(path)
        .map_err(|err| verify_output_error(path, None, err.message.clone()))?;
    if should_verify_entries_in_parallel(&entries) {
        entries.par_iter().try_for_each(|entry| {
            verify_entry_content(entry)
                .map_err(|err| verify_output_error(path, Some(&entry.path), err.message))
        })?;
    } else {
        for entry in &entries {
            verify_entry_content(entry)
                .map_err(|err| verify_output_error(path, Some(&entry.path), err.message))?;
        }
    }
    Ok(())
}

fn should_verify_entries_in_parallel(entries: &[SourceEntry]) -> bool {
    entries.len() >= 64
        && entries.first().is_some_and(|entry| {
            matches!(
                entry.kind,
                crate::backup::artifact::entry::SourceKind::Filesystem
                    | crate::backup::artifact::entry::SourceKind::DirArtifact
                    | crate::backup::artifact::entry::SourceKind::XunbakArtifact
            )
        })
}

fn verify_entry_content(entry: &SourceEntry) -> CliResult {
    let mut sink = VerifyingSink::new();
    copy_entry_to_writer(entry, &mut sink).map_err(|err| CliError::new(1, err.message))?;
    if sink.bytes_written != entry.size {
        return Err(CliError::new(
            1,
            format!(
                "content size mismatch: expected {}, got {}",
                entry.size, sink.bytes_written
            ),
        ));
    }
    if let Some(expected) = entry.content_hash
        && sink.finalize() != expected
    {
        return Err(CliError::new(1, "content hash mismatch"));
    }
    Ok(())
}

fn verify_output_error(path: &Path, entry_path: Option<&str>, message: String) -> CliError {
    let mut details = vec![format!("Source: {}", path.display())];
    if let Some(entry_path) = entry_path {
        details.push(format!("Path: {entry_path}"));
    }
    details.push("Fix: Re-run export and inspect output integrity.".to_string());
    let refs: Vec<&str> = details.iter().map(String::as_str).collect();
    CliError::with_details(
        1,
        format!("backup convert output verify failed: {message}"),
        &refs,
    )
}

struct VerifyingSink {
    bytes_written: u64,
    hasher: blake3::Hasher,
}

impl VerifyingSink {
    fn new() -> Self {
        Self {
            bytes_written: 0,
            hasher: blake3::Hasher::new(),
        }
    }

    fn finalize(self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Write for VerifyingSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes_written += buf.len() as u64;
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{build_7z_verify_failure_details, first_external_7z_error_line};
    use std::os::windows::process::ExitStatusExt;
    use std::path::Path;
    use std::process::Output;

    fn fake_output(stdout: &str, stderr: &str) -> Output {
        Output {
            status: std::process::ExitStatus::from_raw(2),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn first_external_7z_error_line_prefers_meaningful_error_text() {
        let output = fake_output("ERROR: Unsupported Method : notes.txt\n", "");
        assert_eq!(
            first_external_7z_error_line(&output).as_deref(),
            Some("ERROR: Unsupported Method : notes.txt")
        );
    }

    #[test]
    fn build_7z_verify_failure_details_handles_missing_archive_methods() {
        let output = fake_output("", "Can not open the file as archive\n");
        let details = build_7z_verify_failure_details(Path::new("missing.7z"), &output);
        assert!(
            details
                .iter()
                .any(|line| line.contains("inspect split/archive compatibility"))
        );
        assert!(
            details
                .iter()
                .any(|line| line.contains("Can not open the file as archive"))
        );
    }
}
