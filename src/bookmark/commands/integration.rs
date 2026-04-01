use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::bookmark::storage::db_path;
use crate::bookmark::undo::record_undo_batch;
use crate::bookmark_core::{BookmarkSource, normalize_name};
use crate::bookmark_state::Store;
use crate::cli::{BookmarkInitCmd, ImportCmd, LearnCmd};
use crate::config;
use crate::model::{ImportMode, IoFormat, ListItem, parse_import_mode, parse_io_format};
use crate::output::{CliError, CliResult};

pub(crate) fn cmd_learn(args: LearnCmd) -> CliResult {
    let cfg = config::load_config();
    if !cfg.bookmark.auto_learn.enabled {
        return Ok(());
    }

    let excludes = effective_excludes(&cfg.bookmark.exclude_dirs);
    if is_excluded(&args.path, &excludes) {
        return Ok(());
    }

    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|err| CliError::new(1, format!("load failed: {err}")))?;
    let home = home_dir();
    store
        .learn(&args.path, &current_dir_fallback(), home.as_deref(), crate::store::now_secs())
        .map_err(|err| CliError::new(1, format!("learn failed: {err}")))?;
    store
        .save(&file, crate::store::now_secs())
        .map_err(|err| CliError::new(1, format!("save failed: {err}")))?;
    Ok(())
}

pub(crate) fn cmd_bookmark_import(args: ImportCmd) -> CliResult {
    let file = db_path();
    let mut store =
        Store::load_or_default(&file).map_err(|err| CliError::new(1, format!("load failed: {err}")))?;
    let before = store.clone();
    let now = crate::store::now_secs();
    let cwd = current_dir_fallback();
    let home = home_dir();
    let mode = parse_import_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: merge | overwrite"],
        )
    })?;

    let imported = if let Some(from) = args.from.as_deref() {
        import_from_external(from, args.input.as_deref(), now)?
    } else {
        import_from_native(&args)?
    };
    let imported_count = imported.len();

    if mode == ImportMode::Overwrite && imported_count > 0 && !args.yes {
        return Err(CliError::with_details(
            2,
            "Overwrite import requires --yes.".to_string(),
            &["Fix: Re-run with `--yes`, or switch back to `--mode merge`."],
        ));
    }

    let native_import = imported
        .iter()
        .all(|seed| seed.source == BookmarkSource::Explicit);
    if mode == ImportMode::Overwrite && !native_import {
        store
            .bookmarks
            .retain(|bookmark| bookmark.source == BookmarkSource::Explicit);
    }

    for item in imported {
        if item.source == BookmarkSource::Explicit {
            apply_native_import(&mut store, &item, &cwd, home.as_deref(), now, mode)?;
        } else {
            store
                .import_entry(&item.path, &cwd, home.as_deref(), item.score, now)
                .map_err(|err| CliError::new(1, format!("import failed: {err}")))?;
        }
    }

    store
        .save(&file, now)
        .map_err(|err| CliError::new(1, format!("save failed: {err}")))?;
    let after = store.clone();
    if let Err(err) = record_undo_batch(&file, "import", &before, &after) {
        crate::output::emit_warning(
            format!("Undo history not recorded: {}", err.message),
            &[],
        );
    }
    ui_println!("Imported {} item(s).", imported_count);
    Ok(())
}

fn apply_native_import(
    store: &mut Store,
    seed: &ImportSeed,
    cwd: &PathBuf,
    home: Option<&std::path::Path>,
    now: u64,
    mode: ImportMode,
) -> CliResult {
    let name = seed
        .name
        .as_deref()
        .ok_or_else(|| CliError::new(1, "native import entry missing name"))?;
    let name_norm = normalize_name(name);

    let exists = store.bookmarks.iter().any(|bookmark| {
        bookmark.source == BookmarkSource::Explicit
            && bookmark.name_norm.as_deref() == Some(&name_norm)
    });

    if !seed.path.trim().is_empty() {
        store
            .set(name, &seed.path, cwd, home, now)
            .map_err(|err| CliError::new(1, format!("import failed: {err}")))?;
    } else if !exists {
        return Err(CliError::with_details(
            2,
            format!("Import entry '{}' is missing a path.", name),
            &["Fix: Ensure exported JSON/TSV contains a non-empty path."],
        ));
    }

    let bookmark = store
        .bookmarks
        .iter_mut()
        .find(|bookmark| {
            bookmark.source == BookmarkSource::Explicit
                && bookmark.name_norm.as_deref() == Some(&name_norm)
        })
        .ok_or_else(|| CliError::new(1, format!("Import upsert failed for '{name}'.")))?;

    match mode {
        ImportMode::Merge => {
            merge_tags(&mut bookmark.tags, &seed.tags);
            let visits = seed.visits.unwrap_or(0);
            bookmark.visit_count = Some(bookmark.visit_count.unwrap_or(0).max(visits));
            bookmark.last_visited = match (bookmark.last_visited, seed.last_visited) {
                (Some(left), Some(right)) => Some(left.max(right)),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            };
        }
        ImportMode::Overwrite => {
            bookmark.tags = dedup_case_insensitive(&seed.tags);
            bookmark.visit_count = Some(seed.visits.unwrap_or(0));
            bookmark.last_visited = seed.last_visited;
        }
    }

    bookmark.name = Some(name.to_string());
    bookmark.name_norm = Some(name_norm);
    Ok(())
}

