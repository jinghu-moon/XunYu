#Requires -Version 7.0

[CmdletBinding()]
param(
    [ValidateSet("backup", "restore", "alias")]
    [string]$Preset,

    [ValidateSet("lib", "test")]
    [string]$Mode,

    [string]$Target,

    [string]$Filter,

    [string[]]$ExtraArgs
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$findLocker = Join-Path $PSScriptRoot "find-locker.ps1"

if ($Preset) {
    switch ($Preset) {
        "backup" {
            $Mode = "test"
            $Target = "module_backup_restore"
            if (-not $Filter) {
                $Filter = "backup_"
            }
        }
        "restore" {
            $Mode = "test"
            $Target = "module_backup_restore"
            if (-not $Filter) {
                $Filter = "restore_"
            }
        }
        "alias" {
            $Mode = "test"
            $Target = "module_alias"
            if (-not $Filter) {
                $Filter = "alias_"
            }
            $ExtraArgs = @("--features", "alias") + @($ExtraArgs)
        }
    }
}

if (-not $Mode -or -not $Target -or -not $Filter) {
    throw "Either pass -Preset <backup|restore|alias> or provide -Mode, -Target, and -Filter."
}

function Invoke-CargoTest {
    param(
        [string]$Mode,
        [string]$Target,
        [string]$Filter,
        [string[]]$ExtraArgs
    )

    $args = @("test", $Filter)
    switch ($Mode) {
        "lib" {
            $args += @("--lib")
        }
        "test" {
            $args += @("--test", $Target)
        }
    }
    if ($ExtraArgs) {
        $args += @($ExtraArgs | Where-Object { $_ -and $_.Trim().Length -gt 0 })
    }
    $args += @("--", "--nocapture")

    Write-Host "Running: cargo $($args -join ' ')" -ForegroundColor Cyan
    $output = & cargo @args 2>&1
    $exitCode = $LASTEXITCODE

    [PSCustomObject]@{
        ExitCode = $exitCode
        Output = @($output)
        Text = ($output -join [Environment]::NewLine)
    }
}

function Ensure-AliasShim {
    $args = @("build", "-p", "alias-shim", "--profile", "release-shim")
    Write-Host "Running: cargo $($args -join ' ')" -ForegroundColor Cyan
    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build alias-shim prerequisite."
    }
}

if ($Target -eq "module_alias" -or $Target -eq "special_alias_perf") {
    Ensure-AliasShim
}

$result = Invoke-CargoTest -Mode $Mode -Target $Target -Filter $Filter -ExtraArgs $ExtraArgs
$result.Output | ForEach-Object { $_ }

if ($result.ExitCode -eq 0) {
    exit 0
}

if ($result.Text -match "LNK1104") {
    Write-Host ""
    Write-Host "Detected LNK1104. Trying to find locker..." -ForegroundColor Yellow

    $pathMatch = [regex]::Match($result.Text, "cannot open file '([^']+\.exe)'")
    if ($pathMatch.Success) {
        $lockedPath = $pathMatch.Groups[1].Value
        if (Test-Path $findLocker) {
            & pwsh -File $findLocker -Path $lockedPath
        }
    } elseif (Test-Path $findLocker) {
        & pwsh -File $findLocker -RecentDeps
    }
}

exit $result.ExitCode
