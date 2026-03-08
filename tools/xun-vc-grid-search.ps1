[CmdletBinding()]
param(
    [string]$RepoRoot = "",
    [string]$DatasetRoot = "",
    [string]$OutRoot = "",
    [string]$Xun = "",
    [int]$Threads = 1,
    [int]$TimeoutSec = 300,
    [int]$MaxFiles = 0,
    [string]$RsvgConvert = "rsvg-convert",
    [string[]]$QuantShiftComplexCandidates = @("2", "3", "4"),
    [string[]]$MergeColorDeltaComplexCandidates = @("24", "32", "40"),
    [string[]]$MergeSmallAreaComplexCandidates = @("1"),
    [string[]]$MinComponentAreaComplexCandidates = @("1"),
    [int]$MaxProcesses = 10,
    [int]$RepeatPerCase = 3,
    [string]$Tag = "xun-vc-grid"
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

function Ensure-Dir {
    param([string]$Path)
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Expand-IntCandidates {
    param(
        [object[]]$Values,
        [string]$Name
    )
    $out = New-Object System.Collections.Generic.List[int]
    foreach ($v in $Values) {
        if ($null -eq $v) {
            continue
        }
        $parts = ("$v" -split "[,;\s]+")
        foreach ($p in $parts) {
            if ([string]::IsNullOrWhiteSpace($p)) {
                continue
            }
            $n = 0
            if (-not [int]::TryParse($p.Trim(), [ref]$n)) {
                throw "$Name contains non-integer value: '$p'"
            }
            $out.Add($n) | Out-Null
        }
    }
    if ($out.Count -eq 0) {
        throw "$Name is empty after parsing"
    }
    return @($out)
}

function As-Double {
    param(
        [object]$Value,
        [double]$Default = 0.0
    )
    if ($null -eq $Value) {
        return $Default
    }
    $s = "$Value"
    $n = 0.0
    if ([double]::TryParse($s, [System.Globalization.NumberStyles]::Float, [System.Globalization.CultureInfo]::InvariantCulture, [ref]$n)) {
        return $n
    }
    if ([double]::TryParse($s, [ref]$n)) {
        return $n
    }
    return $Default
}

function Mean-Value {
    param([object[]]$Values)
    if ($Values.Count -eq 0) {
        return 0.0
    }
    return [double](($Values | Measure-Object -Average).Average)
}

function Std-Value {
    param([object[]]$Values)
    if ($Values.Count -le 1) {
        return 0.0
    }
    $avg = Mean-Value -Values $Values
    $sumSq = 0.0
    foreach ($v in $Values) {
        $d = [double]$v - $avg
        $sumSq += ($d * $d)
    }
    return [Math]::Sqrt($sumSq / $Values.Count)
}

function Normalize-Up {
    param(
        [double]$Value,
        [double]$Min,
        [double]$Max
    )
    if ($Max -le $Min) {
        return 1.0
    }
    return ($Value - $Min) / ($Max - $Min)
}

function Normalize-Down {
    param(
        [double]$Value,
        [double]$Min,
        [double]$Max
    )
    if ($Max -le $Min) {
        return 1.0
    }
    return ($Max - $Value) / ($Max - $Min)
}

function Min-Value {
    param(
        [object[]]$Rows,
        [string]$Property
    )
    $vals = @($Rows | ForEach-Object { As-Double $_.$Property } | Sort-Object)
    if ($vals.Count -eq 0) {
        return 0.0
    }
    return [double]$vals[0]
}

function Max-Value {
    param(
        [object[]]$Rows,
        [string]$Property
    )
    $vals = @($Rows | ForEach-Object { As-Double $_.$Property } | Sort-Object)
    if ($vals.Count -eq 0) {
        return 1.0
    }
    return [double]$vals[$vals.Count - 1]
}

if ($MaxProcesses -lt 1) {
    throw "MaxProcesses must be >= 1"
}
if ($RepeatPerCase -lt 1) {
    throw "RepeatPerCase must be >= 1"
}

$quantCandidates = Expand-IntCandidates -Values $QuantShiftComplexCandidates -Name "QuantShiftComplexCandidates"
$mergeDeltaCandidates = Expand-IntCandidates -Values $MergeColorDeltaComplexCandidates -Name "MergeColorDeltaComplexCandidates"
$mergeAreaCandidates = Expand-IntCandidates -Values $MergeSmallAreaComplexCandidates -Name "MergeSmallAreaComplexCandidates"
$minAreaCandidates = Expand-IntCandidates -Values $MinComponentAreaComplexCandidates -Name "MinComponentAreaComplexCandidates"

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

$benchScript = Join-Path $repo "tools/vtracer-vs-official-bench.ps1"
$qualityScript = Join-Path $repo "tools/svg-quality-bench.py"
if (-not (Test-Path -LiteralPath $benchScript)) {
    throw "benchmark script not found: $benchScript"
}
if (-not (Test-Path -LiteralPath $qualityScript)) {
    throw "quality script not found: $qualityScript"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$gridRoot = Join-Path $OutRoot "$Tag-$stamp"
Ensure-Dir -Path $gridRoot

$cases = New-Object System.Collections.Generic.List[object]
$seq = 0
foreach ($q in $quantCandidates) {
    foreach ($delta in $mergeDeltaCandidates) {
        foreach ($mergeArea in $mergeAreaCandidates) {
            foreach ($minArea in $minAreaCandidates) {
                $seq += 1
                $cases.Add([PSCustomObject]@{
                        case_id = ("case-{0:D3}" -f $seq)
                        quant_shift_complex = [int]$q
                        merge_color_delta_complex = [int]$delta
                        merge_small_area_complex = [int]$mergeArea
                        min_component_area_complex = [int]$minArea
                    }) | Out-Null
            }
        }
    }
}
if ($cases.Count -eq 0) {
    throw "no parameter combinations generated"
}

$caseWorker = {
    param(
        [pscustomobject]$Case,
        [int]$CaseIndex,
        [int]$CaseTotal,
        [int]$RepeatPerCase,
        [string]$CaseRoot,
        [string]$Repo,
        [string]$DatasetRoot,
        [string]$BenchScript,
        [string]$QualityScript,
        [string]$Xun,
        [int]$Threads,
        [int]$TimeoutSec,
        [int]$MaxFiles,
        [string]$RsvgConvert
    )

    Set-StrictMode -Version Latest
    $ErrorActionPreference = "Stop"
    $ProgressPreference = "SilentlyContinue"

    function As-DoubleLocal {
        param(
            [object]$Value,
            [double]$Default = 0.0
        )
        if ($null -eq $Value) {
            return $Default
        }
        $s = "$Value"
        $n = 0.0
        if ([double]::TryParse($s, [System.Globalization.NumberStyles]::Float, [System.Globalization.CultureInfo]::InvariantCulture, [ref]$n)) {
            return $n
        }
        if ([double]::TryParse($s, [ref]$n)) {
            return $n
        }
        return $Default
    }

    function Mean-ValueLocal {
        param([object[]]$Values)
        if ($Values.Count -eq 0) {
            return 0.0
        }
        return [double](($Values | Measure-Object -Average).Average)
    }

    function Std-ValueLocal {
        param([object[]]$Values)
        if ($Values.Count -le 1) {
            return 0.0
        }
        $avg = Mean-ValueLocal -Values $Values
        $sumSq = 0.0
        foreach ($v in $Values) {
            $d = [double]$v - $avg
            $sumSq += ($d * $d)
        }
        return [Math]::Sqrt($sumSq / $Values.Count)
    }

    function Build-CaseResult {
        param(
            [bool]$RunOk,
            [int]$RepeatTotal,
            [int]$RepeatSuccess,
            [string]$ErrorMessage,
            [string]$CaseRoot,
            [string]$RepeatCsv,
            [string]$BenchRunDir,
            [string]$QualityDir,
            [double]$AvgElapsed,
            [double]$StdElapsed,
            [double]$AvgOutputBytes,
            [double]$StdOutputBytes,
            [double]$AvgPathCount,
            [double]$StdPathCount,
            [double]$PsnrMean,
            [double]$PsnrStd,
            [double]$SsimMean,
            [double]$SsimStd,
            [double]$MaeMean,
            [double]$MaeStd
        )
        return [PSCustomObject]@{
            case_id = $Case.case_id
            quant_shift_complex = $Case.quant_shift_complex
            merge_color_delta_complex = $Case.merge_color_delta_complex
            merge_small_area_complex = $Case.merge_small_area_complex
            min_component_area_complex = $Case.min_component_area_complex
            run_ok = $RunOk
            repeat_total = $RepeatTotal
            repeat_success = $RepeatSuccess
            repeat_failed = $RepeatTotal - $RepeatSuccess
            error = $ErrorMessage
            case_root = $CaseRoot
            repeat_csv = $RepeatCsv
            bench_run_dir = $BenchRunDir
            quality_dir = $QualityDir
            avg_elapsed_ms = [Math]::Round($AvgElapsed, 6)
            std_elapsed_ms = [Math]::Round($StdElapsed, 6)
            avg_output_bytes = [Math]::Round($AvgOutputBytes, 6)
            std_output_bytes = [Math]::Round($StdOutputBytes, 6)
            avg_path_count = [Math]::Round($AvgPathCount, 6)
            std_path_count = [Math]::Round($StdPathCount, 6)
            psnr_mean = [Math]::Round($PsnrMean, 6)
            psnr_std = [Math]::Round($PsnrStd, 6)
            ssim_mean = [Math]::Round($SsimMean, 6)
            ssim_std = [Math]::Round($SsimStd, 6)
            mae_mean = [Math]::Round($MaeMean, 6)
            mae_std = [Math]::Round($MaeStd, 6)
        }
    }

    try {
        New-Item -ItemType Directory -Force -Path $CaseRoot | Out-Null
        $repeatRows = New-Object System.Collections.Generic.List[object]

        for ($r = 1; $r -le $RepeatPerCase; $r++) {
            $repeatTag = ("repeat-{0:D2}" -f $r)
            $repeatRoot = Join-Path $CaseRoot $repeatTag
            New-Item -ItemType Directory -Force -Path $repeatRoot | Out-Null

            $runOk = $true
            $err = ""
            $runDirPath = ""
            $qualityDirPath = ""
            $perf = $null
            $qual = $null

            try {
                $env:XUN_VC_CLUSTER_BACKEND = "xun"
                $env:XUN_VC_QUANT_SHIFT_COMPLEX = "$($Case.quant_shift_complex)"
                $env:XUN_VC_MERGE_COLOR_DELTA_COMPLEX = "$($Case.merge_color_delta_complex)"
                $env:XUN_VC_MERGE_SMALL_AREA_COMPLEX = "$($Case.merge_small_area_complex)"
                $env:XUN_VC_MIN_COMPONENT_AREA_COMPLEX = "$($Case.min_component_area_complex)"

                $benchArgs = @(
                    "-ExecutionPolicy", "Bypass",
                    "-File", $BenchScript,
                    "-RepoRoot", $Repo,
                    "-DatasetRoot", $DatasetRoot,
                    "-OutRoot", $repeatRoot,
                    "-Threads", "$Threads",
                    "-TimeoutSec", "$TimeoutSec",
                    "-SkipVtracer"
                )
                if (-not [string]::IsNullOrWhiteSpace($Xun)) {
                    $benchArgs += @("-Xun", $Xun)
                }
                if ($MaxFiles -gt 0) {
                    $benchArgs += @("-MaxFiles", "$MaxFiles")
                }

                & pwsh @benchArgs 2>&1 | Out-Null
                if ($LASTEXITCODE -ne 0) {
                    throw "benchmark failed with exit code $LASTEXITCODE"
                }

                $runDir = Get-ChildItem -Path $repeatRoot -Directory -Filter "vtracer-vs-official-*" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
                if ($null -eq $runDir) {
                    throw "benchmark run directory not found under $repeatRoot"
                }
                $runDirPath = $runDir.FullName

                $resultsCsv = Join-Path $runDirPath "results.csv"
                if (-not (Test-Path -LiteralPath $resultsCsv)) {
                    throw "results csv not found: $resultsCsv"
                }

                $qualityDirPath = Join-Path $runDirPath "quality-eval"
                $qualityArgs = @(
                    $QualityScript,
                    "--results-csv", $resultsCsv,
                    "--dataset-root", $DatasetRoot,
                    "--out-dir", $qualityDirPath,
                    "--rsvg-convert", $RsvgConvert
                )
                & python @qualityArgs 2>&1 | Out-Null
                if ($LASTEXITCODE -ne 0) {
                    throw "quality evaluation failed with exit code $LASTEXITCODE"
                }

                $perf = Import-Csv (Join-Path $runDirPath "summary.by_tool.csv") | Where-Object { $_.tool -eq "xun_visioncortex" } | Select-Object -First 1
                if ($null -eq $perf) {
                    throw "xun_visioncortex row missing in summary.by_tool.csv"
                }
                $qual = Import-Csv (Join-Path $qualityDirPath "quality.summary.by_tool.csv") | Where-Object { $_.tool -eq "xun_visioncortex" } | Select-Object -First 1
                if ($null -eq $qual) {
                    throw "xun_visioncortex row missing in quality.summary.by_tool.csv"
                }
            } catch {
                $runOk = $false
                $err = $_.Exception.Message
            }

            $repeatRows.Add([PSCustomObject]@{
                    case_id = $Case.case_id
                    repeat_id = $repeatTag
                    run_ok = $runOk
                    error = $err
                    bench_run_dir = $runDirPath
                    quality_dir = $qualityDirPath
                    avg_elapsed_ms = $(if ($runOk) { As-DoubleLocal $perf.avg_elapsed_ms } else { 0.0 })
                    avg_output_bytes = $(if ($runOk) { As-DoubleLocal $perf.avg_output_bytes } else { 0.0 })
                    avg_path_count = $(if ($runOk) { As-DoubleLocal $perf.avg_path_count } else { 0.0 })
                    psnr_mean = $(if ($runOk) { As-DoubleLocal $qual.psnr_mean } else { 0.0 })
                    ssim_mean = $(if ($runOk) { As-DoubleLocal $qual.ssim_mean } else { 0.0 })
                    mae_mean = $(if ($runOk) { As-DoubleLocal $qual.mae_mean } else { 0.0 })
                }) | Out-Null
        }

        $repeatCsv = Join-Path $CaseRoot "case.repeats.csv"
        $repeatRows | Export-Csv -Path $repeatCsv -NoTypeInformation -Encoding UTF8

        $okRows = @($repeatRows | Where-Object { $_.run_ok -eq $true })
        $okCount = $okRows.Count
        $allOk = $okCount -eq $RepeatPerCase
        $firstErr = ""
        if (-not $allOk) {
            $firstErr = ($repeatRows | Where-Object { $_.run_ok -eq $false } | Select-Object -First 1 -ExpandProperty error)
        }

        $benchRunDir = ""
        $qualityDir = ""
        if ($okCount -gt 0) {
            $lastOk = $okRows | Select-Object -Last 1
            $benchRunDir = $lastOk.bench_run_dir
            $qualityDir = $lastOk.quality_dir
        }

        $elapsedVals = @($okRows | ForEach-Object { [double]$_.avg_elapsed_ms })
        $bytesVals = @($okRows | ForEach-Object { [double]$_.avg_output_bytes })
        $pathVals = @($okRows | ForEach-Object { [double]$_.avg_path_count })
        $psnrVals = @($okRows | ForEach-Object { [double]$_.psnr_mean })
        $ssimVals = @($okRows | ForEach-Object { [double]$_.ssim_mean })
        $maeVals = @($okRows | ForEach-Object { [double]$_.mae_mean })

        return Build-CaseResult `
            -RunOk $allOk `
            -RepeatTotal $RepeatPerCase `
            -RepeatSuccess $okCount `
            -ErrorMessage $firstErr `
            -CaseRoot $CaseRoot `
            -RepeatCsv $repeatCsv `
            -BenchRunDir $benchRunDir `
            -QualityDir $qualityDir `
            -AvgElapsed (Mean-ValueLocal -Values $elapsedVals) `
            -StdElapsed (Std-ValueLocal -Values $elapsedVals) `
            -AvgOutputBytes (Mean-ValueLocal -Values $bytesVals) `
            -StdOutputBytes (Std-ValueLocal -Values $bytesVals) `
            -AvgPathCount (Mean-ValueLocal -Values $pathVals) `
            -StdPathCount (Std-ValueLocal -Values $pathVals) `
            -PsnrMean (Mean-ValueLocal -Values $psnrVals) `
            -PsnrStd (Std-ValueLocal -Values $psnrVals) `
            -SsimMean (Mean-ValueLocal -Values $ssimVals) `
            -SsimStd (Std-ValueLocal -Values $ssimVals) `
            -MaeMean (Mean-ValueLocal -Values $maeVals) `
            -MaeStd (Std-ValueLocal -Values $maeVals)
    } catch {
        $repeatCsvFallback = Join-Path $CaseRoot "case.repeats.csv"
        return [PSCustomObject]@{
            case_id = $Case.case_id
            quant_shift_complex = $Case.quant_shift_complex
            merge_color_delta_complex = $Case.merge_color_delta_complex
            merge_small_area_complex = $Case.merge_small_area_complex
            min_component_area_complex = $Case.min_component_area_complex
            run_ok = $false
            repeat_total = $RepeatPerCase
            repeat_success = 0
            repeat_failed = $RepeatPerCase
            error = $_.Exception.Message
            case_root = $CaseRoot
            repeat_csv = $repeatCsvFallback
            bench_run_dir = ""
            quality_dir = ""
            avg_elapsed_ms = 0.0
            std_elapsed_ms = 0.0
            avg_output_bytes = 0.0
            std_output_bytes = 0.0
            avg_path_count = 0.0
            std_path_count = 0.0
            psnr_mean = 0.0
            psnr_std = 0.0
            ssim_mean = 0.0
            ssim_std = 0.0
            mae_mean = 0.0
            mae_std = 0.0
        }
    }
}

$rows = New-Object System.Collections.Generic.List[object]
$activeJobs = @()
$jobMeta = @{}

Write-Host ("[grid] combinations={0}" -f $cases.Count)
Write-Host ("[grid] output={0}" -f $gridRoot)
Write-Host ("[grid] max_processes={0}, repeat_per_case={1}" -f $MaxProcesses, $RepeatPerCase)

$dispatched = 0
while ($dispatched -lt $cases.Count -or $activeJobs.Count -gt 0) {
    while ($dispatched -lt $cases.Count -and $activeJobs.Count -lt $MaxProcesses) {
        $case = $cases[$dispatched]
        $dispatched += 1
        $caseRoot = Join-Path $gridRoot $case.case_id
        Ensure-Dir -Path $caseRoot

        Write-Host ("[grid] dispatch {0}/{1} {2} q={3} delta={4} mergeA={5} minA={6}" -f $dispatched, $cases.Count, $case.case_id, $case.quant_shift_complex, $case.merge_color_delta_complex, $case.merge_small_area_complex, $case.min_component_area_complex)

        $job = Start-Job -ScriptBlock $caseWorker -ArgumentList @(
            $case,
            $dispatched,
            $cases.Count,
            $RepeatPerCase,
            $caseRoot,
            $repo,
            $DatasetRoot,
            $benchScript,
            $qualityScript,
            $Xun,
            $Threads,
            $TimeoutSec,
            $MaxFiles,
            $RsvgConvert
        )
        $activeJobs += $job
        $jobMeta[$job.Id] = $case
    }

    if ($activeJobs.Count -eq 0) {
        break
    }

    $null = Wait-Job -Job $activeJobs -Any -Timeout 1
    $finished = @($activeJobs | Where-Object { $_.State -in @("Completed", "Failed", "Stopped") })
    if ($finished.Count -eq 0) {
        continue
    }

    foreach ($j in $finished) {
        $meta = $jobMeta[$j.Id]
        $resultRows = @()
        try {
            $resultRows = @(Receive-Job -Job $j -ErrorAction SilentlyContinue)
        } catch {
            $resultRows = @()
        }

        $row = $resultRows | Where-Object { $_ -and $_.PSObject -and $_.PSObject.Properties["case_id"] } | Select-Object -Last 1
        if ($null -eq $row) {
            $reason = ""
            if ($j.State -eq "Failed" -and $j.ChildJobs.Count -gt 0 -and $null -ne $j.ChildJobs[0].JobStateInfo.Reason) {
                $reason = $j.ChildJobs[0].JobStateInfo.Reason.Message
            }
            if ([string]::IsNullOrWhiteSpace($reason)) {
                $reason = "job finished without result row"
            }
            $caseRoot = Join-Path $gridRoot $meta.case_id
            $row = [PSCustomObject]@{
                case_id = $meta.case_id
                quant_shift_complex = $meta.quant_shift_complex
                merge_color_delta_complex = $meta.merge_color_delta_complex
                merge_small_area_complex = $meta.merge_small_area_complex
                min_component_area_complex = $meta.min_component_area_complex
                run_ok = $false
                repeat_total = $RepeatPerCase
                repeat_success = 0
                repeat_failed = $RepeatPerCase
                error = $reason
                case_root = $caseRoot
                repeat_csv = Join-Path $caseRoot "case.repeats.csv"
                bench_run_dir = ""
                quality_dir = ""
                avg_elapsed_ms = 0.0
                std_elapsed_ms = 0.0
                avg_output_bytes = 0.0
                std_output_bytes = 0.0
                avg_path_count = 0.0
                std_path_count = 0.0
                psnr_mean = 0.0
                psnr_std = 0.0
                ssim_mean = 0.0
                ssim_std = 0.0
                mae_mean = 0.0
                mae_std = 0.0
            }
        }

        $rows.Add($row) | Out-Null
        Write-Host ("[grid] done {0} ok={1} repeats={2}/{3} elapsed={4}ms" -f $row.case_id, $row.run_ok, $row.repeat_success, $row.repeat_total, $row.avg_elapsed_ms)

        Remove-Job -Job $j -Force -ErrorAction SilentlyContinue
        $activeJobs = @($activeJobs | Where-Object { $_.Id -ne $j.Id })
        $null = $jobMeta.Remove($j.Id)
    }
}

$successRows = @($rows | Where-Object { $_.run_ok -eq $true })
$scoredRows = New-Object System.Collections.Generic.List[object]

if ($successRows.Count -gt 0) {
    $psnrMin = Min-Value -Rows $successRows -Property "psnr_mean"
    $psnrMax = Max-Value -Rows $successRows -Property "psnr_mean"
    $ssimMin = Min-Value -Rows $successRows -Property "ssim_mean"
    $ssimMax = Max-Value -Rows $successRows -Property "ssim_mean"
    $maeMin = Min-Value -Rows $successRows -Property "mae_mean"
    $maeMax = Max-Value -Rows $successRows -Property "mae_mean"
    $elapsedMin = Min-Value -Rows $successRows -Property "avg_elapsed_ms"
    $elapsedMax = Max-Value -Rows $successRows -Property "avg_elapsed_ms"
    $bytesMin = Min-Value -Rows $successRows -Property "avg_output_bytes"
    $bytesMax = Max-Value -Rows $successRows -Property "avg_output_bytes"
    $pathMin = Min-Value -Rows $successRows -Property "avg_path_count"
    $pathMax = Max-Value -Rows $successRows -Property "avg_path_count"

    foreach ($row in $rows) {
        if (-not $row.run_ok) {
            $scoredRows.Add([PSCustomObject]@{
                    case_id = $row.case_id
                    quant_shift_complex = $row.quant_shift_complex
                    merge_color_delta_complex = $row.merge_color_delta_complex
                    merge_small_area_complex = $row.merge_small_area_complex
                    min_component_area_complex = $row.min_component_area_complex
                    run_ok = $false
                    repeat_total = $row.repeat_total
                    repeat_success = $row.repeat_success
                    repeat_failed = $row.repeat_failed
                    error = $row.error
                    case_root = $row.case_root
                    repeat_csv = $row.repeat_csv
                    bench_run_dir = $row.bench_run_dir
                    quality_dir = $row.quality_dir
                    avg_elapsed_ms = $row.avg_elapsed_ms
                    std_elapsed_ms = $row.std_elapsed_ms
                    avg_output_bytes = $row.avg_output_bytes
                    std_output_bytes = $row.std_output_bytes
                    avg_path_count = $row.avg_path_count
                    std_path_count = $row.std_path_count
                    psnr_mean = $row.psnr_mean
                    psnr_std = $row.psnr_std
                    ssim_mean = $row.ssim_mean
                    ssim_std = $row.ssim_std
                    mae_mean = $row.mae_mean
                    mae_std = $row.mae_std
                    quality_gate = $false
                    score = -1.0
                }) | Out-Null
            continue
        }

        $psnrN = Normalize-Up -Value (As-Double $row.psnr_mean) -Min $psnrMin -Max $psnrMax
        $ssimN = Normalize-Up -Value (As-Double $row.ssim_mean) -Min $ssimMin -Max $ssimMax
        $maeN = Normalize-Down -Value (As-Double $row.mae_mean) -Min $maeMin -Max $maeMax
        $elapsedN = Normalize-Down -Value (As-Double $row.avg_elapsed_ms) -Min $elapsedMin -Max $elapsedMax
        $bytesN = Normalize-Down -Value (As-Double $row.avg_output_bytes) -Min $bytesMin -Max $bytesMax
        $pathN = Normalize-Down -Value (As-Double $row.avg_path_count) -Min $pathMin -Max $pathMax

        $score =
            (0.35 * $psnrN) +
            (0.25 * $ssimN) +
            (0.15 * $maeN) +
            (0.10 * $elapsedN) +
            (0.10 * $bytesN) +
            (0.05 * $pathN)

        $qualityGate =
            ((As-Double $row.psnr_mean) -ge 38.0) -and
            ((As-Double $row.ssim_mean) -ge 0.97) -and
            ((As-Double $row.mae_mean) -le 2.0)

        $scoredRows.Add([PSCustomObject]@{
                case_id = $row.case_id
                quant_shift_complex = $row.quant_shift_complex
                merge_color_delta_complex = $row.merge_color_delta_complex
                merge_small_area_complex = $row.merge_small_area_complex
                min_component_area_complex = $row.min_component_area_complex
                run_ok = $true
                repeat_total = $row.repeat_total
                repeat_success = $row.repeat_success
                repeat_failed = $row.repeat_failed
                error = ""
                case_root = $row.case_root
                repeat_csv = $row.repeat_csv
                bench_run_dir = $row.bench_run_dir
                quality_dir = $row.quality_dir
                avg_elapsed_ms = [Math]::Round((As-Double $row.avg_elapsed_ms), 2)
                std_elapsed_ms = [Math]::Round((As-Double $row.std_elapsed_ms), 2)
                avg_output_bytes = [Math]::Round((As-Double $row.avg_output_bytes), 2)
                std_output_bytes = [Math]::Round((As-Double $row.std_output_bytes), 2)
                avg_path_count = [Math]::Round((As-Double $row.avg_path_count), 2)
                std_path_count = [Math]::Round((As-Double $row.std_path_count), 2)
                psnr_mean = [Math]::Round((As-Double $row.psnr_mean), 6)
                psnr_std = [Math]::Round((As-Double $row.psnr_std), 6)
                ssim_mean = [Math]::Round((As-Double $row.ssim_mean), 6)
                ssim_std = [Math]::Round((As-Double $row.ssim_std), 6)
                mae_mean = [Math]::Round((As-Double $row.mae_mean), 6)
                mae_std = [Math]::Round((As-Double $row.mae_std), 6)
                quality_gate = $qualityGate
                score = [Math]::Round($score, 6)
            }) | Out-Null
    }
} else {
    foreach ($row in $rows) {
        $scoredRows.Add([PSCustomObject]@{
                case_id = $row.case_id
                quant_shift_complex = $row.quant_shift_complex
                merge_color_delta_complex = $row.merge_color_delta_complex
                merge_small_area_complex = $row.merge_small_area_complex
                min_component_area_complex = $row.min_component_area_complex
                run_ok = $false
                repeat_total = $row.repeat_total
                repeat_success = $row.repeat_success
                repeat_failed = $row.repeat_failed
                error = $row.error
                case_root = $row.case_root
                repeat_csv = $row.repeat_csv
                bench_run_dir = $row.bench_run_dir
                quality_dir = $row.quality_dir
                avg_elapsed_ms = $row.avg_elapsed_ms
                std_elapsed_ms = $row.std_elapsed_ms
                avg_output_bytes = $row.avg_output_bytes
                std_output_bytes = $row.std_output_bytes
                avg_path_count = $row.avg_path_count
                std_path_count = $row.std_path_count
                psnr_mean = $row.psnr_mean
                psnr_std = $row.psnr_std
                ssim_mean = $row.ssim_mean
                ssim_std = $row.ssim_std
                mae_mean = $row.mae_mean
                mae_std = $row.mae_std
                quality_gate = $false
                score = -1.0
            }) | Out-Null
    }
}

$sortedRows = @(
    $scoredRows |
    Sort-Object `
        @{ Expression = { if ($_.run_ok) { 1 } else { 0 } }; Descending = $true }, `
        @{ Expression = { if ($_.quality_gate) { 1 } else { 0 } }; Descending = $true }, `
        @{ Expression = "score"; Descending = $true }
)

$resultsCsv = Join-Path $gridRoot "grid.results.csv"
$sortedRows | Export-Csv -Path $resultsCsv -NoTypeInformation -Encoding UTF8

$topRows = @($sortedRows | Select-Object -First 10)
$best = $null
if ($topRows.Count -gt 0) {
    $best = $topRows[0]
}

$reportPath = Join-Path $gridRoot "grid.report.md"
$report = New-Object System.Text.StringBuilder
[void]$report.AppendLine("# XUN Visioncortex Grid Search")
[void]$report.AppendLine("")
[void]$report.AppendLine("- Generated at: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')")
[void]$report.AppendLine("- RepoRoot: ``$repo``")
[void]$report.AppendLine("- DatasetRoot: ``$DatasetRoot``")
[void]$report.AppendLine("- OutRoot: ``$gridRoot``")
[void]$report.AppendLine("- Total combinations: $($cases.Count)")
[void]$report.AppendLine("- Success runs: $($successRows.Count)")
[void]$report.AppendLine("- Threads: $Threads")
[void]$report.AppendLine("- TimeoutSec: $TimeoutSec")
[void]$report.AppendLine("- MaxFiles: $MaxFiles")
[void]$report.AppendLine("- MaxProcesses: $MaxProcesses")
[void]$report.AppendLine("- RepeatPerCase: $RepeatPerCase")
[void]$report.AppendLine("- RsvgConvert: ``$RsvgConvert``")
[void]$report.AppendLine("- QuantShiftComplexCandidates: $($quantCandidates -join ', ')")
[void]$report.AppendLine("- MergeColorDeltaComplexCandidates: $($mergeDeltaCandidates -join ', ')")
[void]$report.AppendLine("- MergeSmallAreaComplexCandidates: $($mergeAreaCandidates -join ', ')")
[void]$report.AppendLine("- MinComponentAreaComplexCandidates: $($minAreaCandidates -join ', ')")
[void]$report.AppendLine("")
[void]$report.AppendLine("## Best Candidate")
[void]$report.AppendLine("")
if ($null -eq $best) {
    [void]$report.AppendLine("No successful candidate.")
} else {
    [void]$report.AppendLine("| case_id | q_shift_complex | merge_delta_complex | merge_small_area_complex | min_comp_area_complex | repeat_success | repeat_total | avg_elapsed_ms | avg_output_bytes | avg_path_count | psnr_mean | ssim_mean | mae_mean | quality_gate | score |")
    [void]$report.AppendLine("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    [void]$report.AppendLine("| $($best.case_id) | $($best.quant_shift_complex) | $($best.merge_color_delta_complex) | $($best.merge_small_area_complex) | $($best.min_component_area_complex) | $($best.repeat_success) | $($best.repeat_total) | $($best.avg_elapsed_ms) | $($best.avg_output_bytes) | $($best.avg_path_count) | $($best.psnr_mean) | $($best.ssim_mean) | $($best.mae_mean) | $($best.quality_gate) | $($best.score) |")
    [void]$report.AppendLine("")
    [void]$report.AppendLine("- case_root: ``$($best.case_root)``")
    [void]$report.AppendLine("- repeat_csv: ``$($best.repeat_csv)``")
}
[void]$report.AppendLine("")
[void]$report.AppendLine("## Top 10")
[void]$report.AppendLine("")
[void]$report.AppendLine("| rank | case_id | q_shift_complex | merge_delta_complex | merge_small_area_complex | min_comp_area_complex | repeat_success | repeat_total | avg_elapsed_ms | std_elapsed_ms | avg_output_bytes | avg_path_count | psnr_mean | ssim_mean | mae_mean | quality_gate | score | run_ok |")
[void]$report.AppendLine("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
$rank = 0
foreach ($row in $topRows) {
    $rank += 1
    [void]$report.AppendLine("| $rank | $($row.case_id) | $($row.quant_shift_complex) | $($row.merge_color_delta_complex) | $($row.merge_small_area_complex) | $($row.min_component_area_complex) | $($row.repeat_success) | $($row.repeat_total) | $($row.avg_elapsed_ms) | $($row.std_elapsed_ms) | $($row.avg_output_bytes) | $($row.avg_path_count) | $($row.psnr_mean) | $($row.ssim_mean) | $($row.mae_mean) | $($row.quality_gate) | $($row.score) | $($row.run_ok) |")
}

$report.ToString() | Set-Content -Path $reportPath -Encoding UTF8

Write-Host ""
Write-Host "[done] grid root: $gridRoot"
Write-Host "[done] results: $resultsCsv"
Write-Host "[done] report: $reportPath"
