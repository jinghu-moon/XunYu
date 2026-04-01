use crate::cli::{InitCmd, SubCommand};
use crate::commands::bookmarks::integration::render_bookmark_init;
use crate::output::{CliError, CliResult};

fn cmd_init(args: InitCmd) -> CliResult {
    let script = render_init_script(&args.shell)?;
    out_println!("{script}");
    Ok(())
}

fn render_init_script(shell: &str) -> CliResult<String> {
    match shell.to_ascii_lowercase().as_str() {
        "powershell" | "pwsh" => render_powershell_init(),
        "bash" | "zsh" => render_bash_init(),
        _ => Err(CliError::with_details(
            2,
            format!("Unsupported shell: {}.", shell),
            &["Fix: Use `xun init powershell` or `xun init bash`."],
        )),
    }
}

fn render_powershell_init() -> CliResult<String> {
    let bookmark = render_bookmark_init("powershell", "z")?;
    Ok(format!(
        r#"
$xunExe = if ($env:XUN_EXE) {{ $env:XUN_EXE }} else {{ "xun.exe" }}
function xun {{ & $xunExe @args }}
Set-Alias xyu xun
Set-Alias xy xun

function _xun_apply_magic {{
    param([string[]]$lines)
    $printed = @()
    foreach ($line in $lines) {{
        if ($line -match "^__CD__:(.*)") {{
            $target = $matches[1]
            if (Test-Path $target -PathType Container) {{
                Set-Location $target
            }}
        }} elseif ($line -match "^__BM_CD__ (.+)$") {{
            $target = $matches[1]
            if (Test-Path $target -PathType Container) {{
                Set-Location $target
            }}
        }} elseif ($line -match "^__ENV_SET__:(.+?)=(.*)$") {{
            Set-Item "Env:\$($matches[1])" $matches[2]
        }} elseif ($line -match "^__ENV_UNSET__:(.+)$") {{
            Remove-Item "Env:\$($matches[1])" -ErrorAction SilentlyContinue
        }} elseif ($line -ne $null -and $line -ne '') {{
            $printed += $line
        }}
    }}
    if ($printed.Count -gt 0) {{
        $printed | ForEach-Object {{ Write-Output $_ }}
    }}
}}

function x {{
    $old = $env:XUN_UI
    $env:XUN_UI = "1"
    $out = & $xunExe @args
    if ($null -ne $old) {{ $env:XUN_UI = $old }} else {{ Remove-Item Env:\XUN_UI -ErrorAction SilentlyContinue }}
    if ($LASTEXITCODE -ne 0) {{ return }}
    if ($out -is [array]) {{
        _xun_apply_magic $out
    }} elseif ($out) {{
        _xun_apply_magic @($out)
    }}
}}

function ctx {{
    if (-not $env:XUN_CTX_STATE) {{
        $env:XUN_CTX_STATE = Join-Path $env:TEMP ("xun-ctx-{{0}}.json" -f $PID)
    }}
    x ctx @args
}}

function delete {{ xun delete @args }}
function pon {{ x pon @args }}
function poff {{ x poff @args }}
function pst {{ x pst @args }}
function px {{ xun px @args }}
function backup {{ xun backup @args }}
function bak {{ xun bak @args }}
function xtree {{ xun tree @args }}
function xr {{ xun redirect @args }}
function redir {{ xun redirect @args }}

try {{
    $comp = & $xunExe completion powershell 2>$null
    if ($LASTEXITCODE -eq 0 -and $comp) {{
        Invoke-Expression $comp
    }}
}} catch {{}}

{bookmark}
"#,
        bookmark = bookmark
    ))
}

fn render_bash_init() -> CliResult<String> {
    let bookmark = render_bookmark_init("bash", "z")?;
    Ok(format!(
        r#"
xun() {{
    local exe="${{XUN_EXE:-xun.exe}}"
    "$exe" "$@"
}}

alias xyu='xun'
alias xy='xun'

_xun_apply_magic() {{
    local line
    local out_lines=()
    while IFS= read -r line; do
        if [[ "$line" == __CD__:* ]]; then
            local target="${{line#__CD__:}}"
            if command -v cygpath &>/dev/null; then
                target=$(cygpath -u "$target")
            fi
            cd "$target" || return 1
        elif [[ "$line" == __BM_CD__* ]]; then
            local target="${{line#__BM_CD__ }}"
            if command -v cygpath &>/dev/null; then
                target=$(cygpath -u "$target")
            fi
            cd "$target" || return 1
        elif [[ "$line" == __ENV_SET__:* ]]; then
            local kv="${{line#__ENV_SET__:}}"
            local k="${{kv%%=*}}"
            local v="${{kv#*=}}"
            export "$k=$v"
        elif [[ "$line" == __ENV_UNSET__:* ]]; then
            local k="${{line#__ENV_UNSET__:}}"
            unset "$k"
        else
            out_lines+=("$line")
        fi
    done
    if [ ${{#out_lines[@]}} -gt 0 ]; then
        printf '%s\n' "${{out_lines[@]}}"
    fi
}}

x() {{
    local old="$XUN_UI"
    export XUN_UI=1
    local out
    out=$(xun "$@")
    if [ -n "$old" ]; then export XUN_UI="$old"; else unset XUN_UI; fi
    printf '%s\n' "$out" | _xun_apply_magic
}}

ctx() {{
    if [ -z "$XUN_CTX_STATE" ]; then
        local tmp="${{TEMP:-${{TMPDIR:-/tmp}}}}"
        export XUN_CTX_STATE="$tmp/xun-ctx-$$.json"
    fi
    x ctx "$@"
}}

delete() {{ xun delete "$@"; }}
pon() {{ x pon "$@"; }}
poff() {{ x poff "$@"; }}
pst() {{ x pst "$@"; }}
px() {{ xun px "$@"; }}
backup() {{ xun backup "$@"; }}
bak() {{ xun bak "$@"; }}
xtree() {{ xun tree "$@"; }}
xr() {{ xun redirect "$@"; }}
redir() {{ xun redirect "$@"; }}

if completions=$(xun completion bash 2>/dev/null); then
    eval "$completions"
fi

{bookmark}
"#,
        bookmark = bookmark
    ))
}

#[allow(clippy::result_large_err)]
pub(super) fn try_dispatch(cmd: SubCommand) -> Result<CliResult, SubCommand> {
    match cmd {
        SubCommand::Init(a) => Ok(cmd_init(a)),
        other => Err(other),
    }
}
