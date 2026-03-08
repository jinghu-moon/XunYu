# =============================================================================
# xun-bookmarks.ps1 — 书签管理 v0.0.1
# 依赖：PwshSpectreConsole、xun.exe
#
# 命令：
#   sv  [name] [--tag t1,t2]   保存书签（FileSystem provider 限定）
#   z   [pattern] [--tag t]    跳转（frecency + 模糊，Spectre 选单）
#   list [--tag t]             列出书签（Spectre 表格）
#   delete <n>              删除书签
#   gc                         清理失效书签（交互确认）
#   o   [pattern]              在资源管理器中打开（不切换 Shell 路径）
#   ws  <tag>                  工作区：在新 WT 标签页打开 tag 下所有路径
#
# 跳转钩子（可选，在 $PROFILE 中配置）：
#   $XunHooks = @{
#       '*'          = { param($p) Get-ChildItem $p | Select-Object -First 8 }
#       'Cargo.toml' = { param($p) Write-Host "Rust 项目" }
#       '.env'       = { param($p) Write-Host ".env 已存在" }
#   }
# =============================================================================

Import-Module PwshSpectreConsole -ErrorAction SilentlyContinue

$script:XUN = if ($env:XUN_EXE) { $env:XUN_EXE }
             elseif (Test-Path "$PSScriptRoot\xun.exe") { "$PSScriptRoot\xun.exe" }
             elseif (Get-Command xun -ErrorAction SilentlyContinue) { "xun" }
             else { Write-SpectreHost "[red]✗[/]  找不到 xun.exe"; $null }

# 用户可在 $PROFILE 中定义 $XunHooks 覆盖此默认值
if (-not (Get-Variable XunHooks -Scope Global -ErrorAction SilentlyContinue)) {
    $global:XunHooks = @{}   # 默认无钩子
}

function _XUN { & $script:XUN @args }

# ── TSV 解析 ──────────────────────────────────────────────────────────────────

function _Parse-Entries ([string[]]$lines) {
    $lines | Where-Object { $_ } | ForEach-Object {
        $c = $_ -split "`t"
        [PSCustomObject]@{
            Key    = $c[0]; Path   = $c[1]
            Tags   = $c[2]; Visits = [int]($c[3] -replace '\D')
        }
    }
}

# ── save ─────────────────────────────────────────────────────────────────────

function save {
    param([string]$Name, [string]$Tag = "")
    if (-not $script:XUN) { return }

    if ($PWD.Provider.Name -ne 'FileSystem') {
        Write-SpectreHost "  [red]✗[/]  当前路径不是文件系统路径（$($PWD.Provider.Name)），无法保存"
        return
    }

    if (-not $Name) { $Name = Split-Path -Leaf $PWD.Path }
    switch (_XUN set $Name $PWD.Path $Tag) {
        "new"     { Write-SpectreHost "  [green]OK[/]  [bold]$Name[/] 已保存 → [dim]$($PWD.Path)[/]" }
        "updated" { Write-SpectreHost "  [yellow] ~[/]  [bold]$Name[/] 已更新 → [dim]$($PWD.Path)[/]" }
    }
    if ($Tag) { Write-SpectreHost "  [dim]标签: $Tag[/]" }
}

Set-Alias sv save

# ── delete ───────────────────────────────────────────────────────────────────

function delete {
    param([Parameter(Mandatory)][string]$Name)
    if (-not $script:XUN) { return }
    switch (_XUN del $Name) {
        "ok"       { Write-SpectreHost "  [yellow]--[/]  书签 [bold]$Name[/] 已删除" }
        "notfound" { Write-SpectreHost "  [red]✗[/]   书签 [bold]$Name[/] 不存在" }
    }
}

# ── list ─────────────────────────────────────────────────────────────────────

