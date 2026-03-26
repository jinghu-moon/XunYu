param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug',
    [string]$SevenZipHome = 'C:\A_Softwares\7-Zip',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
. (Join-Path $repoRoot 'scripts/xunbak_7z_plugin_common.ps1')
$xunExe = Join-Path $repoRoot 'target/debug/xun.exe'
$workRoot = Join-Path $env:TEMP ('xunbak-7z-plugin-system-' + [guid]::NewGuid().ToString())
$src = Join-Path $workRoot 'src'
$single = Join-Path $workRoot 'sample.xunbak'
$ppmd = Join-Path $workRoot 'ppmd_sample.xunbak'
$split = Join-Path $workRoot 'split_sample.xunbak'
$splitFirst = "$split.001"
$sevenZipExe = Join-Path $SevenZipHome '7z.exe'
$sevenZipArgs = @('-sccUTF-8')
$installScript = Join-Path $repoRoot 'scripts/install_xunbak_7z_plugin.ps1'
$uninstallScript = Join-Path $repoRoot 'scripts/uninstall_xunbak_7z_plugin.ps1'

if (-not $SkipBuild) {
    & (Join-Path $repoRoot 'scripts/build_xunbak_7z_plugin.ps1') -Config $Config
    if ($LASTEXITCODE -ne 0) { throw 'build script failed' }
}

New-Item -ItemType Directory -Path $workRoot | Out-Null
$src = Initialize-XunbakPluginFixture -SourceRoot $workRoot

$env:XUN_DB = Join-Path $workRoot '.xun.json'
$env:USERPROFILE = $workRoot
$env:HOME = $workRoot
$env:XUN_NON_INTERACTIVE = '1'

& $xunExe backup create -C $src --format xunbak -o $single
if ($LASTEXITCODE -ne 0) { throw 'create single failed' }
& $xunExe backup create -C $src --format xunbak -o $ppmd --compression ppmd
if ($LASTEXITCODE -ne 0) { throw 'create ppmd failed' }
& $xunExe backup create -C $src --format xunbak -o $split --split-size 1900
if ($LASTEXITCODE -ne 0) { throw 'create split failed' }

