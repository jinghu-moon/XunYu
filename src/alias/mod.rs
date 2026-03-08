pub(crate) mod apppaths;
pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod output;
pub(crate) mod scanner;
pub(crate) mod shell;
pub(crate) mod shim_gen;

mod app_alias_cmd;
mod context;
mod query;
mod shell_alias_cmd;
mod sync;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use comfy_table::{Cell, Color, Table};

use crate::alias::config::{AliasMode, AppAlias, Config, ShellAlias};
use crate::alias::output::{fuzzy_score, parse_selection};
use crate::alias::scanner::ScanSource;
use crate::alias::shell::ShellBackend;
use crate::alias::shell::cmd::CmdBackend;
use crate::alias::shell::ps::PsBackend;
#[cfg(feature = "alias-shell-extra")]
use crate::alias::shell::{bash::BashBackend, nu::NuBackend};
use crate::cli::*;
use crate::output::{apply_pretty_table_style, print_table};

use context::AliasCtx;

pub(crate) fn cmd_alias(args: AliasCmd) -> Result<()> {
    let ctx = AliasCtx::from_cli(&args);
    match args.cmd {
        AliasSubCommand::Setup(cmd) => shell_alias_cmd::cmd_setup(&ctx, cmd),
        AliasSubCommand::Add(cmd) => shell_alias_cmd::cmd_add(&ctx, cmd),
        AliasSubCommand::Rm(cmd) => shell_alias_cmd::cmd_rm(&ctx, cmd),
        AliasSubCommand::Ls(cmd) => query::cmd_ls(&ctx, cmd),
        AliasSubCommand::Find(cmd) => query::cmd_find(&ctx, cmd),
        AliasSubCommand::Which(cmd) => query::cmd_which(&ctx, &cmd.name, false),
        AliasSubCommand::Sync(_) => sync::cmd_sync(&ctx),
        AliasSubCommand::Export(cmd) => shell_alias_cmd::cmd_export(&ctx, cmd),
        AliasSubCommand::Import(cmd) => shell_alias_cmd::cmd_import(&ctx, cmd),
        AliasSubCommand::App(cmd) => match cmd.cmd {
            AliasAppSubCommand::Add(c) => app_alias_cmd::cmd_app_add(&ctx, c),
            AliasAppSubCommand::Rm(c) => app_alias_cmd::cmd_app_rm(&ctx, c),
            AliasAppSubCommand::Ls(c) => app_alias_cmd::cmd_app_ls(&ctx, c),
            AliasAppSubCommand::Scan(c) => app_alias_cmd::cmd_app_scan(&ctx, c),
            AliasAppSubCommand::Which(c) => query::cmd_which(&ctx, &c.name, true),
            AliasAppSubCommand::Sync(_) => sync::cmd_app_sync(&ctx),
        },
    }
}
