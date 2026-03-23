$ErrorActionPreference = 'Stop'

$debugExe = Join-Path $PSScriptRoot '..\target\debug\xun.exe'
$releaseExe = Join-Path $PSScriptRoot '..\target\release\xun.exe'
$exe = if ($env:XUN_BIN) {
    $env:XUN_BIN
} elseif (Test-Path $debugExe) {
    $debugExe
} else {
    $releaseExe
}
$logDir = Join-Path $PSScriptRoot '..\logs'
$logPath = Join-Path $logDir 'backup_export_baseline_20260323.md'

if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

function New-BenchProject([string]$root) {
    if (Test-Path $root) {
        Remove-Item -Recurse -Force $root
    }
    New-Item -ItemType Directory -Path $root | Out-Null
    $dirs = @('src', 'docs', 'assets', 'deep/nested', 'path with spaces')
    foreach ($d in $dirs) {
        New-Item -ItemType Directory -Path (Join-Path $root $d) -Force | Out-Null
    }
    for ($i = 0; $i -lt 200; $i++) {
        $dir = $dirs[$i % $dirs.Count]
        $size = 1024 + (($i * 37) % 8192)
        $content = 'x' * $size
        Set-Content -Path (Join-Path (Join-Path $root $dir) ("file_{0:d4}.txt" -f $i)) -Value $content -NoNewline
    }
    Set-Content -Path (Join-Path $root 'empty.txt') -Value '' -NoNewline
    $cfg = @"
{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "bench" },
  "retention": { "maxBackups": 20, "deleteCount": 1 },
  "include": [ "src", "docs", "assets", "deep", "path with spaces", "empty.txt" ],
  "exclude": []
}
"@
    Set-Content -Path (Join-Path $root '.xun-bak.json') -Value $cfg -NoNewline
}

function Run-Xun([string]$root, [string[]]$argv) {
    $env:XUN_DB = Join-Path $root '.xun.json'
    $env:USERPROFILE = $root
    $env:HOME = $root
    $env:XUN_NON_INTERACTIVE = '1'
    $output = & $exe @argv 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "xun failed ($LASTEXITCODE): $output"
    }
}

function Measure-Scenario([string]$name, [scriptblock]$setup, [scriptblock]$run) {
    $samples = @()
    for ($i = 0; $i -lt 3; $i++) {
        & $setup $i
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & $run $i
        $sw.Stop()
        $samples += [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
    }
    [pscustomobject]@{
        Name = $name
        AvgMs = [math]::Round((($samples | Measure-Object -Average).Average), 2)
        MinMs = [math]::Round((($samples | Measure-Object -Minimum).Minimum), 2)
        MaxMs = [math]::Round((($samples | Measure-Object -Maximum).Maximum), 2)
        Samples = ($samples -join ', ')
    }
}

$base = Join-Path $env:TEMP ('xun-export-bench-' + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $base | Out-Null
$results = @()

$results += Measure-Scenario 'backup create --format xunbak' {
    param($i)
    $script:root = Join-Path $base ("create-xunbak-$i")
    New-BenchProject $script:root
} {
    param($i)
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'xunbak', '-o', 'artifact.xunbak')
}

$results += Measure-Scenario 'backup create --format zip' {
    param($i)
    $script:root = Join-Path $base ("create-zip-$i")
    New-BenchProject $script:root
} {
    param($i)
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'zip', '-o', 'artifact.zip')
}

$results += Measure-Scenario 'backup create --format 7z' {
    param($i)
    $script:root = Join-Path $base ("create-7z-$i")
    New-BenchProject $script:root
} {
    param($i)
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', '7z', '-o', 'artifact.7z')
}

$results += Measure-Scenario 'backup convert xunbak -> zip' {
    param($i)
    $script:root = Join-Path $base ("convert-zip-$i")
    New-BenchProject $script:root
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'xunbak', '-o', 'artifact.xunbak')
} {
    param($i)
    Run-Xun $script:root @('backup', 'convert', (Join-Path $script:root 'artifact.xunbak'), '--format', 'zip', '-o', (Join-Path $script:root 'converted.zip'))
}

$results += Measure-Scenario 'backup convert xunbak -> 7z' {
    param($i)
    $script:root = Join-Path $base ("convert-7z-$i")
    New-BenchProject $script:root
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'xunbak', '-o', 'artifact.xunbak')
} {
    param($i)
    Run-Xun $script:root @('backup', 'convert', (Join-Path $script:root 'artifact.xunbak'), '--format', '7z', '-o', (Join-Path $script:root 'converted.7z'))
}

$results += Measure-Scenario 'backup restore xunbak' {
    param($i)
    $script:root = Join-Path $base ("restore-xunbak-$i")
    New-BenchProject $script:root
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'xunbak', '-o', 'artifact.xunbak')
} {
    param($i)
    $restore = Join-Path $script:root 'restore-xunbak'
    if (Test-Path $restore) { Remove-Item -Recurse -Force $restore }
    Run-Xun $script:root @('backup', 'restore', (Join-Path $script:root 'artifact.xunbak'), '--to', $restore, '-C', $script:root, '-y')
}

$results += Measure-Scenario 'backup restore zip' {
    param($i)
    $script:root = Join-Path $base ("restore-zip-$i")
    New-BenchProject $script:root
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', 'zip', '-o', 'artifact.zip')
} {
    param($i)
    $restore = Join-Path $script:root 'restore-zip'
    if (Test-Path $restore) { Remove-Item -Recurse -Force $restore }
    Run-Xun $script:root @('backup', 'restore', (Join-Path $script:root 'artifact.zip'), '--to', $restore, '-C', $script:root, '-y')
}

$results += Measure-Scenario 'backup restore 7z' {
    param($i)
    $script:root = Join-Path $base ("restore-7z-$i")
    New-BenchProject $script:root
    Run-Xun $script:root @('backup', 'create', '-C', $script:root, '--format', '7z', '-o', 'artifact.7z')
} {
    param($i)
    $restore = Join-Path $script:root 'restore-7z'
    if (Test-Path $restore) { Remove-Item -Recurse -Force $restore }
    Run-Xun $script:root @('backup', 'restore', (Join-Path $script:root 'artifact.7z'), '--to', $restore, '-C', $script:root, '-y')
}

$lines = @(
    '# Backup Export Baseline',
    '',
    '- Date: 2026-03-23',
    '- Binary: target/release/xun.exe',
    '- Dataset: synthetic 200 files + empty file + nested/space paths',
    '- Samples per scenario: 3',
    '',
    '| Scenario | Avg ms | Min ms | Max ms | Samples |',
    '|---|---:|---:|---:|---|'
)

foreach ($r in $results) {
    $lines += "| $($r.Name) | $($r.AvgMs) | $($r.MinMs) | $($r.MaxMs) | $($r.Samples) |"
}

$lines | Set-Content -Path $logPath
Remove-Item -Recurse -Force $base
Write-Output "Wrote $logPath"
