# =============================================================================
# xun-proxy.ps1 — 终端代理统一管理 v0.0.1
# 依赖：PwshSpectreConsole、xun.exe
#
# 命令：
#   pon  [url]           开启代理
#   poff                 关闭代理
#   pst                  状态（含延迟测试）
#   px   <command>       临时代理执行单条命令（不污染全局）
# =============================================================================

Import-Module PwshSpectreConsole -ErrorAction SilentlyContinue

$script:XUN = if ($env:XUN_EXE) { $env:XUN_EXE }
             elseif (Test-Path "$PSScriptRoot\xun.exe") { "$PSScriptRoot\xun.exe" }
             elseif (Get-Command xun -ErrorAction SilentlyContinue) { "xun" }
             else { $null }

# ── 工具函数 ──────────────────────────────────────────────────────────────────

function _Get-SystemProxyUrl ([string]$Fallback) {
    $reg = Get-ItemProperty `
        "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings" `
        -ErrorAction SilentlyContinue
    if ($reg.ProxyEnable -eq 1 -and $reg.ProxyServer) {
        $raw    = $reg.ProxyServer
        $ipPort = if ($raw -match "http=([^;]+)") { $matches[1] } else { $raw.Split(';')[0] }
        return ("http://$ipPort") -replace "http://http://", "http://"
    }
    return $Fallback
}

function _XUN { & $script:XUN @args }

# ── pon ───────────────────────────────────────────────────────────────────────

function proxy-on {
    param(
        [string]$FallbackUrl = "http://127.0.0.1:7897",
        [string]$NoProxy     = "localhost,127.0.0.1,::1,.local"
    )

    $ProxyUrl = _Get-SystemProxyUrl -Fallback $FallbackUrl
    $detected = if ($ProxyUrl -eq $FallbackUrl) {
        "[yellow]~[/]   系统代理未开启，使用保底地址 [yellow]$ProxyUrl[/]"
    } else {
        "[grey]~[/]   检测到系统代理 [cyan]$ProxyUrl[/]"
    }

    $lines = @($detected, "")

    # 1. 环境变量（必须在当前 Shell 进程设置，子进程无法代劳）
    $env:HTTP_PROXY  = $env:http_proxy  = $ProxyUrl
    $env:HTTPS_PROXY = $env:https_proxy = $ProxyUrl
    $env:ALL_PROXY   = $env:all_proxy   = $ProxyUrl
    $env:NO_PROXY    = $env:no_proxy    = $NoProxy
    $lines += "[green]OK[/]  环境变量   [dim]$ProxyUrl[/]"

    # 2. 文件级配置委托 xun.exe
    if ($script:XUN) {
        foreach ($r in @(_XUN proxy set $ProxyUrl $NoProxy)) {
            if (-not $r) { continue }
            $parts = $r -split ":", 3
            $ok    = $parts[0] -eq "ok"
            $tool  = $parts[1].ToUpper().PadRight(7)
            $extra = if ($parts.Count -gt 2) { $parts[2] } else { $ProxyUrl }
            if ($ok) { $lines += "[green]OK[/]  $tool  [dim]$extra[/]" }
            else      { $lines += "[grey]--[/]  $tool  [dim]$extra[/]" }
        }
    } else {
        $lines += "[yellow]~[/]   xun.exe 未找到，跳过 git/npm/Cargo/MSYS2"
    }

    $lines += ""
    $lines += "[dim]>>[/]  排除本地流量 [dim]$NoProxy[/]"

    ($lines -join "`n") |
        Format-SpectrePanel -Header " 开启代理 " -Color "SteelBlue1" |
        Out-SpectreHost
}

# ── poff ──────────────────────────────────────────────────────────────────────

function proxy-off {
    $lines = @()

    # 清除环境变量
    'HTTP_PROXY','HTTPS_PROXY','ALL_PROXY','NO_PROXY',
    'http_proxy','https_proxy','all_proxy','no_proxy' | ForEach-Object {
        Remove-Item "Env:\$_" -ErrorAction SilentlyContinue
    }
    $lines += "[yellow]--[/]  环境变量   已清除"

    # 委托 xun.exe
    if ($script:XUN) {
        foreach ($r in @(_XUN proxy del)) {
            if (-not $r) { continue }
            $parts = $r -split ":", 3
            $ok    = $parts[0] -eq "ok"
            $tool  = $parts[1].ToUpper().PadRight(7)
            $extra = if ($parts.Count -gt 2) { $parts[2] } else { "已清除" }
            if ($ok) { $lines += "[yellow]--[/]  $tool  $extra" }
            else      { $lines += "[grey]--[/]  $tool  [dim]$extra[/]" }
        }
    }

    ($lines -join "`n") |
        Format-SpectrePanel -Header " 关闭代理 " -Color "Orange1" |
        Out-SpectreHost
}

# ── pst ───────────────────────────────────────────────────────────────────────

