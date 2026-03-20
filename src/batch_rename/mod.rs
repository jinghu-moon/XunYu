// batch_rename/mod.rs

pub mod collect;
pub mod compute;
pub mod conflict;
pub mod conflict_strategy;
pub mod cycle_break;
pub mod natural_sort;
pub mod ntfs_case;
pub mod output_format;
pub mod preflight;
pub mod types;
pub mod undo;

/// Test-facing helpers that convert internal CliError to String.
/// Only compiled when running tests.
/// Internal helpers for integration tests — not part of the public API.
#[doc(hidden)]
pub mod testing {
    use std::path::{Path, PathBuf};

    pub fn collect_files(dir: &str, exts: &[String], recursive: bool) -> Result<Vec<PathBuf>, String> {
        super::collect::collect_files(dir, exts, recursive).map_err(|e| e.message)
    }

    pub fn collect_files_filtered(
        dir: &str,
        exts: &[String],
        recursive: bool,
        filter: Option<&str>,
        exclude: Option<&str>,
    ) -> Result<Vec<PathBuf>, String> {
        super::collect::collect_files_filtered(dir, exts, recursive, filter, exclude)
            .map_err(|e| e.message)
    }

    pub fn collect_files_depth(
        dir: &str,
        exts: &[String],
        depth: Option<usize>,
        filter: Option<&str>,
        exclude: Option<&str>,
    ) -> Result<Vec<PathBuf>, String> {
        super::collect::collect_files_depth(dir, exts, depth, filter, exclude)
            .map_err(|e| e.message)
    }

    pub fn compute_ops(
        files: &[PathBuf],
        mode: &super::compute::RenameMode,
    ) -> Result<Vec<super::types::RenameOp>, String> {
        super::compute::compute_ops(files, mode).map_err(|e| e.message)
    }

    pub fn compute_ops_chain(
        files: &[PathBuf],
        steps: &[super::compute::RenameMode],
    ) -> Result<Vec<super::types::RenameOp>, String> {
        super::compute::compute_ops_chain(files, steps).map_err(|e| e.message)
    }

    pub fn write_undo(dir: &Path, records: &[super::undo::UndoRecord]) -> Result<(), String> {
        super::undo::write_undo(dir, records).map_err(|e| e.message)
    }

    pub fn append_undo(dir: &Path, records: &[super::undo::UndoRecord]) -> Result<(), String> {
        super::undo::append_undo(dir, records).map_err(|e| e.message)
    }

    pub fn read_undo_history(dir: &Path) -> Result<Vec<super::undo::UndoBatch>, String> {
        super::undo::read_undo_history(dir).map_err(|e| e.message)
    }

    pub fn run_undo(dir: &str) -> Result<(), String> {
        super::undo::run_undo(dir).map_err(|e| e.message)
    }

    pub fn run_undo_steps(dir: &str, steps: usize) -> Result<(), String> {
        super::undo::run_undo_steps(dir, steps).map_err(|e| e.message)
    }

    pub fn apply_conflict_strategy(
        ops: Vec<super::types::RenameOp>,
        strategy: super::conflict_strategy::OnConflict,
        existing: &[PathBuf],
    ) -> Result<Vec<super::types::RenameOp>, String> {
        super::conflict_strategy::apply_conflict_strategy(ops, strategy, existing)
            .map_err(|e| e.message)
    }
}
