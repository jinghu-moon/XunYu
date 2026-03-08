[CmdletBinding()]
param(
    [string]$RepoRoot = "",
    [string]$DatasetRoot = "",
    [string]$OutRoot = "",
    [string]$Xun = "",
    [string]$Vtracer = "",
    [int]$Threads = 1,
    [int]$TimeoutSec = 300,
    [int]$MaxFiles = 0,
    [string]$VtracerPreset = "photo",
    [switch]$SkipVtracer
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$ProgressPreference = "SilentlyContinue"

function Resolve-RepoRoot {
    param([string]$ParamRepoRoot)
    if ($ParamRepoRoot) {
        return (Resolve-Path -LiteralPath $ParamRepoRoot).Path
    }
    return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
}

function Resolve-XunExe {
    param(
        [string]$Repo,
        [string]$Preferred
    )
    if ($Preferred) {
        if (-not (Test-Path -LiteralPath $Preferred)) {
            throw "xun executable not found: $Preferred"
        }
        return (Resolve-Path -LiteralPath $Preferred).Path
    }

    $candidates = @(
        (Join-Path $Repo "target/release-img/xun.exe"),
        (Join-Path $Repo "target/release/xun.exe"),
        (Join-Path $Repo "target/debug/xun.exe")
    )
    foreach ($c in $candidates) {
        if (Test-Path -LiteralPath $c) {
            return (Resolve-Path -LiteralPath $c).Path
        }
    }
    throw "xun executable not found. Checked: $($candidates -join ', ')"
}

function Resolve-VtracerExe {
    param([string]$Preferred)
    if ($Preferred) {
        if (-not (Test-Path -LiteralPath $Preferred)) {
            throw "vtracer executable not found: $Preferred"
        }
        return (Resolve-Path -LiteralPath $Preferred).Path
    }

    $cmd = Get-Command vtracer -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
        throw "vtracer executable not found in PATH"
    }
    return $cmd.Source
}