function proxy-status {
    $gitProxy = if ($script:XUN) { @(_XUN proxy get)[0] } else { $null }
    $npmProxy = if (Get-Command npm -ErrorAction SilentlyContinue) {
        $v = npm config get proxy 2>$null
        if ($v -and $v -ne 'null') { $v }
    }
    $cargoProxy = (Get-Content "$env:USERPROFILE\.cargo\config.toml" -ErrorAction SilentlyContinue |
        Select-String 'proxy\s*=\s*"([^"]+)"')?.Matches?.Groups[1]?.Value

    # 主状态表格
    $rows = @(
        [PSCustomObject]@{
            工具 = "环境变量"
            状态 = if ($env:HTTP_PROXY) { "[green]开启[/]" } else { "[grey]关闭[/]" }
            地址 = if ($env:HTTP_PROXY) { "[cyan]$($env:HTTP_PROXY)[/]" } else { "[grey dim]—[/]" }
            备注 = if ($env:NO_PROXY)   { "[dim]$($env:NO_PROXY)[/]" } else { "" }
        },
        [PSCustomObject]@{
            工具 = "Git"
            状态 = if ($gitProxy)   { "[green]开启[/]" } else { "[grey]关闭[/]" }
            地址 = if ($gitProxy)   { "[cyan]$gitProxy[/]" } else { "[grey dim]—[/]" }
            备注 = ""
        },
        [PSCustomObject]@{
            工具 = "npm"
            状态 = if ($npmProxy)   { "[green]开启[/]" } else { "[grey]关闭[/]" }
            地址 = if ($npmProxy)   { "[cyan]$npmProxy[/]" } else { "[grey dim]—[/]" }
            备注 = ""
        },
        [PSCustomObject]@{
            工具 = "Cargo"
            状态 = if ($cargoProxy) { "[green]开启[/]" } else { "[grey]关闭[/]" }
            地址 = if ($cargoProxy) { "[cyan]$cargoProxy[/]" } else { "[grey dim]—[/]" }
            备注 = if ($cargoProxy) { "[dim]config.toml[/]" } else { "" }
        }
    )

    $table = $rows | Format-SpectreTable `
        -Border "Rounded" -Color "SteelBlue1" -HeaderColor "grey" -AllowMarkup

    # 延迟测试（仅当代理开启时）
    $latencySection = ""
    $proxyUrl = $env:HTTP_PROXY
    if ($proxyUrl -and $script:XUN) {
        Write-SpectreHost "[dim]  正在测试连通性...[/]"
        $probeLines = @(_XUN proxy test $proxyUrl) | Where-Object { $_ }

        $latencyRows = $probeLines | ForEach-Object {
            $cols = $_ -split "`t"
            $target = $cols[0]; $ms = $cols[1]; $status = $cols[2]

            $label = switch ($target) {
                "proxy"   { "代理自身" }
                "8.8.8.8" { "Google DNS" }
                "1.1.1.1" { "Cloudflare" }
                default   { $target }
            }

            $stateStr = if ($status -eq "ok") {
                $msInt = [int]$ms
                $color = if ($msInt -lt 100) { "green" }
                         elseif ($msInt -lt 500) { "yellow" }
                         else { "red" }
                "[$color]${ms}ms[/]"
            } else {
                "[red]超时[/]"
            }

            $detail = if ($status -ne "ok") { "[red dim]$status[/]" } else { "" }

            [PSCustomObject]@{ 目标 = $label; 延迟 = $stateStr; 详情 = $detail }
        }

        if ($latencyRows) {
            $latencySection = $latencyRows | Format-SpectreTable `
                -Border "Simple" -Color "grey" -HeaderColor "grey" -AllowMarkup
        }
    }

    # 组合输出
    $content = if ($latencySection) {
        @($table, "`n", $latencySection) -join ""
    } else { $table }

    $content | Format-SpectrePanel -Header " 代理状态 " -Color "SteelBlue1" | Out-SpectreHost
}

# ── px（临时代理执行单条命令）──────────────────────────────────────────────────
#
# 用法：px git clone https://...
#        px cargo build --release

function px {
    param([Parameter(ValueFromRemainingArguments)][string[]]$Cmd)

    if (-not $Cmd) {
        Write-SpectreHost "  [red]✗[/]   用法: px <command> [args]"; return
    }

    $url = _Get-SystemProxyUrl -Fallback "http://127.0.0.1:7897"

    # 备份当前环境变量
    $vars = 'HTTP_PROXY','HTTPS_PROXY','ALL_PROXY','NO_PROXY',
            'http_proxy','https_proxy','all_proxy','no_proxy'
    $backup = @{}
    foreach ($v in $vars) {
        $backup[$v] = [Environment]::GetEnvironmentVariable($v)
    }

    # 临时注入
    $env:HTTP_PROXY  = $env:http_proxy  = $url
    $env:HTTPS_PROXY = $env:https_proxy = $url
    $env:ALL_PROXY   = $env:all_proxy   = $url
    $env:NO_PROXY    = $env:no_proxy    = "localhost,127.0.0.1,::1,.local"

    Write-SpectreHost "  [dim]px →[/] [cyan]$url[/]  [dim]$($Cmd -join ' ')[/]"

    try {
        & $Cmd[0] $Cmd[1..$Cmd.Length]
    } finally {
        # 还原（无论命令是否成功）
        foreach ($v in $vars) {
            if ($null -eq $backup[$v]) {
                Remove-Item "Env:\$v" -ErrorAction SilentlyContinue
            } else {
                Set-Item "Env:\$v" $backup[$v]
            }
        }
    }
}

Set-Alias pon  proxy-on
Set-Alias poff proxy-off
Set-Alias pst  proxy-status


