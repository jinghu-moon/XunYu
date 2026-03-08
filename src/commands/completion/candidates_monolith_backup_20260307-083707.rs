use crate::fuzzy::{cwd_boost, frecency};

#[cfg(feature = "redirect")]
use super::cache::cached_audit_txs;
use super::cache::{cached_config_keys_and_profiles, cached_ctx_profiles, cached_db};
use super::types::CompletionItem;
use super::{
    ACL_SUBCOMMANDS, CONFIG_SUBCOMMANDS, CTX_SUBCOMMANDS, ENV_ANNOTATE_SUBCOMMANDS,
    ENV_BATCH_SUBCOMMANDS, ENV_CONFIG_SUBCOMMANDS, ENV_EXPORT_FORMATS, ENV_IMPORT_MODES,
    ENV_PATH_SUBCOMMANDS, ENV_PROFILE_SUBCOMMANDS, ENV_SCHEMA_SUBCOMMANDS, ENV_SCOPES,
    ENV_SNAPSHOT_SUBCOMMANDS, ENV_SUBCOMMANDS, ENV_WRITE_SCOPES, FILTER_DIRS, FILTER_EXT, FORMATS,
    IMPORT_MODES, IO_FORMATS, LIST_SORTS, NO_FILE_COMP, PROXY_SUBCOMMANDS, TREE_SORTS,
};

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

pub(super) fn static_candidates(list: &[&str], prefix_lower: &str) -> Vec<CompletionItem> {
    let mut out: Vec<CompletionItem> = list
        .iter()
        .filter(|s| starts_with_ci(s, prefix_lower))
        .map(|s| CompletionItem::new((*s).to_string()))
        .collect();
    out.sort_by(|a, b| {
        a.value
            .to_ascii_lowercase()
            .cmp(&b.value.to_ascii_lowercase())
    });
    out
}

fn starts_with_ci(value: &str, prefix_lower: &str) -> bool {
    if prefix_lower.is_empty() {
        return true;
    }
    value.to_ascii_lowercase().starts_with(prefix_lower)
}

