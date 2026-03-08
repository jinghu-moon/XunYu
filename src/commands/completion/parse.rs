use std::env;

use super::{
    ACL_SUBCOMMANDS, CONFIG_SUBCOMMANDS, CTX_SUBCOMMANDS, ENV_SUBCOMMANDS, PROXY_SUBCOMMANDS,
    SUBCOMMANDS,
};

pub(super) fn split_flag_value(raw: &str) -> Option<(String, String)> {
    if !raw.starts_with("--") {
        return None;
    }
    let mut iter = raw.splitn(2, '=');
    let flag = iter.next()?;
    let value = iter.next()?;
    Some((flag.to_string(), value.to_string()))
}

pub(super) fn env_flag(key: &str) -> bool {
    matches!(
        env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

pub(super) fn find_subcommand(tokens: &[String]) -> Option<(String, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        if token == "--" {
            break;
        }
        if token.starts_with('-') {
            continue;
        }
        if SUBCOMMANDS.iter().any(|s| s == token) {
            return Some((token.clone(), idx));
        }
        break;
    }
    None
}

pub(super) fn find_subsub(
    subcmd: &str,
    tokens: &[String],
    start: usize,
) -> (Option<String>, usize) {
    let list = match subcmd {
        "acl" => ACL_SUBCOMMANDS,
        "config" => CONFIG_SUBCOMMANDS,
        "ctx" => CTX_SUBCOMMANDS,
        "proxy" => PROXY_SUBCOMMANDS,
        "env" => ENV_SUBCOMMANDS,
        _ => &[][..],
    };
    for (idx, token) in tokens.iter().enumerate().skip(start) {
        if token == "--" {
            break;
        }
        if token.starts_with('-') {
            continue;
        }
        if list.iter().any(|s| s == token) {
            return (Some(token.clone()), idx + 1);
        }
        break;
    }
    (None, start)
}
