#Requires -Version 7.0
# Xun Dev - PowerShell shell, cargo/vite commands

$ErrorActionPreference = "Stop"

$Root = $PSScriptRoot
$UI = "$Root/dashboard-ui"
$BackendPort = 9527
$SpinnerFrames = @('⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏')
$LogRoot = Join-Path $env:TEMP "xun-dev-logs"

Import-Module PwshSpectreConsole -ErrorAction SilentlyContinue
if (-not (Get-Command Write-SpectreHost -ErrorAction SilentlyContinue) -or
    -not (Get-Command Read-SpectreSelection -ErrorAction SilentlyContinue)) {
    Write-Host "PwshSpectreConsole not found or incomplete. Please install it first."
    exit 1
}

if (-not (Test-Path $LogRoot)) {
    New-Item -ItemType Directory -Path $LogRoot | Out-Null
}

function Write-Spin([string]$Text, [int]$Frame) {
    $f = $SpinnerFrames[$Frame % $SpinnerFrames.Count]
    Write-Host "`r  $f $Text" -NoNewline -ForegroundColor Cyan
}

function Test-ListeningOnce([int]$Port, [int]$TimeoutMs = 150) {
    $client = $null
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $task = $client.ConnectAsync("127.0.0.1", $Port)
        if ($task.Wait($TimeoutMs) -and $client.Connected) {
            return $true
        }
    } catch {
    } finally {
        if ($client) { $client.Dispose() }
    }
    return $false
}

function Wait-ForPort {
    param(
        [string]$Title,
        [int]$Port,
        $Process,
        [int]$TimeoutSec = 25
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $frame = 0
    while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
        if ($Process -and $Process.HasExited) {
            Write-Host "`r  ✗ $Title exited early" -ForegroundColor Red
            return $false
        }
        if (Test-ListeningOnce $Port 150) {
            Write-Host "`r  ✓ $Title ready" -ForegroundColor Green
            return $true
        }
        $pct = [int]([Math]::Min(($sw.Elapsed.TotalSeconds / $TimeoutSec) * 100, 99))
        Write-Spin "$Title ${pct}%" $frame
        $frame = $frame + 1
        Start-Sleep -Milliseconds 120
    }
    Write-Host "`r  ✗ $Title timeout" -ForegroundColor Red
    return $false
}

function Get-FrontendPort {
    $vite = "$UI/vite.config.ts"
    if (Test-Path "$vite") {
        $raw = Get-Content "$vite" -Raw -ErrorAction SilentlyContinue
        if ($raw -match "port:\s*(\d+)") {
            return [int]$Matches[1]
        }
    }
    return 5173
}

function Join-Args([string[]]$Args) {
    return ($Args | ForEach-Object {
        if ($_ -match '\s') { '"' + $_ + '"' } else { $_ }
    }) -join ' '
}

function Start-Command {
    param(
        [string]$Command,
        [string[]]$Arguments,
        [string]$WorkDir,
        [string]$Prefix
    )
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
    $outLog = Join-Path $LogRoot "$Prefix-$stamp.out.log"
    $errLog = Join-Path $LogRoot "$Prefix-$stamp.err.log"
    New-Item -ItemType File -Path $outLog -Force | Out-Null
    New-Item -ItemType File -Path $errLog -Force | Out-Null

    $cmdLine = (Join-Args @($Command)) + " " + (Join-Args $Arguments)
    $proc = Start-Process -FilePath $env:ComSpec -ArgumentList @("/c", $cmdLine) `
        -WorkingDirectory "$WorkDir" -NoNewWindow -PassThru `
        -RedirectStandardOutput "$outLog" -RedirectStandardError "$errLog"

    return [PSCustomObject]@{
        Process = $proc
        OutLog = $outLog
        ErrLog = $errLog
        CmdLine = $cmdLine
    }
}

function Stop-ProcTree($Proc) {
    if ($Proc -and -not $Proc.HasExited) {
        & $env:ComSpec /c "taskkill /T /F /PID $($Proc.Id)" | Out-Null
    }
}

function Ensure-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-SpectreHost "[red]✗[/]  未找到命令: [dim]$Name[/]"
        return $false
    }
    return $true
}

$choice = Read-SpectreSelection `
    -Message "Xun Dev" `
    -Choices @("启动后端+前端", "退出") `
    -Color "SteelBlue1"

if ($choice -eq "退出") { return }

if (-not (Ensure-Command "cargo")) { return }
if (-not (Get-Command vite -ErrorAction SilentlyContinue) -and -not (Ensure-Command "npx")) {
    Write-SpectreHost "[red]✗[/]  未找到 vite 或 npx"
    return
}

$viteCmd = if (Get-Command vite -ErrorAction SilentlyContinue) { "vite" } else { "npx" }
$viteArgs = if ($viteCmd -eq "npx") { @("vite") } else { @() }

$backend = $null
$frontend = $null

try {
    Write-SpectreHost "[dim]启动后端...[/]"
    $backend = Start-Command -Command "cargo" -Arguments @(
        "run",
        "--features",
        "dashboard,lock",
        "--",
        "serve",
        "-p",
        "$BackendPort"
    ) -WorkDir "$Root" -Prefix "backend"

    if (-not (Wait-ForPort -Title "Backend" -Port $BackendPort -Process $backend.Process -TimeoutSec 30)) {
        Write-SpectreHost "[grey]后端日志: $($backend.ErrLog)[/]"
        return
    }

    $fePort = Get-FrontendPort
    Write-SpectreHost "[dim]启动前端...[/]"
    $frontend = Start-Command -Command $viteCmd -Arguments $viteArgs -WorkDir "$UI" -Prefix "frontend"

    if (-not (Wait-ForPort -Title "Frontend" -Port $fePort -Process $frontend.Process -TimeoutSec 30)) {
        Write-SpectreHost "[grey]前端日志: $($frontend.ErrLog)[/]"
        return
    }

    @(
        "Backend  → http://localhost:$BackendPort",
        "Frontend → http://localhost:$fePort",
        "",
        '[grey]选择"停止并退出"结束进程[/]'
    ) -join "`n" |
        Format-SpectrePanel -Header " Xun Dev " -Color "SteelBlue1" |
        Out-SpectreHost

    $null = Read-SpectreSelection `
        -Message "运行中" `
        -Choices @("停止并退出") `
        -Color "SteelBlue1"
} finally {
    if ($frontend) { Stop-ProcTree $frontend.Process }
    if ($backend) { Stop-ProcTree $backend.Process }
    Write-SpectreHost "[grey]已停止[/]"
}