pub(crate) fn cmd_bookmark_init(args: BookmarkInitCmd) -> CliResult {
    let script = render_bookmark_init(&args.shell, args.cmd.as_deref().unwrap_or("z"))?;
    out_println!("{script}");
    Ok(())
}

pub(crate) fn render_bookmark_init(shell: &str, prefix: &str) -> CliResult<String> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Err(CliError::new(2, "bookmark init --cmd cannot be empty."));
    }

    let fzf_opts = crate::config::bookmark_fzf_opts();
    let script = match shell.to_ascii_lowercase().as_str() {
        "powershell" | "pwsh" => generate_powershell_init(prefix, &fzf_opts),
        "bash" | "zsh" => generate_bash_init(prefix, &fzf_opts),
        "fish" => generate_fish_init(prefix, &fzf_opts),
        other => {
            return Err(CliError::new(
                2,
                format!("Unsupported bookmark shell: {other}."),
            ))
        }
    };
    Ok(script)
}

#[derive(Clone)]
struct ImportSeed {
    name: Option<String>,
    path: String,
    tags: Vec<String>,
    visits: Option<u32>,
    last_visited: Option<u64>,
    score: f64,
    source: BookmarkSource,
}

fn import_from_native(args: &ImportCmd) -> CliResult<Vec<ImportSeed>> {
    let format = parse_io_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: json | tsv"],
        )
    })?;
    let _mode = parse_import_mode(&args.mode).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid mode: {}.", args.mode),
            &["Fix: Use one of: merge | overwrite"],
        )
    })?;
    let content = read_optional_input(args.input.as_deref())?;
    match format {
        IoFormat::Json => {
            let parsed: Vec<ListItem> = serde_json::from_str(&content)
                .map_err(|e| CliError::new(1, format!("import json error: {e}")))?;
            Ok(parsed
                .into_iter()
                .map(|item| ImportSeed {
                    name: Some(item.name),
                    path: item.path,
                    tags: item.tags,
                    visits: Some(item.visits),
                    last_visited: Some(item.last_visited),
                    score: item.visits.max(1) as f64,
                    source: BookmarkSource::Explicit,
                })
                .collect())
        }
        IoFormat::Tsv => Ok(parse_native_tsv(&content)),
    }
}

fn import_from_external(from: &str, input: Option<&str>, now: u64) -> CliResult<Vec<ImportSeed>> {
    match from.to_ascii_lowercase().as_str() {
        "autojump" => {
            let path = input
                .map(PathBuf::from)
                .unwrap_or_else(default_autojump_path);
            let content = fs::read_to_string(&path)
                .map_err(|e| CliError::new(1, format!("failed to read autojump db: {e}")))?;
            Ok(parse_autojump(&content))
        }
        "zoxide" => {
            let output = std::process::Command::new("zoxide")
                .args(["query", "--list", "--score"])
                .output()
                .map_err(|e| CliError::new(1, format!("failed to execute zoxide: {e}")))?;
            if !output.status.success() {
                return Err(CliError::new(1, "zoxide query --list --score failed"));
            }
            Ok(parse_zoxide_output(&String::from_utf8_lossy(&output.stdout)))
        }
        "z" => {
            let path = input.map(PathBuf::from).unwrap_or_else(default_z_path);
            let content = fs::read_to_string(&path)
                .map_err(|e| CliError::new(1, format!("failed to read z db: {e}")))?;
            Ok(parse_z_family(&content))
        }
        "fasd" => {
            let path = input.map(PathBuf::from).unwrap_or_else(default_fasd_path);
            let content = fs::read_to_string(&path)
                .map_err(|e| CliError::new(1, format!("failed to read fasd db: {e}")))?;
            Ok(parse_fasd(&content))
        }
        "history" => {
            let path = input
                .map(PathBuf::from)
                .unwrap_or_else(default_powershell_history_path);
            let content = fs::read_to_string(&path)
                .map_err(|e| CliError::new(1, format!("failed to read history: {e}")))?;
            let parsed = parse_shell_history(&content);
            let seed_score = (average_prefill_seed(now) * 0.3).max(1.0);
            Ok(parsed
                .into_iter()
                .map(|path| ImportSeed {
                    name: None,
                    path,
                    tags: Vec::new(),
                    visits: None,
                    last_visited: None,
                    score: seed_score,
                    source: BookmarkSource::Imported,
                })
                .collect())
        }
        other => Err(CliError::new(2, format!("unsupported import source: {other}"))),
    }
}

