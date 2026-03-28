param(
    [switch]$IncludeExport,
    [string]$Features = 'xunbak',
    [string]$OutPath
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$logDir = Join-Path $repoRoot 'logs'
if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

if ([string]::IsNullOrWhiteSpace($OutPath)) {
    $stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
    $OutPath = Join-Path $logDir ("backup_perf_baseline_" + $stamp + ".md")
}

function Invoke-BenchMedian {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Bench,
        [Parameter(Mandatory = $true)]
        [string]$Filter
    )

    Push-Location $repoRoot
    try {
        $command = "cargo bench --bench $Bench --features $Features -- $Filter 2>&1"
        $output = & cmd /c $command | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "cargo bench failed for ${Bench}::${Filter}`n$output"
        }
    }
    finally {
        Pop-Location
    }

    $line = ($output -split "`r?`n" | Where-Object { $_ -match [regex]::Escape($Filter) } | Select-Object -First 1)
    if (-not $line) {
        throw "failed to locate benchmark row for ${Bench}::${Filter}`n$output"
    }

    $columnSep = [string][char]0x2502
    $parts = $line -split [regex]::Escape($columnSep)
    if ($parts.Count -lt 4) {
        throw "failed to parse benchmark row for ${Bench}::${Filter}`n$line"
    }
    $left = $parts[0]
    $nameIndex = $left.IndexOf($Filter)
    if ($nameIndex -lt 0) {
        throw "failed to locate benchmark name in row for ${Bench}::${Filter}`n$line"
    }
    $fastest = $left.Substring($nameIndex + $Filter.Length).Trim()
    $slowest = $parts[1].Trim()
    $median = $parts[2].Trim()
    $mean = $parts[3].Trim()

    [pscustomobject]@{
        Bench = $Bench
        Name = $Filter
        Fastest = $fastest
        Slowest = $slowest
        Median = $median
        Mean = $mean
    }
}

function Invoke-RestoreTimingSample {
    Push-Location $repoRoot
    try {
        $env:XUN_XUNBAK_RESTORE_TIMING = '1'
        $build = & cmd /c "cargo build --profile bench --features $Features --bench backup_perf_bench_divan 2>&1" | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for restore timing sample`n$build"
        }
        $exe = Get-ChildItem -Path (Join-Path $repoRoot "target/release/deps") -Filter "backup_perf_bench_divan-*.exe" |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if (-not $exe) {
            throw "failed to locate backup_perf_bench_divan executable"
        }
        $command = '"' + $exe.FullName + '" --test xunbak_restore_all_1000_files 2>&1'
        $output = & cmd /c $command | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "restore timing sample failed`n$output"
        }
    }
    finally {
        Remove-Item Env:\XUN_XUNBAK_RESTORE_TIMING -ErrorAction SilentlyContinue
        Pop-Location
    }

    $match = [regex]::Match($output, 'perf: xunbak restore .*')
    if (-not $match.Success) {
        throw "failed to parse restore timing sample`n$output"
    }
    return $match.Value.Trim()
}

$benchmarks = @(
    @{ Bench = 'backup_perf_bench_divan'; Name = 'hash_file_content_64mb' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'sidecar_build_missing_hash_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'sidecar_build_prehash_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'verify_entries_content_dir_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'verify_entries_content_xunbak_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'verify_full_xunbak_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'xunbak_restore_all_1000_files' },
    @{ Bench = 'backup_perf_bench_divan'; Name = 'xunbak_restore_incremental_1000_files' }
)

if ($IncludeExport) {
    $benchmarks += @(
        @{ Bench = 'backup_export_bench_divan'; Name = 'create_zip_200_files' },
        @{ Bench = 'backup_export_bench_divan'; Name = 'create_7z_200_files' },
        @{ Bench = 'backup_export_bench_divan'; Name = 'restore_zip_200_files' },
        @{ Bench = 'backup_export_bench_divan'; Name = 'restore_7z_200_files' }
    )
}

$results = foreach ($item in $benchmarks) {
    Invoke-BenchMedian -Bench $item.Bench -Filter $item.Name
}

$restoreTiming = Invoke-RestoreTimingSample

$lines = @(
    '# Backup Perf Baseline',
    '',
    "- Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
    "- Repo: $repoRoot",
    "- Features: $Features",
    "- IncludeExport: $IncludeExport",
    '',
    '## Benchmarks',
    '',
    '| Bench | Median | Mean | Fastest | Slowest |',
    '| --- | ---: | ---: | ---: | ---: |'
)

foreach ($result in $results) {
    $lines += "| $($result.Name) | $($result.Median) | $($result.Mean) | $($result.Fastest) | $($result.Slowest) |"
}

$lines += ''
$lines += '## Restore Timing Sample'
$lines += ''
$lines += "````text"
$lines += $restoreTiming
$lines += "````"
$lines += ''
$lines += '## Commands'
$lines += ''
$lines += '```powershell'
$lines += '.\scripts\backup_perf_baseline.ps1'
$lines += '.\scripts\backup_perf_baseline.ps1 -IncludeExport'
$lines += '```'

$lines | Set-Content -Path $OutPath
Write-Output "Wrote $OutPath"
