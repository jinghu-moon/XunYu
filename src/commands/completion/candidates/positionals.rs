use super::dynamic_config::dynamic_config_keys;
use super::dynamic_profiles::dynamic_ctx_profiles;
use super::values::value_flags_for;
use crate::bookmark::completion::bookmark_completion_candidates;
use crate::commands::completion::BOOKMARK_SUBCOMMANDS;
use super::*;

pub(super) fn count_positionals(
    tokens_before_current: &[String],
    start: usize,
    subcmd: &str,
    subsub: Option<&str>,
) -> usize {
    if tokens_before_current.len() <= start {
        return 0;
    }
    let mut count = 0;
    let mut skip_next = false;
    let value_flags = value_flags_for(subcmd, subsub);
    let mut idx = start;
    while idx < tokens_before_current.len() {
        let token = &tokens_before_current[idx];
        if skip_next {
            skip_next = false;
            idx += 1;
            continue;
        }
        if token == "--" {
            count += tokens_before_current.len() - idx - 1;
            break;
        }
        if token.starts_with('-') {
            if value_flags.iter().any(|f| *f == token) {
                skip_next = true;
            }
            idx += 1;
            continue;
        }
        count += 1;
        idx += 1;
    }
    count
}

pub(super) fn positional_candidates(
    subcmd: &str,
    subsub: Option<&str>,
    index: usize,
    prefix_lower: &str,
    cwd: Option<&str>,
    bookmark_mode: bool,
) -> (Vec<CompletionItem>, u32) {
    const BOOKMARK_TAG_SUBCOMMANDS: &[&str] = &["add", "rm", "list", "rename"];

    if subcmd == "bookmark" && subsub.is_none() && index == 0 {
        return (
            static_candidates(BOOKMARK_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "tag" && subsub.is_none() && index == 0 {
        return (
            static_candidates(BOOKMARK_TAG_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "ctx" && subsub.is_none() && index == 0 {
        return (
            static_candidates(CTX_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub.is_none() && index == 0 {
        return (
            static_candidates(ENV_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "xunbak" && subsub.is_none() && index == 0 {
        return (
            static_candidates(XUNBAK_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "xunbak" && subsub == Some("plugin") && index == 0 {
        return (
            static_candidates(XUNBAK_PLUGIN_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("path") && index == 0 {
        return (
            static_candidates(ENV_PATH_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("snapshot") && index == 0 {
        return (
            static_candidates(ENV_SNAPSHOT_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("profile") && index == 0 {
        return (
            static_candidates(ENV_PROFILE_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("batch") && index == 0 {
        return (
            static_candidates(ENV_BATCH_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("schema") && index == 0 {
        return (
            static_candidates(ENV_SCHEMA_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("annotate") && index == 0 {
        return (
            static_candidates(ENV_ANNOTATE_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("config") && index == 0 {
        return (
            static_candidates(ENV_CONFIG_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "env" && subsub == Some("import") && index == 0 {
        return (Vec::new(), FILTER_EXT);
    }
    if subcmd == "ctx"
        && matches!(
            subsub,
            Some("use") | Some("rm") | Some("show") | Some("rename") | Some("set")
        )
        && index == 0
    {
        return (dynamic_ctx_profiles(prefix_lower), NO_FILE_COMP);
    }
    if subcmd == "redirect" && index == 0 {
        return (Vec::new(), FILTER_DIRS);
    }
    if subcmd == "set" && index == 1 {
        return (Vec::new(), FILTER_DIRS);
    }
    if subcmd == "config" && (subsub == Some("get") || subsub == Some("set")) && index == 0 {
        return (dynamic_config_keys(prefix_lower), NO_FILE_COMP);
    }
    if subcmd == "proxy" && subsub.is_none() && index == 0 {
        return (
            static_candidates(PROXY_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "acl" && subsub.is_none() && index == 0 {
        return (
            static_candidates(ACL_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "config" && subsub.is_none() && index == 0 {
        return (
            static_candidates(CONFIG_SUBCOMMANDS, prefix_lower),
            NO_FILE_COMP,
        );
    }
    if subcmd == "rm" {
        if bookmark_mode {
            return (
                bookmark_completion_candidates(prefix_lower, cwd)
                    .into_iter()
                    .map(CompletionItem::new)
                    .collect(),
                NO_FILE_COMP,
            );
        }
        return (Vec::new(), 0);
    }
    if matches!(subcmd, "z" | "zi" | "o" | "oi" | "open" | "touch" | "rename" | "pin" | "unpin")
        && index == 0
    {
        return (
            bookmark_completion_candidates(prefix_lower, cwd)
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
            NO_FILE_COMP,
        );
    }
    (Vec::new(), NO_FILE_COMP)
}
