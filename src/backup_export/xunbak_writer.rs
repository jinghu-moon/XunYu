#![cfg(feature = "xunbak")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::backup_export::dir_writer::write_entries_to_dir;
use crate::backup_export::output_plan::{XunbakOutputPlan, XunbakSplitOutputPlan};
use crate::backup_export::source::SourceEntry;
use crate::backup_formats::OverwriteMode;
use crate::output::{CliError, CliResult};
use crate::xunbak::writer::{BackupOptions, ContainerWriter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XunbakWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
    pub destination: PathBuf,
}

pub(crate) fn write_entries_to_xunbak(
    entries: &[&SourceEntry],
    output: &Path,
    options: &BackupOptions,
    overwrite: OverwriteMode,
) -> CliResult<XunbakWriteSummary> {
    let staging_dir = create_staging_dir("xunbak-export")?;
    let stage_result = write_entries_to_dir(entries, &staging_dir);
    let stage_summary = match stage_result {
        Ok(summary) => summary,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(err);
        }
    };

    let write_result = if options.split_size.is_none() {
        let plan = XunbakOutputPlan::prepare(output, overwrite)?;
        let result = ContainerWriter::backup(plan.temp_path(), &staging_dir, options)
            .map_err(|err| CliError::new(2, err.to_string()));
        match result {
            Ok(_) => {
                plan.finalize()?;
                Ok(())
            }
            Err(err) => {
                let _ = plan.cleanup();
                Err(err)
            }
        }
    } else {
        let plan = XunbakSplitOutputPlan::prepare(output, overwrite)?;
        let result = ContainerWriter::backup(plan.temp_base_path(), &staging_dir, options)
            .map_err(|err| CliError::new(2, err.to_string()));
        match result {
            Ok(_) => {
                plan.finalize()?;
                Ok(())
            }
            Err(err) => {
                let _ = plan.cleanup();
                Err(err)
            }
        }
    };
    let _ = fs::remove_dir_all(&staging_dir);
    write_result?;

    Ok(XunbakWriteSummary {
        entry_count: stage_summary.entry_count,
        bytes_in: stage_summary.bytes_in,
        destination: output.to_path_buf(),
    })
}

fn create_staging_dir(prefix: &str) -> CliResult<PathBuf> {
    let mut path = std::env::temp_dir();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    path.push(format!("{prefix}-{}-{millis}", Uuid::new_v4()));
    fs::create_dir_all(&path).map_err(|err| {
        CliError::new(
            1,
            format!("Create staging directory failed {}: {err}", path.display()),
        )
    })?;
    Ok(path)
}