fn read_optional_input(input: Option<&str>) -> CliResult<String> {
    if let Some(path) = input {
        return fs::read_to_string(path).map_err(|e| CliError::new(1, format!("read failed: {e}")));
    }
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| CliError::new(1, format!("stdin read failed: {e}")))?;
    Ok(buf)
}

fn parse_native_tsv(content: &str) -> Vec<ImportSeed> {
    let mut out = Vec::new();
    for line in content.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 2 {
            continue;
        }
        let path = cols[1].trim();
        if path.is_empty() {
            continue;
        }
        let score = cols
            .get(3)
            .and_then(|value| value.trim().parse::<f64>().ok())
            .unwrap_or(1.0);
        out.push(ImportSeed {
            name: Some(cols[0].trim().to_string()),
            path: path.to_string(),
            tags: cols
                .get(2)
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|tag| !tag.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            visits: cols
                .get(3)
                .and_then(|value| value.trim().parse::<u32>().ok()),
            last_visited: cols
                .get(4)
                .and_then(|value| value.trim().parse::<u64>().ok()),
            score,
            source: BookmarkSource::Explicit,
        });
    }
    out
}

fn parse_autojump(content: &str) -> Vec<ImportSeed> {
    let mut out = Vec::new();
    for line in content.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 2 {
            continue;
        }
        if let Ok(score) = cols[0].trim().parse::<f64>() {
            let path = cols[1].trim();
            if !path.is_empty() {
                out.push(ImportSeed {
                    name: None,
                    path: path.to_string(),
                    tags: Vec::new(),
                    visits: None,
                    last_visited: None,
                    score,
                    source: BookmarkSource::Imported,
                });
            }
        }
    }
    out
}

fn parse_z_family(content: &str) -> Vec<ImportSeed> {
    let mut out = Vec::new();
    for line in content.lines() {
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() != 3 {
            continue;
        }
        let path_first = cols[0].trim();
        let path_last = cols[2].trim();
        let first_is_path = path_first.contains('/') || path_first.contains('\\');
        let (path, score_col) = if first_is_path {
            (path_first, cols[1].trim())
        } else {
            (path_last, cols[0].trim())
        };
        if let Ok(score) = score_col.parse::<f64>() {
            out.push(ImportSeed {
                name: None,
                path: path.to_string(),
                tags: Vec::new(),
                visits: None,
                last_visited: None,
                score,
                source: BookmarkSource::Imported,
            });
        }
    }
    out
}

fn parse_fasd(content: &str) -> Vec<ImportSeed> {
    let mut out = Vec::new();
    for line in content.lines() {
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() != 4 {
            continue;
        }
        if cols[3].trim() != "d" {
            continue;
        }
        if let Ok(score) = cols[1].trim().parse::<f64>() {
            out.push(ImportSeed {
                name: None,
                path: cols[0].trim().to_string(),
                tags: Vec::new(),
                visits: None,
                last_visited: None,
                score,
                source: BookmarkSource::Imported,
            });
        }
    }
    out
}

fn parse_zoxide_output(content: &str) -> Vec<ImportSeed> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let Some(score_raw) = parts.next() else { continue };
        let Some(path_raw) = parts.next() else { continue };
        if let Ok(score) = score_raw.trim().parse::<f64>() {
            let path = path_raw.trim();
            if !path.is_empty() {
                out.push(ImportSeed {
                    name: None,
                    path: path.to_string(),
                    tags: Vec::new(),
                    visits: None,
                    last_visited: None,
                    score,
                    source: BookmarkSource::Imported,
                });
            }
        }
    }
    out
}

