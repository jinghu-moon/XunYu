param(
    [int]$Runs = 3,
    [int]$Warmup = 1
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$debugExe = Join-Path $repoRoot 'target/debug/xun.exe'
$releaseExe = Join-Path $repoRoot 'target/release/xun.exe'
$exe = if ($env:XUN_BIN) {
    $env:XUN_BIN
} elseif (Test-Path $debugExe) {
    $debugExe
} else {
    $releaseExe
}
$logDir = Join-Path $repoRoot 'logs'
$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$logPath = Join-Path $logDir ("backup_file_id_baseline_" + $stamp + ".md")
$base = Join-Path $env:TEMP ('xun-fileid-bench-' + [guid]::NewGuid().ToString())

if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

Push-Location $repoRoot
try {
    cargo build
    if ($LASTEXITCODE -ne 0) {
        throw 'cargo build failed'
    }
}
finally {
    Pop-Location
}

function Write-BenchConfig([string]$root, [string[]]$includePaths) {
    $includeJson = ($includePaths | ForEach-Object { '"' + $_ + '"' }) -join ', '
    $cfg = @"
{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "bench" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ $includeJson ],
  "exclude": []
}
"@
    Set-Content -Path (Join-Path $root '.xun-bak.json') -Value $cfg -NoNewline
}

function Invoke-BackupJson {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [Parameter(Mandatory = $true)]
        [string[]]$Args,
        [switch]$DisableFileId
    )

    $env:XUN_DB = Join-Path $Root '.xun.json'
    $env:USERPROFILE = $Root
    $env:HOME = $Root
    $env:XUN_NON_INTERACTIVE = '1'
    if ($DisableFileId) {
        $env:XUN_BACKUP_DISABLE_FILE_ID = '1'
    } else {
        Remove-Item Env:\XUN_BACKUP_DISABLE_FILE_ID -ErrorAction SilentlyContinue
    }

    try {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $stdout = & $exe @Args 2>$null
        $code = $LASTEXITCODE
        $sw.Stop()
        if ($code -ne 0) {
            throw "xun failed ($code): $stdout"
        }
        $text = ($stdout | Out-String).Trim()
        $json = $text | ConvertFrom-Json
        return [PSCustomObject]@{
            ElapsedMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
            HashCacheHits = [int64]$json.hash_cache_hits
            HashComputedFiles = [int64]$json.hash_computed_files
            HashCheckedFiles = [int64]$json.hash_checked_files
        }
    }
    finally {
        Remove-Item Env:\XUN_BACKUP_DISABLE_FILE_ID -ErrorAction SilentlyContinue
    }
}

function New-RenameFixture {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [int]$Count = 200
    )

    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Root 'src') -Force | Out-Null
    for ($i = 0; $i -lt $Count; $i++) {
        Set-Content -Path (Join-Path $Root 'src' ("old_{0:d4}.txt" -f $i)) -Value ("payload-" + $i) -NoNewline
    }
    Write-BenchConfig $Root @('src')
}

function Prepare-RenameScenario {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [switch]$DisableFileId,
        [int]$Count = 200
    )

    New-RenameFixture -Root $Root -Count $Count
    Invoke-BackupJson -Root $Root -Args @('backup', '-C', $Root, '-m', 'v1', '--json') -DisableFileId:$DisableFileId | Out-Null
    Start-Sleep -Milliseconds 50
    for ($i = 0; $i -lt $Count; $i++) {
        Move-Item -LiteralPath (Join-Path $Root 'src' ("old_{0:d4}.txt" -f $i)) -Destination (Join-Path $Root 'src' ("new_{0:d4}.txt" -f $i))
    }
}

function New-SmallChangeFixture {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [int]$Count = 500
    )

    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Root 'src') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Root 'public') -Force | Out-Null
    for ($i = 0; $i -lt $Count; $i++) {
        $dir = if (($i % 2) -eq 0) { 'src' } else { 'public' }
        $body = ('x' * (512 + (($i * 31) % 1024)))
        Set-Content -Path (Join-Path $Root $dir ("file_{0:d4}.txt" -f $i)) -Value $body -NoNewline
    }
    Write-BenchConfig $Root @('src', 'public')
}

