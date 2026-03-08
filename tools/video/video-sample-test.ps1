param(
    [string]$InputRoot = "",
    [int]$SampleCount = 100,
    [int]$CompressCount = 30,
    [int]$RemuxSuccessCount = 12,
    [string]$XunExe = "D:/100_Projects/110_Daily/Xun/target/debug/xun.exe",
    [string]$FfmpegExe = "C:/A_Softwares/ffmpeg-xun-static/bin/ffmpeg.exe",
    [string]$FfprobeExe = "C:/A_Softwares/ffmpeg-xun-static/bin/ffprobe.exe",
    [string]$OutRoot = "D:/100_Projects/110_Daily/Xun/video-test"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-SizeBin {
    param([double]$Mb)
    if ($Mb -lt 1) { return "0-1MB" }
    if ($Mb -lt 5) { return "1-5MB" }
    if ($Mb -lt 10) { return "5-10MB" }
    if ($Mb -lt 20) { return "10-20MB" }
    if ($Mb -lt 50) { return "20-50MB" }
    if ($Mb -lt 100) { return "50-100MB" }
    if ($Mb -lt 500) { return "100-500MB" }
    return "500MB+"
}

function New-SafeName {
    param([string]$Name, [int]$Index, [string]$Ext)
    $base = [System.IO.Path]::GetFileNameWithoutExtension($Name)
    $safe = ($base -replace '[^0-9A-Za-z_\-\u4e00-\u9fff]+', "_")
    if ([string]::IsNullOrWhiteSpace($safe)) {
        $safe = "file"
    }
    return ("{0:D3}_{1}{2}" -f $Index, $safe, $Ext)
}

function Invoke-Xun {
    param([string[]]$CliArgs)
    $stdout = @()
    $stderr = @()
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $argLine = ($CliArgs | ForEach-Object {
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
    $stdoutText = $proc.StandardOutput.ReadToEnd()
    $stderrText = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    $timer.Stop()
    if ($stdoutText) { $stdout = $stdoutText -split "`r?`n" | Where-Object { $_ -ne "" } }
    if ($stderrText) { $stderr = $stderrText -split "`r?`n" | Where-Object { $_ -ne "" } }
    return [pscustomobject]@{
        code = $proc.ExitCode
        ms = $timer.ElapsedMilliseconds
        stdout = $stdout
        stderr = $stderr
    }
}

if (-not (Test-Path -LiteralPath $InputRoot)) {
    throw "Input directory does not exist: $InputRoot"
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

$runId = "video-sample-{0}" -f (Get-Date -Format "yyyyMMdd-HHmmss")
$runDir = Join-Path $OutRoot $runId
$outputDir = Join-Path $runDir "output"
$compressOutDir = Join-Path $outputDir "compress"
$remuxOutDir = Join-Path $outputDir "remux"
$logsDir = Join-Path $runDir "logs"
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
New-Item -ItemType Directory -Path $compressOutDir -Force | Out-Null
New-Item -ItemType Directory -Path $remuxOutDir -Force | Out-Null
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

$extRegex = '^(?i)\.(mp4|mkv|mov|webm|avi|ts|m4v|flv|wmv|mpg|mpeg)$'
$allCandidates = Get-ChildItem -Path $InputRoot -Recurse -File | Where-Object { $_.Extension -match $extRegex } | ForEach-Object {
    $mb = [math]::Round($_.Length / 1MB, 3)
    [pscustomobject]@{
        path = $_.FullName
        name = $_.Name
        ext = $_.Extension.ToLower()
        bytes = $_.Length
        mb = $mb
        size_bin = Get-SizeBin -Mb $mb
    }
}

if ($allCandidates.Count -lt $SampleCount) {
    throw "Not enough candidate videos: need $SampleCount, got $($allCandidates.Count)"
}

$rand = [System.Random]::new()
$groups = @{}
foreach ($f in $allCandidates) {
    $k = "{0}|{1}" -f $f.ext, $f.size_bin
    if (-not $groups.ContainsKey($k)) {
        $groups[$k] = New-Object System.Collections.Generic.List[object]
    }
    $groups[$k].Add($f)
}
foreach ($k in @($groups.Keys)) {
    $groups[$k] = @($groups[$k] | Sort-Object { $rand.Next() })
}

$selected = New-Object System.Collections.Generic.List[object]
$used = [System.Collections.Generic.HashSet[string]]::new()
$groupKeys = $groups.Keys | Sort-Object
while ($selected.Count -lt $SampleCount) {
    $addedInRound = 0
    foreach ($k in $groupKeys) {
        if ($selected.Count -ge $SampleCount) { break }
        $arr = @($groups[$k])
        if ($arr.Count -eq 0) { continue }
        $pick = $arr[0]
        $groups[$k] = @($arr | Select-Object -Skip 1)
        if ($used.Add($pick.path)) {
            $selected.Add($pick)
            $addedInRound++
        }
    }
    if ($addedInRound -eq 0) { break }
}
if ($selected.Count -lt $SampleCount) {
    $remaining = $allCandidates | Where-Object { -not $used.Contains($_.path) } | Sort-Object { $rand.Next() }
    foreach ($r in $remaining) {
        if ($selected.Count -ge $SampleCount) { break }
        if ($used.Add($r.path)) {
            $selected.Add($r)
        }
    }
}

$sampleCsv = Join-Path $runDir "sample.100.csv"
$selected | Sort-Object ext, mb | Export-Csv -Path $sampleCsv -NoTypeInformation -Encoding UTF8

$probeRows = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $selected.Count; $i++) {
    $f = $selected[$i]
    $res = Invoke-Xun -CliArgs @("video", "probe", "-i", $f.path, "--ffprobe", $FfprobeExe)
    $probeRows.Add([pscustomobject]@{
        index = $i + 1
        path = $f.path
        ext = $f.ext
        size_bin = $f.size_bin
        mb = $f.mb
        code = $res.code
        elapsed_ms = $res.ms
        ok = ($res.code -eq 0)
        message = (($res.stderr + $res.stdout) | Select-Object -First 2) -join " | "
    })
}
$probeCsv = Join-Path $runDir "probe.results.csv"
$probeRows | Export-Csv -Path $probeCsv -NoTypeInformation -Encoding UTF8

$compressPool = $selected | Where-Object { $_.mb -le 20 } | Sort-Object { $rand.Next() }
$compressGroups = $compressPool | Group-Object ext, size_bin
$compressTargets = New-Object System.Collections.Generic.List[object]
foreach ($g in $compressGroups | Sort-Object Count -Descending) {
    $pick = $g.Group | Select-Object -First 1
    if ($null -ne $pick) {
        $compressTargets.Add($pick)
    }
}
if ($compressTargets.Count -lt $CompressCount) {
    $needed = $CompressCount - $compressTargets.Count
    $extra = $compressPool | Where-Object { $compressTargets.path -notcontains $_.path } | Select-Object -First $needed
    foreach ($e in $extra) { $compressTargets.Add($e) }
}
$compressTargets = @($compressTargets | Select-Object -First $CompressCount)

$compressRows = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $compressTargets.Count; $i++) {
    $f = $compressTargets[$i]
    $outName = New-SafeName -Name $f.name -Index ($i + 1) -Ext ".mp4"
    $outPath = Join-Path $compressOutDir $outName
    $res = Invoke-Xun -CliArgs @(
        "video", "compress",
        "-i", $f.path,
        "-o", $outPath,
        "--mode", "balanced",
        "--engine", "cpu",
        "--overwrite",
        "--ffmpeg", $FfmpegExe
    )
    $outBytes = 0
    if (Test-Path -LiteralPath $outPath) { $outBytes = (Get-Item -LiteralPath $outPath).Length }
    $compressRows.Add([pscustomobject]@{
        index = $i + 1
        path = $f.path
        ext = $f.ext
        size_bin = $f.size_bin
        input_mb = $f.mb
        code = $res.code
        elapsed_ms = $res.ms
        ok = ($res.code -eq 0)
        output = $outPath
        output_mb = [math]::Round($outBytes / 1MB, 3)
        ratio = if ($f.bytes -gt 0) { [math]::Round($outBytes / $f.bytes, 3) } else { 0 }
        message = (($res.stderr + $res.stdout) | Select-Object -First 2) -join " | "
    })
}
$compressCsv = Join-Path $runDir "compress.results.csv"
$compressRows | Export-Csv -Path $compressCsv -NoTypeInformation -Encoding UTF8

$remuxCheckRows = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $selected.Count; $i++) {
    $f = $selected[$i]
    $outName = New-SafeName -Name $f.name -Index ($i + 1) -Ext ".webm"
    $outPath = Join-Path $remuxOutDir ("failcheck_" + $outName)
    $res = Invoke-Xun -CliArgs @(
        "video", "remux",
        "-i", $f.path,
        "-o", $outPath,
        "--strict", "true",
        "--overwrite",
        "--ffmpeg", $FfmpegExe,
        "--ffprobe", $FfprobeExe
    )
    $isExpectedFail = ($res.code -ne 0)
    $remuxCheckRows.Add([pscustomobject]@{
        index = $i + 1
        path = $f.path
        ext = $f.ext
        size_bin = $f.size_bin
        mb = $f.mb
        code = $res.code
        elapsed_ms = $res.ms
        expected_fail = $isExpectedFail
        ok = $isExpectedFail
        message = (($res.stderr + $res.stdout) | Select-Object -First 2) -join " | "
    })
}
$remuxCheckCsv = Join-Path $runDir "remux.strict-failcheck.results.csv"
$remuxCheckRows | Export-Csv -Path $remuxCheckCsv -NoTypeInformation -Encoding UTF8

$remuxSuccessPool = $selected | Where-Object { $_.mb -le 20 } | Sort-Object { $rand.Next() }
$remuxSuccessTargets = New-Object System.Collections.Generic.List[object]
$successGroups = $remuxSuccessPool | Group-Object ext, size_bin
foreach ($g in $successGroups | Sort-Object Count -Descending) {
    $pick = $g.Group | Select-Object -First 1
    if ($null -ne $pick) { $remuxSuccessTargets.Add($pick) }
}
if ($remuxSuccessTargets.Count -lt $RemuxSuccessCount) {
    $need = $RemuxSuccessCount - $remuxSuccessTargets.Count
    $extra = $remuxSuccessPool | Where-Object { $remuxSuccessTargets.path -notcontains $_.path } | Select-Object -First $need
    foreach ($e in $extra) { $remuxSuccessTargets.Add($e) }
}
$remuxSuccessTargets = @($remuxSuccessTargets | Select-Object -First $RemuxSuccessCount)

$remuxSuccessRows = New-Object System.Collections.Generic.List[object]
for ($i = 0; $i -lt $remuxSuccessTargets.Count; $i++) {
    $f = $remuxSuccessTargets[$i]
    $outName = New-SafeName -Name $f.name -Index ($i + 1) -Ext ".mkv"
    $outPath = Join-Path $remuxOutDir ("success_" + $outName)
    $res = Invoke-Xun -CliArgs @(
        "video", "remux",
        "-i", $f.path,
        "-o", $outPath,
        "--strict", "true",
        "--overwrite",
        "--ffmpeg", $FfmpegExe,
        "--ffprobe", $FfprobeExe
    )
    $outBytes = 0
    if (Test-Path -LiteralPath $outPath) { $outBytes = (Get-Item -LiteralPath $outPath).Length }
    $remuxSuccessRows.Add([pscustomobject]@{
        index = $i + 1
        path = $f.path
        ext = $f.ext
        size_bin = $f.size_bin
        input_mb = $f.mb
        code = $res.code
        elapsed_ms = $res.ms
        ok = ($res.code -eq 0)
        output = $outPath
        output_mb = [math]::Round($outBytes / 1MB, 3)
        ratio = if ($f.bytes -gt 0) { [math]::Round($outBytes / $f.bytes, 3) } else { 0 }
        message = (($res.stderr + $res.stdout) | Select-Object -First 2) -join " | "
    })
}
$remuxSuccessCsv = Join-Path $runDir "remux.strict-success.results.csv"
$remuxSuccessRows | Export-Csv -Path $remuxSuccessCsv -NoTypeInformation -Encoding UTF8

$sampleByFormat = $selected | Group-Object ext | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{
        ext = $_.Name
        count = $_.Count
    }
}
$sampleBySize = $selected | Group-Object size_bin | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{
        size_bin = $_.Name
        count = $_.Count
    }
}
$sampleByFormatCsv = Join-Path $runDir "summary.sample.by_format.csv"
$sampleBySizeCsv = Join-Path $runDir "summary.sample.by_size.csv"
$sampleByFormat | Export-Csv -Path $sampleByFormatCsv -NoTypeInformation -Encoding UTF8
$sampleBySize | Export-Csv -Path $sampleBySizeCsv -NoTypeInformation -Encoding UTF8

