param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
. (Join-Path $repoRoot 'scripts/xunbak_7z_plugin_common.ps1')
$sevenZipHome = 'C:\A_Softwares\7-Zip'
$stageRoot = Join-Path $env:TEMP ('xunbak-7z-plugin-stage-' + [guid]::NewGuid().ToString())
$workRoot = Join-Path $stageRoot 'work'
$portable7z = Join-Path $stageRoot '7zip'
$formatsDir = Join-Path $portable7z 'Formats'
$pluginDll = Join-Path $repoRoot "build/xunbak-7z-plugin/$Config/xunbak.dll"
$xunExe = Join-Path $repoRoot 'target/debug/xun.exe'

function Invoke-Checked([scriptblock]$Action, [string]$Message) {
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Message (exit=$LASTEXITCODE)"
    }
}

function Invoke-TraceList {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ArchivePath,
        [Parameter(Mandatory = $true)]
        [string]$TracePath,
        [switch]$ForceCallbackFailure,
        [string]$FallbackMaxBytes
    )

    Remove-Item $TracePath -Force -ErrorAction SilentlyContinue
    $env:XUN_XUNBAK_PLUGIN_TRACE_FILE = $TracePath
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
        $output = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l $ArchivePath 2>&1
        $code = $LASTEXITCODE
        $trace = if (Test-Path $TracePath) { Get-Content $TracePath -Raw } else { '' }
        return [PSCustomObject]@{
            Code = $code
            Output = $output
            Trace = $trace
        }
    }
    finally {
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_TRACE_FILE -ErrorAction SilentlyContinue
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN -ErrorAction SilentlyContinue
        Remove-Item Env:\XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES -ErrorAction SilentlyContinue
    }
}

if (!(Test-Path $pluginDll)) {
    & (Join-Path $repoRoot 'scripts/build_xunbak_7z_plugin.ps1') -Config $Config
}

New-Item -ItemType Directory -Path $stageRoot | Out-Null
New-Item -ItemType Directory -Path $workRoot | Out-Null
New-Item -ItemType Directory -Path $portable7z | Out-Null
New-Item -ItemType Directory -Path $formatsDir | Out-Null

Copy-Item (Join-Path $sevenZipHome '7z.exe') $portable7z
Copy-Item (Join-Path $sevenZipHome '7z.dll') $portable7z
Copy-Item $pluginDll (Join-Path $formatsDir 'xunbak.dll')

$src = Initialize-XunbakPluginFixture -SourceRoot $workRoot

$env:XUN_DB = Join-Path $workRoot '.xun.json'
$env:USERPROFILE = $workRoot
$env:HOME = $workRoot
$env:XUN_NON_INTERACTIVE = '1'

$single = Join-Path $workRoot 'sample.xunbak'
$split = Join-Path $workRoot 'split_sample.xunbak'
$splitFirst = "$split.001"
$sevenZipArgs = @('-sccUTF-8')

Invoke-Checked { & $xunExe backup create -C $src --format xunbak -o $single } 'xun create single xunbak failed'
Invoke-Checked { & $xunExe backup create -C $src --format xunbak -o $split --split-size 1900 } 'xun create split xunbak failed'

$singleList = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l $single 2>&1
$singleListCode = $LASTEXITCODE
$splitList = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l $splitFirst 2>&1
$splitListCode = $LASTEXITCODE
$singleTechList = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l -slt $single 2>&1
$singleTechListCode = $LASTEXITCODE
$splitTechList = & (Join-Path $portable7z '7z.exe') @sevenZipArgs l -slt $splitFirst 2>&1
$splitTechListCode = $LASTEXITCODE

$singleExtract = Join-Path $workRoot 'extract-single'
$splitExtract = Join-Path $workRoot 'extract-split'
New-Item -ItemType Directory -Path $singleExtract | Out-Null
New-Item -ItemType Directory -Path $splitExtract | Out-Null

$singleExtractOut = & (Join-Path $portable7z '7z.exe') @sevenZipArgs x $single "-o$singleExtract" -y 2>&1
$singleExtractCode = $LASTEXITCODE
$splitExtractOut = & (Join-Path $portable7z '7z.exe') @sevenZipArgs x $splitFirst "-o$splitExtract" -y 2>&1
$splitExtractCode = $LASTEXITCODE

$expected = Get-XunbakPluginTreeHashMap $src
$expected.Remove('.xun-bak.json') | Out-Null
$actualSingle = Get-XunbakPluginTreeHashMap $singleExtract
$actualSplit = Get-XunbakPluginTreeHashMap $splitExtract
$singleDiff = Compare-Object ($expected.GetEnumerator() | Sort-Object Name) ($actualSingle.GetEnumerator() | Sort-Object Name) -Property Name, Value
$splitDiff = Compare-Object ($expected.GetEnumerator() | Sort-Object Name) ($actualSplit.GetEnumerator() | Sort-Object Name) -Property Name, Value
$singleMatch = @($singleDiff).Count -eq 0
$splitMatch = @($splitDiff).Count -eq 0
$singleDisplayOk = (($singleList | Out-String) -match 'Type = XUNBAK') -and (($singleList | Out-String) -match 'nested/深层\.txt')
$splitDisplayOk = (($splitList | Out-String) -match 'Type = XUNBAK') -and (($splitList | Out-String) -match 'Volumes = 2') -and (($splitList | Out-String) -match 'nested/深层\.txt')
$singleTechOk = (($singleTechList | Out-String) -match 'Files = 3') -and (($singleTechList | Out-String) -match 'Method = (Copy|ZSTD)')
$splitTechOk = (($splitTechList | Out-String) -match 'Files = 3') -and (($splitTechList | Out-String) -match 'Volumes = 2') -and (($splitTechList | Out-String) -match 'Method = (Copy|ZSTD)')

