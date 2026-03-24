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

Write-Host "StageRoot: $stageRoot"
Write-Host "Single list exit: $singleListCode"
Write-Host "Split list exit: $splitListCode"
Write-Host "Single extract exit: $singleExtractCode"
Write-Host "Split extract exit: $splitExtractCode"
Write-Host "Single match: $singleMatch"
Write-Host "Split match: $splitMatch"
Write-Host "Single display: $singleDisplayOk"
Write-Host "Split display: $splitDisplayOk"
Write-Host "`n--- single list ---"
$singleList
Write-Host "`n--- split list ---"
$splitList
Write-Host "`n--- single extract ---"
$singleExtractOut
Write-Host "`n--- split extract ---"
$splitExtractOut
if (-not $singleMatch) {
    Write-Host "`n--- single diff ---"
    $singleDiff | Format-Table -AutoSize
}
if (-not $splitMatch) {
    Write-Host "`n--- split diff ---"
    $splitDiff | Format-Table -AutoSize
}

if ($singleListCode -ne 0 -or $splitListCode -ne 0 -or $singleExtractCode -ne 0 -or $splitExtractCode -ne 0 -or -not $singleMatch -or -not $splitMatch -or -not $singleDisplayOk -or -not $splitDisplayOk) {
    throw 'portable 7-Zip plugin smoke failed'
}

Write-Host 'portable 7-Zip plugin smoke passed'
