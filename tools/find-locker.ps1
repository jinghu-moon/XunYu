#Requires -Version 7.0

[CmdletBinding()]
param(
    [string]$Path,
    [switch]$RecentDeps
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$depsDir = Join-Path $root "target/debug/deps"

function Write-Section([string]$Title) {
    Write-Host ""
    Write-Host "== $Title ==" -ForegroundColor Cyan
}

function Find-HandleExe {
    $cmd = Get-Command handle.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    $common = @(
        "$env:ProgramFiles/Handle/handle.exe",
        "$env:ProgramFiles/Sysinternals/handle.exe",
        "$env:ProgramFiles/Sysinternals Suite/handle.exe",
        "$env:USERPROFILE/tools/handle.exe"
    )
    foreach ($candidate in $common) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }
    return $null
}

function Show-ProcessCandidates {
    param([string]$TargetPath)

    Write-Section "Process Candidates"
    $procs = Get-Process -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -like "$depsDir\*.exe" -or
        $_.ProcessName -match '^(cargo|rustc|link|xun|xyu|test_.*|general_.*|module_.*|special_.*|path_guard_bench|acl_test)$'
    } | Select-Object ProcessName, Id, Path

    if ($procs) {
        $procs | Format-Table -AutoSize
    } else {
        Write-Host "No obvious candidate processes are currently running." -ForegroundColor Yellow
    }

    if ($TargetPath) {
        Write-Section "Target File"
        Write-Host $TargetPath
        if (Test-Path $TargetPath) {
            Get-Item $TargetPath | Select-Object FullName, Length, LastWriteTime | Format-List
        } else {
            Write-Host "Target file does not exist." -ForegroundColor Yellow
        }
    }
}

function Show-HandleOutput {
    param(
        [string]$HandleExe,
        [string]$TargetPath
    )

    Write-Section "handle.exe"
    Write-Host "Using: $HandleExe"
    & $HandleExe -nobanner $TargetPath 2>$null
}

function Get-RecentDepExecutables {
    if (-not (Test-Path $depsDir)) {
        return @()
    }
    Get-ChildItem -Path $depsDir -Filter "*.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 15
}

if (-not $Path -and -not $RecentDeps) {
    $RecentDeps = $true
}

$handleExe = Find-HandleExe

if ($Path) {
    $resolved = if (Test-Path $Path) { (Resolve-Path $Path).Path } else { $Path }
    if ($handleExe) {
        Show-HandleOutput -HandleExe $handleExe -TargetPath $resolved
    }
    Show-ProcessCandidates -TargetPath $resolved
    exit 0
}

if ($RecentDeps) {
    $targets = Get-RecentDepExecutables
    Write-Section "Recent Test EXEs"
    if (-not $targets) {
        Write-Host "No executables found in $depsDir" -ForegroundColor Yellow
        Show-ProcessCandidates -TargetPath $null
        exit 0
    }

    $targets | Select-Object Name, LastWriteTime | Format-Table -AutoSize

    foreach ($target in $targets) {
        Write-Section $target.Name
        if ($handleExe) {
            & $handleExe -nobanner $target.FullName 2>$null
        } else {
            Write-Host "handle.exe not found; showing process candidates instead." -ForegroundColor Yellow
            Show-ProcessCandidates -TargetPath $target.FullName
        }
    }
}