pub(super) fn flags_for(subcmd: &str, subsub: Option<&str>) -> &'static [&'static str] {
    match (subcmd, subsub) {
        ("ctx", Some("set")) => &[
            "--path",
            "--proxy",
            "--noproxy",
            "-t",
            "--tag",
            "--env",
            "--env-file",
        ],
        ("ctx", Some("list")) | ("ctx", Some("show")) => &["-f", "--format"],
        ("delete", _) | ("del", _) => &[
            "--bookmark",
            "-bm",
            "--reserved",
            "--any",
            "--name",
            "-e",
            "--exclude",
            "-p",
            "--pattern",
            "--no-default-excludes",
            "--no-tui",
            "--dry-run",
            "--what-if",
            "--collect-info",
            "--log",
            "--level",
            "--on-reboot",
            "-y",
            "--yes",
            "-f",
            "--format",
            "--force",
            "--reason",
        ],
        ("list", _) => &[
            "-t",
            "--tag",
            "-s",
            "--sort",
            "-n",
            "--limit",
            "--offset",
            "--reverse",
            "--tsv",
            "-f",
            "--format",
        ],
        ("z", _) | ("open", _) => &["-t", "--tag"],
        ("save", _) | ("set", _) | ("tag", _) => &["-t", "--tag"],
        ("tree", _) => &[
            "-d",
            "--depth",
            "-o",
            "--output",
            "--hidden",
            "--no-clip",
            "--plain",
            "--stats-only",
            "--fast",
            "--sort",
            "--size",
            "--max-items",
            "--include",
            "--exclude",
        ],
        ("redirect", _) => &[
            "--profile",
            "--explain",
            "--stats",
            "--confirm",
            "--review",
            "--log",
            "--tx",
            "--last",
            "--validate",
            "--plan",
            "--apply",
            "--undo",
            "--watch",
            "--status",
            "--simulate",
            "--dry-run",
            "--copy",
            "-y",
            "--yes",
            "-f",
            "--format",
        ],
        ("acl", Some("view")) => &["--path", "-p", "--detail", "--export"],
        ("acl", Some("add")) => &[
            "--path",
            "-p",
            "--principal",
            "--rights",
            "--ace-type",
            "--inherit",
            "-y",
            "--yes",
        ],
        ("acl", Some("remove")) => &["--path", "-p"],
        ("acl", Some("purge")) => &["--path", "-p", "--principal", "-y", "--yes"],
        ("acl", Some("diff")) => &["--path", "-p", "--reference", "-r", "--output", "-o"],
        ("acl", Some("batch")) => &["--file", "--paths", "--action", "--output", "-y", "--yes"],
        ("acl", Some("effective")) => &["--path", "-p", "--user", "-u"],
        ("acl", Some("copy")) => &["--path", "-p", "--reference", "-r", "-y", "--yes"],
        ("acl", Some("backup")) => &["--path", "-p", "--output", "-o"],
        ("acl", Some("restore")) => &["--path", "-p", "--from", "-y", "--yes"],
        ("acl", Some("inherit")) => &["--path", "-p", "--disable", "--enable", "--preserve"],
        ("acl", Some("owner")) => &["--path", "-p", "--set", "-y", "--yes"],
        ("acl", Some("orphans")) => &[
            "--path",
            "-p",
            "--recursive",
            "--action",
            "--output",
            "-y",
            "--yes",
        ],
        ("acl", Some("repair")) => &["--path", "-p", "--export-errors", "-y", "--yes"],
        ("acl", Some("audit")) => &["--tail", "--export"],
        ("acl", Some("config")) => &["--set"],
        ("export", _) => &["-f", "--format", "-o", "--out"],
        ("import", _) => &[
            "-f", "--format", "-i", "--input", "-m", "--mode", "-y", "--yes",
        ],
        ("env", Some("list")) => &["--scope", "-f", "--format"],
        ("env", Some("search")) => &["--scope", "-f", "--format"],
        ("env", Some("get")) => &["--scope", "-f", "--format"],
        ("env", Some("set")) => &["--scope", "--no-snapshot"],
        ("env", Some("del")) => &["--scope", "-y", "--yes"],
        ("env", Some("check")) => &["--scope", "--fix", "--format"],
        ("env", Some("doctor")) => &["--scope", "--fix", "--format"],
        ("env", Some("path-dedup")) => &["--scope", "--remove-missing", "--dry-run"],
        ("env", Some("profile")) => &["--scope", "-y", "--yes", "-f", "--format"],
        ("env", Some("batch")) => &["--scope", "--dry-run"],
        ("env", Some("apply")) => &["--scope", "-y", "--yes"],
        ("env", Some("export")) => &["--scope", "--format", "--out"],
        ("env", Some("export-live")) => &["--scope", "--format", "--env", "--set", "--out"],
        ("env", Some("env")) => &["--scope", "--format", "--env", "--set"],
        ("env", Some("import")) => &["--scope", "--mode", "--dry-run", "-y", "--yes"],
        ("env", Some("diff-live")) => &["--scope", "--snapshot", "--color", "--format"],
        ("env", Some("template")) => &["--scope", "--validate-only", "--format"],
        ("env", Some("run")) => &[
            "--env",
            "--set",
            "--scope",
            "--shell",
            "--schema-check",
            "--notify",
        ],
        ("env", Some("validate")) => &["--scope", "--format", "--strict"],
        ("env", Some("schema")) => &["--format", "--warn-only", "-y", "--yes"],
        ("env", Some("annotate")) => &["--format"],
        ("env", Some("config")) => &["--format", "-y", "--yes"],
        ("env", Some("audit")) => &["--limit", "--format"],
        ("env", Some("watch")) => &["--scope", "--interval-ms", "--format", "--once"],
        _ => &[],
    }
}

