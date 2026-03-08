#Requires -Version 7.0

[CmdletBinding()]
param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$OutDir = "",
    [switch]$IncludeLogs,
    [switch]$IncludeAbEvidence
)

$ErrorActionPreference = "Stop"

if (-not $OutDir) {
    $OutDir = Join-Path $ProjectRoot "deliverables"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$stageDir = Join-Path $OutDir "img-delivery-$stamp"
$zipPath = "$stageDir.zip"
New-Item -ItemType Directory -Force -Path $stageDir | Out-Null

function Add-File {
    param(
        [Parameter(Mandatory = $true)][string]$RelPath
    )
    $src = Join-Path $ProjectRoot $RelPath
    if (-not (Test-Path $src -PathType Leaf)) {
        Write-Warning "跳过缺失文件: $RelPath"
        return
    }
    $dst = Join-Path $stageDir $RelPath
    $dstDir = Split-Path -Parent $dst
    New-Item -ItemType Directory -Force -Path $dstDir | Out-Null
    Copy-Item -Path $src -Destination $dst -Force
}

function Add-Dir {
    param(
        [Parameter(Mandatory = $true)][string]$RelPath
    )
    $src = Join-Path $ProjectRoot $RelPath
    if (-not (Test-Path $src -PathType Container)) {
        Write-Warning "跳过缺失目录: $RelPath"
        return
    }
    $dst = Join-Path $stageDir $RelPath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dst) | Out-Null
    Copy-Item -Path $src -Destination $dst -Recurse -Force
}

function Add-LatestFileByPattern {
    param(
        [Parameter(Mandatory = $true)][string]$RelDir,
        [Parameter(Mandatory = $true)][string]$Pattern
    )
    $dir = Join-Path $ProjectRoot $RelDir
    if (-not (Test-Path $dir -PathType Container)) {
        Write-Warning "跳过目录: $RelDir"
        return
    }
    $hit = Get-ChildItem -Path $dir -File -Filter $Pattern |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($null -eq $hit) {
        Write-Warning "未找到匹配文件: $RelDir/$Pattern"
        return
    }
    $rel = Resolve-Path -Path $hit.FullName | ForEach-Object {
        $_.Path.Substring($ProjectRoot.Length).TrimStart('\', '/')
    }
    Add-File -RelPath $rel
}

function Add-LatestDirByPattern {
    param(
        [Parameter(Mandatory = $true)][string]$RelDir,
        [Parameter(Mandatory = $true)][string]$Pattern
    )
    $dir = Join-Path $ProjectRoot $RelDir
    if (-not (Test-Path $dir -PathType Container)) {
        Write-Warning "跳过目录: $RelDir"
        return $null
    }
    $hit = Get-ChildItem -Path $dir -Directory -Filter $Pattern |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($null -eq $hit) {
        Write-Warning "未找到匹配目录: $RelDir/$Pattern"
        return $null
    }
    return Resolve-Path -Path $hit.FullName | ForEach-Object {
        $_.Path.Substring($ProjectRoot.Length).TrimStart('\', '/')
    }
}

# 1) 核心代码（默认打包完整 src）
Add-File -RelPath "Cargo.toml"
Add-Dir -RelPath "src"

# 2) 文档
Add-File -RelPath "docs/project/image-recommand.md"

# 3) 最新全量 20 组结果
$latestFinal20 = Add-LatestDirByPattern -RelDir "image-test" -Pattern "final20-*"
if ($latestFinal20) {
    Add-File -RelPath (Join-Path $latestFinal20 "results.csv")
    if ($IncludeLogs) {
        Add-Dir -RelPath (Join-Path $latestFinal20 "logs")
    }
}

# 4) 可选 AB 证据
if ($IncludeAbEvidence) {
    Add-LatestFileByPattern -RelDir "image-test" -Pattern "ab-results-releaseimg-*.csv"
    Add-LatestFileByPattern -RelDir "image-test" -Pattern "ab-results-imagequant-default-*.csv"
    Add-LatestFileByPattern -RelDir "image-test" -Pattern "ab-results-imagequant-default-r2-*.csv"
}

if (Test-Path $zipPath) {
    Remove-Item -Path $zipPath -Force
}
Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath -Force

Write-Host "打包完成"
Write-Host "目录: $stageDir"
Write-Host "压缩包: $zipPath"