function Ensure-Dir {
    param([string]$Path)
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Append-LogSafely {
    param(
        [string]$Path,
        [string]$Text
    )
    try {
        [System.IO.File]::AppendAllText($Path, ($Text + [Environment]::NewLine), [System.Text.Encoding]::UTF8)
    } catch {
        # keep benchmark loop alive
    }
}

function Measure-PathCount {
    param([string]$FilePath)
    $hits = @(Select-String -Path $FilePath -Pattern "<path\b" -AllMatches -ErrorAction SilentlyContinue)
    if ($hits.Count -eq 0) {
        return 0
    }
    $count = 0
    foreach ($h in $hits) {
        if ($null -eq $h) { continue }
        $mprop = $h.PSObject.Properties["Matches"]
        if ($null -eq $mprop -or $null -eq $h.Matches) { continue }
        $count += $h.Matches.Count
    }
    return $count
}

function Run-ProcessWithTimeout {
    param(
        [string]$Exe,
        [string[]]$ArgList,
        [string]$StdoutLog,
        [string]$StderrLog,
        [int]$TimeoutSec
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $timedOut = $false
    $exitCode = -1

    try {
        $proc = Start-Process -FilePath $Exe -ArgumentList $ArgList -NoNewWindow -PassThru -RedirectStandardOutput $StdoutLog -RedirectStandardError $StderrLog
        $completed = $true
        try {
            Wait-Process -Id $proc.Id -Timeout $TimeoutSec -ErrorAction Stop
        } catch {
            $completed = $false
        }
        if ($completed) {
            $proc.Refresh()
            $exitCode = $proc.ExitCode
            if ($null -eq $exitCode) { $exitCode = 0 }
        } else {
            $timedOut = $true
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            $exitCode = -1
            Append-LogSafely -Path $StderrLog -Text ("[bench] timeout after {0}s" -f $TimeoutSec)
        }
    } catch {
        $exitCode = -1
        Append-LogSafely -Path $StderrLog -Text ("[bench] process start/exec failed: {0}" -f $_.Exception.Message)
    } finally {
        $sw.Stop()
    }

    return [PSCustomObject]@{
        exit_code = $exitCode
        timed_out = $timedOut
        elapsed_ms = [Math]::Round($sw.Elapsed.TotalMilliseconds, 1)
    }
}

$repo = Resolve-RepoRoot -ParamRepoRoot $RepoRoot
if (-not $DatasetRoot) {
    $DatasetRoot = Join-Path $repo "image-test/svg-algo-samples"
}
if (-not $OutRoot) {
    $OutRoot = Join-Path $repo "image-test"
}
if (-not (Test-Path -LiteralPath $DatasetRoot)) {
    throw "dataset root not found: $DatasetRoot"
}

$xunExe = Resolve-XunExe -Repo $repo -Preferred $Xun
$vtracerExe = if ($SkipVtracer) { "" } else { Resolve-VtracerExe -Preferred $Vtracer }

$extSet = @(".png", ".jpg", ".jpeg", ".bmp", ".webp")
$allInputs = @(Get-ChildItem -Path $DatasetRoot -Recurse -File | Where-Object {
        $extSet -contains $_.Extension.ToLowerInvariant()
    } | Sort-Object FullName)

if ($allInputs.Count -eq 0) {
    throw "no input files found under: $DatasetRoot"
}
if ($MaxFiles -gt 0) {
    $allInputs = @($allInputs | Select-Object -First $MaxFiles)
}

$cases = New-Object System.Collections.Generic.List[object]
$seq = 0
foreach ($f in $allInputs) {
    $seq += 1
    $id = ("{0:D3}_{1}" -f $seq, [System.IO.Path]::GetFileNameWithoutExtension($f.Name))
    $rel = [System.IO.Path]::GetRelativePath($DatasetRoot, $f.FullName)
    $category = ($rel -split "[/\\]")[0]
    $cases.Add([PSCustomObject]@{
            id = $id
            category = $category
            input_file = $f.Name
            input_rel = $rel
            input_full = $f.FullName
            input_bytes = $f.Length
        }) | Out-Null
}

$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $OutRoot "vtracer-vs-official-$ts"
$runOutputRoot = Join-Path $runDir "output"
$runLogRoot = Join-Path $runDir "logs"
$xunOutRoot = Join-Path $runOutputRoot "xun_visioncortex"
$vtOutRoot = Join-Path $runOutputRoot "vtracer_official"
$xunLogRoot = Join-Path $runLogRoot "xun_visioncortex"
$vtLogRoot = Join-Path $runLogRoot "vtracer_official"
Ensure-Dir -Path $runDir
Ensure-Dir -Path $runOutputRoot
Ensure-Dir -Path $runLogRoot
Ensure-Dir -Path $xunOutRoot
Ensure-Dir -Path $vtOutRoot
Ensure-Dir -Path $xunLogRoot
Ensure-Dir -Path $vtLogRoot

$rows = New-Object System.Collections.Generic.List[object]

Write-Host ("[bench] inputs={0}, timeout={1}s, threads={2}" -f $cases.Count, $TimeoutSec, $Threads)
Write-Host ("[bench] xun={0}" -f $xunExe)
if ($SkipVtracer) {
    Write-Host "[bench] vtracer=<skipped>"
} else {
    Write-Host ("[bench] vtracer={0}" -f $vtracerExe)
}

$totalRuns = if ($SkipVtracer) { $cases.Count } else { $cases.Count * 2 }
$runIdx = 0

foreach ($c in $cases) {
    $runIdx += 1
    $caseOutDir = Join-Path $xunOutRoot $c.id
    Ensure-Dir -Path $caseOutDir
    $stdoutLog = Join-Path $xunLogRoot ($c.id + ".stdout.log")
    $stderrLog = Join-Path $xunLogRoot ($c.id + ".stderr.log")
    $xunArgs = @(
        "img",
        "-i", $c.input_full,
        "-o", $caseOutDir,
        "-f", "svg",
        "--svg-method", "visioncortex",
        "-t", "$Threads",
        "--overwrite"
    )
    $procResult = Run-ProcessWithTimeout -Exe $xunExe -ArgList $xunArgs -StdoutLog $stdoutLog -StderrLog $stderrLog -TimeoutSec $TimeoutSec
    $outSvg = @(Get-ChildItem -Path $caseOutDir -Recurse -File -Filter "*.svg" -ErrorAction SilentlyContinue | Select-Object -First 1)
    $outExists = $outSvg.Count -gt 0
    $outBytes = if ($outExists) { $outSvg[0].Length } else { 0 }
    $pathCount = if ($outExists) { Measure-PathCount -FilePath $outSvg[0].FullName } else { 0 }

    $rows.Add([PSCustomObject]@{
            tool = "xun_visioncortex"
            case_id = $c.id
            category = $c.category
            input_file = $c.input_file
            input_rel = $c.input_rel
            input_bytes = $c.input_bytes
            exit_code = $procResult.exit_code
            timed_out = $procResult.timed_out
            elapsed_ms = $procResult.elapsed_ms
            output_exists = $outExists
            output_bytes = $outBytes
            path_count = $pathCount
            output_file = $(if ($outExists) { $outSvg[0].FullName } else { "" })
            stdout_log = $stdoutLog
            stderr_log = $stderrLog
        }) | Out-Null
    Write-Host ("[run] {0}/{1} xun_visioncortex {2} exit={3} timeout={4} elapsed_ms={5}" -f $runIdx, $totalRuns, $c.input_rel, $procResult.exit_code, $procResult.timed_out, $procResult.elapsed_ms)
}

if (-not $SkipVtracer) {
    foreach ($c in $cases) {
        $runIdx += 1
        $outSvg = Join-Path $vtOutRoot ($c.id + ".svg")
        $stdoutLog = Join-Path $vtLogRoot ($c.id + ".stdout.log")
        $stderrLog = Join-Path $vtLogRoot ($c.id + ".stderr.log")
        $vtArgs = @(
            "--input", $c.input_full,
            "--output", $outSvg
        )
        if (-not [string]::IsNullOrWhiteSpace($VtracerPreset)) {
            $vtArgs += @("--preset", $VtracerPreset)
        }
        $procResult = Run-ProcessWithTimeout -Exe $vtracerExe -ArgList $vtArgs -StdoutLog $stdoutLog -StderrLog $stderrLog -TimeoutSec $TimeoutSec
        $outExists = Test-Path -LiteralPath $outSvg
        $outBytes = if ($outExists) { (Get-Item -LiteralPath $outSvg).Length } else { 0 }
        $pathCount = if ($outExists) { Measure-PathCount -FilePath $outSvg } else { 0 }

        $rows.Add([PSCustomObject]@{
                tool = "vtracer_official"
                case_id = $c.id
                category = $c.category
                input_file = $c.input_file
                input_rel = $c.input_rel
                input_bytes = $c.input_bytes
                exit_code = $procResult.exit_code
                timed_out = $procResult.timed_out
                elapsed_ms = $procResult.elapsed_ms
                output_exists = $outExists
                output_bytes = $outBytes
                path_count = $pathCount
                output_file = $(if ($outExists) { $outSvg } else { "" })
                stdout_log = $stdoutLog
                stderr_log = $stderrLog
            }) | Out-Null
        Write-Host ("[run] {0}/{1} vtracer_official {2} exit={3} timeout={4} elapsed_ms={5}" -f $runIdx, $totalRuns, $c.input_rel, $procResult.exit_code, $procResult.timed_out, $procResult.elapsed_ms)
    }
}

$resultsCsv = Join-Path $runDir "results.csv"
$rows | Export-Csv -Path $resultsCsv -NoTypeInformation -Encoding UTF8

$summaryByTool = $rows | Group-Object tool | ForEach-Object {
    $g = $_.Group
    $ok = @($g | Where-Object { $_.exit_code -eq 0 -and -not $_.timed_out -and $_.output_exists }).Count
    $totalElapsed = ($g | Measure-Object elapsed_ms -Sum).Sum
    $totalOut = ($g | Measure-Object output_bytes -Sum).Sum
    [PSCustomObject]@{
        tool = $_.Name
        runs = $g.Count
        success_runs = $ok
        failed_runs = $g.Count - $ok
        total_elapsed_ms = [Math]::Round($totalElapsed, 1)
        avg_elapsed_ms = [Math]::Round((($g | Measure-Object elapsed_ms -Average).Average), 2)
        p50_elapsed_ms = [Math]::Round((($g | Sort-Object elapsed_ms | Select-Object -Skip ([Math]::Floor($g.Count / 2)) -First 1).elapsed_ms), 2)
        total_output_bytes = [int64]$totalOut
        avg_output_bytes = [Math]::Round((($g | Measure-Object output_bytes -Average).Average), 2)
        avg_path_count = [Math]::Round((($g | Measure-Object path_count -Average).Average), 2)
    }
} | Sort-Object tool

$summaryCsv = Join-Path $runDir "summary.by_tool.csv"
$summaryByTool | Export-Csv -Path $summaryCsv -NoTypeInformation -Encoding UTF8

$summaryByCategory = $rows | Group-Object category, tool | ForEach-Object {
    $g = $_.Group
    $ok = @($g | Where-Object { $_.exit_code -eq 0 -and -not $_.timed_out -and $_.output_exists }).Count
    [PSCustomObject]@{
        category = $g[0].category
        tool = $g[0].tool
        runs = $g.Count
        success_runs = $ok
        failed_runs = $g.Count - $ok
        avg_elapsed_ms = [Math]::Round((($g | Measure-Object elapsed_ms -Average).Average), 2)
        avg_output_bytes = [Math]::Round((($g | Measure-Object output_bytes -Average).Average), 2)
        avg_path_count = [Math]::Round((($g | Measure-Object path_count -Average).Average), 2)
    }
} | Sort-Object category, tool

$summaryCategoryCsv = Join-Path $runDir "summary.by_category.csv"
$summaryByCategory | Export-Csv -Path $summaryCategoryCsv -NoTypeInformation -Encoding UTF8

$xunSummary = $summaryByTool | Where-Object { $_.tool -eq "xun_visioncortex" } | Select-Object -First 1
$vtSummary = $summaryByTool | Where-Object { $_.tool -eq "vtracer_official" } | Select-Object -First 1
$speedRatio = $null
if ($xunSummary -and $vtSummary -and $vtSummary.avg_elapsed_ms -gt 0) {
    $speedRatio = [Math]::Round(($xunSummary.avg_elapsed_ms / [double]$vtSummary.avg_elapsed_ms), 3)
}

$reportPath = Join-Path $runDir "report.md"
$report = New-Object System.Text.StringBuilder
[void]$report.AppendLine("# VTracer vs Project Visioncortex Benchmark")
[void]$report.AppendLine("")
[void]$report.AppendLine("- Generated at: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")")
[void]$report.AppendLine("- RepoRoot: ``$repo``")
[void]$report.AppendLine("- DatasetRoot: ``$DatasetRoot``")
[void]$report.AppendLine("- Input files: $($cases.Count)")
[void]$report.AppendLine("- Xun exe: ``$xunExe``")
[void]$report.AppendLine("- VTracer exe: ``$(if ($SkipVtracer) { '<skipped>' } else { $vtracerExe })``")
[void]$report.AppendLine("- Xun threads: $Threads")
[void]$report.AppendLine("- TimeoutSec: $TimeoutSec")
[void]$report.AppendLine("- VTracer preset: $(if ($SkipVtracer) { '<skipped>' } elseif ([string]::IsNullOrWhiteSpace($VtracerPreset)) { '<default>' } else { $VtracerPreset })")
[void]$report.AppendLine("")
[void]$report.AppendLine("## Output Files")
[void]$report.AppendLine("")
[void]$report.AppendLine("- results.csv: ``results.csv``")
[void]$report.AppendLine("- summary.by_tool.csv: ``summary.by_tool.csv``")
[void]$report.AppendLine("- summary.by_category.csv: ``summary.by_category.csv``")
[void]$report.AppendLine("")
[void]$report.AppendLine("## Summary By Tool")
[void]$report.AppendLine("")
[void]$report.AppendLine("| tool | runs | success | failed | total_elapsed_ms | avg_elapsed_ms | p50_elapsed_ms | avg_output_bytes | avg_path_count |")
[void]$report.AppendLine("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
foreach ($r in $summaryByTool) {
    [void]$report.AppendLine("| $($r.tool) | $($r.runs) | $($r.success_runs) | $($r.failed_runs) | $($r.total_elapsed_ms) | $($r.avg_elapsed_ms) | $($r.p50_elapsed_ms) | $($r.avg_output_bytes) | $($r.avg_path_count) |")
}
[void]$report.AppendLine("")
[void]$report.AppendLine("## Comparison")
[void]$report.AppendLine("")
if ($null -ne $speedRatio) {
    [void]$report.AppendLine("- avg_elapsed ratio (xun_visioncortex / vtracer_official): **$speedRatio**")
} else {
    [void]$report.AppendLine("- avg_elapsed ratio: N/A")
}
[void]$report.AppendLine("")
[void]$report.AppendLine("## Summary By Category")
[void]$report.AppendLine("")
[void]$report.AppendLine("| category | tool | runs | success | failed | avg_elapsed_ms | avg_output_bytes | avg_path_count |")
[void]$report.AppendLine("|---|---|---:|---:|---:|---:|---:|---:|")
foreach ($r in $summaryByCategory) {
    [void]$report.AppendLine("| $($r.category) | $($r.tool) | $($r.runs) | $($r.success_runs) | $($r.failed_runs) | $($r.avg_elapsed_ms) | $($r.avg_output_bytes) | $($r.avg_path_count) |")
}

$report.ToString() | Set-Content -Path $reportPath -Encoding UTF8

Write-Host ""
Write-Host "[done] run dir: $runDir"
Write-Host "[done] results: $resultsCsv"
Write-Host "[done] summary tool: $summaryCsv"
Write-Host "[done] summary category: $summaryCategoryCsv"
Write-Host "[done] report: $reportPath"
