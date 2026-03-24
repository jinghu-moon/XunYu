use std::fs;
use std::path::Path;
use std::process::Command;

use crate::backup_export::sevenz_io::list_7z_entries;
use crate::backup_formats::{BackupArtifactFormat, VerifyOutputMode, VerifySourceMode};
use crate::output::{CliError, CliResult};

pub(crate) fn verify_convert_source(path: &Path, mode: VerifySourceMode) -> CliResult {
    if matches!(mode, VerifySourceMode::Off) {
        return Ok(());
    }
    if !is_xunbak_path(path) {
        return Ok(());
    }

    #[cfg(feature = "xunbak")]
    {
        let report = match mode {
            VerifySourceMode::Quick => crate::xunbak::verify::verify_quick_path(path),
            VerifySourceMode::Full => crate::xunbak::verify::verify_full_path(path),
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
        return Err(CliError::with_details(
            1,
            format!("backup convert source verify failed (mode={mode}): {detail}"),
            &["Fix: Re-run with `--verify-source off` only if you accept skipping integrity checks."],
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
            let file = fs::File::open(path).map_err(|err| {
                CliError::new(1, format!("backup convert output verify failed: open zip: {err}"))
            })?;
            zip::ZipArchive::new(file).map_err(|err| {
                CliError::with_details(
                    1,
                    format!("backup convert output verify failed: invalid zip: {err}"),
                    &["Fix: Re-run export and inspect disk/full-path issues if this persists."],
                )
            })?;
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
                Err(CliError::with_details(
                    1,
                    format!("backup convert output verify failed: {detail}"),
                    &["Fix: Re-run export and inspect output integrity."],
                ))
            }
            #[cfg(not(feature = "xunbak"))]
            {
                Ok(())
            }
        }
        BackupArtifactFormat::SevenZ => {
            list_7z_entries(path).map_err(|err| {
                CliError::with_details(
                    1,
                    format!("backup convert output verify failed: {}", err.message),
                    &["Fix: Re-run export and inspect output integrity."],
                )
            })?;
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
                    return Err(CliError::with_details(
                        1,
                        "backup convert output verify failed: external 7z test failed",
                        &["Fix: Re-run export and inspect split/archive compatibility."],
                    ));
                }
            }
            Ok(())
        }
        BackupArtifactFormat::Dir => Ok(()),
    }
}

fn is_xunbak_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xunbak"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".xunbak.001"))
}

fn find_external_7z() -> Option<String> {
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
