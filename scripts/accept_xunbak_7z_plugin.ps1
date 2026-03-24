param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug',
    [switch]$WithSystem,
    [string]$SevenZipHome = 'C:\A_Softwares\7-Zip'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')

& (Join-Path $repoRoot 'scripts/build_xunbak_7z_plugin.ps1') -Config $Config
if ($LASTEXITCODE -ne 0) {
    throw 'build_xunbak_7z_plugin.ps1 failed'
}

& (Join-Path $repoRoot 'scripts/smoke_xunbak_7z_plugin.ps1')
if ($LASTEXITCODE -ne 0) {
    throw 'smoke_xunbak_7z_plugin.ps1 failed'
}

& (Join-Path $repoRoot 'scripts/test_xunbak_7z_plugin_portable.ps1') -Config $Config
if ($LASTEXITCODE -ne 0) {
    throw 'test_xunbak_7z_plugin_portable.ps1 failed'
}

if ($WithSystem) {
    & (Join-Path $repoRoot 'scripts/test_xunbak_7z_plugin_system.ps1') -Config $Config -SevenZipHome $SevenZipHome
    if ($LASTEXITCODE -ne 0) {
        throw 'test_xunbak_7z_plugin_system.ps1 failed'
    }
}

Write-Host 'xunbak 7-Zip plugin acceptance passed'
