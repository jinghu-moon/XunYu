param(
    [string]$SevenZipHome = 'C:\A_Softwares\7-Zip'
)

$ErrorActionPreference = 'Stop'

$formatsDir = Join-Path $SevenZipHome 'Formats'
$targetDll = Join-Path $formatsDir 'xunbak.dll'

if (Test-Path $targetDll) {
    Remove-Item $targetDll -Force
    Write-Host "Removed: $targetDll"
} else {
    Write-Host "Plugin not installed: $targetDll"
}

$backups = Get-ChildItem -LiteralPath $formatsDir -Filter 'xunbak.dll.bak.*' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending

if ($backups -and $backups.Count -gt 0) {
    $latest = $backups[0]
    Copy-Item $latest.FullName $targetDll -Force
    Write-Host "Restored backup: $($latest.FullName)"
}
