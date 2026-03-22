#Requires -Version 7.0

[CmdletBinding()]
param(
    [ValidateSet("default", "general", "modules", "special", "all")]
    [string]$Scope = "default",

    [ValidateSet("auto", "cargo", "nextest")]
    [string]$Runner = "auto",

    [ValidateSet("default", "ci")]
    [string]$Profile = "default",

    [string]$TargetFilter,

    [switch]$Ignored,

    [switch]$ListOnly,

    [switch]$KeepGoing
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $root "Cargo.toml"

function Get-TestTargets {
    $targets = New-Object System.Collections.Generic.List[object]
    $current = $null

    foreach ($line in Get-Content $cargoToml) {
        $trimmed = $line.Trim()

        if ($trimmed -eq "[[test]]") {
            if ($current -and $current.Name) {
                $targets.Add([PSCustomObject]$current)
            }
            $current = @{
                Name = $null
                RequiredFeatures = @()
            }
            continue
        }

        if (-not $current) {
            continue
        }

        if ($trimmed -match '^name\s*=\s*"([^"]+)"') {
            $current.Name = $Matches[1]
            continue
        }

        if ($trimmed -match '^required-features\s*=\s*\[(.*)\]') {
            $features = $Matches[1] -split "," |
                ForEach-Object { $_.Trim().Trim('"') } |
                Where-Object { $_ }
            $current.RequiredFeatures = @($features)
        }
    }

    if ($current -and $current.Name) {
        $targets.Add([PSCustomObject]$current)
    }

    $targets
}

function Get-ScopeTargets {
    param(
        [object[]]$Targets,
        [string]$Scope,
        [string]$TargetFilter
    )

    $filtered = switch ($Scope) {
        "general" { $Targets | Where-Object { $_.Name -like "general_*" } }
        "modules" { $Targets | Where-Object { $_.Name -like "module_*" } }
        "special" { $Targets | Where-Object { $_.Name -like "special_*" } }
        "all" { $Targets | Where-Object { $_.Name -match "^(general|module|special)_" } }
        default { $Targets | Where-Object { $_.Name -match "^(general|module)_" } }
    }

    if ($TargetFilter) {
        $filtered = $filtered | Where-Object { $_.Name -like "*$TargetFilter*" }
    }

    $filtered | Sort-Object Name
}

function Resolve-Runner {
    param([string]$Requested)

    if ($Requested -eq "cargo" -or $Requested -eq "nextest") {
        return $Requested
    }

    $nextest = Get-Command cargo-nextest -ErrorAction SilentlyContinue
    if (-not $nextest) {
        $cargoHelp = & cargo nextest --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            return "nextest"
        }
    }

    Write-Host "cargo-nextest 未安装，自动回退到 cargo test" -ForegroundColor Yellow
    return "cargo"
}

function Ensure-AliasShim {
    $args = @("build", "-p", "alias-shim", "--profile", "release-shim")
    Write-Host "Running: cargo $($args -join ' ')" -ForegroundColor Cyan
    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to build alias-shim prerequisite."
    }
}

function Invoke-Target {
    param(
        [string]$Runner,
        [string]$Profile,
        [object]$Target,
        [bool]$RunIgnored
    )

    $featureArgs = @()
    if ($Target.RequiredFeatures.Count -gt 0) {
        $featureArgs = @("--features", ($Target.RequiredFeatures -join ","))
    }

    switch ($Runner) {
        "cargo" {
            $args = @("test", "--test", $Target.Name) + $featureArgs
            $args += @("--", "--nocapture")
            if ($RunIgnored) {
                $args += "--ignored"
            }
        }
        "nextest" {
            $args = @("nextest", "run", "--profile", $Profile, "--test", $Target.Name) + $featureArgs
            if ($RunIgnored) {
                $args += @("--run-ignored", "all")
            }
        }
    }

    Write-Host ""
    Write-Host "== $($Target.Name) ==" -ForegroundColor Cyan
    if ($Target.RequiredFeatures.Count -gt 0) {
        Write-Host "features: $($Target.RequiredFeatures -join ',')" -ForegroundColor DarkGray
    }
    Write-Host "Running: cargo $($args -join ' ')" -ForegroundColor Cyan

    & cargo @args 2>&1 | Out-Host
    return [int]$LASTEXITCODE
}

$targets = @(Get-ScopeTargets -Targets (Get-TestTargets) -Scope $Scope -TargetFilter $TargetFilter)
if (-not $targets) {
    throw "No test targets matched scope=$Scope filter=$TargetFilter"
}

$resolvedRunner = Resolve-Runner -Requested $Runner
$runIgnored = $Ignored.IsPresent
if ($Scope -eq "special" -and -not $PSBoundParameters.ContainsKey("Ignored")) {
    $runIgnored = $true
}

Write-Host "Scope   : $Scope" -ForegroundColor Green
Write-Host "Runner  : $resolvedRunner" -ForegroundColor Green
Write-Host "Profile : $Profile" -ForegroundColor Green
Write-Host "Ignored : $runIgnored" -ForegroundColor Green
Write-Host "Targets : $($targets.Count)" -ForegroundColor Green
foreach ($target in $targets) {
    $features = if ($target.RequiredFeatures.Count -gt 0) {
        " [" + ($target.RequiredFeatures -join ",") + "]"
    } else {
        ""
    }
    Write-Host "  - $($target.Name)$features"
}

if ($ListOnly) {
    exit 0
}

if ($targets.Name -contains "module_alias" -or $targets.Name -contains "special_alias_perf") {
    Ensure-AliasShim
}

$failures = New-Object System.Collections.Generic.List[string]
foreach ($target in $targets) {
    $exitCode = Invoke-Target -Runner $resolvedRunner -Profile $Profile -Target $target -RunIgnored:$runIgnored
    if ($exitCode -ne 0) {
        $failures.Add($target.Name)
        if (-not $KeepGoing) {
            break
        }
    }
}

if ($failures.Count -gt 0) {
    Write-Host ""
    Write-Host "Failed targets:" -ForegroundColor Red
    foreach ($failure in $failures) {
        Write-Host "  - $failure" -ForegroundColor Red
    }
    exit 1
}

Write-Host ""
Write-Host "All selected targets passed." -ForegroundColor Green
exit 0
