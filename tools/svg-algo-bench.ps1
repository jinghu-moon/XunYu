[CmdletBinding()]
param(
    [string]$RepoRoot = "",
    [string]$ManifestPath = "",
    [string]$DatasetRoot = "",
    [string]$OutRoot = "",
    [string]$Xun = "",
    [string[]]$Methods = @("visioncortex", "bezier", "potrace", "skeleton", "diffvg"),
    [int]$Threads = 8,
    [int]$TimeoutSec = 600,
    [int]$DiffvgIters = 40,
    [int]$DiffvgStrokes = 64,
    [int]$MaxPerCategory = 0,
    [switch]$SkipDownload,
    [switch]$OnlyDownload,
    [switch]$OverwriteDataset
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

function Ensure-Dir {
    param([string]$Path)
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Download-WithRetry {
    param(
        [string]$Url,
        [string]$OutFile,
        [int]$RetryCount = 3,
        [int]$RequestTimeoutSec = 45
    )
    $tmp = "$OutFile.tmp"
    for ($try = 1; $try -le $RetryCount; $try++) {
        try {
            if (Test-Path -LiteralPath $tmp) {
                Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
            }
            Invoke-WebRequest -Uri $Url -OutFile $tmp -TimeoutSec $RequestTimeoutSec -MaximumRedirection 5
            $len = (Get-Item -LiteralPath $tmp).Length
            if ($len -le 0) {
                throw "downloaded file is empty"
            }
            Move-Item -LiteralPath $tmp -Destination $OutFile -Force
            return
        } catch {
            if (Test-Path -LiteralPath $tmp) {
                Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
            }
            if ($try -ge $RetryCount) {
                throw
            }
            Start-Sleep -Seconds (2 * $try)
        }
    }
}

function Measure-PathCount {
    param([string]$FilePath)
    $hits = @(Select-String -Path $FilePath -Pattern "<path " -AllMatches -ErrorAction SilentlyContinue)
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

function Sum-FileBytes {
    param([object[]]$Items)
    if ($null -eq $Items -or $Items.Count -eq 0) {
        return 0L
    }
    [int64]$sum = 0
    foreach ($item in $Items) {
        if ($null -eq $item) { continue }
        $prop = $item.PSObject.Properties["Length"]
        if ($null -eq $prop) { continue }
        $sum += [int64]$item.Length
    }
    return $sum
}

function Append-LogSafely {
    param(
        [string]$Path,
        [string]$Text
    )
    try {
        [System.IO.File]::AppendAllText($Path, ($Text + [Environment]::NewLine), [System.Text.Encoding]::UTF8)
    } catch {
        # no-op, keep benchmark loop alive
    }
}

function To-FileRows {
    param(
        [string]$Category,
        [string]$Method,
        [System.Collections.Generic.List[object]]$Inputs,
        [string]$MethodOutDir
    )
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($f in $Inputs) {
        $stem = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
        $outSvg = Join-Path $MethodOutDir "$stem.svg"
        if (Test-Path -LiteralPath $outSvg) {
            $outItem = Get-Item -LiteralPath $outSvg
            $pathCount = Measure-PathCount -FilePath $outSvg
            $rows.Add([PSCustomObject]@{
                    category = $Category
                    method = $Method
                    input_file = $f.Name
                    input_bytes = $f.Length
                    output_exists = $true
                    output_bytes = $outItem.Length
                    path_count = $pathCount
                    output_file = $outSvg
                }) | Out-Null
        } else {
            $rows.Add([PSCustomObject]@{
                    category = $Category
                    method = $Method
                    input_file = $f.Name
                    input_bytes = $f.Length
                    output_exists = $false
                    output_bytes = 0
                    path_count = 0
                    output_file = $outSvg
                }) | Out-Null
        }
    }
    return $rows
}

$repo = Resolve-RepoRoot -ParamRepoRoot $RepoRoot

if (-not $ManifestPath) {
    $ManifestPath = Join-Path $PSScriptRoot "svg-algo-sample-manifest.json"
}
if (-not (Test-Path -LiteralPath $ManifestPath)) {
    throw "manifest not found: $ManifestPath"
}

if (-not $DatasetRoot) {
    $DatasetRoot = Join-Path $repo "image-test/svg-algo-samples"
}
if (-not $OutRoot) {
    $OutRoot = Join-Path $repo "image-test"
}

$validMethods = @("bezier", "visioncortex", "potrace", "skeleton", "diffvg")
$methodsLower = @()
foreach ($m in $Methods) {
    $mm = $m.ToLowerInvariant()
    if ($validMethods -notcontains $mm) {
        throw "invalid method: $m (valid: $($validMethods -join ', '))"
    }
    $methodsLower += $mm
}
$methodsLower = @($methodsLower | Select-Object -Unique)

$manifestRaw = Get-Content -Raw -LiteralPath $ManifestPath | ConvertFrom-Json
$manifest = @()
foreach ($entry in $manifestRaw) {
    if (-not $entry.category -or -not $entry.name -or -not $entry.url) {
        throw "manifest item missing required field: $($entry | ConvertTo-Json -Compress)"
    }
    $manifest += $entry
}

Ensure-Dir -Path $DatasetRoot
Ensure-Dir -Path $OutRoot

$downloadRows = New-Object System.Collections.Generic.List[object]

if (-not $SkipDownload) {
    Write-Host "[download] start: $($manifest.Count) files"
    foreach ($item in $manifest) {
        $catDir = Join-Path $DatasetRoot $item.category
        Ensure-Dir -Path $catDir
        $dst = Join-Path $catDir $item.name
        $status = "downloaded"
        $err = ""

        try {
            if ((-not $OverwriteDataset) -and (Test-Path -LiteralPath $dst)) {
                $status = "skipped_exists"
            } else {
                Download-WithRetry -Url $item.url -OutFile $dst
            }
            $size = if (Test-Path -LiteralPath $dst) { (Get-Item -LiteralPath $dst).Length } else { 0 }
        } catch {
            $status = "failed"
            $err = $_.Exception.Message
            $size = 0
        }

        $downloadRows.Add([PSCustomObject]@{
                category = $item.category
                name = $item.name
                url = $item.url
                status = $status
                bytes = $size
                error = $err
                path = $dst
            }) | Out-Null
        Write-Host ("[download] {0}/{1} {2}/{3} -> {4}" -f ($downloadRows.Count), $manifest.Count, $item.category, $item.name, $status)
    }
} else {
    foreach ($item in $manifest) {
        $dst = Join-Path (Join-Path $DatasetRoot $item.category) $item.name
        $exists = Test-Path -LiteralPath $dst
        $downloadRows.Add([PSCustomObject]@{
                category = $item.category
                name = $item.name
                url = $item.url
                status = $(if ($exists) { "existing" } else { "missing" })
                bytes = $(if ($exists) { (Get-Item -LiteralPath $dst).Length } else { 0 })
                error = ""
                path = $dst
            }) | Out-Null
    }
}

$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$downloadCsv = Join-Path $DatasetRoot "download-$ts.csv"
$downloadRows | Export-Csv -Path $downloadCsv -NoTypeInformation -Encoding UTF8
Write-Host "[download] report: $downloadCsv"

if ($OnlyDownload) {
    Write-Host "[done] only download mode"
    return
}

$xunExe = Resolve-XunExe -Repo $repo -Preferred $Xun

$runDir = Join-Path $OutRoot "svg-algo-bench-$ts"
$runInputRoot = Join-Path $runDir "input"
$runOutputRoot = Join-Path $runDir "output"
$runLogRoot = Join-Path $runDir "logs"
Ensure-Dir -Path $runDir
Ensure-Dir -Path $runInputRoot
Ensure-Dir -Path $runOutputRoot
Ensure-Dir -Path $runLogRoot

$categoryRows = New-Object System.Collections.Generic.List[object]
$fileRows = New-Object System.Collections.Generic.List[object]

$categories = $manifest | Group-Object -Property category | Sort-Object Name
$totalRuns = $categories.Count * $methodsLower.Count
$runIndex = 0

foreach ($cg in $categories) {
    $category = $cg.Name
    $entries = $cg.Group | Sort-Object name
    $available = New-Object System.Collections.Generic.List[object]

    foreach ($e in $entries) {
        $p = Join-Path (Join-Path $DatasetRoot $category) $e.name
        if (Test-Path -LiteralPath $p) {
            $available.Add((Get-Item -LiteralPath $p)) | Out-Null
        }
    }

    if ($available.Count -eq 0) {
        Write-Warning "skip category '$category': no available input files"
        continue
    }

    $selected = @($available | Sort-Object Name)
    if ($MaxPerCategory -gt 0) {
        $selected = @($selected | Select-Object -First $MaxPerCategory)
    }

    $categoryInputDir = Join-Path $runInputRoot $category
    Ensure-Dir -Path $categoryInputDir
    foreach ($f in $selected) {
        Copy-Item -LiteralPath $f.FullName -Destination (Join-Path $categoryInputDir $f.Name) -Force
    }

    $inputBytes = Sum-FileBytes -Items $selected

    foreach ($method in $methodsLower) {
        $runIndex += 1
        $methodOutDir = Join-Path (Join-Path $runOutputRoot $category) $method
        $methodLogDir = Join-Path (Join-Path $runLogRoot $category) $method
        Ensure-Dir -Path $methodOutDir
        Ensure-Dir -Path $methodLogDir

        $stdoutLog = Join-Path $methodLogDir "stdout.log"
        $stderrLog = Join-Path $methodLogDir "stderr.log"

        $argsList = @(
            "img",
            "-i", $categoryInputDir,
            "-o", $methodOutDir,
            "-f", "svg",
            "--svg-method", $method,
            "-t", "$Threads",
            "--overwrite"
        )
        if ($method -eq "diffvg") {
            $argsList += @(
                "--svg-diffvg-iters", "$DiffvgIters",
                "--svg-diffvg-strokes", "$DiffvgStrokes"
            )
        }

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $timedOut = $false
        $exitCode = -1

        try {
            $proc = Start-Process -FilePath $xunExe -ArgumentList $argsList -NoNewWindow -PassThru -RedirectStandardOutput $stdoutLog -RedirectStandardError $stderrLog
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
                Append-LogSafely -Path $stderrLog -Text ("[bench] timeout after {0}s" -f $TimeoutSec)
            }
        } catch {
            $exitCode = -1
            Append-LogSafely -Path $stderrLog -Text ("[bench] process start/exec failed: {0}" -f $_.Exception.Message)
        } finally {
            $sw.Stop()
        }

        $outSvgs = @(Get-ChildItem -Path $methodOutDir -Recurse -File -Filter "*.svg" -ErrorAction SilentlyContinue)
        $outBytes = Sum-FileBytes -Items $outSvgs
        $pathCounts = @()
        foreach ($svgFile in $outSvgs) {
            $pathCounts += Measure-PathCount -FilePath $svgFile.FullName
        }

        $avgPaths = 0.0
        $minPaths = 0
        $maxPaths = 0
        if ($pathCounts.Count -gt 0) {
            $avgPaths = [Math]::Round((($pathCounts | Measure-Object -Average).Average), 2)
            $minPaths = ($pathCounts | Measure-Object -Minimum).Minimum
            $maxPaths = ($pathCounts | Measure-Object -Maximum).Maximum
        }

        $elapsedMs = [Math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        $ratio = 0.0
        if ($inputBytes -gt 0) {
            $ratio = [Math]::Round(($outBytes / [double]$inputBytes), 4)
        }

        $categoryRows.Add([PSCustomObject]@{
                category = $category
                method = $method
                exit_code = $exitCode
                timed_out = $timedOut
                elapsed_ms = $elapsedMs
                input_files = $selected.Count
                input_total_bytes = $inputBytes
                output_files = $outSvgs.Count
                output_total_bytes = $outBytes
                output_input_ratio = $ratio
                avg_paths = $avgPaths
                min_paths = $minPaths
                max_paths = $maxPaths
                out_dir = $methodOutDir
                stdout_log = $stdoutLog
                stderr_log = $stderrLog
            }) | Out-Null

        $rows = To-FileRows -Category $category -Method $method -Inputs ([System.Collections.Generic.List[object]]$selected) -MethodOutDir $methodOutDir
        foreach ($r in $rows) { $fileRows.Add($r) | Out-Null }

        Write-Host ("[run] {0}/{1} {2}/{3} exit={4} timeout={5} elapsed_ms={6}" -f $runIndex, $totalRuns, $category, $method, $exitCode, $timedOut, $elapsedMs)
    }
}

$resultsCsv = Join-Path $runDir "results.csv"
$resultsFilesCsv = Join-Path $runDir "results.files.csv"
$categoryRows | Export-Csv -Path $resultsCsv -NoTypeInformation -Encoding UTF8
$fileRows | Export-Csv -Path $resultsFilesCsv -NoTypeInformation -Encoding UTF8

$summaryByMethod = $categoryRows | Group-Object method | ForEach-Object {
    $g = $_.Group
    $ok = @($g | Where-Object { $_.exit_code -eq 0 -and -not $_.timed_out }).Count
    [PSCustomObject]@{
        method = $_.Name
        runs = $g.Count
        success_runs = $ok
        failed_runs = $g.Count - $ok
        avg_elapsed_ms = [Math]::Round((($g | Measure-Object elapsed_ms -Average).Average), 2)
        avg_output_input_ratio = [Math]::Round((($g | Measure-Object output_input_ratio -Average).Average), 4)
        avg_paths = [Math]::Round((($g | Measure-Object avg_paths -Average).Average), 2)
    }
} | Sort-Object method

$summaryByCategory = $categoryRows | Group-Object category | ForEach-Object {
    $g = $_.Group
    $ok = @($g | Where-Object { $_.exit_code -eq 0 -and -not $_.timed_out }).Count
    $fastest = $g | Sort-Object elapsed_ms | Select-Object -First 1
    $detail = $g | Sort-Object avg_paths -Descending | Select-Object -First 1
    [PSCustomObject]@{
        category = $_.Name
        methods_tested = $g.Count
        success_runs = $ok
        failed_runs = $g.Count - $ok
        fastest_method = $fastest.method
        fastest_elapsed_ms = $fastest.elapsed_ms
        richest_detail_method = $detail.method
        richest_avg_paths = $detail.avg_paths
    }
} | Sort-Object category

$summaryMethodCsv = Join-Path $runDir "summary.by_method.csv"
$summaryCategoryCsv = Join-Path $runDir "summary.by_category.csv"
$summaryByMethod | Export-Csv -Path $summaryMethodCsv -NoTypeInformation -Encoding UTF8
$summaryByCategory | Export-Csv -Path $summaryCategoryCsv -NoTypeInformation -Encoding UTF8

$reportPath = Join-Path $runDir "report.md"
$report = New-Object System.Text.StringBuilder
[void]$report.AppendLine("# SVG Algorithm Benchmark Report")
[void]$report.AppendLine("")
[void]$report.AppendLine("- Generated at: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")")
[void]$report.AppendLine("- RepoRoot: ``$repo``")
[void]$report.AppendLine("- Xun: ``$xunExe``")
[void]$report.AppendLine("- DatasetRoot: ``$DatasetRoot``")
[void]$report.AppendLine("- Methods: $($methodsLower -join ", ")")
[void]$report.AppendLine("- TimeoutSec: $TimeoutSec")
[void]$report.AppendLine("- Threads: $Threads")
[void]$report.AppendLine("- DiffvgIters: $DiffvgIters")
[void]$report.AppendLine("- DiffvgStrokes: $DiffvgStrokes")
[void]$report.AppendLine("- MaxPerCategory: $MaxPerCategory")
[void]$report.AppendLine("")
[void]$report.AppendLine("## Output Files")
[void]$report.AppendLine("")
[void]$report.AppendLine("- results.csv: ``results.csv``")
[void]$report.AppendLine("- results.files.csv: ``results.files.csv``")
[void]$report.AppendLine("- summary.by_method.csv: ``summary.by_method.csv``")
[void]$report.AppendLine("- summary.by_category.csv: ``summary.by_category.csv``")
[void]$report.AppendLine("- download report: ``$(Split-Path -Leaf $downloadCsv)``")
[void]$report.AppendLine("")
[void]$report.AppendLine("## Method Summary")
[void]$report.AppendLine("")
[void]$report.AppendLine("| method | runs | success | failed | avg_elapsed_ms | avg_output_input_ratio | avg_paths |")
[void]$report.AppendLine("|---|---:|---:|---:|---:|---:|---:|")
foreach ($r in $summaryByMethod) {
    [void]$report.AppendLine("| $($r.method) | $($r.runs) | $($r.success_runs) | $($r.failed_runs) | $($r.avg_elapsed_ms) | $($r.avg_output_input_ratio) | $($r.avg_paths) |")
}
[void]$report.AppendLine("")
[void]$report.AppendLine("## Category Summary")
[void]$report.AppendLine("")
[void]$report.AppendLine("| category | methods_tested | success | failed | fastest_method | fastest_elapsed_ms | richest_detail_method | richest_avg_paths |")
[void]$report.AppendLine("|---|---:|---:|---:|---|---:|---|---:|")
foreach ($r in $summaryByCategory) {
    [void]$report.AppendLine("| $($r.category) | $($r.methods_tested) | $($r.success_runs) | $($r.failed_runs) | $($r.fastest_method) | $($r.fastest_elapsed_ms) | $($r.richest_detail_method) | $($r.richest_avg_paths) |")
}

$report.ToString() | Set-Content -Path $reportPath -Encoding UTF8

Write-Host ""
Write-Host "[done] run dir: $runDir"
Write-Host "[done] results: $resultsCsv"
Write-Host "[done] file results: $resultsFilesCsv"
Write-Host "[done] method summary: $summaryMethodCsv"
Write-Host "[done] category summary: $summaryCategoryCsv"
Write-Host "[done] markdown report: $reportPath"
