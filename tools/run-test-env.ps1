# Run Xun commands inside the prepared test environment.
[CmdletBinding()]
param(
    [string]$EnvScript = "",
    [switch]$SkipDashboard
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
if (-not $EnvScript) {
    $EnvScript = Join-Path $repo "target\xun-cli-env\enter.ps1"
}

if (-not (Test-Path $EnvScript)) {
    Write-Error "Env script not found: $EnvScript. Run tools\\test-env.ps1 first."
    exit 1
}

. $EnvScript
$env:XUN_NON_INTERACTIVE = "1"
$exe = $env:XUN_EXE

Write-Output "=== basic"
& $exe --version
& $exe --help > $null
& $exe init powershell > $null
& $exe completion powershell > $null
& $exe __complete z ""

Write-Output "=== config"
& $exe config get tree.defaultDepth
& $exe config set tree.defaultDepth 4
& $exe config get tree.defaultDepth
& $exe config set tree.defaultDepth 3

Write-Output "=== bookmarks"
& $exe list
& $exe keys
& $exe all
& $exe fuzzy proj
& $exe z project
& $exe sv demo_save
& $exe set demo "$env:USERPROFILE\project"
& $exe tag add demo tag1
& $exe tag list
& $exe tag remove demo tag1
& $exe del demo
& $exe del demo_save
& $exe touch project
& $exe rename project project2
& $exe rename project2 project
& $exe gc
& $exe check
& $exe stats
& $exe recent
& $exe dedup --yes

Write-Output "=== export/import"
& $exe export -f json -o "$env:USERPROFILE\out\export.json"
& $exe import -f json -i "$env:USERPROFILE\import\bookmarks.json" -m merge --yes

Write-Output "=== tree"
& $exe tree "$env:USERPROFILE\tree" --size

Write-Output "=== redirect"
& $exe redirect "$env:USERPROFILE\redirect-src" --profile default --dry-run
& $exe redirect "$env:USERPROFILE\redirect-src" --profile default --confirm --yes
& $exe redirect --log --last 5

Write-Output "=== proxy"
& $exe proxy detect
& $exe proxy get
& $exe proxy set http://127.0.0.1:7890 -o git,cargo
& $exe proxy del -o git,cargo
& $exe pst
& $exe pon --no-test
& $exe poff

Write-Output "=== ports"
& $exe ports --all

Write-Output "=== bak"
& $exe bak -C "$env:USERPROFILE\bak-src" -m demo -y
$bakRoot = Join-Path "$env:USERPROFILE\bak-src" "A_backups"
$bakName = (Get-ChildItem $bakRoot | Sort-Object LastWriteTime -Descending | Select-Object -First 1).Name
& $exe bak list -C "$env:USERPROFILE\bak-src"
& $exe bak restore $bakName -C "$env:USERPROFILE\bak-src" -y --dry-run

Write-Output "=== protect"
& $exe protect set "$env:USERPROFILE\protect\protected.txt" --deny delete
& $exe protect status
& $exe protect clear "$env:USERPROFILE\protect\protected.txt"

Write-Output "=== crypt"
try { & $exe encrypt "$env:USERPROFILE\crypt\secret.txt" --efs } catch { Write-Output $_.Exception.Message }
try { & $exe decrypt "$env:USERPROFILE\crypt\secret.txt" --efs } catch { Write-Output $_.Exception.Message }

if (-not $SkipDashboard) {
    Write-Output "=== dashboard"
    $proc = Start-Process -FilePath $exe -ArgumentList @("serve","--port","9527") -PassThru
    Start-Sleep -Seconds 1
    Stop-Process -Id $proc.Id -Force
}

Write-Output "=== done"
