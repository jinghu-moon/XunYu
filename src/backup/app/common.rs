use std::path::Path;

use serde::Serialize;

use crate::backup::artifact::common::paths_equal;
use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::sevenz::{
    SevenZMethod, SevenZWriteOptions, parse_sevenz_method_for_cli,
};
use crate::backup::artifact::sidecar::{SidecarPlan, SidecarSourceInfo};
use crate::backup::artifact::zip::{
    ZipCompressionMethod, ZipWriteOptions, parse_zip_method_for_cli,
};
use crate::backup_formats::{BackupAction, BackupArtifactFormat, ExportStatus};
use crate::output::{CliError, CliResult};

#[derive(Serialize)]
pub(crate) struct SummaryActionStatus<A, S> {
    pub action: A,
    pub status: S,
}

#[derive(Serialize)]
pub(crate) struct SummaryPaths {
    pub source: String,
    pub destination: String,
}

#[derive(Serialize)]
pub(crate) struct SummaryVerifyModes {
    pub verify_source: String,
    pub verify_output: String,
}

#[derive(Serialize)]
pub(crate) struct SummaryExecutionStats {
    pub dry_run: bool,
    pub selected: usize,
    pub written: usize,
    pub skipped: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub overwrite_count: usize,
}

#[derive(Serialize)]
pub(crate) struct SummarySelectionStats {
    pub dry_run: bool,
    pub selected: usize,
    pub skipped: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub overwrite_count: usize,
}

#[derive(Serialize)]
pub(crate) struct SummaryDurationOutputs {
    pub duration_ms: u128,
    pub outputs: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct RestoreSummaryStats {
    pub dry_run: bool,
    pub snapshot: bool,
    pub restored: usize,
    pub failed: usize,
}

pub(crate) fn summary_action_status(
    action: BackupAction,
    status: ExportStatus,
) -> SummaryActionStatus<BackupAction, ExportStatus> {
    SummaryActionStatus { action, status }
}

pub(crate) fn ensure_create_output_distinct(
    source: &Path,
    output: &Path,
    format: BackupArtifactFormat,
) -> CliResult {
    if !paths_equal(source, output) {
        return Ok(());
    }
    Err(CliError::with_details(
        2,
        format!("backup create --format {format} output must differ from source directory"),
        &["Fix: Choose a different `--output` path."],
    ))
}

pub(crate) fn ensure_convert_output_distinct(source: &Path, output: &Path) -> CliResult {
    if !paths_equal(source, output) {
        return Ok(());
    }
    Err(CliError::with_details(
        2,
        "backup convert source and destination must be different",
        &["Fix: Choose a different `--output` path."],
    ))
}

pub(crate) fn build_zip_write_options(
    command: &str,
    method_arg: Option<&str>,
    no_compress: bool,
    level: Option<u32>,
    no_sidecar: bool,
    format: BackupArtifactFormat,
    source: &SidecarSourceInfo,
    _entries: &[&SourceEntry],
) -> Result<ZipWriteOptions, CliError> {
    let method = if no_compress {
        ZipCompressionMethod::Stored
    } else {
        parse_zip_method_for_cli(command, method_arg)?
    };
    Ok(ZipWriteOptions {
        method,
        level,
        sidecar: None,
        sidecar_plan: (!no_sidecar).then(|| SidecarPlan {
            format,
            source: source.clone(),
        }),
    })
}

pub(crate) fn build_sevenz_write_options(
    command: &str,
    method_arg: Option<&str>,
    no_compress: bool,
    solid: bool,
    level: Option<u32>,
    no_sidecar: bool,
    format: BackupArtifactFormat,
    source: &SidecarSourceInfo,
    _entries: &[&SourceEntry],
) -> Result<SevenZWriteOptions, CliError> {
    let method = if no_compress {
        SevenZMethod::Copy
    } else {
        parse_sevenz_method_for_cli(command, method_arg)?
    };
    Ok(SevenZWriteOptions {
        solid,
        method,
        level: level.unwrap_or(1),
        sidecar: None,
        sidecar_plan: (!no_sidecar).then(|| SidecarPlan {
            format,
            source: source.clone(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::backup::artifact::entry::{SourceEntry, SourceKind};

    use super::{
        build_sevenz_write_options, build_zip_write_options, ensure_convert_output_distinct,
        ensure_create_output_distinct,
    };

    fn sample_entry() -> SourceEntry {
        SourceEntry {
            path: "a.txt".to_string(),
            source_path: None,
            size: 3,
            mtime_ns: None,
            created_time_ns: None,
            win_attributes: 0,
            content_hash: Some([1; 32]),
            kind: SourceKind::Filesystem,
        }
    }

    fn sample_source() -> crate::backup::artifact::sidecar::SidecarSourceInfo {
        crate::backup::artifact::sidecar::SidecarSourceInfo {
            snapshot_id: "snap".to_string(),
            source_root: "root".to_string(),
        }
    }

    #[test]
    fn ensure_convert_output_distinct_rejects_same_path() {
        let err = ensure_convert_output_distinct(Path::new("a"), Path::new("a")).unwrap_err();
        assert!(
            err.message
                .contains("source and destination must be different")
        );
    }

    #[test]
    fn ensure_create_output_distinct_rejects_same_path() {
        let err = ensure_create_output_distinct(
            Path::new("a"),
            Path::new("a"),
            crate::backup_formats::BackupArtifactFormat::Dir,
        )
        .unwrap_err();
        assert!(err.message.contains("--format dir"));
    }

    #[test]
    fn build_zip_write_options_no_compress_forces_stored() {
        let entry = sample_entry();
        let options = build_zip_write_options(
            "backup create",
            Some("deflated"),
            true,
            Some(9),
            false,
            crate::backup_formats::BackupArtifactFormat::Zip,
            &sample_source(),
            &[&entry],
        )
        .unwrap();

        assert_eq!(
            options.method,
            crate::backup::artifact::zip::ZipCompressionMethod::Stored
        );
        assert!(options.sidecar_plan.is_some());
    }

    #[test]
    fn build_sevenz_write_options_no_compress_forces_copy() {
        let entry = sample_entry();
        let options = build_sevenz_write_options(
            "backup create",
            Some("lzma2"),
            true,
            true,
            None,
            false,
            crate::backup_formats::BackupArtifactFormat::SevenZ,
            &sample_source(),
            &[&entry],
        )
        .unwrap();

        assert_eq!(
            options.method,
            crate::backup::artifact::sevenz::SevenZMethod::Copy
        );
        assert!(options.solid);
        assert_eq!(options.level, 1);
        assert!(options.sidecar_plan.is_some());
    }
}
