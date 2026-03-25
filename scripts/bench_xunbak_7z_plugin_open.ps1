param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug',
    [int]$Runs = 5,
    [int]$Warmup = 1
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
. (Join-Path $repoRoot 'scripts/xunbak_7z_plugin_common.ps1')

$sevenZipHome = 'C:\A_Softwares\7-Zip'
$stageRoot = Join-Path $env:TEMP ('xunbak-7z-plugin-bench-' + [guid]::NewGuid().ToString())
$workRoot = Join-Path $stageRoot 'work'
$portable7z = Join-Path $stageRoot '7zip'
$formatsDir = Join-Path $portable7z 'Formats'
$pluginDll = Join-Path $repoRoot "build/xunbak-7z-plugin/$Config/xunbak.dll"
$xunExe = Join-Path $repoRoot 'target/debug/xun.exe'
$logDir = Join-Path $repoRoot 'logs'
$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$logPath = Join-Path $logDir ("xunbak_7z_plugin_open_baseline_" + $stamp + ".md")
$sevenZipArgs = @('-sccUTF-8')

function Invoke-Checked([scriptblock]$Action, [string]$Message) {
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Message (exit=$LASTEXITCODE)"
    }
}

function New-LargeFixture([string]$Root) {
    $srcDir = Join-Path $Root 'large-src'
    New-Item -ItemType Directory -Path $srcDir -Force | Out-Null
    $bytes = New-Object byte[] (32MB)
    for ($i = 0; $i -lt $bytes.Length; $i++) {
        $bytes[$i] = [byte]($i % 251)
    }
    [System.IO.File]::WriteAllBytes((Join-Path $srcDir 'big.bin'), $bytes)
    Set-Content -Path (Join-Path $srcDir '.xun-bak.json') -Value '{"storage":{"backupsDir":"A_backups","compress":false},"naming":{"prefix":"v","dateFormat":"yyyy-MM-dd_HHmm","defaultDesc":"plugin-bench"},"retention":{"maxBackups":5,"deleteCount":1},"include":["big.bin"],"exclude":[]}' -NoNewline
    return $srcDir
}

function Invoke-BenchList {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ArchivePath,
        [switch]$ForceCallbackFailure,
        [string]$FallbackMaxBytes
    )

    if ($ForceCallbackFailure) {
        $env:XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN = '1'
    } else {
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN -ErrorAction SilentlyContinue
    }
    if ($FallbackMaxBytes) {
        $env:XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES = $FallbackMaxBytes
    } else {
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES -ErrorAction SilentlyContinue
    }

    try {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $output = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l $ArchivePath 2>&1
        $code = $LASTEXITCODE
        $sw.Stop()
        if ($code -ne 0) {
            throw "7z list failed ($code): $output"
        }
        return [math]::Round($sw.Elapsed.TotalMilliseconds, 2)
    }
    finally {
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN -ErrorAction SilentlyContinue
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES -ErrorAction SilentlyContinue
    }
}

function Measure-Scenario([string]$Name, [scriptblock]$Action) {
    for ($i = 0; $i -lt $Warmup; $i++) {
        & $Action | Out-Null
    }
    $samples = @()
    for ($i = 0; $i -lt $Runs; $i++) {
        $samples += (& $Action)
    }
    return [PSCustomObject]@{
        Name = $Name
        AvgMs = [math]::Round((($samples | Measure-Object -Average).Average), 2)
        MinMs = [math]::Round((($samples | Measure-Object -Minimum).Minimum), 2)
        MaxMs = [math]::Round((($samples | Measure-Object -Maximum).Maximum), 2)
        Samples = ($samples -join ', ')
    }
}

if (!(Test-Path $pluginDll)) {
    & (Join-Path $repoRoot 'scripts/build_xunbak_7z_plugin.ps1') -Config $Config
    if ($LASTEXITCODE -ne 0) {
        throw 'build_xunbak_7z_plugin.ps1 failed'
    }
}

if (!(Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

New-Item -ItemType Directory -Path $stageRoot | Out-Null
New-Item -ItemType Directory -Path $workRoot | Out-Null
New-Item -ItemType Directory -Path $portable7z | Out-Null
New-Item -ItemType Directory -Path $formatsDir | Out-Null

Copy-Item (Join-Path $sevenZipHome '7z.exe') $portable7z
Copy-Item (Join-Path $sevenZipHome '7z.dll') $portable7z
Copy-Item $pluginDll (Join-Path $formatsDir 'xunbak.dll')

$src = Initialize-XunbakPluginFixture -SourceRoot $workRoot
$largeSrc = New-LargeFixture $workRoot

$env:XUN_DB = Join-Path $workRoot '.xun.json'
$env:USERPROFILE = $workRoot
$env:HOME = $workRoot
$env:XUN_NON_INTERACTIVE = '1'

$single = Join-Path $workRoot 'sample.xunbak'
$split = Join-Path $workRoot 'split_sample.xunbak'
$splitFirst = "$split.001"
$large = Join-Path $workRoot 'large_sample.xunbak'

Invoke-Checked { & $xunExe backup create -C $src --format xunbak -o $single } 'xun create single xunbak failed'
Invoke-Checked { & $xunExe backup create -C $src --format xunbak -o $split --split-size 1900 } 'xun create split xunbak failed'
Invoke-Checked { & $xunExe backup create -C $largeSrc --format xunbak -o $large --compression none } 'xun create large xunbak failed'

$results = @()
$results += Measure-Scenario 'single file open (callback path)' {
    Invoke-BenchList -ArchivePath $single
}
$results += Measure-Scenario 'split first volume open (.001 path)' {
    Invoke-BenchList -ArchivePath $splitFirst
}
$results += Measure-Scenario 'large file open (callback path)' {
    Invoke-BenchList -ArchivePath $large
}
$results += Measure-Scenario 'large file open (memory fallback path)' {
    Invoke-BenchList -ArchivePath $large -ForceCallbackFailure -FallbackMaxBytes '67108864'
}

$lines = @(
    '# xunbak 7-Zip Plugin Open Baseline',
    '',
    "- Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
    "- Config: $Config",
    "- Runs: $Runs",
    "- Warmup: $Warmup",
    "- 7-Zip: $sevenZipHome",
    '- Dataset:',
    '  - small single-file fixture: README + src/a.txt + nested/深层.txt',
    '  - small split fixture: same content, split-size=1900',
    '  - large single-file fixture: 32 MiB `big.bin`, `--compression none`',
    '',
    '| Scenario | Avg ms | Min ms | Max ms | Samples |',
    '| --- | ---: | ---: | ---: | --- |'
)

foreach ($result in $results) {
    $lines += "| $($result.Name) | $($result.AvgMs) | $($result.MinMs) | $($result.MaxMs) | $($result.Samples) |"
}

$lines += ''
$lines += '## Notes'
$lines += ''
$lines += '1. `large file open (memory fallback path)` 通过 `XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN=1` 强制 callback 打开失败。'
$lines += '2. `XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES=67108864` 用于允许 32 MiB 大文件进入内存 fallback。'
$lines += '3. 结果用于相对回归对比，不代表严格统计学意义上的发布性能。'

$lines | Set-Content -Path $logPath
Write-Host "Wrote $logPath"
