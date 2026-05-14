use std::env;

use crate::cli::{CompleteCmd, CompletionCmd};
use crate::output::{CliError, CliResult};

mod shell_bash;
mod shell_fish;
mod shell_powershell;

mod cache;
mod candidates;
mod debug;
mod emit;
mod parse;
mod types;

use self::candidates::{
    count_positionals, flag_takes_value, flags_for, positional_candidates, static_candidates,
    value_candidates, value_directive,
};
use self::debug::DebugContext;
use self::emit::{emit_fallback, emit_response};
use self::parse::{env_flag, find_subcommand, find_subsub, split_flag_value};

const HARD_LIMIT: usize = 200;
const MAX_VALUE_LEN: usize = 500;
const MAX_DESC_LEN: usize = 500;

const NO_FILE_COMP: u32 = 1;
const NO_SPACE: u32 = 2;
const FILTER_DIRS: u32 = 4;
const FILTER_EXT: u32 = 8;

const SUBCOMMANDS: &[&str] = &[
    "acl",
    "bookmark",
    "init",
    "completion",
    "config",
    "ctx",
    "proxy",
    "pon",
    "poff",
    "pst",
    "px",
    "ports",
    "kill",
    "ps",
    "pkill",
    "backup",
    "xunbak",
    "tree",
    "find",
    "env",
    "video",
    "lock",
    "rm",
    "mv",
    "renfile",
    "protect",
    "encrypt",
    "decrypt",
    "serve",
    "redirect",
    "diff",
    "desktop",
    "brn",
    "img",
    "verify",
];
const BOOKMARK_SUBCOMMANDS: &[&str] = &[
    "z", "zi", "o", "oi", "open", "save", "set", "rm", "tag", "pin", "unpin", "rename",
    "list", "recent", "stats", "check", "gc", "dedup", "export", "import", "init", "touch",
    "learn", "undo", "redo", "keys", "all",
];

const GLOBAL_FLAGS: &[&str] = &[
    "--no-color",
    "--version",
    "--quiet",
    "--verbose",
    "--non-interactive",
    "--help",
    "-q",
    "-v",
];

const FORMATS: &[&str] = &["auto", "table", "tsv", "json"];
const LIST_SORTS: &[&str] = &["name", "last", "visits"];
const TREE_SORTS: &[&str] = &["name", "mtime", "size"];
const CONFIG_SUBCOMMANDS: &[&str] = &["get", "set", "edit"];
const CTX_SUBCOMMANDS: &[&str] = &["set", "use", "off", "list", "show", "rm", "rename"];
const PROXY_SUBCOMMANDS: &[&str] = &["set", "rm", "show", "detect", "test"];
const ENV_SUBCOMMANDS: &[&str] = &[
    "list",
    "search",
    "show",
    "set",
    "rm",
    "check",
    "path",
    "path-dedup",
    "snapshot",
    "doctor",
    "profile",
    "batch",
    "apply",
    "export",
    "export-live",
    "env",
    "import",
    "diff-live",
    "validate",
    "schema",
    "annotate",
    "config",
    "audit",
    "watch",
    "template",
    "run",
    "tui",
];
const ENV_PATH_SUBCOMMANDS: &[&str] = &["add", "rm"];
const ENV_SNAPSHOT_SUBCOMMANDS: &[&str] = &["create", "list", "restore"];
const ENV_PROFILE_SUBCOMMANDS: &[&str] = &["list", "capture", "apply", "diff", "rm"];
const ENV_BATCH_SUBCOMMANDS: &[&str] = &["set", "rm", "rename"];
const ENV_SCHEMA_SUBCOMMANDS: &[&str] = &[
    "show",
    "add-required",
    "add-regex",
    "add-enum",
    "remove",
    "reset",
];
const ENV_ANNOTATE_SUBCOMMANDS: &[&str] = &["set", "list"];
const ENV_CONFIG_SUBCOMMANDS: &[&str] = &["show", "path", "reset", "get", "set"];
const XUNBAK_SUBCOMMANDS: &[&str] = &["plugin"];
const XUNBAK_PLUGIN_SUBCOMMANDS: &[&str] = &["install", "uninstall", "doctor"];
const ENV_SCOPES: &[&str] = &["user", "system", "all"];
const ENV_WRITE_SCOPES: &[&str] = &["user", "system"];
const ENV_EXPORT_FORMATS: &[&str] = &["json", "env", "reg", "csv"];
const ENV_IMPORT_MODES: &[&str] = &["merge", "overwrite"];
const ACL_SUBCOMMANDS: &[&str] = &[
    "show",
    "add",
    "rm",
    "purge",
    "diff",
    "batch",
    "effective",
    "copy",
    "backup",
    "restore",
    "inherit",
    "owner",
    "orphans",
    "repair",
    "audit",
    "config",
];
const IO_FORMATS: &[&str] = &["json", "tsv"];
const IMPORT_MODES: &[&str] = &["merge", "overwrite"];