function list {
    param([string]$Tag = "")
    if (-not $script:XUN) { return }

    $entries = _Parse-Entries @(_XUN all $Tag)
    if (-not $entries) {
        Write-SpectreHost "`n  [grey]暂无书签，使用 [bold]sv[/] 添加[/]`n"; return
    }

    $rows = $entries | ForEach-Object {
        [PSCustomObject]@{
            名称 = "[bold]$($_.Key)[/]"
            路径 = if (Test-Path $_.Path) { "[dim]$($_.Path)[/]" }
                   else                   { "[red]$($_.Path)[/] [red dim](不存在)[/]" }
            标签 = if ($_.Tags)  { "[dim]$($_.Tags)[/]" }  else { "[grey dim]—[/]" }
            访问 = if ($_.Visits -gt 0) { "[dim]$($_.Visits)次[/]" } else { "[grey dim]—[/]" }
        }
    }

    $rows | Format-SpectreTable `
        -Border "Rounded" -Color "SteelBlue1" -HeaderColor "grey" -AllowMarkup |
        Format-SpectrePanel -Header " 书签列表 " -Color "SteelBlue1" |
        Out-SpectreHost
}

# ── gc（死链清理）─────────────────────────────────────────────────────────────

function gc {
    if (-not $script:XUN) { return }

    $dead = @(_XUN gc) | Where-Object { $_ }
    if (-not $dead) {
        Write-SpectreHost "`n  [green]✓[/]   所有书签路径均有效`n"; return
    }

    $rows = $dead | ForEach-Object {
        $parts = $_ -split "`t", 2
        [PSCustomObject]@{
            名称 = "[bold]$($parts[0])[/]"
            路径 = "[red dim]$($parts[1])[/]"
        }
    }

    $rows | Format-SpectreTable `
        -Border "Rounded" -Color "Orange1" -HeaderColor "grey" -AllowMarkup |
        Format-SpectrePanel -Header " 失效书签 ($($dead.Count) 个) " -Color "Orange1" |
        Out-SpectreHost

    $confirm = Read-Host "  删除全部失效书签？[y/N]"
    if ($confirm -match '^[yY]$') {
        $result = @(_XUN gc --purge)[0]
        $n = ($result -split ":")[1]
        Write-SpectreHost "  [yellow]--[/]  已删除 [bold]$n[/] 个失效书签"
    } else {
        Write-SpectreHost "  [grey]已取消[/]"
    }
}

# ── o（在资源管理器中打开）───────────────────────────────────────────────────

function o {
    param([string]$Pattern)
    if (-not $script:XUN) { return }

    if (-not $Pattern) {
        # 无参数 → 打开当前目录
        Start-Process explorer.exe $PWD.Path; return
    }

    $raw     = @(_XUN fuzzy $Pattern)
    $entries = _Parse-Entries $raw

    if (-not $entries) {
        Write-SpectreHost "  [red]✗[/]   没有匹配 [bold]$Pattern[/] 的书签"; return
    }

    # 唯一匹配 → 直接打开
    if (@($entries).Count -eq 1) {
        _Open-Path $entries[0].Key $entries[0].Path; return
    }

    # 多个 → Spectre 选单
    $maxLen  = ($entries | Measure-Object { $_.Key.Length } -Maximum).Maximum
    $choices = $entries | ForEach-Object {
        "$($_.Key)$(' ' * ($maxLen - $_.Key.Length + 2))[dim]$($_.Path)[/]"
    }

    $selected = Get-SpectreSelection `
        -Choices $choices -Title "[SteelBlue1] 打开目录 [/]" `
        -Color "SteelBlue1" -AllowMarkup

    if (-not $selected) { return }
    $key  = ($selected -replace '\[.*?\]','').Trim() -split '\s+' | Select-Object -First 1
    $path = ($entries | Where-Object { $_.Key -eq $key } | Select-Object -First 1).Path
    _Open-Path $key $path
}

function _Open-Path ([string]$Key, [string]$Path) {
    if (-not (Test-Path $Path)) {
        Write-SpectreHost "  [red]✗[/]   路径不存在: [dim]$Path[/]"; return
    }
    Start-Process explorer.exe $Path
    Write-SpectreHost "  [green]↗[/]   [bold]$Key[/]  [dim]$Path[/]"
}

# ── ws（工作区：在新 WT 标签页打开 tag 下所有路径）──────────────────────────

function workspace {
    param([Parameter(Mandatory)][string]$Tag)
    if (-not $script:XUN) { return }

    $entries = _Parse-Entries @(_XUN all $Tag) | Where-Object { Test-Path $_.Path }
    if (-not $entries) {
        Write-SpectreHost "  [grey]标签 [bold]$Tag[/] 下无有效书签[/]"; return
    }

    if (-not (Get-Command wt -ErrorAction SilentlyContinue)) {
        Write-SpectreHost "  [red]✗[/]   未找到 wt (Windows Terminal)，请通过 winget install Microsoft.WindowsTerminal 安装"
        return
    }

    # 第一个路径在当前标签页跳转，其余开新标签
    $first = $entries[0]
    Set-Location $first.Path
    Write-SpectreHost "  [green]→[/]   [bold]$($first.Key)[/]  [dim]$($first.Path)[/]"

    foreach ($e in $entries[1..($entries.Count - 1)]) {
        Start-Process wt -ArgumentList "--window 0 new-tab --startingDirectory `"$($e.Path)`""
        Write-SpectreHost "  [green]↗[/]   [bold]$($e.Key)[/]  [dim]$($e.Path)[/]  [dim](新标签)[/]"
    }
}

