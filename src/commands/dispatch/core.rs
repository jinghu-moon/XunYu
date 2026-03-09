use crate::cli::{InitCmd, SubCommand};
use crate::output::{CliError, CliResult};

fn cmd_init(args: InitCmd) -> CliResult {
    let ps_script = r#"
$xun = if ($env:XUN_EXE) { $env:XUN_EXE } else { "xun.exe" }
function xun { & $xun @args }
Set-Alias xyu xun
Set-Alias xy xun

if (-not (Get-Variable XunHooks -Scope Global -ErrorAction SilentlyContinue)) {
    $global:XunHooks = @{}
}

$global:XunSubcommands = @(
    "acl","alias","init","completion","config","ctx","list","z","open","ws","save","set","delete","del","check","gc","touch","rename","tag",
    "recent","stats","dedup","export","import","proxy","pon","poff","pst","px","ports","kill","ps","pkill","keys","all","fuzzy",
    "bak","tree","find","env","img","video","lock","rm","mv","renfile","protect","encrypt","decrypt","serve","redirect",
    "brn","cstat"
)
$global:XunProxySubcommands = @("set","del","get","detect","test")
$global:XunCtxSubcommands = @("set","use","off","list","show","del","rename")
$global:XunAclSubcommands = @("view","add","remove","purge","diff","batch","effective","copy","backup","restore","inherit","owner","orphans","repair","audit","config")
$global:XunFormats = @("auto","table","tsv","json")
$global:XunTreeSort = @("name","mtime","size")

function _xun_config_path {
    if ($env:XUN_CONFIG) { return $env:XUN_CONFIG }
    return (Join-Path $env:USERPROFILE ".xun.config.json")
}

function _xun_db_path {
    if ($env:XUN_DB) { return $env:XUN_DB }
    return (Join-Path $env:USERPROFILE ".xun.json")
}

function _xun_audit_path {
    $db = _xun_db_path
    return (Join-Path (Split-Path $db) "audit.jsonl")
}

function _xun_redirect_profiles {
    $cfgPath = _xun_config_path
    if (-not (Test-Path $cfgPath)) { return @() }
    try {
        $cfg = Get-Content $cfgPath -Raw | ConvertFrom-Json
        $profiles = $cfg.redirect.profiles
        if ($profiles -eq $null) { return @() }
        return $profiles.PSObject.Properties.Name
    } catch {
        return @()
    }
}

function _xun_redirect_txs {
    $auditPath = _xun_audit_path
    if (-not (Test-Path $auditPath)) { return @() }
    $lines = Get-Content $auditPath -Tail 200
    $txs = @()
    foreach ($line in $lines) {
        if ($line -match '"tx"\s*:\s*"([^"]+)"') {
            $txs += $matches[1]
        } elseif ($line -match 'tx=([^\s"}]+)') {
            $txs += $matches[1]
        }
    }
    return $txs | Select-Object -Unique
}

function _xun_apply_magic {
    param([string[]]$lines)
    $printed = @()
    foreach ($line in $lines) {
        if ($line -match "^__CD__:(.*)") {
            $target = $matches[1]
            if (Test-Path $target -PathType Container) {
                Set-Location $target
            }
            foreach ($pattern in $global:XunHooks.Keys) {
                $trigger = if ($pattern -eq '*') { $true } else { Test-Path (Join-Path $target $pattern) }
                if ($trigger) {
                    try { & $global:XunHooks[$pattern] $target } catch {}
                }
            }
        } elseif ($line -match "^__ENV_SET__:(.+?)=(.*)$") {
            Set-Item "Env:\$($matches[1])" $matches[2]
        } elseif ($line -match "^__ENV_UNSET__:(.+)$") {
            Remove-Item "Env:\$($matches[1])" -ErrorAction SilentlyContinue
        } elseif ($line -ne $null -and $line -ne '') {
            $printed += $line
        }
    }
    if ($printed.Count -gt 0) {
        $printed | ForEach-Object { Write-Output $_ }
    }
}

function x {
    $old = $env:XUN_UI
    $env:XUN_UI = "1"
    $out = & $xun @args
    if ($null -ne $old) { $env:XUN_UI = $old } else { Remove-Item Env:\XUN_UI -ErrorAction SilentlyContinue }
    if ($LASTEXITCODE -ne 0) { return }

    if ($out -is [array]) {
        _xun_apply_magic $out
    } elseif ($out) {
        _xun_apply_magic @($out)
    }
}

function ctx {
    if (-not $env:XUN_CTX_STATE) {
        $env:XUN_CTX_STATE = Join-Path $env:TEMP ("xun-ctx-{0}.json" -f $PID)
    }
    x ctx @args
}

function sv { xun sv @args }
function list { x list @args }
function delete { xun delete @args }
function gc { x gc @args }
function z { x z @args }
function o { x o @args }
function ws { x ws @args }
function pon { x pon @args }
function poff { x poff @args }
function pst { x pst @args }
function px { xun px @args }
function rename { xun rename @args }
function tag { xun tag @args }
function recent { x recent @args }
function stats { x stats @args }
function dedup { xun dedup @args }
function bak { xun bak @args }
function xtree { xun tree @args }
function xr { xun redirect @args }
function redir { xun redirect @args }

$global:XunCompletionLoaded = $false
try {
    $comp = & $xun completion powershell 2>$null
    if ($LASTEXITCODE -eq 0 -and $comp) {
        Invoke-Expression $comp
        $global:XunCompletionLoaded = $true
    }
} catch {}

if (-not $global:XunCompletionLoaded) {
$_xunComp = {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    if (-not $xun) { return }
    if ($commandName -eq "delete" -or $commandName -eq "del") {
        $hasBookmark = $false
        foreach ($elem in $commandAst.CommandElements) {
            if ($elem -and $elem.Extent -and ($elem.Extent.Text -eq "--bookmark" -or $elem.Extent.Text -eq "-bm")) {
                $hasBookmark = $true
                break
            }
        }
        if (-not $hasBookmark) { return }
    }
    @(& $xun keys) | Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

$_xunMainComp = {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    $elems = $commandAst.CommandElements
    if ($elems.Count -le 2) {
        $list = $global:XunSubcommands
    } else {
        $sub = $elems[1].Value
        $prev = $elems[$elems.Count - 1].Value
        if ($prev -eq "-f" -or $prev -eq "--format") {
            $list = $global:XunFormats
        } elseif ($sub -eq "redirect" -and $prev -eq "--profile") {
            $list = _xun_redirect_profiles
        } elseif ($sub -eq "redirect" -and ($prev -eq "--undo" -or $prev -eq "--tx")) {
            $list = _xun_redirect_txs
        } elseif ($sub -eq "ctx" -and $elems.Count -le 3) {
            $list = $global:XunCtxSubcommands
        } elseif ($sub -eq "tree" -and $prev -eq "--sort") {
            $list = $global:XunTreeSort
    } elseif ($sub -eq "proxy" -and $elems.Count -le 3) {
        $list = $global:XunProxySubcommands
    } elseif ($sub -eq "acl" -and $elems.Count -le 3) {
        $list = $global:XunAclSubcommands
    } else {
        return
    }
    }
    $list | Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

Register-ArgumentCompleter -CommandName 'z'      -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'delete' -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'del'    -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'o'      -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'rename' -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'xun'    -ScriptBlock $_xunMainComp
Register-ArgumentCompleter -CommandName 'x'      -ScriptBlock $_xunMainComp
Register-ArgumentCompleter -CommandName 'xyu'    -ScriptBlock $_xunMainComp
Register-ArgumentCompleter -CommandName 'xy'     -ScriptBlock $_xunMainComp
}
"#;

    let sh_script = r#"
xun() {
    local exe="${XUN_EXE:-xun.exe}"
    "$exe" "$@"
}

alias xyu='xun'
alias xy='xun'

_xun_config_path() {
    if [ -n "$XUN_CONFIG" ]; then echo "$XUN_CONFIG"; else echo "$HOME/.xun.config.json"; fi
}

_xun_db_path() {
    if [ -n "$XUN_DB" ]; then echo "$XUN_DB"; else echo "$HOME/.xun.json"; fi
}

_xun_audit_path() {
    local db=$(_xun_db_path)
    local dir
    dir=$(dirname "$db")
    echo "$dir/audit.jsonl"
}

_xun_redirect_profiles() {
    local cfg=$(_xun_config_path)
    [ -f "$cfg" ] || return
    if command -v python &>/dev/null; then
        python - "$cfg" <<'PY'
import json, sys
path = sys.argv[1]
try:
    with open(path, 'r', encoding='utf-8') as f:
        cfg = json.load(f)
    profiles = cfg.get('redirect', {}).get('profiles', {})
    if isinstance(profiles, dict):
        for k in profiles.keys():
            print(k)
except Exception:
    pass
PY
        return
    fi
    if command -v jq &>/dev/null; then
        jq -r '.redirect.profiles | keys[]?' "$cfg" 2>/dev/null
        return
    fi
    grep -o '"profiles"[[:space:]]*:[[:space:]]*{[^}]*}' "$cfg" 2>/dev/null | \
        grep -o '"[^"]*"[[:space:]]*:' | sed 's/[": ]//g'
}

_xun_redirect_txs() {
    local audit=$(_xun_audit_path)
    [ -f "$audit" ] || return
    tail -n 200 "$audit" | \
        sed -n 's/.*"tx"[[:space:]]*:[[:space:]]*"\([^"]\+\)".*/\1/p; s/.*tx=\([^"[:space:]]\+\).*/\1/p' | \
        awk '!seen[$0]++'
}

_xun_apply_magic() {
    local line
    local out_lines=()
    while IFS= read -r line; do
        if [[ "$line" == __CD__:* ]]; then
            local target="${line#__CD__:}"
            if command -v cygpath &>/dev/null; then
                target=$(cygpath -u "$target")
            fi
            cd "$target" || return 1
        elif [[ "$line" == __ENV_SET__:* ]]; then
            local kv="${line#__ENV_SET__:}"
            local k="${kv%%=*}"
            local v="${kv#*=}"
            export "$k=$v"
        elif [[ "$line" == __ENV_UNSET__:* ]]; then
            local k="${line#__ENV_UNSET__:}"
            unset "$k"
        else
            out_lines+=("$line")
        fi
    done
    if [ ${#out_lines[@]} -gt 0 ]; then
        printf '%s\n' "${out_lines[@]}"
    fi
}

x() {
    local old="$XUN_UI"
    export XUN_UI=1
    local out
    out=$(xun "$@")
    if [ -n "$old" ]; then export XUN_UI="$old"; else unset XUN_UI; fi
    printf '%s\n' "$out" | _xun_apply_magic
}

ctx() {
    if [ -z "$XUN_CTX_STATE" ]; then
        local tmp="${TEMP:-${TMPDIR:-/tmp}}"
        export XUN_CTX_STATE="$tmp/xun-ctx-$$.json"
    fi
    x ctx "$@"
}

sv() { xun sv "$@"; }
list() { x list "$@"; }
delete() { xun delete "$@"; }
gc() { x gc "$@"; }
z() { x z "$@"; }
o() { x o "$@"; }
ws() { x ws "$@"; }
pon() { x pon "$@"; }
poff() { x poff "$@"; }
pst() { x pst "$@"; }
px() { xun px "$@"; }
rename() { xun rename "$@"; }
tag() { xun tag "$@"; }
recent() { x recent "$@"; }
stats() { x stats "$@"; }
dedup() { xun dedup "$@"; }
bak() { xun bak "$@"; }
xtree() { xun tree "$@"; }
xr() { xun redirect "$@"; }
redir() { xun redirect "$@"; }

_xun_completion_loaded=0
if completions=$(xun completion bash 2>/dev/null); then
    eval "$completions"
    _xun_completion_loaded=1
fi

if [[ $_xun_completion_loaded -eq 0 ]]; then
_xun_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    local sub="${COMP_WORDS[1]}"
    local subcommands="acl alias init completion config ctx list z open ws save set delete del check gc touch rename tag recent stats dedup export import proxy pon poff pst px ports kill ps pkill keys all fuzzy bak tree find env img video lock rm mv renfile protect encrypt decrypt serve redirect brn cstat"
    local formats="auto table tsv json"
    local proxy_sub="set del get detect test"
    local ctx_sub="set use off list show del rename"
    local tree_sort="name mtime size"
    local acl_sub="view add remove purge diff batch effective copy backup restore inherit owner orphans repair audit config"

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
        return
    fi
    if [[ "$prev" == "-f" || "$prev" == "--format" ]]; then
        COMPREPLY=( $(compgen -W "$formats" -- "$cur") )
        return
    fi
    if [[ "$sub" == "redirect" && "$prev" == "--profile" ]]; then
        COMPREPLY=( $(compgen -W "$(_xun_redirect_profiles)" -- "$cur") )
        return
    fi
    if [[ "$sub" == "redirect" && ( "$prev" == "--undo" || "$prev" == "--tx" ) ]]; then
        COMPREPLY=( $(compgen -W "$(_xun_redirect_txs)" -- "$cur") )
        return
    fi
    if [[ "$sub" == "ctx" && $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "$ctx_sub" -- "$cur") )
        return
    fi
    if [[ "$sub" == "tree" && "$prev" == "--sort" ]]; then
        COMPREPLY=( $(compgen -W "$tree_sort" -- "$cur") )
        return
    fi
    if [[ "$sub" == "proxy" && $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "$proxy_sub" -- "$cur") )
        return
    fi
    if [[ "$sub" == "acl" && $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=( $(compgen -W "$acl_sub" -- "$cur") )
        return
    fi
}

complete -F _xun_complete xun x xyu xy
fi
"#;

    match args.shell.to_lowercase().as_str() {
        "powershell" | "pwsh" => out_println!("{}", ps_script),
        "bash" | "zsh" => out_println!("{}", sh_script),
        _ => {
            return Err(CliError::with_details(
                2,
                format!("Unsupported shell: {}.", args.shell),
                &["Fix: Use `xun init powershell` or `xun init bash`."],
            ));
        }
    }

    Ok(())
}

#[allow(clippy::result_large_err)]
pub(super) fn try_dispatch(cmd: SubCommand) -> Result<CliResult, SubCommand> {
    match cmd {
        SubCommand::Init(a) => Ok(cmd_init(a)),
        other => Err(other),
    }
}
