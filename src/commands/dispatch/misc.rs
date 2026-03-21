use crate::cli::SubCommand;
use crate::output::CliResult;

use super::super::acl_cmd;
#[cfg(feature = "alias")]
use super::super::alias;
use super::super::app_config;
use super::super::bak;
#[cfg(feature = "batch_rename")]
use super::super::batch_rename;
use super::super::bookmarks;
use super::super::completion;
#[cfg(feature = "crypt")]
use super::super::crypt;
#[cfg(feature = "crypt")]
use super::super::vault;
use super::super::delete;
#[cfg(feature = "diff")]
use super::super::diff;
use super::super::find;
#[cfg(feature = "fs")]
use super::super::fs;
#[cfg(feature = "lock")]
use super::super::lock;
use super::super::ports;
#[cfg(feature = "protect")]
use super::super::protect;
use super::super::proxy;
#[cfg(feature = "redirect")]
use super::super::redirect;
#[cfg(feature = "desktop")]
use super::super::desktop;
use super::super::restore;

use super::super::tree;
use super::super::video;

pub(super) fn dispatch(cmd: SubCommand) -> CliResult {
    match cmd {
        SubCommand::Acl(a) => acl_cmd::cmd_acl(a),
        SubCommand::Completion(a) => completion::cmd_completion(a),
        SubCommand::Complete(a) => completion::cmd_complete(a),
        SubCommand::Config(a) => app_config::cmd_config(a),
        SubCommand::List(a) => bookmarks::cmd_list(a),
        SubCommand::Z(a) => bookmarks::cmd_z(a),
        SubCommand::Open(a) => bookmarks::cmd_open(a),
        SubCommand::Workspace(a) => bookmarks::cmd_workspace(a),
        SubCommand::Save(a) => bookmarks::cmd_save(a),
        SubCommand::Set(a) => bookmarks::cmd_set(a),
        SubCommand::Delete(a) => delete::cmd_delete(a),
        SubCommand::Check(a) => bookmarks::cmd_check(a),
        SubCommand::Gc(a) => bookmarks::cmd_gc(a),
        SubCommand::Touch(a) => bookmarks::cmd_touch(a),
        SubCommand::Rename(a) => bookmarks::cmd_rename(a),
        SubCommand::Tag(a) => bookmarks::cmd_tag(a),
        SubCommand::Recent(a) => bookmarks::cmd_recent(a),
        SubCommand::Stats(a) => bookmarks::cmd_stats(a),
        SubCommand::Dedup(a) => bookmarks::cmd_dedup(a),
        SubCommand::Export(a) => bookmarks::cmd_export(a),
        SubCommand::Import(a) => bookmarks::cmd_import(a),
        SubCommand::Proxy(a) => proxy::cmd_proxy(a),
        SubCommand::Pon(a) => proxy::cmd_proxy_on(a),
        SubCommand::Poff(a) => proxy::cmd_proxy_off(a),
        SubCommand::Pst(a) => proxy::cmd_proxy_status(a),
        SubCommand::Px(a) => proxy::cmd_proxy_exec(a),
        SubCommand::Ports(a) => ports::cmd_ports(a),
        SubCommand::Kill(a) => ports::cmd_kill(a),
        SubCommand::Ps(a) => ports::cmd_ps(a),
        SubCommand::Pkill(a) => ports::cmd_pkill(a),
        SubCommand::Keys(a) => bookmarks::cmd_keys(a),
        SubCommand::All(a) => bookmarks::cmd_all(a),
        SubCommand::Fuzzy(a) => bookmarks::cmd_fuzzy(a),
        SubCommand::Bak(a) => bak::cmd_bak(a),
        SubCommand::Tree(a) => tree::cmd_tree(a),
        SubCommand::Find(a) => find::cmd_find(a),
        #[cfg(feature = "alias")]
        SubCommand::Alias(a) => alias::cmd_alias(a),
        #[cfg(feature = "lock")]
        SubCommand::Lock(a) => lock::cmd_lock(a),
        #[cfg(feature = "fs")]
        SubCommand::Rm(a) => fs::cmd_rm(a),
        #[cfg(feature = "lock")]
        SubCommand::Mv(a) => lock::cmd_mv(a),
        #[cfg(feature = "lock")]
        SubCommand::RenFile(a) => lock::cmd_ren_file(a),
        #[cfg(feature = "protect")]
        SubCommand::Protect(a) => protect::cmd_protect(a),
        #[cfg(feature = "crypt")]
        SubCommand::Encrypt(a) => crypt::cmd_encrypt(a),
        #[cfg(feature = "crypt")]
        SubCommand::Decrypt(a) => crypt::cmd_decrypt(a),
        #[cfg(feature = "crypt")]
        SubCommand::Vault(a) => vault::cmd_vault(a),
        #[cfg(feature = "diff")]
        SubCommand::Diff(a) => diff::cmd_diff(a),
        #[cfg(feature = "redirect")]
        SubCommand::Redirect(a) => redirect::cmd_redirect(a),
        #[cfg(feature = "desktop")]
        SubCommand::Desktop(a) => desktop::cmd_desktop(a),
        #[cfg(feature = "batch_rename")]
        SubCommand::Brn(a) => batch_rename::cmd_brn(a),
        SubCommand::Video(a) => video::cmd_video(a),
        SubCommand::Restore(a) => restore::cmd_restore(a),
        SubCommand::Init(_) | SubCommand::Ctx(_) | SubCommand::Env(_) => unreachable!(),
        #[cfg(feature = "dashboard")]
        SubCommand::Serve(_) => unreachable!(),
        #[cfg(feature = "cstat")]
        SubCommand::Cstat(_) => unreachable!(),
        #[cfg(feature = "img")]
        SubCommand::Img(_) => unreachable!(),
    }
}
