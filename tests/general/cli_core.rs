#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

#[path = "cli_core_cases/bookmark_ops.rs"]
mod bookmark_ops;
#[path = "cli_core_cases/gc_dedup.rs"]
mod gc_dedup;
#[path = "cli_core_cases/import_export.rs"]
mod import_export;
#[path = "cli_core_cases/list.rs"]
mod list;
#[path = "cli_core_cases/misc.rs"]
mod misc;
#[path = "cli_core_cases/tag.rs"]
mod tag;
#[path = "cli_core_cases/tree.rs"]
mod tree;
