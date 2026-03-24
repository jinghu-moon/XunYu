param(
    [ValidateSet('Debug', 'Release')]
    [string]$Config = 'Debug',
    [string]$SevenZipHome = 'C:\A_Softwares\7-Zip'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$pluginDll = Join-Path $repoRoot "build/xunbak-7z-plugin/$Config/xunbak.dll"
$formatsDir = Join-Path $SevenZipHome 'Formats'
$targetDll = Join-Path $formatsDir 'xunbak.dll'

if (!(Test-Path $pluginDll)) {
    & (Join-Path $repoRoot 'scripts/build_xunbak_7z_plugin.ps1') -Config $Config
}

if (!(Test-Path $formatsDir)) {
    New-Item -ItemType Directory -Path $formatsDir -Force | Out-Null
}

$backupDll = $null
if (Test-Path $targetDll) {
    $timestamp = Get-Date -Format 'yyyyMMdd_HHmmss'
    $backupDll = Join-Path $formatsDir ("xunbak.dll.bak." + $timestamp)
    Copy-Item $targetDll $backupDll -Force
}

Copy-Item $pluginDll $targetDll -Force

Write-Host "Installed: $targetDll"
if ($backupDll) {
    Write-Host "Backup: $backupDll"
}