pub(super) fn value_flags_for(subcmd: &str, subsub: Option<&str>) -> &'static [&'static str] {
    match (subcmd, subsub) {
        ("ctx", Some("set")) => &[
            "--path",
            "--proxy",
            "--noproxy",
            "-t",
            "--tag",
            "--env",
            "--env-file",
        ],
        ("ctx", Some("list")) | ("ctx", Some("show")) => &["-f", "--format"],
        ("delete", _) | ("del", _) => &[
            "--name",
            "-e",
            "--exclude",
            "-p",
            "--pattern",
            "--log",
            "--level",
            "-f",
            "--format",
            "--reason",
        ],
        ("list", _) => &[
            "-t", "--tag", "-s", "--sort", "-n", "--limit", "--offset", "-f", "--format",
        ],
        ("acl", Some("view")) => &["--path", "-p", "--export"],
        ("acl", Some("add")) => &[
            "--path",
            "-p",
            "--principal",
            "--rights",
            "--ace-type",
            "--inherit",
        ],
        ("acl", Some("purge")) => &["--path", "-p", "--principal"],
        ("acl", Some("diff")) => &["--path", "-p", "--reference", "-r", "--output", "-o"],
        ("acl", Some("batch")) => &["--file", "--paths", "--action", "--output"],
        ("acl", Some("effective")) => &["--path", "-p", "--user", "-u"],
        ("acl", Some("copy")) => &["--path", "-p", "--reference", "-r"],
        ("acl", Some("backup")) => &["--path", "-p", "--output", "-o"],
        ("acl", Some("restore")) => &["--path", "-p", "--from"],
        ("acl", Some("inherit")) => &["--path", "-p", "--preserve"],
        ("acl", Some("owner")) => &["--path", "-p", "--set"],
        ("acl", Some("orphans")) => &["--path", "-p", "--recursive", "--action", "--output"],
        ("acl", Some("repair")) => &["--path", "-p"],
        ("acl", Some("audit")) => &["--tail", "--export"],
        ("acl", Some("config")) => &["--set"],
        ("save", _) | ("set", _) | ("tag", _) | ("rename", _) => &["-t", "--tag"],
        ("tree", _) => &[
            "-d",
            "--depth",
            "-o",
            "--output",
            "--sort",
            "--size",
            "--max-items",
            "--include",
            "--exclude",
        ],
        ("redirect", _) => &[
            "--profile",
            "--tx",
            "--last",
            "--plan",
            "--apply",
            "--undo",
            "-f",
            "--format",
        ],
        ("export", _) => &["-f", "--format", "-o", "--out"],
        ("import", _) => &["-f", "--format", "-i", "--input", "-m", "--mode"],
        ("proxy", Some("set")) => &["-n", "--noproxy", "-m", "--msys2", "-o", "--only"],
        ("proxy", Some("del")) => &["-m", "--msys2", "-o", "--only"],
        ("proxy", Some("test")) => &["-t", "--targets", "-w", "--timeout"],
        ("env", Some("list")) => &["--scope", "-f", "--format"],
        ("env", Some("search")) => &["--scope", "-f", "--format"],
        ("env", Some("get")) => &["--scope", "-f", "--format"],
        ("env", Some("set")) => &["--scope"],
        ("env", Some("del")) => &["--scope"],
        ("env", Some("check")) => &["--scope", "--format"],
        ("env", Some("doctor")) => &["--scope", "--format"],
        ("env", Some("path-dedup")) => &["--scope"],
        ("env", Some("profile")) => &["--scope", "-f", "--format"],
        ("env", Some("batch")) => &["--scope"],
        ("env", Some("apply")) => &["--scope"],
        ("env", Some("export")) => &["--scope", "--format", "--out"],
        ("env", Some("export-live")) => &["--scope", "--format", "--env", "--set", "--out"],
        ("env", Some("env")) => &["--scope", "--format", "--env", "--set"],
        ("env", Some("import")) => &["--scope", "--mode"],
        ("env", Some("diff-live")) => &["--scope", "--snapshot", "--format"],
        ("env", Some("template")) => &["--scope", "--format"],
        ("env", Some("run")) => &["--env", "--set", "--scope", "--shell"],
        ("env", Some("validate")) => &["--scope", "--format"],
        ("env", Some("schema")) => &["--format"],
        ("env", Some("annotate")) => &["--format"],
        ("env", Some("config")) => &["--format"],
        ("env", Some("audit")) => &["--limit", "--format"],
        ("env", Some("watch")) => &["--scope", "--interval-ms", "--format"],
        _ => &[],
    }
}

