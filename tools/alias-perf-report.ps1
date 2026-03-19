#Requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Dir,
    [string]$PrevDir = "",
    [string]$OutMarkdown = "",
    [string]$OutJson = ""
)

$ErrorActionPreference = "Stop"

function Get-MetricStats {
    param([double[]]$Values)

    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) {
        throw "Metric values cannot be empty"
    }

    $count = $sorted.Count
    $median = if ($count % 2 -eq 1) {
        $sorted[[int]($count / 2)]
    } else {
        [math]::Round((($sorted[$count / 2 - 1] + $sorted[$count / 2]) / 2), 2)
    }

    return [ordered]@{
        min = [math]::Round($sorted[0], 2)
        median = [math]::Round($median, 2)
        mean = [math]::Round((($sorted | Measure-Object -Average).Average), 2)
        max = [math]::Round($sorted[$count - 1], 2)
    }
}

function Add-MetricValue {
    param(
        [hashtable]$Metrics,
        [string]$Name,
        [string]$RunKey,
        [int]$Value
    )

    if (-not $Metrics.ContainsKey($Name)) {
        $Metrics[$Name] = @{}
    }

    $Metrics[$Name][$RunKey] = $Value
}

function Get-PreviousMetricMap {
    param([object]$PreviousSummary)

    $map = @{}
    foreach ($metric in $PreviousSummary.metrics) {
        $map[$metric.name] = $metric
    }
    return $map
}

$resolvedDir = (Resolve-Path -LiteralPath $Dir).Path
if (-not $OutMarkdown) {
    $OutMarkdown = Join-Path $resolvedDir "baseline.md"
}
if (-not $OutJson) {
    $OutJson = Join-Path $resolvedDir "summary.json"
}

$runFiles = @(Get-ChildItem -LiteralPath $resolvedDir -File | Where-Object { $_.Name -match '^run\d+\.txt$' } | Sort-Object Name)
if ($runFiles.Count -ne 5) {
    throw "Expected exactly 5 run files in '$resolvedDir', found $($runFiles.Count)"
}

$metrics = @{}
$metricOrder = @()
$runTotals = [ordered]@{}

foreach ($runFile in $runFiles) {
    $runKey = [System.IO.Path]::GetFileNameWithoutExtension($runFile.Name)
    $lines = Get-Content -LiteralPath $runFile.FullName
    $foundPerfLine = $false

    foreach ($line in $lines) {
        if ($line -match '.*perf:\s*sync idempotent first=([0-9]+)ms second=([0-9]+)ms$') {
            $firstName = 'sync idempotent first'
            $secondName = 'sync idempotent second'
            Add-MetricValue -Metrics $metrics -Name $firstName -RunKey $runKey -Value ([int]$Matches[1])
            Add-MetricValue -Metrics $metrics -Name $secondName -RunKey $runKey -Value ([int]$Matches[2])
            if ($metricOrder -notcontains $firstName) { $metricOrder += $firstName }
            if ($metricOrder -notcontains $secondName) { $metricOrder += $secondName }
            $foundPerfLine = $true
            continue
        }

        if ($line -match '.*perf:\s*(.+?)\s*=\s*([0-9]+)ms$') {
            $name = $Matches[1].Trim()
            Add-MetricValue -Metrics $metrics -Name $name -RunKey $runKey -Value ([int]$Matches[2])
            if ($metricOrder -notcontains $name) { $metricOrder += $name }
            $foundPerfLine = $true
            continue
        }

        if ($line -match 'finished in ([0-9]+(?:\.[0-9]+)?)s') {
            $runTotals[$runKey] = [double]$Matches[1]
        }
    }

    if (-not $foundPerfLine) {
        throw "No perf metrics found in '$($runFile.FullName)'"
    }
}

