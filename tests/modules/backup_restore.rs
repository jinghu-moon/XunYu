#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

#[path = "backup_restore_cases/backup.rs"]
mod backup;
#[path = "backup_restore_cases/restore.rs"]
mod restore;