fn parse_shell_history(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        for prefix in ["cd ", "z ", "j "] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let path = rest.trim().trim_matches('"');
                if !path.is_empty() {
                    out.push(path.replace('\\', "/"));
                }
            }
        }
    }
    out
}

fn average_prefill_seed(_now: u64) -> f64 {
    50.0
}

fn merge_tags(existing: &mut Vec<String>, incoming: &[String]) {
    for tag in incoming {
        if !existing
            .iter()
            .any(|current| current.eq_ignore_ascii_case(tag))
        {
            existing.push(tag.clone());
        }
    }
}

fn dedup_case_insensitive(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        if !out.iter().any(|current: &String| current.eq_ignore_ascii_case(tag)) {
            out.push(tag.clone());
        }
    }
    out
}

fn effective_excludes(config_excludes: &[String]) -> Vec<String> {
    if let Some(raw) = env::var("_BM_EXCLUDE_DIRS").ok().filter(|v| !v.trim().is_empty()) {
        let separator = if cfg!(windows) { ';' } else { ':' };
        return raw
            .split(separator)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
    }
    config_excludes.to_vec()
}

fn is_excluded(path: &str, excludes: &[String]) -> bool {
    let path_norm = path.replace('\\', "/").to_ascii_lowercase();
    excludes.iter().any(|exclude| {
        let ex = exclude.replace('\\', "/").to_ascii_lowercase();
        path_norm.ends_with(&format!("/{ex}")) || path_norm.contains(&format!("/{ex}/"))
    })
}

fn current_dir_fallback() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .ok()
        .or_else(|| env::var("HOME").ok())
        .map(PathBuf::from)
}

fn default_autojump_path() -> PathBuf {
    if cfg!(windows) {
        let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        return PathBuf::from(appdata).join("autojump").join("autojump.txt");
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("autojump")
        .join("autojump.txt")
}

fn default_z_path() -> PathBuf {
    let home = env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .unwrap_or_else(|| ".".to_string());
    PathBuf::from(home).join(".z")
}

fn default_fasd_path() -> PathBuf {
    let home = env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .unwrap_or_else(|| ".".to_string());
    PathBuf::from(home).join(".fasd")
}

fn default_powershell_history_path() -> PathBuf {
    let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("PowerShell")
        .join("PSReadLine")
        .join("ConsoleHost_history.txt")
}

fn generate_powershell_init(prefix: &str, fzf_opts: &str) -> String {
    let open = if prefix == "z" {
        "o".to_string()
    } else {
        format!("{prefix}o")
    };
    format!(
        r#"$__bm_exe = if ($env:XUN_BM_EXE) {{ $env:XUN_BM_EXE }} elseif (Get-Command bm.exe -ErrorAction SilentlyContinue) {{ "bm.exe" }} elseif (Get-Command bm -CommandType Application -ErrorAction SilentlyContinue) {{ "bm" }} elseif ($env:XUN_EXE) {{ $env:XUN_EXE }} else {{ "xun.exe" }}
$__bm_prefix = if ($__bm_exe -match '(?i)(^|[\\/])(xun|xyu)(\.exe)?$') {{ @('bookmark') }} else {{ @() }}
if (-not $env:_BM_FZF_OPTS -and "{fzf_opts}" -ne "") {{ $env:_BM_FZF_OPTS = "{fzf_opts}" }}
function __bm_invoke {{ & $__bm_exe @($__bm_prefix + $args) }}
function bm {{ __bm_invoke @args }}
function {p} {{ $result = __bm_invoke z @args; if ($result -match '^__BM_CD__ (.+)$') {{ Set-Location $Matches[1] }} elseif ($result) {{ Write-Output $result }} }}
function {p}i {{ $result = __bm_invoke zi @args; if ($result -match '^__BM_CD__ (.+)$') {{ Set-Location $Matches[1] }} }}
function {o} {{ __bm_invoke o @args }}
function {o}i {{ __bm_invoke oi @args }}
$__bm_prev_pwd = $PWD.Path
function __bm_hook {{
    $cur = $PWD.Path
    if ($cur -ne $__bm_prev_pwd) {{
        Start-Process -FilePath $__bm_exe -ArgumentList @($__bm_prefix + @('learn','--path',$cur)) -WindowStyle Hidden | Out-Null
        $script:__bm_prev_pwd = $cur
    }}
}}
Register-ArgumentCompleter -CommandName @('{p}','{p}i','{o}','{o}i') -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $candidates = __bm_invoke z --list --tsv $wordToComplete 2>$null
    $candidates | ForEach-Object {{
        $parts = $_ -split "`t"
        [System.Management.Automation.CompletionResult]::new($parts[0], $parts[0], 'ParameterValue', $parts[1])
    }}
}}
$__bm_subcommands = @('z','zi','o','oi','open','save','set','delete','tag','pin','unpin','undo','redo','rename','list','recent','stats','check','gc','dedup','export','import','init','touch','learn','keys','all')
Register-ArgumentCompleter -CommandName 'bm' -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $__bm_subcommands | Where-Object {{ $_ -like "$wordToComplete*" }} | ForEach-Object {{
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }}
}}
# Set-Alias cd {p}
"#,
        p = prefix,
        o = open,
        fzf_opts = escape_powershell_double_quoted(fzf_opts)
    )
}

