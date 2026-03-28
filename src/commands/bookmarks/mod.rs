use std::io::{Error, ErrorKind};
use std::path::Path;

use crate::output::{CliError, CliResult};

pub(crate) mod io;
pub(crate) mod list;
pub(crate) mod maintenance;
pub(crate) mod mutate;
pub(crate) mod navigation;
pub(crate) mod tags;

pub(crate) use io::{cmd_export, cmd_import};
pub(crate) use list::{cmd_all, cmd_fuzzy, cmd_keys, cmd_list, cmd_recent, cmd_stats};
pub(crate) use maintenance::cmd_check;
pub(crate) use maintenance::{cmd_dedup, cmd_gc};
pub(crate) use mutate::{cmd_rename, cmd_save, cmd_set, cmd_touch, delete_bookmark};
pub(crate) use navigation::{cmd_open, cmd_workspace, cmd_z};
pub(crate) use tags::cmd_tag;

pub(crate) fn load_bookmark_db(path: &Path) -> CliResult<crate::store::Db> {
    crate::store::load_strict(path).map_err(|err| map_bookmark_db_load_error(path, err))
}

fn map_bookmark_db_load_error(path: &Path, err: Error) -> CliError {
    if err.kind() == ErrorKind::InvalidData {
        let details = [
            format!("Path: {}", path.display()),
            format!("Details: {err}"),
            "Fix: Repair the bookmark db JSON or restore it from backup before retrying."
                .to_string(),
        ];
        return CliError::with_details(1, "Bookmark db is corrupted.".to_string(), &details);
    }

    CliError::new(
        1,
        format!("Failed to load bookmark db {}: {err}", path.display()),
    )
}