$singleCallbackTrace = Invoke-TraceList -ArchivePath $single -TracePath (Join-Path $workRoot 'trace-single-callback.log')
$splitCallbackTrace = Invoke-TraceList -ArchivePath $splitFirst -TracePath (Join-Path $workRoot 'trace-split-callback.log')
$singleFallbackTrace = Invoke-TraceList -ArchivePath $single -TracePath (Join-Path $workRoot 'trace-single-fallback.log') -ForceCallbackFailure -FallbackMaxBytes '67108864'
$largeRejectTrace = Invoke-TraceList -ArchivePath $single -TracePath (Join-Path $workRoot 'trace-large-reject.log') -ForceCallbackFailure -FallbackMaxBytes '1'

$singleCallbackOk = $singleCallbackTrace.Code -eq 0 -and
    $singleCallbackTrace.Trace.Contains('open.callback.success') -and
    -not $singleCallbackTrace.Trace.Contains('open.readall.begin')
$splitCallbackTry = 'open.callback.try candidate=split_sample.xunbak.001'
$splitReadAll = 'open.readall.begin'
$splitCallbackOk = $splitCallbackTrace.Code -eq 0 -and
    $splitCallbackTrace.Trace.Contains($splitCallbackTry) -and
    ($splitCallbackTrace.Trace.IndexOf($splitCallbackTry) -ge 0) -and
    (($splitCallbackTrace.Trace.IndexOf($splitReadAll) -lt 0) -or
        ($splitCallbackTrace.Trace.IndexOf($splitCallbackTry) -lt $splitCallbackTrace.Trace.IndexOf($splitReadAll)))
$singleFallbackOk = $singleFallbackTrace.Code -eq 0 -and
    $singleFallbackTrace.Trace.Contains('open.fallback.allowed') -and
    $singleFallbackTrace.Trace.Contains('open.readall.begin') -and
    $singleFallbackTrace.Trace.Contains('open.direct.success')
$largeRejectOk = $largeRejectTrace.Code -ne 0 -and
    $largeRejectTrace.Trace.Contains('open.fallback.rejected reason=threshold') -and
    -not $largeRejectTrace.Trace.Contains('open.readall.begin')

Write-Host "StageRoot: $stageRoot"
Write-Host "Single list exit: $singleListCode"
Write-Host "Split list exit: $splitListCode"
Write-Host "Single extract exit: $singleExtractCode"
Write-Host "Split extract exit: $splitExtractCode"
Write-Host "Single tech exit: $singleTechListCode"
Write-Host "Split tech exit: $splitTechListCode"
Write-Host "Single match: $singleMatch"
Write-Host "Split match: $splitMatch"
Write-Host "Single display: $singleDisplayOk"
Write-Host "Split display: $splitDisplayOk"
Write-Host "Single tech: $singleTechOk"
Write-Host "Split tech: $splitTechOk"
Write-Host "Single callback trace: $singleCallbackOk"
Write-Host "Split callback trace: $splitCallbackOk"
Write-Host "Single fallback trace: $singleFallbackOk"
Write-Host "Large reject trace: $largeRejectOk"
Write-Host "`n--- single list ---"
$singleList
Write-Host "`n--- split list ---"
$splitList
Write-Host "`n--- single tech list ---"
$singleTechList
Write-Host "`n--- split tech list ---"
$splitTechList
Write-Host "`n--- single extract ---"
$singleExtractOut
Write-Host "`n--- split extract ---"
$splitExtractOut
Write-Host "`n--- single callback trace ---"
$singleCallbackTrace.Trace
Write-Host "`n--- split callback trace ---"
$splitCallbackTrace.Trace
Write-Host "`n--- single fallback trace ---"
$singleFallbackTrace.Trace
Write-Host "`n--- large reject trace ---"
$largeRejectTrace.Trace
if (-not $singleMatch) {
    Write-Host "`n--- single diff ---"
    $singleDiff | Format-Table -AutoSize
}
if (-not $splitMatch) {
    Write-Host "`n--- split diff ---"
    $splitDiff | Format-Table -AutoSize
}

if ($singleListCode -ne 0 -or $splitListCode -ne 0 -or $singleTechListCode -ne 0 -or $splitTechListCode -ne 0 -or $singleExtractCode -ne 0 -or $splitExtractCode -ne 0 -or -not $singleMatch -or -not $splitMatch -or -not $singleDisplayOk -or -not $splitDisplayOk -or -not $singleTechOk -or -not $splitTechOk -or -not $singleCallbackOk -or -not $splitCallbackOk -or -not $singleFallbackOk -or -not $largeRejectOk) {
    throw 'portable 7-Zip plugin smoke failed'
}

Write-Host 'portable 7-Zip plugin smoke passed'