pub(crate) fn cmd_completion(args: CompletionCmd) -> CliResult {
    let script = match args.shell.to_lowercase().as_str() {
        "powershell" | "pwsh" => shell_powershell::completion_powershell(),
        "bash" | "zsh" => shell_bash::completion_bash(),
        "fish" => shell_fish::completion_fish(),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Unsupported shell: {}.", args.shell),
                &["Fix: Use `xun completion powershell|bash|zsh|fish`."],
            ));
        }
    };

    out_println!("{}", script);
    Ok(())
}

pub(crate) fn cmd_complete(args: CompleteCmd) -> CliResult {
    let debug = DebugContext::new();
    if env_flag("XUN_DISABLE_DYNAMIC_COMPLETE") {
        return emit_fallback(&debug, "disabled");
    }

    let mut tokens = args.args;
    let mut current = tokens.last().cloned().unwrap_or_default();
    debug.log(format!("start tokens={:?} current={}", tokens, current));
    let mut value_flag: Option<String> = None;
    let mut value_prefix: Option<String> = None;

    if let Some(last) = tokens.last().cloned()
        && let Some((flag, value)) = split_flag_value(&last)
    {
        value_flag = Some(flag.clone());
        value_prefix = Some(format!("{flag}="));
        current = value;
        tokens.pop();
        tokens.push(flag);
        debug.log(format!(
            "split_flag_value flag={} value={}",
            value_flag.as_deref().unwrap_or(""),
            current
        ));
    }

    let current_lower = current.to_ascii_lowercase();
    let current_is_flag = current.starts_with('-') && value_flag.is_none();
    let after_double_dash = tokens
        .iter()
        .position(|t| t == "--")
        .map(|i| i < tokens.len().saturating_sub(1))
        .unwrap_or(false);
    let Some((subcmd, cmd_start)) = find_subcommand(&tokens) else {
        let items = static_candidates(SUBCOMMANDS, &current_lower);
        return emit_response(items, NO_FILE_COMP, None, value_prefix.as_deref(), &debug);
    };

    let bookmark_mode = tokens.iter().any(|t| t == "--bookmark" || t == "-bm") || subcmd == "bookmark";

    let (subsub, subsub_start) = find_subsub(&subcmd, &tokens, cmd_start + 1);
    let cmd_start = subsub_start;
    let effective_subcmd = if subcmd == "bookmark" {
        subsub.clone().unwrap_or_else(|| "bookmark".to_string())
    } else {
        subcmd.clone()
    };
    let effective_subsub = if subcmd == "bookmark" {
        None
    } else {
        subsub.clone()
    };

    if current_is_flag {
        if subcmd == "completion" {
            let items = static_candidates(&["bash", "zsh", "fish", "powershell"], &current_lower);
            return emit_response(items, NO_FILE_COMP | NO_SPACE, None, None, &debug);
        }
        if subcmd == "redirect" && subsub.is_none() {
            let items = static_candidates(GLOBAL_FLAGS, &current_lower);
            return emit_response(items, NO_FILE_COMP | NO_SPACE, None, None, &debug);
        }
        let items = static_candidates(
            flags_for(&effective_subcmd, effective_subsub.as_deref()),
            &current_lower,
        );
        return emit_response(items, NO_FILE_COMP | NO_SPACE, None, None, &debug);
    }

    if after_double_dash {
        return emit_response(
            Vec::new(),
            NO_FILE_COMP,
            None,
            value_prefix.as_deref(),
            &debug,
        );
    }

    let tokens_before_current = if current.is_empty() {
        &tokens[..]
    } else {
        &tokens[..tokens.len() - 1]
    };

    let prev_token = tokens_before_current.last().map(|s| s.as_str());

    let value_flag_name = value_flag
        .as_deref()
        .or_else(|| {
            prev_token.filter(|t| flag_takes_value(&effective_subcmd, effective_subsub.as_deref(), t))
        });

    if let Some(flag) = value_flag_name {
        debug.log(format!(
            "route=flag_value subcmd={} subsub={} flag={}",
            effective_subcmd,
            effective_subsub.as_deref().unwrap_or(""),
            flag
        ));
        let items = value_candidates(
            &effective_subcmd,
            effective_subsub.as_deref(),
            flag,
            &current_lower,
        );
        let (directive, ext) =
            value_directive(&effective_subcmd, effective_subsub.as_deref(), flag);
        return emit_response(items, directive, ext, value_prefix.as_deref(), &debug);
    }

    let positionals_before =
        count_positionals(tokens_before_current, cmd_start, &effective_subcmd, effective_subsub.as_deref());
    let cwd = env::var("XUN_COMPLETE_CWD").ok();
    let (items, directive) = positional_candidates(
        &effective_subcmd,
        effective_subsub.as_deref(),
        positionals_before,
        &current_lower,
        cwd.as_deref(),
        bookmark_mode,
    );

    debug.log(format!(
        "route=positional subcmd={} subsub={} index={}",
        effective_subcmd,
        effective_subsub.as_deref().unwrap_or(""),
        positionals_before
    ));

    emit_response(items, directive, None, value_prefix.as_deref(), &debug)
}
