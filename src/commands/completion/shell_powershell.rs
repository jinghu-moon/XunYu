pub(crate) fn completion_powershell() -> &'static str {
    r#"
$xun = if ($env:XUN_EXE) { $env:XUN_EXE } else { "xun.exe" }

$global:XunSubcommands = @(
    "init","completion","config","ctx","list","z","open","ws","save","set","del","delete","check","gc","touch","rename","tag",
    "recent","stats","dedup","export","import","proxy","pon","poff","pst","px","ports","kill","keys","all","fuzzy",
    "bak","tree","env","video","lock","rm","mv","renfile","protect","encrypt","decrypt","serve","redirect"
)
$global:XunProxySubcommands = @("set","del","get","detect","test")
$global:XunCtxSubcommands = @("set","use","off","list","show","del","rename")
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

$_xunStaticKeys = {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    if (-not $xun) { return }
    @(& $xun keys) | Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

$_xunStaticMain = {
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
        } else {
            return
        }
    }
    $list | Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

function _xun_parse_complete_lines {
    param([string[]]$lines)
    $items = @()
    $sentinel = $null
    foreach ($line in $lines) {
        if ($line -like "__XUN_COMPLETE__=*") {
            $sentinel = $line
            continue
        }
        if (-not $line) { continue }
        $parts = $line -split "`t", 2
        $value = $parts[0]
        if (-not $value) { continue }
        $desc = if ($parts.Count -gt 1 -and $parts[1]) { $parts[1] } else { $value }
        $items += [System.Management.Automation.CompletionResult]::new($value, $value, 'ParameterValue', $desc)
    }
    return @{ Items = $items; Sentinel = $sentinel }
}

$_xunDynamic = {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    if (-not $xun) { return $null }
    if ($env:XUN_DISABLE_DYNAMIC_COMPLETE -in @("1","true","yes")) { return $null }
    $elems = $commandAst.CommandElements
    if ($elems.Count -lt 1) { return $null }
    $args = @()
    for ($i = 1; $i -lt $elems.Count; $i++) { $args += $elems[$i].Value }
    if ($wordToComplete -eq "" -and ($elems.Count -le 1 -or $elems[$elems.Count - 1].Value -ne "")) {
        $args += ""
    }
    if ($commandName -ne "xun" -and $commandName -ne "x" -and $commandName -ne "xyu" -and $commandName -ne "xy") {
        $args = @($commandName) + $args
    }
    $timeoutMs = $env:XUN_COMPLETE_TIMEOUT_MS
    if ($timeoutMs -and [int]$timeoutMs -gt 0) {
        $job = Start-Job -ScriptBlock { param($exe, $argv) & $exe __complete @argv 2>$null } -ArgumentList $xun, $args
        $done = Wait-Job $job -Timeout ([double]$timeoutMs / 1000)
        if (-not $done) {
            Stop-Job $job -ErrorAction SilentlyContinue
            Remove-Job $job -ErrorAction SilentlyContinue
            return $null
        }
        $out = Receive-Job $job
        Remove-Job $job -ErrorAction SilentlyContinue
        $LASTEXITCODE = 0
    } else {
        $out = & $xun __complete @args 2>$null
        if ($LASTEXITCODE -ne 0) { return $null }
    }
    $lines = @($out) | Where-Object { $_ -ne "" }
    $parsed = _xun_parse_complete_lines $lines
    if (-not $parsed.Sentinel) { return $null }
    if ($parsed.Sentinel -eq "__XUN_COMPLETE__=fallback") { return $null }
    $ver = 0
    if ($parsed.Sentinel -match 'v=([0-9]+)') { $ver = [int]$matches[1] }
    if ($ver -ne 1) { return $null }
    return $parsed.Items
}

$_xunComplete = {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    $dynamic = & $_xunDynamic $commandName $parameterName $wordToComplete $commandAst $fakeBoundParameters
    if ($null -ne $dynamic) { return $dynamic }
    if ($commandName -eq "delete") {
        $hasBookmark = $false
        foreach ($elem in $commandAst.CommandElements) {
            if ($elem -and $elem.Extent -and ($elem.Extent.Text -eq "--bookmark" -or $elem.Extent.Text -eq "-bm")) {
                $hasBookmark = $true
                break
            }
        }
        if ($hasBookmark) {
            return & $_xunStaticKeys $commandName $parameterName $wordToComplete $commandAst $fakeBoundParameters
        }
    }
    if ($commandName -eq "z" -or $commandName -eq "o" -or $commandName -eq "rename") {
        return & $_xunStaticKeys $commandName $parameterName $wordToComplete $commandAst $fakeBoundParameters
    }
    return & $_xunStaticMain $commandName $parameterName $wordToComplete $commandAst $fakeBoundParameters
}

Register-ArgumentCompleter -CommandName 'z'      -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'delete' -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'o'      -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'rename' -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'xun'    -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'x'      -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'xyu'    -ScriptBlock $_xunComplete
Register-ArgumentCompleter -CommandName 'xy'     -ScriptBlock $_xunComplete
"#
}
