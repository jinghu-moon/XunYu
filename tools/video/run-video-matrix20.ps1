param(
    [string]$InputRoot = "D:/300_Resources/330_视频/02-视频/Telegram",
    [string]$Root = "D:/100_Projects/110_Daily/Xun",
    [string]$XunExe = "",
    [string]$FfmpegExe = "C:/A_Softwares/ffmpeg-xun-static/bin/ffmpeg.exe",
    [string]$FfprobeExe = "C:/A_Softwares/ffmpeg-xun-static/bin/ffprobe.exe",
    [int]$CountPerFormat = 1
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

if ([string]::IsNullOrWhiteSpace($XunExe)) {
    $XunExe = Join-Path $Root "target/debug/xun.exe"
}

if (-not (Test-Path -LiteralPath $InputRoot)) {
    throw "input root missing: $InputRoot"
}
if (-not (Test-Path -LiteralPath $XunExe)) {
    throw "xun executable not found: $XunExe"
}
if (-not (Test-Path -LiteralPath $FfmpegExe)) {
    throw "ffmpeg not found: $FfmpegExe"
}
if (-not (Test-Path -LiteralPath $FfprobeExe)) {
    throw "ffprobe not found: $FfprobeExe"
}

$env:Path = "C:/A_Softwares/MSYS2/mingw64/bin;C:/A_Softwares/ffmpeg-xun-static/bin;" + $env:Path

$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $Root "video-test/matrix20-$ts"
$logsDir = Join-Path $runDir "logs"
$outputRoot = Join-Path $runDir "output"
$inputSubsetRoot = Join-Path $runDir "input-subset"

New-Item -ItemType Directory -Force -Path $runDir | Out-Null
New-Item -ItemType Directory -Force -Path $logsDir | Out-Null
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
New-Item -ItemType Directory -Force -Path $inputSubsetRoot | Out-Null

$inputFormats = @(
    @{ name = "mp4"; ext = ".mp4" },
    @{ name = "mov"; ext = ".mov" },
    @{ name = "m4v"; ext = ".m4v" },
    @{ name = "ts"; ext = ".ts" }
)
$selectionRows = New-Object System.Collections.Generic.List[object]

# Keep runtime controlled: pick smallest files first inside each format.
foreach ($fmt in $inputFormats) {
    $dstDir = Join-Path $inputSubsetRoot $fmt.name
    New-Item -ItemType Directory -Force -Path $dstDir | Out-Null

    $filesAll = @(Get-ChildItem -Path $InputRoot -Recurse -File | Where-Object {
        $_.Extension.ToLower() -eq $fmt.ext
    } | Sort-Object Length, Name)
    $available = $filesAll.Count

    if ($available -eq 0) {
        throw "format $($fmt.name) has 0 file"
    }

    $picked = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $CountPerFormat; $i++) {
        $picked.Add($filesAll[$i % $available]) | Out-Null
    }

    $copyIdx = 0
    foreach ($f in $picked) {
        $copyIdx += 1
        $base = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
        $base = ($base -replace '[^0-9A-Za-z_\-\u4e00-\u9fff]+', "_")
        if ([string]::IsNullOrWhiteSpace($base)) { $base = "file" }
        if ($base.Length -gt 60) { $base = $base.Substring(0, 60) }
        $dstName = ("{0:D3}_{1}{2}" -f $copyIdx, $base, $f.Extension.ToLower())
        [System.IO.File]::Copy($f.FullName, (Join-Path $dstDir $dstName), $true)
    }

    $dupCount = [Math]::Max(0, $CountPerFormat - $available)
    $selectionRows.Add([pscustomobject]@{
        input_format = $fmt.name
        requested = $CountPerFormat
        unique_available = $available
        duplicated = $dupCount
    }) | Out-Null
}

# 4 input formats x 5 profiles = 20 cases
$profiles = @(
    @{ name = "compress-fastest-cpu-mp4"; mode = "compress"; outExt = ".mp4"; args = @("--mode", "fastest", "--engine", "cpu") },
    @{ name = "compress-balanced-cpu-mp4"; mode = "compress"; outExt = ".mp4"; args = @("--mode", "balanced", "--engine", "cpu") },
    @{ name = "compress-smallest-cpu-mp4"; mode = "compress"; outExt = ".mp4"; args = @("--mode", "smallest", "--engine", "cpu") },
    @{ name = "compress-balanced-auto-mp4"; mode = "compress"; outExt = ".mp4"; args = @("--mode", "balanced", "--engine", "auto") },
    @{ name = "compress-balanced-cpu-webm"; mode = "compress"; outExt = ".webm"; args = @("--mode", "balanced", "--engine", "cpu") }
)