try {
    & $installScript -Config $Config -SevenZipHome $SevenZipHome
    if ($LASTEXITCODE -ne 0) { throw 'install script failed' }

    $singleList = & $sevenZipExe @sevenZipArgs l $single 2>&1
    $singleListCode = $LASTEXITCODE
    $ppmdList = & $sevenZipExe @sevenZipArgs l $ppmd 2>&1
    $ppmdListCode = $LASTEXITCODE
    $splitList = & $sevenZipExe @sevenZipArgs l $splitFirst 2>&1
    $splitListCode = $LASTEXITCODE
    $singleTechList = & $sevenZipExe @sevenZipArgs l -slt $single 2>&1
    $singleTechListCode = $LASTEXITCODE
    $ppmdTechList = & $sevenZipExe @sevenZipArgs l -slt $ppmd 2>&1
    $ppmdTechListCode = $LASTEXITCODE
    $splitTechList = & $sevenZipExe @sevenZipArgs l -slt $splitFirst 2>&1
    $splitTechListCode = $LASTEXITCODE

    $singleExtract = Join-Path $workRoot 'extract-single'
    $ppmdExtract = Join-Path $workRoot 'extract-ppmd'
    $splitExtract = Join-Path $workRoot 'extract-split'
    New-Item -ItemType Directory -Path $singleExtract | Out-Null
    New-Item -ItemType Directory -Path $ppmdExtract | Out-Null
    New-Item -ItemType Directory -Path $splitExtract | Out-Null

    $singleExtractOut = & $sevenZipExe @sevenZipArgs x $single "-o$singleExtract" -y 2>&1
    $singleExtractCode = $LASTEXITCODE
    $ppmdExtractOut = & $sevenZipExe @sevenZipArgs x $ppmd "-o$ppmdExtract" -y 2>&1
    $ppmdExtractCode = $LASTEXITCODE
    $splitExtractOut = & $sevenZipExe @sevenZipArgs x $splitFirst "-o$splitExtract" -y 2>&1
    $splitExtractCode = $LASTEXITCODE

    $expected = Get-XunbakPluginTreeHashMap $src
    $expected.Remove('.xun-bak.json') | Out-Null
    $actualSingle = Get-XunbakPluginTreeHashMap $singleExtract
    $actualPpmd = Get-XunbakPluginTreeHashMap $ppmdExtract
    $actualSplit = Get-XunbakPluginTreeHashMap $splitExtract
    $singleDiff = Compare-Object @($expected.GetEnumerator() | Sort-Object Name) @($actualSingle.GetEnumerator() | Sort-Object Name) -Property Name, Value
    $ppmdDiff = Compare-Object @($expected.GetEnumerator() | Sort-Object Name) @($actualPpmd.GetEnumerator() | Sort-Object Name) -Property Name, Value
    $splitDiff = Compare-Object @($expected.GetEnumerator() | Sort-Object Name) @($actualSplit.GetEnumerator() | Sort-Object Name) -Property Name, Value
    $singleMatch = @($singleDiff).Count -eq 0
    $ppmdMatch = @($ppmdDiff).Count -eq 0
    $splitMatch = @($splitDiff).Count -eq 0
    $singleDisplayOk = (($singleList | Out-String) -match 'Type = XUNBAK') -and (($singleList | Out-String) -match 'nested/深层\.txt')
    $ppmdDisplayOk = (($ppmdList | Out-String) -match 'Type = XUNBAK') -and (($ppmdList | Out-String) -match 'nested/深层\.txt')
    $splitDisplayOk = (($splitList | Out-String) -match 'Type = XUNBAK') -and (($splitList | Out-String) -match 'Volumes = 2') -and (($splitList | Out-String) -match 'nested/深层\.txt')
    $singleTechOk = (($singleTechList | Out-String) -match 'Files = 3') -and (($singleTechList | Out-String) -match 'Method = (Copy|ZSTD)')
    $ppmdTechOk = (($ppmdTechList | Out-String) -match 'Files = 3') -and (($ppmdTechList | Out-String) -match 'Method = PPMD')
    $splitTechOk = (($splitTechList | Out-String) -match 'Files = 3') -and (($splitTechList | Out-String) -match 'Volumes = 2') -and (($splitTechList | Out-String) -match 'Method = (Copy|ZSTD)')

    Write-Host "WorkRoot: $workRoot"
    Write-Host "Single list exit: $singleListCode"
    Write-Host "PPMD list exit: $ppmdListCode"
    Write-Host "Split list exit: $splitListCode"
    Write-Host "Single extract exit: $singleExtractCode"
    Write-Host "PPMD extract exit: $ppmdExtractCode"
    Write-Host "Split extract exit: $splitExtractCode"
    Write-Host "Single tech exit: $singleTechListCode"
    Write-Host "PPMD tech exit: $ppmdTechListCode"
    Write-Host "Split tech exit: $splitTechListCode"
    Write-Host "Single match: $singleMatch"
    Write-Host "PPMD match: $ppmdMatch"
    Write-Host "Split match: $splitMatch"
    Write-Host "Single display: $singleDisplayOk"
    Write-Host "PPMD display: $ppmdDisplayOk"
    Write-Host "Split display: $splitDisplayOk"
    Write-Host "Single tech: $singleTechOk"
    Write-Host "PPMD tech: $ppmdTechOk"
    Write-Host "Split tech: $splitTechOk"
    Write-Host "`n--- single list ---"
    $singleList
    Write-Host "`n--- split list ---"
    $splitList
    Write-Host "`n--- ppmd list ---"
    $ppmdList
    Write-Host "`n--- single tech list ---"
    $singleTechList
    Write-Host "`n--- ppmd tech list ---"
    $ppmdTechList
    Write-Host "`n--- split tech list ---"
    $splitTechList
    Write-Host "`n--- single extract ---"
    $singleExtractOut
    Write-Host "`n--- ppmd extract ---"
    $ppmdExtractOut
    Write-Host "`n--- split extract ---"
    $splitExtractOut

    if (-not $ppmdMatch) {
        Write-Host "`n--- ppmd diff ---"
        $ppmdDiff | Format-Table -AutoSize
    }

    if ($singleListCode -ne 0 -or $ppmdListCode -ne 0 -or $splitListCode -ne 0 -or $singleTechListCode -ne 0 -or $ppmdTechListCode -ne 0 -or $splitTechListCode -ne 0 -or $singleExtractCode -ne 0 -or $ppmdExtractCode -ne 0 -or $splitExtractCode -ne 0 -or -not $singleMatch -or -not $ppmdMatch -or -not $splitMatch -or -not $singleDisplayOk -or -not $ppmdDisplayOk -or -not $splitDisplayOk -or -not $singleTechOk -or -not $ppmdTechOk -or -not $splitTechOk) {
        throw 'system 7-Zip plugin smoke failed'
    }

    Write-Host 'system 7-Zip plugin smoke passed'
    $global:LASTEXITCODE = 0
}
finally {
    & $uninstallScript -SevenZipHome $SevenZipHome
}
