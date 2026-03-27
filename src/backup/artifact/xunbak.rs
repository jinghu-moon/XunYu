#![cfg(feature = "xunbak")]

use std::path::{Path, PathBuf};

use crate::backup::artifact::entry::SourceEntry;
use crate::backup::artifact::output_plan::{
    XunbakOutputPlan, XunbakSplitOutputPlan, commit_output_plan,
};
use crate::backup::artifact::reader::copy_entry_to_writer;
use crate::backup_formats::OverwriteMode;
use crate::output::{CliError, CliResult};
use crate::xunbak::writer::{BackupOptions, ContainerWriter, VirtualBackupEntry, WriterError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XunbakWriteSummary {
    pub entry_count: usize,
    pub bytes_in: u64,
    pub destination: PathBuf,
}

pub(crate) fn write_entries_to_xunbak(
    entries: &[&SourceEntry],
    output: &Path,
    source_root: &str,
    options: &BackupOptions,
    overwrite: OverwriteMode,
) -> CliResult<XunbakWriteSummary> {
    let byte_count: u64 = entries.iter().map(|entry| entry.size).sum();
    let adapters: Vec<StreamingSourceEntry<'_>> = entries
        .iter()
        .map(|entry| StreamingSourceEntry { entry })
        .collect();

    if options.split_size.is_none() {
        let plan = XunbakOutputPlan::prepare(output, overwrite)?;
        commit_output_plan(plan, |plan| {
            ContainerWriter::backup_virtual_entries(
                plan.temp_path(),
                &source_root,
                &adapters,
                options,
            )
            .map_err(|err| CliError::new(2, err.to_string()))
        })?;
    } else {
        let plan = XunbakSplitOutputPlan::prepare(output, overwrite)?;
        commit_output_plan(plan, |plan| {
            ContainerWriter::backup_virtual_entries(
                plan.temp_base_path(),
                &source_root,
                &adapters,
                options,
            )
            .map_err(|err| CliError::new(2, err.to_string()))
        })?;
    }

    Ok(XunbakWriteSummary {
        entry_count: entries.len(),
        bytes_in: byte_count,
        destination: output.to_path_buf(),
    })
}

struct StreamingSourceEntry<'a> {
    entry: &'a SourceEntry,
}

impl VirtualBackupEntry for StreamingSourceEntry<'_> {
    fn rel(&self) -> &str {
        &self.entry.path
    }

    fn size(&self) -> u64 {
        self.entry.size
    }

    fn mtime_ns(&self) -> u64 {
        self.entry.mtime_ns.unwrap_or(0)
    }

    fn created_time_ns(&self) -> u64 {
        self.entry.created_time_ns.unwrap_or(0)
    }

    fn win_attributes(&self) -> u32 {
        self.entry.win_attributes
    }

    fn content_hash_hint(&self) -> Option<[u8; 32]> {
        self.entry.content_hash
    }

    fn stream_into(&self, writer: &mut dyn std::io::Write) -> Result<(), WriterError> {
        copy_entry_to_writer(self.entry, writer).map_err(|err| WriterError::Io(err.message))
    }
}
