use super::dynamic_config::dynamic_config_keys;
use super::dynamic_profiles::{dynamic_profiles, dynamic_txs};
use super::*;

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