Set-Alias ws workspace

# ── z（跳转）─────────────────────────────────────────────────────────────────

function z {
    param([string]$Pattern, [string]$Tag = "")
    if (-not $script:XUN) { return }

    $raw     = if ($Pattern) { @(_XUN fuzzy $Pattern $Tag) } else { @(_XUN all $Tag) }
    $entries = _Parse-Entries $raw

    if (-not $entries) {
        $msg = if ($Pattern) { "没有匹配 [bold]$Pattern[/] 的书签" }
               else          { "暂无书签，使用 [bold]sv[/] 添加" }
        Write-SpectreHost "`n  [grey]$msg[/]`n"; return
    }

    if (@($entries).Count -eq 1) {
        _Xun-Jump $entries[0].Key $entries[0].Path; return
    }

    $maxLen  = ($entries | Measure-Object { $_.Key.Length } -Maximum).Maximum
    $choices = $entries | ForEach-Object {
        $tagPart = if ($_.Tags) { "  [dim][$($_.Tags)][/]" } else { "" }
        "$($_.Key)$(' ' * ($maxLen - $_.Key.Length + 2))[dim]$($_.Path)[/]$tagPart"
    }

    $selected = Get-SpectreSelection `
        -Choices $choices -Title "[SteelBlue1] 跳转到书签 [/]" `
        -Color "SteelBlue1" -AllowMarkup

    if (-not $selected) { return }
    $key  = ($selected -replace '\[.*?\]','').Trim() -split '\s+' | Select-Object -First 1
    $path = ($entries | Where-Object { $_.Key -eq $key } | Select-Object -First 1).Path
    _Xun-Jump $key $path
}

function _Xun-Jump ([string]$Key, [string]$Path) {
    if (-not (Test-Path $Path)) {
        Write-SpectreHost "  [red]✗[/]   路径不存在: [dim]$Path[/]"; return
    }
    Set-Location $Path
    Write-SpectreHost "  [green]→[/]   [bold]$Key[/]  [dim]$Path[/]"

    # 静默更新 frecency（后台，不阻塞提示符）
    Start-Process -FilePath $script:XUN -ArgumentList "touch", $Key `
        -WindowStyle Hidden -NoNewWindow 2>$null

    # 执行用户定义的跳转钩子
    foreach ($pattern in $global:XunHooks.Keys) {
        $trigger = if ($pattern -eq '*') { $true }
                   else { Test-Path (Join-Path $Path $pattern) }
        if ($trigger) {
            try { & $global:XunHooks[$pattern] $Path } catch {}
        }
    }
}

# ── Tab 补全 ──────────────────────────────────────────────────────────────────

$_xunComp = {
    param($cmd, $word)
    if (-not $script:XUN) { return }
    @(_XUN keys) | Where-Object { $_ -like "$word*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

Register-ArgumentCompleter -CommandName 'z'      -Native $false -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'delete' -Native $false -ScriptBlock $_xunComp
Register-ArgumentCompleter -CommandName 'o'      -Native $false -ScriptBlock $_xunComp



