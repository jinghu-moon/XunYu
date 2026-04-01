#[cfg(feature = "redirect")]
use super::cache::cached_audit_txs;
use super::cache::{cached_config_keys_and_profiles, cached_ctx_profiles};
use super::types::CompletionItem;
use super::{
    ACL_SUBCOMMANDS, CONFIG_SUBCOMMANDS, CTX_SUBCOMMANDS, ENV_ANNOTATE_SUBCOMMANDS,
    ENV_BATCH_SUBCOMMANDS, ENV_CONFIG_SUBCOMMANDS, ENV_EXPORT_FORMATS, ENV_IMPORT_MODES,
    ENV_PATH_SUBCOMMANDS, ENV_PROFILE_SUBCOMMANDS, ENV_SCHEMA_SUBCOMMANDS, ENV_SCOPES,
    ENV_SNAPSHOT_SUBCOMMANDS, ENV_SUBCOMMANDS, ENV_WRITE_SCOPES, FILTER_DIRS, FILTER_EXT, FORMATS,
    IMPORT_MODES, IO_FORMATS, LIST_SORTS, NO_FILE_COMP, PROXY_SUBCOMMANDS, TREE_SORTS,
    XUNBAK_PLUGIN_SUBCOMMANDS, XUNBAK_SUBCOMMANDS,
};

mod common;
mod dynamic_config;
mod dynamic_profiles;
mod flags;
mod positionals;
mod values;

pub(super) fn count_positionals(
    tokens_before_current: &[String],
    start: usize,
    subcmd: &str,
    subsub: Option<&str>,
) -> usize {
    positionals::count_positionals(tokens_before_current, start, subcmd, subsub)
}

pub(super) fn static_candidates(list: &[&str], prefix_lower: &str) -> Vec<CompletionItem> {
    common::static_candidates(list, prefix_lower)
}

pub(super) fn flags_for(subcmd: &str, subsub: Option<&str>) -> &'static [&'static str] {
    flags::flags_for(subcmd, subsub)
}

pub(super) fn flag_takes_value(subcmd: &str, subsub: Option<&str>, flag: &str) -> bool {
    values::flag_takes_value(subcmd, subsub, flag)
}

pub(super) fn value_candidates(
    subcmd: &str,
    subsub: Option<&str>,
    flag: &str,
    prefix_lower: &str,
) -> Vec<CompletionItem> {
    values::value_candidates(subcmd, subsub, flag, prefix_lower)
}

pub(super) fn value_directive(
    subcmd: &str,
    subsub: Option<&str>,
    flag: &str,
) -> (u32, Option<&'static str>) {
    values::value_directive(subcmd, subsub, flag)
}

pub(super) fn positional_candidates(
    subcmd: &str,
    subsub: Option<&str>,
    index: usize,
    prefix_lower: &str,
    cwd: Option<&str>,
    bookmark_mode: bool,
) -> (Vec<CompletionItem>, u32) {
    positionals::positional_candidates(subcmd, subsub, index, prefix_lower, cwd, bookmark_mode)
}