function Prepare-ColdSmallChangeScenario {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [switch]$DisableFileId,
        [int]$Count = 500,
        [int]$Changed = 10
    )

    New-SmallChangeFixture -Root $Root -Count $Count
    Invoke-BackupJson -Root $Root -Args @('backup', '-C', $Root, '-m', 'v1', '--json') -DisableFileId:$DisableFileId | Out-Null
    Start-Sleep -Milliseconds 50
    for ($i = 0; $i -lt $Changed; $i++) {
        $dir = if (($i % 2) -eq 0) { 'src' } else { 'public' }
        Set-Content -Path (Join-Path $Root $dir ("file_{0:d4}.txt" -f $i)) -Value ("modified-" + $i + ('y' * 768)) -NoNewline
    }
    Remove-Item (Join-Path $Root '.xun-bak-hash-cache.json') -Force -ErrorAction SilentlyContinue
}

function Measure-Scenario {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Setup,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Run
    )

    $samples = @()
    for ($i = 0; $i -lt ($Warmup + $Runs); $i++) {
        $root = Join-Path $base ($Name.Replace(' ', '_') + '-' + $i)
        if (Test-Path $root) {
            Remove-Item -Recurse -Force $root
        }
        & $Setup $root
        $result = & $Run $root
        if ($i -lt $Warmup) {
            continue
        }
        $samples += $result
    }

    return [PSCustomObject]@{
        Name = $Name
        AvgMs = [math]::Round((($samples.ElapsedMs | Measure-Object -Average).Average), 2)
        AvgHits = [math]::Round((($samples.HashCacheHits | Measure-Object -Average).Average), 2)
        AvgComputed = [math]::Round((($samples.HashComputedFiles | Measure-Object -Average).Average), 2)
        SamplesMs = ($samples | ForEach-Object { $_.ElapsedMs }) -join ', '
    }
}

New-Item -ItemType Directory -Path $base | Out-Null

$results = @()

$results += Measure-Scenario 'rename-only v2 (file_id off)' {
    param($root)
    Prepare-RenameScenario -Root $root -DisableFileId
} {
    param($root)
    Invoke-BackupJson -Root $root -Args @('backup', '-C', $root, '-m', 'v2', '--json') -DisableFileId
}

$results += Measure-Scenario 'rename-only v2 (file_id on)' {
    param($root)
    Prepare-RenameScenario -Root $root
} {
    param($root)
    Invoke-BackupJson -Root $root -Args @('backup', '-C', $root, '-m', 'v2', '--json')
}

$results += Measure-Scenario 'cold small-change v2 (file_id off)' {
    param($root)
    Prepare-ColdSmallChangeScenario -Root $root -DisableFileId
} {
    param($root)
    Invoke-BackupJson -Root $root -Args @('backup', '-C', $root, '-m', 'v2', '--json') -DisableFileId
}

$results += Measure-Scenario 'cold small-change v2 (file_id on)' {
    param($root)
    Prepare-ColdSmallChangeScenario -Root $root
} {
    param($root)
    Invoke-BackupJson -Root $root -Args @('backup', '-C', $root, '-m', 'v2', '--json')
}

$lines = @(
    '# Backup File ID Baseline',
    '',
    "- Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
    "- Binary: $exe",
    "- Runs: $Runs",
    "- Warmup: $Warmup",
    '- Scenarios:',
    '  - rename-only: 200 files renamed in-place after v1',
    '  - cold small-change: 500 files, 10 modified after v1, hash cache file removed before v2',
    '',
    '| Scenario | Avg ms | Avg hash_cache_hits | Avg hash_computed_files | Samples ms |',
    '| --- | ---: | ---: | ---: | --- |'
)

foreach ($result in $results) {
    $lines += "| $($result.Name) | $($result.AvgMs) | $($result.AvgHits) | $($result.AvgComputed) | $($result.SamplesMs) |"
}

$lines += ''
$lines += '## Notes'
$lines += ''
$lines += '1. `file_id off` 通过 `XUN_BACKUP_DISABLE_FILE_ID=1` 禁用扫描阶段的真实 file_id 采集。'
$lines += '2. rename-only 场景用于观察跨路径 hash cache 命中收益。'
$lines += '3. cold small-change 场景用于观察启用 file_id 后在冷缓存下的总体开销变化。'

$lines | Set-Content -Path $logPath
Write-Host "Wrote $logPath"