$metricSummaries = @()
foreach ($name in $metricOrder) {
    $entry = $metrics[$name]
    $values = @()
    $runs = [ordered]@{}

    foreach ($runFile in $runFiles) {
        $runKey = [System.IO.Path]::GetFileNameWithoutExtension($runFile.Name)
        if (-not $entry.ContainsKey($runKey)) {
            throw "Metric '$name' is missing value for $runKey"
        }
        $value = [double]$entry[$runKey]
        $runs[$runKey] = [int]$value
        $values += $value
    }

    $stats = Get-MetricStats -Values $values
    $metricSummaries += [pscustomobject]@{
        name = $name
        runs = [pscustomobject]$runs
        min = $stats.min
        median = $stats.median
        mean = $stats.mean
        max = $stats.max
    }
}

$runtimeValues = @($runTotals.Values)
$runtimeStats = Get-MetricStats -Values $runtimeValues

$commit = ""
try {
    $commit = (git rev-parse HEAD).Trim()
} catch {
    $commit = ""
}

$summary = [ordered]@{
    generatedAt = (Get-Date).ToString('o')
    dir = $resolvedDir
    commit = $commit
    command = 'cargo test --test test_alias perf_ --features alias -- --nocapture --test-threads=1'
    runFiles = @($runFiles | ForEach-Object { $_.FullName })
    totalRuntimeSeconds = [pscustomobject]@{
        runs = [pscustomobject]$runTotals
        min = $runtimeStats.min
        median = $runtimeStats.median
        mean = $runtimeStats.mean
        max = $runtimeStats.max
    }
    metrics = $metricSummaries
}

if ($PrevDir) {
    $resolvedPrevDir = (Resolve-Path -LiteralPath $PrevDir).Path
    $previousSummaryPath = Join-Path $resolvedPrevDir 'summary.json'
    if (-not (Test-Path -LiteralPath $previousSummaryPath)) {
        throw "Previous summary not found: $previousSummaryPath"
    }

    $previousSummary = Get-Content -Raw -LiteralPath $previousSummaryPath | ConvertFrom-Json
    $previousMetricMap = Get-PreviousMetricMap -PreviousSummary $previousSummary
    $comparisons = @()

    foreach ($metric in $metricSummaries) {
        if (-not $previousMetricMap.ContainsKey($metric.name)) {
            continue
        }

        $previousMetric = $previousMetricMap[$metric.name]
        $currentMedian = [double]$metric.median
        $previousMedian = [double]$previousMetric.median
        $delta = [math]::Round(($currentMedian - $previousMedian), 2)
        $deltaPct = if ($previousMedian -eq 0) { 0 } else { [math]::Round(($delta / $previousMedian) * 100, 2) }
        $trend = if ($delta -lt 0) { 'faster' } elseif ($delta -gt 0) { 'slower' } else { 'flat' }

        $comparisons += [pscustomobject]@{
            name = $metric.name
            previousMedian = $previousMedian
            currentMedian = $currentMedian
            deltaMs = $delta
            deltaPct = $deltaPct
            trend = $trend
        }
    }

    $summary['comparison'] = [pscustomobject]@{
        previousDir = $resolvedPrevDir
        basis = 'median'
        metrics = $comparisons
    }
}

$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutJson -Encoding UTF8

