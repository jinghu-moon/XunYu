#![allow(unused_imports)]

mod audit;
mod batch;
mod common;
mod config;
mod edit;
mod repair;
mod view;

pub(super) use common::*;

use std::path::{Path, PathBuf};

use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};

use crate::acl;
use crate::acl::audit::{AuditEntry, AuditLog};
use crate::acl::error::AclError;
use crate::acl::types::{AceType, InheritanceFlags, PropagationFlags, RIGHTS_TABLE};
use crate::cli::{
    AclAddCmd, AclAuditCmd, AclBackupCmd, AclBatchCmd, AclCmd, AclConfigCmd, AclCopyCmd,
    AclDiffCmd, AclEffectiveCmd, AclInheritCmd, AclOrphansCmd, AclOwnerCmd, AclPurgeCmd,
    AclRemoveCmd, AclRepairCmd, AclRestoreCmd, AclSubCommand, AclViewCmd,
};
use crate::config::{AclConfig, load_config, save_config};
use crate::output::{CliError, CliResult, apply_pretty_table_style, can_interact, print_table};
use crate::runtime;
use comfy_table::{Attribute, Cell, Color, Table};

pub(crate) fn cmd_acl(args: AclCmd) -> CliResult {
    match args.cmd {
        AclSubCommand::Show(a) => view::cmd_view(a),
        AclSubCommand::Add(a) => edit::cmd_add(a),
        AclSubCommand::Rm(a) => edit::cmd_remove(a),
        AclSubCommand::Purge(a) => edit::cmd_purge(a),
        AclSubCommand::Diff(a) => view::cmd_diff(a),
        AclSubCommand::Batch(a) => batch::cmd_batch(a),
        AclSubCommand::Effective(a) => view::cmd_effective(a),
        AclSubCommand::Copy(a) => edit::cmd_copy(a),
        AclSubCommand::Backup(a) => batch::cmd_backup(a),
        AclSubCommand::Restore(a) => batch::cmd_restore(a),
        AclSubCommand::Inherit(a) => edit::cmd_inherit(a),
        AclSubCommand::Owner(a) => edit::cmd_owner(a),
        AclSubCommand::Orphans(a) => repair::cmd_orphans(a),
        AclSubCommand::Repair(a) => repair::cmd_repair(a),
        AclSubCommand::Audit(a) => audit::cmd_audit(a),
        AclSubCommand::Config(a) => config::cmd_config(a),
    }
}
