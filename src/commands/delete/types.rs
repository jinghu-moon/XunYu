use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::deleter;
use super::file_info;

#[derive(Clone, Debug)]
pub(crate) struct DeleteRecord {
    pub(super) path: PathBuf,
    pub(super) outcome: deleter::Outcome,
    pub(super) info: Option<file_info::FileInfo>,
    pub(super) ts_ms: u128,
}

impl DeleteRecord {
    pub(super) fn new(
        path: PathBuf,
        outcome: deleter::Outcome,
        info: Option<file_info::FileInfo>,
    ) -> Self {
        Self {
            path,
            outcome,
            info,
            ts_ms: now_millis(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DeleteOptions {
    pub(super) level: u8,
    pub(super) dry_run: bool,
    pub(super) collect_info: bool,
    pub(super) on_reboot: bool,
    #[cfg_attr(not(feature = "protect"), allow(dead_code))]
    pub(super) force: bool,
    #[cfg_attr(not(feature = "protect"), allow(dead_code))]
    pub(super) reason: Option<String>,
}

pub(super) enum PathKind {
    File,
    Dir,
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
