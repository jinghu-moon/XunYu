#Requires -Version 7.0

[CmdletBinding()]
param(
    [string]$Bin,
    [string[]]$CompleteArgs = @("z", ""),
    [int]$Runs = 100,
    [int]$Warmup = 5,
    [string[]]$Env = @(),
    [string]$Out = "",
    [string]$OutTsv = "",
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
if (-not $Bin) {
    $release = Join-Path $Root "target\release\xun.exe"
    $debug = Join-Path $Root "target\debug\xun.exe"
    if (Test-Path $release) {
        $Bin = $release
    } elseif (Test-Path $debug) {
        $Bin = $debug
    }
}

if (-not $Bin -or -not (Test-Path $Bin)) {
    Write-Host "xun binary not found. Build first: cargo build --release"
    exit 1
}

$cmdArgs = @("__complete") + $CompleteArgs

$envBackup = @{}
foreach ($pair in $Env) {
    if ($pair -notmatch "^(?<name>[^=]+)=(?<value>.*)$") {
        throw "Env must be NAME=VALUE: $pair"
    }
    $name = $Matches["name"]
    $value = $Matches["value"]
    $envBackup[$name] = (Get-Item -Path "Env:$name" -ErrorAction SilentlyContinue).Value
    Set-Item -Path "Env:$name" -Value $value
}

function Get-Percentile([double[]]$arr, [double]$p) {
    if ($arr.Count -eq 0) { return 0 }
    $index = [math]::Ceiling($p * $arr.Count) - 1
    if ($index -lt 0) { $index = 0 }
    if ($index -ge $arr.Count) { $index = $arr.Count - 1 }
    return [math]::Round($arr[$index], 2)
}

try {
    for ($i = 0; $i -lt $Warmup; $i++) {
        & $Bin @cmdArgs > $null
        $null = $LASTEXITCODE
    }

    $durations = New-Object System.Collections.Generic.List[double]
    $candidates = New-Object System.Collections.Generic.List[int]
    $exitCodes = New-Object System.Collections.Generic.List[int]
    $fallbacks = 0

    for ($i = 0; $i -lt $Runs; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $output = & $Bin @cmdArgs 2>$null
        $exit = $LASTEXITCODE
        $sw.Stop()

        $durations.Add($sw.Elapsed.TotalMilliseconds)
        $exitCodes.Add($exit)

        $lines = $output -split "`r?`n" | Where-Object { $_ -ne "" }
        $sentinel = $lines | Where-Object { $_ -like "__XUN_COMPLETE__=*" } | Select-Object -Last 1
        if ($sentinel -eq "__XUN_COMPLETE__=fallback") {
            $fallbacks++
        }
        $cand = ($lines | Where-Object { $_ -notlike "__XUN_COMPLETE__=*" }).Count
        $candidates.Add($cand)
    }

    $sorted = $durations | Sort-Object
    $avg = [math]::Round(($durations | Measure-Object -Average).Average, 2)
    $min = [math]::Round(($durations | Measure-Object -Minimum).Minimum, 2)
    $max = [math]::Round(($durations | Measure-Object -Maximum).Maximum, 2)
    $p50 = Get-Percentile $sorted 0.50
    $p95 = Get-Percentile $sorted 0.95
    $p99 = Get-Percentile $sorted 0.99
    $exitNonZero = ($exitCodes | Where-Object { $_ -ne 0 }).Count

    $candSorted = $candidates | Sort-Object
    $candMin = if ($candSorted.Count -gt 0) { $candSorted[0] } else { 0 }
    $candMax = if ($candSorted.Count -gt 0) { $candSorted[$candSorted.Count - 1] } else { 0 }
    $candP50 = if ($candSorted.Count -gt 0) { [int](Get-Percentile $candSorted 0.50) } else { 0 }

    $summary = [ordered]@{
        ts = (Get-Date).ToString("s")
        bin = $Bin
        args = ($CompleteArgs -join " ")
        runs = $Runs
        warmup = $Warmup
        avg_ms = $avg
        min_ms = $min
        p50_ms = $p50
        p95_ms = $p95
        p99_ms = $p99
        max_ms = $max
        exit_nonzero = $exitNonZero
        fallback_runs = $fallbacks
        cand_min = $candMin
        cand_p50 = $candP50
        cand_max = $candMax
        env = $Env
    }

    if (-not $Quiet) {
        Write-Host "xun __complete bench"
        Write-Host "  bin: $Bin"
        Write-Host "  args: $($CompleteArgs -join ' ')"
        Write-Host "  runs: $Runs (warmup: $Warmup)"
        Write-Host "  avg_ms: $avg | p50: $p50 | p95: $p95 | p99: $p99 | min: $min | max: $max"
        Write-Host "  candidates: min=$candMin p50=$candP50 max=$candMax"
        Write-Host "  exit_nonzero: $exitNonZero | fallback_runs: $fallbacks"
    }

    if ($Out) {
        $json = $summary | ConvertTo-Json -Compress
        Add-Content -LiteralPath $Out -Value $json
    }

    if ($OutTsv) {
        $envJoined = $Env -join ";"
        $line = @(
            $summary.ts,
            $summary.bin,
            $summary.args,
            $summary.runs,
            $summary.warmup,
            $summary.avg_ms,
            $summary.min_ms,
            $summary.p50_ms,
            $summary.p95_ms,
            $summary.p99_ms,
            $summary.max_ms,
            $summary.exit_nonzero,
            $summary.fallback_runs,
            $summary.cand_min,
            $summary.cand_p50,
            $summary.cand_max,
            $envJoined
        ) -join "`t"
        Add-Content -LiteralPath $OutTsv -Value $line
    }
} finally {
    foreach ($name in $envBackup.Keys) {
        if ($envBackup[$name] -eq $null) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            Set-Item -Path "Env:$name" -Value $envBackup[$name]
        }
    }
}