$tick = [char]96
$lines = New-Object System.Collections.Generic.List[string]
$null = $lines.Add('# Alias 性能基线')
$null = $lines.Add('')
$null = $lines.Add(('- 基线目录：{0}{1}{0}' -f $tick, $resolvedDir))
if ($commit) {
    $null = $lines.Add(('- Commit：{0}{1}{0}' -f $tick, $commit))
}
$null = $lines.Add(('- 采集时间：{0}{1}{0}' -f $tick, (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')))
$null = $lines.Add(('- 基线命令：{0}{1}{0}' -f $tick, $summary.command))
$null = $lines.Add('- 轮次：5 次，串行执行，避免并发抖动')
$null = $lines.Add(('- 统计口径：以 {0}median{0} 作为后续优化对比主指标，{0}min/mean/max{0} 作为波动参考' -f $tick))
$null = $lines.Add('')
$null = $lines.Add('## 运行文件')
$null = $lines.Add('')
foreach ($runFile in $runFiles) {
    $null = $lines.Add(('- {0}{1}{0}' -f $tick, $runFile.FullName))
}
$null = $lines.Add('')
$null = $lines.Add('## 整体耗时')
$null = $lines.Add('')
$null = $lines.Add('| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |')
$null = $lines.Add('| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
$null = $lines.Add(('| total runtime (s) | {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} |' -f $summary.totalRuntimeSeconds.runs.run1, $summary.totalRuntimeSeconds.runs.run2, $summary.totalRuntimeSeconds.runs.run3, $summary.totalRuntimeSeconds.runs.run4, $summary.totalRuntimeSeconds.runs.run5, $summary.totalRuntimeSeconds.min, $summary.totalRuntimeSeconds.median, $summary.totalRuntimeSeconds.mean, $summary.totalRuntimeSeconds.max))
$null = $lines.Add('')
$null = $lines.Add('## 指标明细')
$null = $lines.Add('')
$null = $lines.Add('| 指标 | Run1 | Run2 | Run3 | Run4 | Run5 | Min | Median | Mean | Max |')
$null = $lines.Add('| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
foreach ($metric in $metricSummaries) {
    $null = $lines.Add(('| {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} | {9} |' -f $metric.name, $metric.runs.run1, $metric.runs.run2, $metric.runs.run3, $metric.runs.run4, $metric.runs.run5, $metric.min, $metric.median, $metric.mean, $metric.max))
}

if ($summary.Contains('comparison')) {
    $null = $lines.Add('')
    $null = $lines.Add('## 与上一版基线对比')
    $null = $lines.Add('')
    $null = $lines.Add(('- 上一版基线目录：{0}{1}{0}' -f $tick, $summary.comparison.previousDir))
    $null = $lines.Add(('- 对比口径：{0}{1}{0}' -f $tick, $summary.comparison.basis))
    $null = $lines.Add('')
    $null = $lines.Add('| 指标 | Prev Median | Current Median | Delta(ms) | Delta(%) | 趋势 |')
    $null = $lines.Add('| --- | ---: | ---: | ---: | ---: | --- |')
    foreach ($metric in $summary.comparison.metrics) {
        $trendText = switch ($metric.trend) {
            'faster' { '更快' }
            'slower' { '更慢' }
            default { '持平' }
        }
        $null = $lines.Add(('| {0} | {1} | {2} | {3} | {4}% | {5} |' -f $metric.name, $metric.previousMedian, $metric.currentMedian, $metric.deltaMs, $metric.deltaPct, $trendText))
    }
}

$null = $lines.Add('')
$null = $lines.Add('## 后续执行约定')
$null = $lines.Add('')
$null = $lines.Add(('- 每次优化后新建一个时间戳目录，例如：{0}logs/alias-perf/<timestamp>{0}' -f $tick))
$null = $lines.Add(('- 在新目录中按相同命令连续执行 5 次，并分别写入 {0}run1.txt{0} 到 {0}run5.txt{0}' -f $tick))
$nextCommand = 'powershell -ExecutionPolicy Bypass -File "tools/alias-perf-report.ps1" -Dir "logs/alias-perf/<timestamp>" -PrevDir "' + $resolvedDir + '"'
$null = $lines.Add(('- 使用以下命令生成新基线并对比上一版：{0}{1}{0}' -f $tick, $nextCommand))
$null = $lines.Add('- 若新基线确认生效，则将该目录视为下一轮优化的对比基线')

Set-Content -LiteralPath $OutMarkdown -Value $lines -Encoding UTF8
Write-Host "Generated: $OutMarkdown"
Write-Host "Generated: $OutJson"