function New-SafeName {
    param([string]$Name, [int]$Index, [string]$Ext)
    $base = [System.IO.Path]::GetFileNameWithoutExtension($Name)
    $safe = ($base -replace '[^0-9A-Za-z_\-\u4e00-\u9fff]+', "_")
    if ([string]::IsNullOrWhiteSpace($safe)) { $safe = "file" }
    return ("{0:D2}_{1}{2}" -f $Index, $safe, $Ext)
}

$caseRows = New-Object System.Collections.Generic.List[object]
$fileRows = New-Object System.Collections.Generic.List[object]
$idx = 0

foreach ($fmt in $inputFormats) {
    $inputDir = Join-Path $inputSubsetRoot $fmt.name
    $inputs = @(Get-ChildItem -Path $inputDir -File)
    $inputBytes = ($inputs | Measure-Object -Property Length -Sum).Sum
    if ($null -eq $inputBytes) { $inputBytes = 0 }

    foreach ($profile in $profiles) {
        $idx += 1
        $caseName = "$($fmt.name)-to-$($profile.name)"
        $outDir = Join-Path $outputRoot $caseName
        $logFile = Join-Path $logsDir "$caseName.log"
        New-Item -ItemType Directory -Force -Path $outDir | Out-Null

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $caseFail = 0
        $outputBytesCase = 0L
        $fileIndex = 0

        foreach ($src in $inputs) {
            $fileIndex += 1
            $outName = New-SafeName -Name $src.Name -Index $fileIndex -Ext $profile.outExt
            $outPath = Join-Path $outDir $outName

            $argsList = @("video", $profile.mode, "-i", $src.FullName, "-o", $outPath, "--overwrite")
            if ($profile.mode -eq "compress") {
                $argsList += @("--ffmpeg", $FfmpegExe)
            } else {
                $argsList += @("--ffmpeg", $FfmpegExe, "--ffprobe", $FfprobeExe)
            }
            if ($profile.args.Count -gt 0) {
                $argsList += $profile.args
            }

            $fileLog = Join-Path $logsDir "$caseName.$fileIndex.log"
            $argLine = ($argsList | ForEach-Object {
                if ($_ -match '[\s"]') {
                    '"' + ($_ -replace '"', '\"') + '"'
                } else {
                    $_
                }
            }) -join " "

            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = $XunExe
            $psi.Arguments = $argLine
            $psi.UseShellExecute = $false
            $psi.RedirectStandardOutput = $true
            $psi.RedirectStandardError = $true
            $proc = New-Object System.Diagnostics.Process
            $proc.StartInfo = $psi
            [void]$proc.Start()
            $stdOut = $proc.StandardOutput.ReadToEnd()
            $stdErr = $proc.StandardError.ReadToEnd()
            $proc.WaitForExit()
            $exitCode = $proc.ExitCode

            $all = @()
            if ($stdOut) { $all += ($stdOut -split "`r?`n" | Where-Object { $_ -ne "" }) }
            if ($stdErr) { $all += ($stdErr -split "`r?`n" | Where-Object { $_ -ne "" }) }
            Set-Content -LiteralPath $fileLog -Value $all -Encoding UTF8
            if ($exitCode -ne 0) { $caseFail += 1 }

            $outBytes = 0L
            if (Test-Path -LiteralPath $outPath) {
                $outBytes = (Get-Item -LiteralPath $outPath).Length
                $outputBytesCase += $outBytes
            }

            $msg = @()
            if (@($all).Count -gt 0) { $msg += (@($all) | Select-Object -First 1) }

            $fileRows.Add([pscustomobject]@{
                case_idx = $idx
                case_name = $caseName
                input_format = $fmt.name
                profile = $profile.name
                input_file = $src.FullName
                input_bytes = $src.Length
                output_file = $outPath
                output_bytes = $outBytes
                exit_code = $exitCode
                ok = ($exitCode -eq 0)
                note = ($msg -join " | ")
            }) | Out-Null
        }

        $sw.Stop()
        if ($inputBytes -gt 0) {
            $savingsPct = [Math]::Round((1.0 - ($outputBytesCase / [double]$inputBytes)) * 100.0, 2)
        } else {
            $savingsPct = 0.0
        }
        $wallMs = [Math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        if ($wallMs -gt 0 -and $inputBytes -gt 0) {
            $throughput = [Math]::Round(($inputBytes / 1MB) / ($wallMs / 1000.0), 2)
        } else {
            $throughput = 0.0
        }

        $caseRows.Add([pscustomobject]@{
            idx = $idx
            case_name = $caseName
            input_format = $fmt.name
            output_profile = $profile.name
            command_mode = $profile.mode
            files_total = $inputs.Count
            fail_count = $caseFail
            exit_code = if ($caseFail -eq 0) { 0 } else { 1 }
            input_bytes = $inputBytes
            output_bytes = $outputBytesCase
            savings_pct = $savingsPct
            throughput_mb_s = $throughput
            wall_ms = $wallMs
            output_dir = $outDir
            log_hint = $logFile
        }) | Out-Null

        Write-Host ("[{0}/20] {1} done (fails={2}, wall={3} ms)" -f $idx, $caseName, $caseFail, $wallMs)
    }
}

$resultsCsv = Join-Path $runDir "results.csv"
$caseRows | Export-Csv -Path $resultsCsv -NoTypeInformation -Encoding UTF8

$fileResultsCsv = Join-Path $runDir "results.files.csv"
$fileRows | Export-Csv -Path $fileResultsCsv -NoTypeInformation -Encoding UTF8

$summaryByProfile = $caseRows | Group-Object output_profile | ForEach-Object {
    $g = $_.Group
    [pscustomobject]@{
        output_profile = $_.Name
        avg_wall_ms = [Math]::Round((($g | Measure-Object wall_ms -Average).Average), 2)
        avg_throughput_mb_s = [Math]::Round((($g | Measure-Object throughput_mb_s -Average).Average), 2)
        avg_savings_pct = [Math]::Round((($g | Measure-Object savings_pct -Average).Average), 2)
        fail_cases = @($g | Where-Object { $_.exit_code -ne 0 }).Count
    }
}
$summaryByProfileCsv = Join-Path $runDir "summary.by_profile.csv"
$summaryByProfile | Export-Csv -Path $summaryByProfileCsv -NoTypeInformation -Encoding UTF8

$summaryByFormat = $caseRows | Group-Object input_format | ForEach-Object {
    $g = $_.Group
    [pscustomobject]@{
        input_format = $_.Name
        avg_wall_ms = [Math]::Round((($g | Measure-Object wall_ms -Average).Average), 2)
        avg_throughput_mb_s = [Math]::Round((($g | Measure-Object throughput_mb_s -Average).Average), 2)
        avg_savings_pct = [Math]::Round((($g | Measure-Object savings_pct -Average).Average), 2)
        fail_cases = @($g | Where-Object { $_.exit_code -ne 0 }).Count
    }
}
$summaryByFormatCsv = Join-Path $runDir "summary.by_format.csv"
$summaryByFormat | Export-Csv -Path $summaryByFormatCsv -NoTypeInformation -Encoding UTF8

$selectionCsv = Join-Path $runDir "summary.input_selection.csv"
$selectionRows | Export-Csv -Path $selectionCsv -NoTypeInformation -Encoding UTF8

$reportMd = Join-Path $runDir "summary.md"
$lines = @()
$lines += "# Video Matrix20 Summary"
$lines += ""
$lines += "- input root: $InputRoot"
$lines += "- count per format: $CountPerFormat"
$lines += "- run dir: $runDir"
$lines += "- results: results.csv"
$lines += "- file results: results.files.csv"
$lines += "- summary by profile: summary.by_profile.csv"
$lines += "- summary by format: summary.by_format.csv"
$lines += "- input selection: summary.input_selection.csv"
$lines += ""
$lines += "## Case Status"
$totalCases = [int]$caseRows.Count
$failedCases = [int]@($caseRows | Where-Object { $_.exit_code -ne 0 }).Count
$lines += "- total cases: $totalCases"
$lines += "- failed cases: $failedCases"
[System.IO.File]::WriteAllLines($reportMd, $lines, [System.Text.UTF8Encoding]::new($false))

Write-Host "RUN_DIR=$runDir"
Write-Host "RESULTS=$resultsCsv"
Write-Host "RESULTS_FILES=$fileResultsCsv"
Write-Host "SUMMARY_PROFILE=$summaryByProfileCsv"
Write-Host "SUMMARY_FORMAT=$summaryByFormatCsv"
Write-Host "SUMMARY_INPUT_SELECTION=$selectionCsv"
