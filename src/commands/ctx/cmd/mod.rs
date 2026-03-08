use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::{
    CtxCmd, CtxDelCmd, CtxListCmd, CtxOffCmd, CtxRenameCmd, CtxSetCmd, CtxShowCmd, CtxSubCommand,
    CtxUseCmd,
};
use crate::commands::proxy::config::{del_proxy, load_proxy_state, set_proxy};
use crate::ctx_store::{
    CtxProxyMode, CtxProxyState, CtxSession, ctx_store_path, load_session, load_store,
    save_session, save_store, session_path_from_env,
};
use crate::model::ListFormat;
use crate::output::{CliError, CliResult, apply_pretty_table_style, emit_warning, print_table};
use crate::util::parse_tags;

use super::DEFAULT_NOPROXY;
use super::env::{load_env_file, parse_env_kv};
use super::proxy::{
    apply_proxy_updates, emit_proxy_off, emit_proxy_set, normalize_proxy_url, proxy_summary,
};
use super::session::active_profile_name;
use super::validate::validate_name;

mod delete;
mod list;
mod off;
mod rename;
mod set;
mod show;
mod use_ctx;

pub(crate) fn cmd_ctx(args: CtxCmd) -> CliResult {
    match args.cmd {
        CtxSubCommand::Set(a) => set::cmd_set(a),
        CtxSubCommand::Use(a) => use_ctx::cmd_use(a),
        CtxSubCommand::Off(a) => off::cmd_off(a),
        CtxSubCommand::List(a) => list::cmd_list(a),
        CtxSubCommand::Show(a) => show::cmd_show(a),
        CtxSubCommand::Del(a) => delete::cmd_del(a),
        CtxSubCommand::Rename(a) => rename::cmd_rename(a),
    }
}
