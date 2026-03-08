param(
    [string]$Msys2Root = "C:/A_Softwares/MSYS2",
    [string]$Prefix = "",
    [string]$Source = "",
    [string]$Branch = "n8.0.1",
    [switch]$DisableHw,
    [switch]$EnableNonfree,
    [switch]$EnableFdkAac,
    [switch]$EnableSvtAv1,
    [ValidateSet("shared", "static")]
    [string]$LinkMode = "shared"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$bash = Join-Path $Msys2Root "usr/bin/bash.exe"
if (-not (Test-Path $bash)) {
    throw "未找到 MSYS2 bash: $bash"
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$scriptPath = "$repoRoot/tools/video/build_ffmpeg_msys2.sh"
if (-not (Test-Path $scriptPath)) {
    throw "未找到脚本: $scriptPath"
}

$envArgs = @()
if ($Prefix) {
    $prefixUnix = $Prefix -replace "\\", "/"
    $envArgs += "FFMPEG_PREFIX='$prefixUnix'"
}
if ($Branch) {
    $envArgs += "FFMPEG_BRANCH='$Branch'"
}
if ($Source) {
    $sourceUnix = $Source -replace "\\", "/"
    $envArgs += "FFMPEG_SRC='$sourceUnix'"
}
if ($DisableHw) {
    $envArgs += "ENABLE_HW='0'"
}
if ($EnableNonfree) {
    $envArgs += "ENABLE_NONFREE='1'"
}
if ($EnableFdkAac) {
    $envArgs += "ENABLE_FDK_AAC='1'"
}
if ($EnableSvtAv1) {
    $envArgs += "ENABLE_SVTAV1='1'"
}
if ($LinkMode -eq "static") {
    $envArgs += "ENABLE_SHARED='0'"
} else {
    $envArgs += "ENABLE_SHARED='1'"
}

$cmdParts = @()
if ($envArgs.Count -gt 0) {
    $cmdParts += ($envArgs -join " ")
}
$cmdParts += "MSYSTEM='MINGW64' CHERE_INVOKING='1' PATH='/mingw64/bin:/usr/bin:`$PATH' bash '$($scriptPath -replace '\\','/')'"
$cmd = $cmdParts -join " "

Write-Host "执行命令: $cmd"
& $bash -lc $cmd
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