fn generate_bash_init(prefix: &str, fzf_opts: &str) -> String {
    let open = if prefix == "z" {
        "o".to_string()
    } else {
        format!("{prefix}o")
    };
    format!(
        r#"# generated by `xun bookmark init bash`
__bm_exe="${{XUN_BM_EXE:-}}"
if [[ -z "$__bm_exe" ]]; then
    if command -v bm.exe &>/dev/null; then __bm_exe="bm.exe";
    elif command -v bm &>/dev/null; then __bm_exe="bm";
    else __bm_exe="${{XUN_EXE:-xun.exe}}";
    fi
fi
if [[ -z "$_BM_FZF_OPTS" && "{fzf_opts}" != "" ]]; then export _BM_FZF_OPTS="{fzf_opts}"; fi
__bm_invoke() {{
    case "$__bm_exe" in
        *xun.exe|*xun|xun|*xyu.exe|*xyu|xyu) "$__bm_exe" bookmark "$@" ;;
        *) "$__bm_exe" "$@" ;;
    esac
}}
function bm() {{ __bm_invoke "$@"; }}
function {p}() {{
    local result
    result=$(__bm_invoke z "$@")
    if [[ "$result" == __BM_CD__* ]]; then
        builtin cd "${{result#__BM_CD__ }}"
    else
        echo "$result"
    fi
}}
function {p}i() {{ local r=$(__bm_invoke zi "$@"); [[ "$r" == __BM_CD__* ]] && builtin cd "${{r#__BM_CD__ }}"; }}
function {o}()  {{ __bm_invoke o "$@"; }}
function {o}i() {{ __bm_invoke oi "$@"; }}
__bm_hook() {{ __bm_invoke learn --path "$PWD" &>/dev/null & }}
[[ "$PROMPT_COMMAND" != *__bm_hook* ]] && PROMPT_COMMAND="__bm_hook;${{PROMPT_COMMAND}}"
_bm_query_complete() {{
    COMPREPLY=( $(__bm_invoke z --list --tsv "${{COMP_WORDS[COMP_CWORD]}}" 2>/dev/null | cut -f1) )
}}
_bm_root_complete() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local sub="${{COMP_WORDS[1]}}"
    local subcommands="z zi o oi open save set delete tag pin unpin undo redo rename list recent stats check gc dedup export import init touch learn keys all"
    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
        return
    fi
    if [[ "$sub" == "z" || "$sub" == "zi" || "$sub" == "o" || "$sub" == "oi" ]]; then
        COMPREPLY=( $(__bm_invoke z --list --tsv "$cur" 2>/dev/null | cut -f1) )
        return
    fi
    COMPREPLY=()
}}
complete -F _bm_query_complete {p} {p}i {o} {o}i
complete -F _bm_root_complete bm
# alias cd='{p}'
"#,
        p = prefix,
        o = open,
        fzf_opts = escape_shell_double_quoted(fzf_opts)
    )
}

