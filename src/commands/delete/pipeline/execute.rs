use std::path::PathBuf;

use rayon::prelude::*;

#[cfg(feature = "protect")]
use crate::output::emit_warning;

use crate::commands::delete::types::{DeleteOptions, DeleteRecord};
use crate::commands::delete::{deleter, file_info, progress};
use crate::windows::ctrlc;

pub(super) fn delete_paths(
    paths: Vec<PathBuf>,
    opts: &DeleteOptions,
    progress: Option<&progress::Progress>,
) -> Vec<DeleteRecord> {
    if paths.is_empty() {
        return Vec::new();
    }
    let snapshot = crate::commands::delete::winapi::handle_snapshot();
    paths
        .into_par_iter()
        .filter_map(|path| {
            if ctrlc::is_cancelled() {
                return None;
            }

            let info = if opts.collect_info {
                file_info::collect(&path)
            } else {
                None
            };

            #[cfg(feature = "protect")]
            if let Err(msg) = crate::protect::check_protection(
                &path,
                "delete",
                opts.force,
                opts.reason.as_deref(),
            ) {
                emit_warning(format!("Protection check failed: {msg}"), &[]);
                return Some(DeleteRecord::new(path, deleter::Outcome::Error(5), info));
            }

            let path_str = path.to_string_lossy();
            let mut outcome = if opts.dry_run {
                deleter::Outcome::WhatIf
            } else {
                deleter::try_delete_from_level(path_str.as_ref(), opts.level, snapshot)
            };

            if matches!(outcome, deleter::Outcome::Error(_)) && opts.on_reboot && !opts.dry_run {
                outcome = deleter::try_delete_from_level(path_str.as_ref(), 6, snapshot);
            }

            if let Some(p) = progress {
                p.inc_processed();
                if outcome.is_success() {
                    p.inc_succeeded();
                } else if outcome.is_error() {
                    p.inc_failed();
                }
            }

            Some(DeleteRecord::new(path, outcome, info))
        })
        .collect()
}