pub(super) fn flag_takes_value(subcmd: &str, subsub: Option<&str>, flag: &str) -> bool {
    value_flags_for(subcmd, subsub).iter().any(|f| *f == flag)
}

pub(super) fn value_candidates(
    subcmd: &str,
    subsub: Option<&str>,
    flag: &str,
    prefix_lower: &str,
) -> Vec<CompletionItem> {
    if flag == "-f" || flag == "--format" {
        if subcmd == "import" || subcmd == "export" {
            return static_candidates(IO_FORMATS, prefix_lower);
        }
        return static_candidates(FORMATS, prefix_lower);
    }
    if flag == "-s" || flag == "--sort" {
        if subcmd == "list" {
            return static_candidates(LIST_SORTS, prefix_lower);
        }
        if subcmd == "tree" {
            return static_candidates(TREE_SORTS, prefix_lower);
        }
    }
    if subcmd == "redirect" && flag == "--profile" {
        return dynamic_profiles(prefix_lower);
    }
    if subcmd == "redirect" && (flag == "--undo" || flag == "--tx") {
        return dynamic_txs(prefix_lower);
    }
    if subcmd == "ctx" && subsub == Some("set") && flag == "--proxy" {
        return static_candidates(&["keep", "off"], prefix_lower);
    }
    if subcmd == "import" && (flag == "-m" || flag == "--mode") {
        return static_candidates(IMPORT_MODES, prefix_lower);
    }
    if subcmd == "env" {
        if flag == "--scope" {
            if matches!(
                subsub,
                Some("set")
                    | Some("del")
                    | Some("import")
                    | Some("path")
                    | Some("path-dedup")
                    | Some("profile")
                    | Some("batch")
                    | Some("apply")
            ) {
                return static_candidates(ENV_WRITE_SCOPES, prefix_lower);
            }
            return static_candidates(ENV_SCOPES, prefix_lower);
        }
        if flag == "--mode" && subsub == Some("import") {
            return static_candidates(ENV_IMPORT_MODES, prefix_lower);
        }
        if flag == "--format" && subsub == Some("export") {
            return static_candidates(ENV_EXPORT_FORMATS, prefix_lower);
        }
        if flag == "--format" && subsub == Some("export-live") {
            return static_candidates(&["dotenv", "sh", "json", "reg"], prefix_lower);
        }
        if flag == "--format" && subsub == Some("env") {
            return static_candidates(&["text", "json"], prefix_lower);
        }
        if flag == "--format"
            && matches!(
                subsub,
                Some("doctor")
                    | Some("diff-live")
                    | Some("template")
                    | Some("validate")
                    | Some("annotate")
                    | Some("config")
                    | Some("audit")
                    | Some("watch")
            )
        {
            return static_candidates(&["text", "json"], prefix_lower);
        }
        if flag == "--scope" && subsub == Some("run") {
            return static_candidates(ENV_SCOPES, prefix_lower);
        }
        if flag == "--scope" && subsub == Some("watch") {
            return static_candidates(ENV_SCOPES, prefix_lower);
        }
        if flag == "--shell" && subsub == Some("run") {
            return static_candidates(&["bash", "powershell", "cmd"], prefix_lower);
        }
    }
    if subcmd == "config" && (subsub == Some("get") || subsub == Some("set")) {
        return dynamic_config_keys(prefix_lower);
    }
    Vec::new()
}