fn generate_fish_init(prefix: &str, fzf_opts: &str) -> String {
    let open = if prefix == "z" {
        "o".to_string()
    } else {
        format!("{prefix}o")
    };
    format!(
        r#"# generated by `xun bookmark init fish`
if test -n "$XUN_BM_EXE"
    set -g __bm_exe $XUN_BM_EXE
else if command -sq bm.exe
    set -g __bm_exe bm.exe
else if command -sq bm
    set -g __bm_exe bm
else if test -n "$XUN_EXE"
    set -g __bm_exe $XUN_EXE
else
    set -g __bm_exe xun.exe
end

if string match -rq '(?i)(^|[\\/])(xun|xyu)(\.exe)?$' -- $__bm_exe
    set -g __bm_prefix bookmark
else
    set -g __bm_prefix
end

if test -z "$_BM_FZF_OPTS"; and test "{fzf_opts}" != ""
    set -gx _BM_FZF_OPTS "{fzf_opts}"
end

function __bm_invoke
    if test -n "$__bm_prefix"
        command $__bm_exe $__bm_prefix $argv
    else
        command $__bm_exe $argv
    end
end

function bm
    __bm_invoke $argv
end

function {p}
    set -l result (__bm_invoke z $argv)
    if string match -rq '^__BM_CD__ ' -- $result
        cd (string replace '__BM_CD__ ' '' -- $result)
    else if test -n "$result"
        printf '%s\n' $result
    end
end

function {p}i
    set -l result (__bm_invoke zi $argv)
    if string match -rq '^__BM_CD__ ' -- $result
        cd (string replace '__BM_CD__ ' '' -- $result)
    end
end

function {o}
    __bm_invoke o $argv
end

function {o}i
    __bm_invoke oi $argv
end

function __bm_hook --on-variable PWD
    __bm_invoke learn --path $PWD >/dev/null 2>/dev/null &
end

complete -c bm -f -a "z zi o oi open save set delete tag pin unpin undo redo rename list recent stats check gc dedup export import init touch learn keys all"
complete -c {p} -f -a "(__bm_invoke z --list --tsv (commandline -ct) 2>/dev/null | string split '\t' | sed -n '1~3p')"
complete -c {p}i -f -a "(__bm_invoke z --list --tsv (commandline -ct) 2>/dev/null | string split '\t' | sed -n '1~3p')"
complete -c {o} -f -a "(__bm_invoke z --list --tsv (commandline -ct) 2>/dev/null | string split '\t' | sed -n '1~3p')"
complete -c {o}i -f -a "(__bm_invoke z --list --tsv (commandline -ct) 2>/dev/null | string split '\t' | sed -n '1~3p')"
"#,
        p = prefix,
        o = open,
        fzf_opts = escape_shell_double_quoted(fzf_opts)
    )
}

fn escape_shell_double_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_powershell_double_quoted(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_autojump_database() {
        let parsed = parse_autojump("10\tC:/work/foo\n");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, "C:/work/foo");
    }

    #[test]
    fn parse_zoxide_query_output_with_spaces_in_path() {
        let parsed = parse_zoxide_output("87.3  C:/Users/dev/My Project\n");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, "C:/Users/dev/My Project");
    }

    #[test]
    fn parse_fasd_only_dirs() {
        let parsed = parse_fasd("C:/work/foo|10|100|d\nC:/work/file.txt|10|100|f\n");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, "C:/work/foo");
    }

    #[test]
    fn parse_shell_history_extracts_cd_paths() {
        let parsed = parse_shell_history("cd C:\\work\\foo\nz C:\\work\\bar\n");
        assert_eq!(parsed, vec!["C:/work/foo", "C:/work/bar"]);
    }

    #[test]
    fn ps_init_uses_start_process_not_start_job() {
        let script = generate_powershell_init("z", "");
        assert!(script.contains("Start-Process"));
        assert!(!script.contains("Start-Job"));
        assert!(script.contains("Register-ArgumentCompleter"));
    }

    #[test]
    fn ps_init_with_cmd_prefix_j() {
        let script = generate_powershell_init("j", "");
        assert!(script.contains("function j"));
        assert!(script.contains("function ji"));
        assert!(script.contains("function jo"));
        assert!(script.contains("function joi"));
    }

    #[test]
    fn bash_init_does_not_eval_generate_itself() {
        let script = generate_bash_init("z", "");
        assert!(!script.contains("eval \"$(xun bookmark init bash)\""));
        assert!(script.contains("_bm_root_complete"));
        assert!(script.contains("_bm_query_complete"));
    }

    #[test]
    fn fish_init_uses_native_functions() {
        let script = generate_fish_init("z", "--height 40%");
        assert!(script.contains("function __bm_invoke"));
        assert!(script.contains("complete -c bm"));
        assert!(script.contains("function zi"));
        assert!(script.contains("_BM_FZF_OPTS"));
    }
}
