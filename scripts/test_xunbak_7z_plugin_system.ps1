param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug',
    [string]$SevenZipHome = 'C:\A_Softwares\7-Zip'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
. (Join-Path $repoRoot 'scripts/xunbak_7z_plugin_common.ps1')
$xunExe = Join-Path $repoRoot 'target/debug/xun.exe'
$workRoot = Join-Path $env:TEMP ('xunbak-7z-plugin-system-' + [guid]::NewGuid().ToString())
$src = Join-Path $workRoot 'src'
$single = Join-Path $workRoot 'sample.xunbak'
$split = Join-Path $workRoot 'split_sample.xunbak'
$splitFirst = "$split.001"
$sevenZipExe = Join-Path $SevenZipHome '7z.exe'
$sevenZipArgs = @('-sccUTF-8')
$installScript = Join-Path $repoRoot 'scripts/install_xunbak_7z_plugin.ps1'
$uninstallScript = Join-Path $repoRoot 'scripts/uninstall_xunbak_7z_plugin.ps1'

New-Item -ItemType Directory -Path $workRoot | Out-Null
$src = Initialize-XunbakPluginFixture -SourceRoot $workRoot

$env:XUN_DB = Join-Path $workRoot '.xun.json'
$env:USERPROFILE = $workRoot
$env:HOME = $workRoot
$env:XUN_NON_INTERACTIVE = '1'

& $xunExe backup create -C $src --format xunbak -o $single
if ($LASTEXITCODE -ne 0) { throw 'create single failed' }
& $xunExe backup create -C $src --format xunbak -o $split --split-size 1900
if ($LASTEXITCODE -ne 0) { throw 'create split failed' }

try {
    & $installScript -Config $Config -SevenZipHome $SevenZipHome
    if ($LASTEXITCODE -ne 0) { throw 'install script failed' }

    $singleList = & $sevenZipExe @sevenZipArgs l $single 2>&1
    $singleListCode = $LASTEXITCODE
    $splitList = & $sevenZipExe @sevenZipArgs l $splitFirst 2>&1
    $splitListCode = $LASTEXITCODE

    $singleExtract = Join-Path $workRoot 'extract-single'
    $splitExtract = Join-Path $workRoot 'extract-split'
    New-Item -ItemType Directory -Path $singleExtract | Out-Null
    New-Item -ItemType Directory -Path $splitExtract | Out-Null

    $singleExtractOut = & $sevenZipExe @sevenZipArgs x $single "-o$singleExtract" -y 2>&1
    $singleExtractCode = $LASTEXITCODE
    $splitExtractOut = & $sevenZipExe @sevenZipArgs x $splitFirst "-o$splitExtract" -y 2>&1
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

    Write-Host "WorkRoot: $workRoot"
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

    if ($singleListCode -ne 0 -or $splitListCode -ne 0 -or $singleExtractCode -ne 0 -or $splitExtractCode -ne 0 -or -not $singleMatch -or -not $splitMatch -or -not $singleDisplayOk -or -not $splitDisplayOk) {
        throw 'system 7-Zip plugin smoke failed'
    }

    Write-Host 'system 7-Zip plugin smoke passed'
}
finally {
    & $uninstallScript -SevenZipHome $SevenZipHome
}