pub(super) fn value_directive(
    subcmd: &str,
    subsub: Option<&str>,
    flag: &str,
) -> (u32, Option<&'static str>) {
    if subcmd == "ctx" && subsub == Some("set") && flag == "--path" {
        return (FILTER_DIRS, None);
    }
    if subcmd == "ctx" && subsub == Some("set") && flag == "--env-file" {
        return (NO_FILE_COMP, None);
    }
    if subcmd == "import" && (flag == "-i" || flag == "--input") {
        return (FILTER_EXT, Some("json|tsv"));
    }
    if subcmd == "export" && (flag == "-o" || flag == "--out") {
        return (FILTER_EXT, Some("json|tsv"));
    }
    if subcmd == "env" && subsub == Some("export") && flag == "--out" {
        return (FILTER_EXT, Some("json|env|reg|csv"));
    }
    if subcmd == "env" && subsub == Some("export-live") && flag == "--out" {
        return (FILTER_EXT, Some("env|sh|json|reg"));
    }
    if subcmd == "env"
        && matches!(subsub, Some("run") | Some("export-live") | Some("env"))
        && flag == "--env"
    {
        return (FILTER_EXT, Some("env|json|reg|csv"));
    }
    (NO_FILE_COMP, None)
}

pub(super) fn positional_candidates(
    subcmd: &str,
    subsub: Option<&str>,
    index: usize,
    prefix_lower: &str,
    cwd: Option<&str>,
    bookmark_mode: bool,
) -> (Vec<CompletionItem>, u32) {
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
            Some("use") | Some("del") | Some("show") | Some("rename") | Some("set")
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
    if matches!(subcmd, "delete" | "del") {
        if bookmark_mode {
            return (bookmark_candidates(prefix_lower, cwd), NO_FILE_COMP);
        }
        return (Vec::new(), 0);
    }
    if matches!(subcmd, "z" | "open" | "touch" | "rename") && index == 0 {
        return (bookmark_candidates(prefix_lower, cwd), NO_FILE_COMP);
    }
    (Vec::new(), NO_FILE_COMP)
}

fn bookmark_candidates(prefix_lower: &str, cwd: Option<&str>) -> Vec<CompletionItem> {
    let db = cached_db();
    let mut scored: Vec<(f64, String)> = Vec::new();
    for (name, entry) in db.iter() {
        if !starts_with_ci(name, prefix_lower) {
            continue;
        }
        let mut score = frecency(entry);
        if let Some(cwd) = cwd {
            score *= cwd_boost(cwd, &entry.path);
        }
        scored.push((score, name.clone()));
    }
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    scored
        .into_iter()
        .map(|(_, name)| CompletionItem::new(name))
        .collect()
}

#[cfg(feature = "redirect")]
fn dynamic_profiles(prefix_lower: &str) -> Vec<CompletionItem> {
    let (_, profiles) = cached_config_keys_and_profiles();
    profiles
        .iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}

fn dynamic_ctx_profiles(prefix_lower: &str) -> Vec<CompletionItem> {
    let profiles = cached_ctx_profiles();
    profiles
        .iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}

#[cfg(not(feature = "redirect"))]
fn dynamic_profiles(_prefix_lower: &str) -> Vec<CompletionItem> {
    Vec::new()
}

#[cfg(feature = "redirect")]
fn dynamic_txs(prefix_lower: &str) -> Vec<CompletionItem> {
    let txs = cached_audit_txs();
    txs.iter()
        .filter(|tx| starts_with_ci(tx, prefix_lower))
        .map(|tx| CompletionItem::new(tx.clone()))
        .collect()
}

#[cfg(not(feature = "redirect"))]
fn dynamic_txs(_prefix_lower: &str) -> Vec<CompletionItem> {
    Vec::new()
}

fn dynamic_config_keys(prefix_lower: &str) -> Vec<CompletionItem> {
    let (keys, _) = cached_config_keys_and_profiles();
    keys.iter()
        .filter(|k| starts_with_ci(k, prefix_lower))
        .map(|k| CompletionItem::new(k.clone()))
        .collect()
}
