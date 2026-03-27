use std::path::Path;

use crate::backup::artifact::common::paths_equal;
use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::sevenz::{
    SevenZMethod, SevenZWriteOptions, parse_sevenz_method_for_cli,
};
use crate::backup::artifact::sidecar::{
    SidecarPackingHint, SidecarSourceInfo, build_sidecar_bytes,
};
use crate::backup::artifact::zip::{
    ZipCompressionMethod, ZipWriteOptions, parse_zip_method_for_cli,
};
use crate::backup_formats::BackupArtifactFormat;
use crate::output::{CliError, CliResult};

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
    entries: &[&SourceEntry],
) -> Result<ZipWriteOptions, CliError> {
    let method = if no_compress {
        ZipCompressionMethod::Stored
    } else {
        parse_zip_method_for_cli(command, method_arg)?
    };
    Ok(ZipWriteOptions {
        method,
        level,
        sidecar: if no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                format,
                SidecarPackingHint::Zip(method),
                source,
                entries,
            )?)
        },
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
    entries: &[&SourceEntry],
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
        sidecar: if no_sidecar {
            None
        } else {
            Some(build_sidecar_bytes(
                format,
                SidecarPackingHint::SevenZ(method),
                source,
                entries,
            )?)
        },
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
        assert!(options.sidecar.is_some());
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
        assert!(options.sidecar.is_some());
    }
}