$probePass = @($probeRows | Where-Object { $_.ok }).Count
$compressPass = @($compressRows | Where-Object { $_.ok }).Count
$remuxFailcheckPass = @($remuxCheckRows | Where-Object { $_.ok }).Count
$remuxSuccessPass = @($remuxSuccessRows | Where-Object { $_.ok }).Count

$summaryMd = Join-Path $runDir "summary.md"
$lines = @()
$lines += "# Video Sample Test Summary (100 Files, Stratified by Format + Size Bin)"
$lines += ""
$lines += "## Scope"
$lines += "- Input root: $InputRoot"
$lines += "- Sample count: $SampleCount (stratified)"
$lines += "- xun exe: $XunExe"
$lines += "- ffmpeg: $FfmpegExe"
$lines += "- ffprobe: $FfprobeExe"
$lines += "- Output run dir: $runDir"
$lines += ""
$lines += "## Sample Distribution (Format)"
foreach ($r in $sampleByFormat) {
    $lines += "- $($r.ext): $($r.count)"
}
$lines += ""
$lines += "## Sample Distribution (Size Bin)"
foreach ($r in $sampleBySize) {
    $lines += "- $($r.size_bin): $($r.count)"
}
$lines += ""
$lines += "## Functional Results"
$lines += "- probe: $probePass / $($probeRows.Count) passed"
$lines += "- compress (balanced + cpu): $compressPass / $($compressRows.Count) passed"
$lines += "- remux strict failcheck (expected failure): $remuxFailcheckPass / $($remuxCheckRows.Count) matched expectation"
$lines += "- remux strict success (mp4/mov/m4v/ts -> mkv): $remuxSuccessPass / $($remuxSuccessRows.Count) passed"
$lines += ""
$lines += "## Output Files"
$lines += "- Sample list: sample.100.csv"
$lines += "- Probe: probe.results.csv"
$lines += "- Compress: compress.results.csv"
$lines += "- Remux strict failcheck: remux.strict-failcheck.results.csv"
$lines += "- Remux strict success: remux.strict-success.results.csv"
$lines += "- Sample dist by format: summary.sample.by_format.csv"
$lines += "- Sample dist by size: summary.sample.by_size.csv"
$lines += ""
[System.IO.File]::WriteAllLines($summaryMd, $lines, [System.Text.UTF8Encoding]::new($false))

$meta = [pscustomobject]@{
    run_dir = $runDir
    sample_csv = $sampleCsv
    probe_csv = $probeCsv
    compress_csv = $compressCsv
    remux_failcheck_csv = $remuxCheckCsv
    remux_success_csv = $remuxSuccessCsv
    summary_md = $summaryMd
    sample_count = $selected.Count
    probe_pass = $probePass
    probe_total = $probeRows.Count
    compress_pass = $compressPass
    compress_total = $compressRows.Count
    remux_failcheck_pass = $remuxFailcheckPass
    remux_failcheck_total = $remuxCheckRows.Count
    remux_success_pass = $remuxSuccessPass
    remux_success_total = $remuxSuccessRows.Count
}
$meta | ConvertTo-Json -Depth 4 -Compress
